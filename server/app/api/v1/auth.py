import csv
import hashlib
import io
import logging
from datetime import UTC, timedelta

import jwt
from fastapi import APIRouter, Depends, Request, Response, status
from fastapi.responses import JSONResponse
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db, utcnow
from app.deps import current_user
from app.errors import ApiError, ErrorCode, bad_request, conflict, not_found, unauthorized
from app.ids import new_id, new_tag
from app.models import AuthToken, SecurityAudit, Session, User, UserSettings
from app.realtime.bus import bus
from app.schemas.auth import (
    EmailVerifyIn,
    ForgotPasswordIn,
    LoginIn,
    PasswordChangeIn,
    PasswordResetIn,
    RefreshIn,
    RegisterIn,
    TokenOut,
    TwoFactorCodeIn,
)
from app.security import (
    create_access_token,
    create_refresh_token,
    decode_token,
    hash_password,
    verify_password,
)
from app.services import identity, mailer, ratelimit

router = APIRouter(prefix="/auth", tags=["auth"])
log = logging.getLogger("server.auth")


def _digest(token: str) -> str:
    return hashlib.sha256(token.encode()).hexdigest()


def _issue_tokens(db: AsyncSession, user: User, device: str = "", ip: str = "") -> TokenOut:
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


async def _create_token(db: AsyncSession, user: User, kind: str, ttl_minutes: int) -> str:
    token = new_id()
    db.add(
        AuthToken(
            id=new_id(),
            user_id=user.id,
            kind=kind,
            code_hash=_digest(token),
            expires_at=utcnow() + timedelta(minutes=ttl_minutes),
        )
    )
    return token


async def _consume_token(db: AsyncSession, token: str, kind: str) -> User:
    row = await db.scalar(
        select(AuthToken).where(AuthToken.code_hash == _digest(token), AuthToken.kind == kind)
    )
    if row is None or row.used:
        raise bad_request("凭证无效或已使用", "token")
    expires_at = row.expires_at if row.expires_at.tzinfo else row.expires_at.replace(tzinfo=UTC)
    if expires_at < utcnow():
        raise bad_request("凭证已过期，请重新获取", "token")
    user = await db.get(User, row.user_id)
    if user is None:
        raise bad_request("账号不存在", "token")
    row.used = True
    return user


async def _revoke_all(db: AsyncSession, user_id: str) -> None:
    for sess in (await db.scalars(select(Session).where(Session.user_id == user_id))).all():
        sess.revoked = True


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
    await db.flush()
    token = await _create_token(db, user, "verify_email", settings.email_token_ttl_minutes)
    tokens = _issue_tokens(db, user, request.headers.get("user-agent", ""), _client_ip(request))
    await db.commit()

    mailer.send_verification(user.email, token)
    await identity.audit(
        "account_registered",
        user=user,
        ip=_client_ip(request),
        device=request.headers.get("user-agent", ""),
    )
    return tokens


@router.post("/login", response_model=TokenOut)
async def login(body: LoginIn, request: Request, db: AsyncSession = Depends(get_db)) -> TokenOut:
    ip = _client_ip(request)
    device = request.headers.get("user-agent", "")
    ratelimit.check(f"login:{ip}", settings.rate_limit_logins, settings.rate_limit_login_window_seconds)

    user = await db.scalar(select(User).where(User.email == body.email.lower()))
    if user is None or not verify_password(body.password, user.password_hash):
        log.info("login failed for %s (user_exists=%s)", body.email.lower(), user is not None)
        await identity.audit(
            "login_failed", user=user, email=body.email.lower(), success=False, ip=ip, device=device
        )
        raise unauthorized("邮箱或密码不正确")

    if user.totp_enabled:
        code = (body.totp_code or "").strip()
        if not code:
            raise ApiError(401, "MFA_REQUIRED", "需要双因素验证码", "totp_code")
        if not identity.verify_totp(user.totp_secret or "", code) and not identity.consume_recovery_code(user, code):
            await identity.audit("mfa_failed", user=user, success=False, ip=ip, device=device)
            raise ApiError(401, "MFA_INVALID", "双因素验证码不正确", "totp_code")
        # 恢复码被消耗后需要落库
        await db.commit()

    tokens = _issue_tokens(db, user, device, ip)
    await db.commit()
    await identity.audit("login_success", user=user, ip=ip, device=device)
    return tokens


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
    tokens = _issue_tokens(db, user, session.device, session.ip)
    await db.commit()
    return tokens


@router.post("/logout", status_code=status.HTTP_204_NO_CONTENT)
async def logout(body: RefreshIn, db: AsyncSession = Depends(get_db)) -> None:
    session = await db.scalar(select(Session).where(Session.refresh_hash == _digest(body.refresh_token)))
    if session is not None:
        session.revoked = True
        await db.commit()


