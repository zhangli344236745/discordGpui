"""邀请创建、预览、撤销与加入。"""

from datetime import UTC, datetime, timedelta

from fastapi import APIRouter, Depends, status
from sqlalchemy import func, or_, select, update
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ChannelContext, ServerContext, channel_context, current_user, server_context
from app.errors import ApiError, ErrorCode, forbidden, not_found
from app.ids import new_code
from app.models import (
    Invite,
    MemberRole,
    Role,
    Server,
    ServerMember,
    User,
    WelcomeAccept,
    WelcomeRecommended,
    WelcomeRule,
    WelcomeScreen,
)
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.community import InviteCreate
from app.schemas.guild import ServerOut
from app.services import audit, guild_admin
from app.services.permissions import effective_permissions

router = APIRouter(tags=["invites"])

EXPIRY_MAP = {"1h": 3600, "1d": 86400, "7d": 7 * 86400, "never": None}


def _as_utc(value: datetime | None) -> datetime | None:
    if value is None:
        return None
    return value.replace(tzinfo=UTC) if value.tzinfo is None else value


def _state(invite: Invite, now: datetime) -> dict:
    expires_at = _as_utc(invite.expires_at)
    expired = expires_at is not None and expires_at <= now
    exhausted = invite.max_uses is not None and invite.uses >= invite.max_uses
    return {"expired": expired, "exhausted": exhausted, "revoked": invite.revoked}


def _invite_out(invite: Invite, now: datetime) -> dict:
    state = _state(invite, now)
    return {
        "code": invite.code,
        "server_id": invite.server_id,
        "channel_id": invite.channel_id,
        "created_by": invite.created_by,
        "max_uses": invite.max_uses,
        "uses": invite.uses,
        "expires_at": guild_admin.iso(invite.expires_at),
        "revoked": invite.revoked,
        "valid": not any(state.values()),
        "created_at": guild_admin.iso(invite.created_at),
        **state,
    }


async def _welcome_payload(db: AsyncSession, server_id: str, user_id: str) -> dict:
    welcome = await db.get(WelcomeScreen, server_id)
    accepted = await db.get(WelcomeAccept, (server_id, user_id))
    rules = (
        await db.scalars(
            select(WelcomeRule).where(WelcomeRule.server_id == server_id).order_by(WelcomeRule.position)
        )
    ).all()
    recommended = (
        await db.scalars(select(WelcomeRecommended.channel_id).where(WelcomeRecommended.server_id == server_id))
    ).all()
    return {
        "welcome": None
        if welcome is None
        else {
            "headline": welcome.headline,
            "description": welcome.description,
            "require_rules": welcome.require_rules,
            "rules": [{"title": r.title, "body": r.body} for r in rules],
            "recommended": list(recommended),
            "accepted": accepted is not None,
        }
    }


async def _unique_code(db: AsyncSession) -> str:
    for _ in range(10):
        code = new_code()
        if await db.scalar(select(Invite.code).where(Invite.code == code)) is None:
            return code
    raise ApiError(409, ErrorCode.CONFLICT, "邀请码生成失败，请重试", "code")


@router.post("/channels/{channel_id}/invites", status_code=status.HTTP_201_CREATED)
async def create_invite(
    body: InviteCreate,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("CREATE_INVITE")
    now = datetime.now(UTC)
    seconds = EXPIRY_MAP[body.expiry]
    invite = Invite(
        code=await _unique_code(db),
        server_id=ctx.server.id,
        channel_id=ctx.channel.id,
        created_by=ctx.user.id,
        max_uses=body.max_uses,
        uses=0,
        expires_at=None if seconds is None else now + timedelta(seconds=seconds),
        revoked=False,
    )
    db.add(invite)
    await db.flush()
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_INVITE, "创建了邀请")
    payload = _invite_out(invite, now)
    outbox.queue(db, "server", ctx.server.id, Event.INVITE_CREATE, payload)
    await db.commit()
    return payload


