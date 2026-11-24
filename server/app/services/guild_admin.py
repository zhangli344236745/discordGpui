"""成员、角色层级与封禁的共用规则。"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

from sqlalchemy import select, update
from sqlalchemy.ext.asyncio import AsyncSession

from app.errors import ApiError, ErrorCode, not_found
from app.models import Ban, Channel, MemberRole, Message, Role, ServerMember, User
from app.services.permissions import assert_can_act

OWNER_POSITION = 1_000_000


def iso(value: datetime | None) -> str | None:
    return value.isoformat().replace("+00:00", "Z") if value else None


def effective_status(user: User) -> str:
    return "offline" if user.status == "invisible" else user.status


async def members_payload(db: AsyncSession, server_id: str, owner_id: str) -> list[dict]:
    rows = (
        await db.execute(
            select(ServerMember, User)
            .join(User, User.id == ServerMember.user_id)
            .where(ServerMember.server_id == server_id)
            .order_by(User.display_name)
        )
    ).all()
    role_rows = (
        await db.execute(
            select(MemberRole.user_id, MemberRole.role_id, Role.position)
            .join(Role, Role.id == MemberRole.role_id)
            .where(MemberRole.server_id == server_id)
            .order_by(Role.position.desc())
        )
    ).all()
    roles_by_user: dict[str, list[str]] = {}
    for user_id, role_id, _position in role_rows:
        roles_by_user.setdefault(user_id, []).append(role_id)
    return [
        {
            "user_id": user.id,
            "username": user.username,
            "tag": user.tag,
            "display_name": user.display_name,
            "nickname": member.nickname,
            "avatar_color": user.avatar_color,
            "status": effective_status(user),
            "role_ids": roles_by_user.get(user.id, []),
            "joined_at": iso(member.joined_at),
            "is_owner": user.id == owner_id,
        }
        for member, user in rows
    ]


async def get_member(db: AsyncSession, server_id: str, user_id: str) -> ServerMember:
    member = await db.get(ServerMember, (server_id, user_id))
    if member is None:
        raise not_found("成员")
    return member


async def member_top_position(db: AsyncSession, server_id: str, user_id: str, owner_id: str) -> int:
    if user_id == owner_id:
        return OWNER_POSITION
    role_ids = list(
        (
            await db.scalars(
                select(MemberRole.role_id).where(MemberRole.server_id == server_id, MemberRole.user_id == user_id)
            )
        ).all()
    )
    if not role_ids:
        return 0
    positions = list((await db.scalars(select(Role.position).where(Role.id.in_(role_ids)))).all())
    return max(positions, default=0)


async def get_role(db: AsyncSession, role_id: str) -> Role:
    role = await db.get(Role, role_id)
    if role is None:
        raise not_found("角色")
    return role


def assert_role_manageable(actor_position: int, role: Role) -> None:
    if role.position >= actor_position:
        raise ApiError(403, ErrorCode.FORBIDDEN, "无法操作不低于自己最高角色的角色", field="position")


async def is_banned(db: AsyncSession, server_id: str, user_id: str) -> bool:
    return await db.get(Ban, (server_id, user_id)) is not None


async def soft_delete_recent_messages(
    db: AsyncSession, server_id: str, author_id: str, days: int, now: datetime | None = None
) -> int:
    if days <= 0:
        return 0
    now = now or datetime.now(UTC)
    cutoff = now - timedelta(days=days)
    channel_ids = list((await db.scalars(select(Channel.id).where(Channel.server_id == server_id))).all())
    if not channel_ids:
        return 0
    result = await db.execute(
        update(Message)
        .where(
            Message.channel_id.in_(channel_ids),
            Message.author_id == author_id,
            Message.deleted_at.is_(None),
            Message.created_at >= cutoff,
        )
        .values(deleted_at=now, version=Message.version + 1)
    )
    return result.rowcount or 0


__all__ = [
    "OWNER_POSITION",
    "assert_can_act",
    "assert_role_manageable",
    "effective_status",
    "get_member",
    "get_role",
    "is_banned",
    "iso",
    "member_top_position",
    "members_payload",
    "soft_delete_recent_messages",
]