@router.post("/logout-all", status_code=status.HTTP_204_NO_CONTENT)
async def logout_all(
    request: Request, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> None:
    await _revoke_all(db, user.id)
    await db.commit()
    await bus.disconnect_user(user.id)
    await identity.audit(
        "logout_all", user=user, ip=_client_ip(request), device=request.headers.get("user-agent", "")
    )


@router.post("/password", status_code=status.HTTP_204_NO_CONTENT)
async def change_password(
    body: PasswordChangeIn,
    request: Request,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    if not verify_password(body.old_password, user.password_hash):
        raise bad_request("原密码不正确", "old_password")
    user.password_hash = hash_password(body.new_password)
    await _revoke_all(db, user.id)
    await db.commit()
    await bus.disconnect_user(user.id)
    await identity.audit(
        "password_changed", user=user, ip=_client_ip(request), device=request.headers.get("user-agent", "")
    )


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
    request: Request,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    session = await db.get(Session, session_id)
    if session is None or session.user_id != user.id:
        raise not_found("会话")
    session.revoked = True
    await db.commit()
    await bus.disconnect_user(user.id)
    await identity.audit(
        "session_revoked", user=user, ip=_client_ip(request), detail=session_id
    )


@router.post("/verify-email/send", status_code=status.HTTP_202_ACCEPTED)
async def send_verification(
    db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    if user.email_verified:
        return {"sent": False, "verified": True}
    token = await _create_token(db, user, "verify_email", settings.email_token_ttl_minutes)
    await db.commit()
    mailer.send_verification(user.email, token)
    return {"sent": True, "verified": False}


@router.post("/verify-email", status_code=status.HTTP_202_ACCEPTED)
async def verify_email(
    request: Request,
    body: EmailVerifyIn | None = None,
    token: str = "",
    db: AsyncSession = Depends(get_db),
) -> dict:
    raw = (body.token if body is not None else "") or token
    # 重复确认幂等：已用凭证但账号已验证时直接成功
    existing = await db.scalar(select(AuthToken).where(AuthToken.code_hash == _digest(raw)))
    if existing is not None:
        verified_user = await db.get(User, existing.user_id)
        if verified_user is not None and verified_user.email_verified:
            return {"verified": True}
    user = await _consume_token(db, raw, "verify_email")
    user.email_verified = True
    await db.commit()
    await identity.audit("email_verified", user=user, ip=_client_ip(request))
    return {"verified": True}


@router.post("/password/forgot", status_code=status.HTTP_202_ACCEPTED)
async def forgot_password(
    body: ForgotPasswordIn | None = None,
    email: str = "",
    db: AsyncSession = Depends(get_db),
) -> dict:
    raw = (body.email if body is not None else "") or email
    user = await db.scalar(select(User).where(User.email == raw.lower()))
    if user is None:
        return {"sent": True}  # 不泄露邮箱是否存在
    token = await _create_token(db, user, "reset_password", settings.email_token_ttl_minutes)
    await db.commit()
    mailer.send_password_reset(user.email, token)
    return {"sent": True}


@router.post("/password/reset", status_code=status.HTTP_204_NO_CONTENT)
async def reset_password(
    body: PasswordResetIn,
    request: Request,
    db: AsyncSession = Depends(get_db),
) -> None:
    user = await _consume_token(db, body.token, "reset_password")
    user.password_hash = hash_password(body.new_password)
    await _revoke_all(db, user.id)
    await db.commit()
    await bus.disconnect_user(user.id)
    await identity.audit(
        "password_reset", user=user, ip=_client_ip(request), device=request.headers.get("user-agent", "")
    )


@router.post("/2fa/setup")
async def setup_two_factor(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    if user.totp_enabled:
        raise conflict("双因素认证已启用", "totp")
    secret = identity.generate_totp_secret()
    user.totp_secret = secret
    await db.commit()
    return {"secret": secret, "otpauth_url": identity.provisioning_uri(secret, user.email)}


@router.post("/2fa/enable")
async def enable_two_factor(
    body: TwoFactorCodeIn,
    request: Request,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if user.totp_enabled:
        raise conflict("双因素认证已启用", "totp")
    if not identity.verify_totp(user.totp_secret or "", body.code):
        raise bad_request("验证码不正确", "code")
    plain, hashed = identity.generate_recovery_codes()
    user.totp_enabled = True
    user.recovery_codes = hashed
    await db.commit()
    await identity.audit(
        "mfa_enabled", user=user, ip=_client_ip(request), device=request.headers.get("user-agent", "")
    )
    # 恢复码只返回一次
    return {"enabled": True, "recovery_codes": plain}


@router.post("/2fa/disable", status_code=status.HTTP_204_NO_CONTENT)
async def disable_two_factor(
    body: TwoFactorCodeIn,
    request: Request,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    if not user.totp_enabled:
        raise bad_request("双因素认证未启用", "totp")
    if not identity.verify_totp(user.totp_secret or "", body.code) and not identity.consume_recovery_code(
        user, body.code
    ):
        raise bad_request("验证码不正确", "code")
    user.totp_enabled = False
    user.totp_secret = None
    user.recovery_codes = None
    await db.commit()
    await identity.audit(
        "mfa_disabled", user=user, ip=_client_ip(request), device=request.headers.get("user-agent", "")
    )


@router.get("/security-audit")
async def security_audit(
    format: str = "json",
    limit: int = 100,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> Response:
    limit = max(1, min(limit, 500))
    rows = (
        await db.scalars(
            select(SecurityAudit)
            .where(SecurityAudit.user_id == user.id)
            .order_by(SecurityAudit.created_at.desc())
            .limit(limit)
        )
    ).all()
    items = [
        {
            "id": row.id,
            "event": row.event,
            "success": row.success,
            "ip": row.ip,
            "device": row.device,
            "detail": row.detail,
            "created_at": row.created_at.isoformat().replace("+00:00", "Z"),
        }
        for row in rows
    ]
    if format != "csv":
        return JSONResponse(content=items)

    buffer = io.StringIO()
    writer = csv.DictWriter(buffer, fieldnames=["id", "event", "success", "ip", "device", "detail", "created_at"])
    writer.writeheader()
    writer.writerows(items)
    return Response(
        content=buffer.getvalue(),
        media_type="text/csv",
        headers={"Content-Disposition": 'attachment; filename="security-audit.csv"'},
    )


def _client_ip(request: Request) -> str:
    fwd = request.headers.get("x-forwarded-for")
    if fwd:
        return fwd.split(",")[0].strip()
    return request.client.host if request.client else ""
