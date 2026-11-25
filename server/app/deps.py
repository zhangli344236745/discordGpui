import hashlib
from dataclasses import dataclass

import jwt
from fastapi import Depends, Request
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.errors import ErrorCode, forbidden, not_found, unauthorized
from app.models import BotToken, Channel, Server, ServerMember, User
from app.security import decode_token
from app.services.permissions import effective_permissions

bearer = HTTPBearer(auto_error=False)


async def current_user(
    cred: HTTPAuthorizationCredentials | None = Depends(bearer),
    db: AsyncSession = Depends(get_db),
) -> User:
    if cred is None or not cred.credentials:
        raise unauthorized()
    try:
        payload = decode_token(cred.credentials, "access")
    except jwt.ExpiredSignatureError:
        raise unauthorized("登录已过期", ErrorCode.TOKEN_EXPIRED) from None
    except jwt.InvalidTokenError:
        raise unauthorized("凭证无效") from None

    user = await db.get(User, payload["sub"])
    if user is None:
        raise unauthorized("账号不存在")
    return user


async def principal(
    request: Request,
    cred: HTTPAuthorizationCredentials | None = Depends(bearer),
    db: AsyncSession = Depends(get_db),
) -> User:
    """用户 JWT 或 Bot 令牌二选一：Authorization: Bot <token> 走 Bot 通道。"""
    header = request.headers.get("authorization", "")
    if header.lower().startswith("bot "):
        raw = header[4:].strip()
        row = await db.scalar(
            select(BotToken).where(
                BotToken.token_hash == hashlib.sha256(raw.encode()).hexdigest(),
                BotToken.revoked.is_(False),
            )
        )
        if row is None:
            raise unauthorized("Bot 令牌无效")
        user = await db.get(User, row.bot_user_id)
        if user is None:
            raise unauthorized("账号不存在")
        return user
    return await current_user(cred, db)


@dataclass
class ServerContext:
    server: Server
    member: ServerMember
    perms: set[str]
    user: User

    def require(self, permission: str) -> None:
        if permission not in self.perms:
            raise forbidden(permission)

    @property
    def is_owner(self) -> bool:
        return self.server.owner_id == self.user.id


async def server_context(
    server_id: str,
    user: User = Depends(current_user),
    db: AsyncSession = Depends(get_db),
) -> ServerContext:
    server = await db.get(Server, server_id)
    if server is None:
        raise not_found("服务器")
    member = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == server_id, ServerMember.user_id == user.id)
    )
    if member is None:
        # 不区分「不存在」与「无权」，避免枚举
        raise not_found("服务器")
    perms = await effective_permissions(db, server_id, user.id)
    return ServerContext(server=server, member=member, perms=perms, user=user)


@dataclass
class ChannelContext:
    channel: Channel
    server: Server
    member: ServerMember
    perms: set[str]
    user: User

    def require(self, permission: str) -> None:
        if permission not in self.perms:
            raise forbidden(permission)

    @property
    def server_id(self) -> str:
        return self.server.id


async def channel_context(
    channel_id: str,
    user: User = Depends(current_user),
    db: AsyncSession = Depends(get_db),
) -> ChannelContext:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    server = await db.get(Server, channel.server_id)
    if server is None:
        raise not_found("服务器")
    member = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == server.id, ServerMember.user_id == user.id)
    )
    if member is None:
        raise not_found("频道")
    perms = await effective_permissions(db, server.id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    return ChannelContext(channel=channel, server=server, member=member, perms=perms, user=user)


__all__ = [
    "ChannelContext",
    "principal",
    "ServerContext",
    "channel_context",
    "current_user",
    "get_db",
    "server_context",
]
