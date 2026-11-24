import hashlib
from datetime import timedelta

import jwt
from fastapi import APIRouter, Depends, Request, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db, utcnow
from app.deps import current_user
from app.errors import ErrorCode, bad_request, conflict, not_found, unauthorized
from app.ids import new_id, new_tag
from app.models import AuthToken, Session, User, UserSettings
from app.schemas.auth import LoginIn, PasswordChangeIn, RefreshIn, RegisterIn, TokenOut
from app.security import (
    create_access_token,
    create_refresh_token,
    decode_token,
    hash_password,
    verify_password,
)

router = APIRouter(prefix="/auth", tags=["auth"])


def _digest(token: str) -> str:
    return hashlib.sha256(token.encode()).hexdigest()


async def _issue_tokens(db: AsyncSession, user: User, device: str = "", ip: str = "") -> TokenOut:
    refresh = create_refresh_token(user.id)
    db.add(
        Session(
            id=new_id(),
            user_id=user.id,
            refresh_hash=_digest(refresh),
            device=device[:120],
            ip=ip[:64],
            expires_at=utcnow() + timedelta(days=settings.refresh_ttl_days),
        )
    )
    await db.commit()
    return TokenOut(
        access_token=create_access_token(user.id),
        refresh_token=refresh,
        expires_in=settings.access_ttl_minutes * 60,
    )


async def _unique_tag(db: AsyncSession, username: str) -> str:
    for _ in range(10):
        tag = new_tag()
        if await db.scalar(select(User.id).where(User.username == username, User.tag == tag)) is None:
            return tag
    raise conflict("用户名太热门，请换一个", "username")


@router.post("/register", response_model=TokenOut, status_code=status.HTTP_201_CREATED)
async def register(body: RegisterIn, request: Request, db: AsyncSession = Depends(get_db)) -> TokenOut:
    if await db.scalar(select(User.id).where(User.email == body.email.lower())) is not None:
        raise conflict("邮箱已被注册", "email")

    user = User(
        id=new_id(),
        username=body.username,
        tag=await _unique_tag(db, body.username),
        email=body.email.lower(),
        password_hash=hash_password(body.password),
        display_name=(body.display_name or body.username)[:32],
        about="",
    )
    db.add(user)
    db.add(UserSettings(user_id=user.id))
    await db.commit()
    return await _issue_tokens(db, user, request.headers.get("user-agent", ""), _client_ip(request))


@router.post("/login", response_model=TokenOut)
async def login(body: LoginIn, request: Request, db: AsyncSession = Depends(get_db)) -> TokenOut:
    user = await db.scalar(select(User).where(User.email == body.email.lower()))
    if user is None or not verify_password(body.password, user.password_hash):
        raise unauthorized("邮箱或密码不正确")
    return await _issue_tokens(db, user, request.headers.get("user-agent", ""), _client_ip(request))


@router.post("/refresh", response_model=TokenOut)
async def refresh(body: RefreshIn, db: AsyncSession = Depends(get_db)) -> TokenOut:
    try:
        payload = decode_token(body.refresh_token, "refresh")
    except jwt.ExpiredSignatureError:
        raise unauthorized("登录已过期，请重新登录", ErrorCode.TOKEN_EXPIRED) from None
    except jwt.InvalidTokenError:
        raise unauthorized("凭证无效") from None

    session = await db.scalar(
        select(Session).where(
            Session.user_id == payload["sub"],
            Session.refresh_hash == _digest(body.refresh_token),
            Session.revoked.is_(False),
        )
    )
    if session is None:
        raise unauthorized("会话已失效，请重新登录", ErrorCode.TOKEN_EXPIRED)

    user = await db.get(User, payload["sub"])
    if user is None:
        raise unauthorized("账号不存在")

    session.revoked = True  # refresh 轮换，旧的立刻作废
    await db.commit()
    return await _issue_tokens(db, user, session.device, session.ip)


@router.post("/logout", status_code=status.HTTP_204_NO_CONTENT)
async def logout(body: RefreshIn, db: AsyncSession = Depends(get_db)) -> None:
    session = await db.scalar(select(Session).where(Session.refresh_hash == _digest(body.refresh_token)))
    if session is not None:
        session.revoked = True
        await db.commit()


@router.post("/password", status_code=status.HTTP_204_NO_CONTENT)
async def change_password(
    body: PasswordChangeIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    if not verify_password(body.old_password, user.password_hash):
        raise bad_request("原密码不正确", "old_password")
    user.password_hash = hash_password(body.new_password)
    for sess in (await db.scalars(select(Session).where(Session.user_id == user.id))).all():
        sess.revoked = True
    await db.commit()


@router.get("/sessions")
async def list_sessions(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    rows = (
        await db.scalars(
            select(Session).where(Session.user_id == user.id, Session.revoked.is_(False)).order_by(Session.id.desc())
        )
    ).all()
    return [{"id": s.id, "device": s.device, "ip": s.ip, "created_at": s.created_at.isoformat()} for s in rows]


@router.delete("/sessions/{session_id}", status_code=status.HTTP_204_NO_CONTENT)
async def revoke_session(
    session_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    session = await db.get(Session, session_id)
    if session is None or session.user_id != user.id:
        raise not_found("会话")
    session.revoked = True
    await db.commit()


@router.post("/verify-email", status_code=status.HTTP_202_ACCEPTED)
async def verify_email(token: str, db: AsyncSession = Depends(get_db)) -> dict:
    """Demo 不接邮件服务：token 直接按已下发凭证校验。"""
    row = await db.scalar(select(AuthToken).where(AuthToken.code_hash == _digest(token)))
    if row is None or row.used:
        raise bad_request("验证码无效", "token")
    user = await db.get(User, row.user_id)
    if user is not None:
        user.email_verified = True
    row.used = True
    await db.commit()
    return {"verified": True}


@router.post("/password/forgot", status_code=status.HTTP_202_ACCEPTED)
async def forgot_password(email: str, db: AsyncSession = Depends(get_db)) -> dict:
    user = await db.scalar(select(User).where(User.email == email.lower()))
    if user is None:
        return {"sent": True}  # 不泄露邮箱是否存在

    token = new_id()
    db.add(
        AuthToken(
            id=new_id(),
            user_id=user.id,
            kind="reset_password",
            code_hash=_digest(token),
            expires_at=utcnow() + timedelta(minutes=30),
        )
    )
    await db.commit()
    return {"sent": True}


def _client_ip(request: Request) -> str:
    fwd = request.headers.get("x-forwarded-for")
    if fwd:
        return fwd.split(",")[0].strip()
    return request.client.host if request.client else ""