@router.get("/servers/{server_id}/invites")
async def list_invites(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> list[dict]:
    ctx.require("MANAGE_SERVER")
    now = datetime.now(UTC)
    rows = (
        await db.scalars(
            select(Invite).where(Invite.server_id == ctx.server.id).order_by(Invite.created_at.desc())
        )
    ).all()
    return [_invite_out(row, now) for row in rows]


@router.delete("/invites/{code}")
async def revoke_invite(
    code: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    invite = await db.get(Invite, code)
    if invite is None:
        raise not_found("邀请")
    if invite.created_by != user.id:
        member = await db.get(ServerMember, (invite.server_id, user.id))
        if member is None:
            raise not_found("邀请")
        perms = await effective_permissions(db, invite.server_id, user.id)
        if "MANAGE_SERVER" not in perms:
            raise forbidden("MANAGE_SERVER")
    invite.revoked = True
    audit.log(db, invite.server_id, user.id, audit.KIND_INVITE, "撤销了邀请")
    outbox.queue(db, "server", invite.server_id, Event.INVITE_DELETE, {"code": code})
    await db.commit()
    return {"code": code, "revoked": True}


@router.get("/invites/{code}")
async def preview_invite(code: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    invite = await db.get(Invite, code)
    if invite is None:
        raise not_found("邀请")
    server = await db.get(Server, invite.server_id)
    if server is None:
        raise not_found("邀请")
    now = datetime.now(UTC)
    body = _invite_out(invite, now)
    online = await db.scalar(
        select(func.count())
        .select_from(ServerMember)
        .join(User, User.id == ServerMember.user_id)
        .where(
            ServerMember.server_id == server.id,
            User.status.in_(("online", "idle", "dnd")),
        )
    )
    body.update(
        {
            "server": {
                "id": server.id,
                "name": server.name,
                "icon_color": server.icon_color,
                "description": server.description,
                "member_count": server.member_count,
            },
            "online_count": int(online or 0),
            "joined": await db.get(ServerMember, (server.id, user.id)) is not None,
        }
    )
    return body


@router.post("/invites/{code}/accept")
async def accept_invite(
    code: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    invite = await db.get(Invite, code)
    now = datetime.now(UTC)
    if invite is None:
        raise not_found("邀请")
    state = _state(invite, now)
    if any(state.values()):
        raise not_found("邀请")
    if await guild_admin.is_banned(db, invite.server_id, user.id):
        raise forbidden("JOIN_SERVER")

    server = await db.get(Server, invite.server_id)
    if server is None:
        raise not_found("邀请")

    member = await db.get(ServerMember, (server.id, user.id))
    if member is None:
        # SQLite 存的是 naive UTC，比较参数也去掉 tz，避免 ORM 同步求值报混用
        now_sql = now.replace(tzinfo=None)
        result = await db.execute(
            update(Invite)
            .where(
                Invite.code == code,
                Invite.revoked.is_(False),
                or_(Invite.max_uses.is_(None), Invite.uses < Invite.max_uses),
                or_(Invite.expires_at.is_(None), Invite.expires_at > now_sql),
            )
            .values(uses=Invite.uses + 1)
        )
        if (result.rowcount or 0) == 0:
            raise not_found("邀请")
        db.add(ServerMember(server_id=server.id, user_id=user.id, nickname=""))
        everyone = await db.scalar(
            select(Role).where(Role.server_id == server.id, Role.is_everyone.is_(True))
        )
        if everyone is not None:
            db.add(MemberRole(server_id=server.id, user_id=user.id, role_id=everyone.id))
        server.member_count += 1
        audit.log(db, server.id, user.id, audit.KIND_MEMBER, "通过邀请加入了服务器")
        outbox.queue(db, "server", server.id, Event.MEMBER_JOIN, {"server_id": server.id, "user_id": user.id})
        await db.commit()
        await db.refresh(invite)
    else:
        await db.refresh(invite)

    welcome = await _welcome_payload(db, server.id, user.id)
    return {
        "server": ServerOut.model_validate(server).model_dump(),
        "invite": _invite_out(invite, now),
        **welcome,
    }
