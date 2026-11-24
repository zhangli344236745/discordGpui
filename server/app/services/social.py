"""社交关系规则：好友、屏蔽、共同服务器与私信门控。"""

from __future__ import annotations

from datetime import datetime

from sqlalchemy import func, or_, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.errors import not_found
from app.models import Block, DmChannel, Friendship, ServerMember, User


def effective_status(user: User) -> str:
    """隐身对外表现为离线，但本人仍看到真实状态。"""
    return "offline" if user.status == "invisible" else user.status


def public_user(user: User) -> dict:
    return {
        "id": user.id,
        "username": user.username,
        "tag": user.tag,
        "display_name": user.display_name,
        "avatar_color": user.avatar_color,
        "about": user.about,
        "status": effective_status(user),
    }


def friend_out(user: User, group_id: str | None, created_at: datetime) -> dict:
    return {
        "id": user.id,
        "username": user.username,
        "tag": user.tag,
        "display_name": user.display_name,
        "avatar_color": user.avatar_color,
        "status": effective_status(user),
        "online": effective_status(user) != "offline",
        "group_id": group_id,
        "created_at": created_at.isoformat().replace("+00:00", "Z"),
    }


async def find_user(db: AsyncSession, username: str, tag: str) -> User | None:
    return await db.scalar(
        select(User).where(func.lower(User.username) == username.strip().lower(), User.tag == tag.strip())
    )


async def get_user(db: AsyncSession, user_id: str) -> User:
    user = await db.get(User, user_id)
    if user is None:
        raise not_found("用户")
    return user


async def are_friends(db: AsyncSession, a: str, b: str) -> bool:
    row = await db.scalar(select(Friendship).where(Friendship.user_id == a, Friendship.friend_id == b))
    return row is not None


async def blocked(db: AsyncSession, a: str, b: str) -> bool:
    row = await db.scalar(
        select(Block).where(
            or_(
                (Block.owner_id == a) & (Block.target_id == b),
                (Block.owner_id == b) & (Block.target_id == a),
            )
        )
    )
    return row is not None


async def blocked_ids(db: AsyncSession, user_id: str) -> set[str]:
    """我屏蔽的人 + 屏蔽我的人：好友列表与搜索结果都要排除。"""
    rows = (
        await db.scalars(
            select(Block).where(or_(Block.owner_id == user_id, Block.target_id == user_id))
        )
    ).all()
    return {row.target_id if row.owner_id == user_id else row.owner_id for row in rows}


async def shared_server_ids(db: AsyncSession, a: str, b: str) -> set[str]:
    a_servers = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == a))).all())
    b_servers = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == b))).all())
    return a_servers & b_servers


async def existing_dm(db: AsyncSession, a: str, b: str) -> DmChannel | None:
    low, high = sorted((a, b))
    return await db.scalar(select(DmChannel).where(DmChannel.user_a == low, DmChannel.user_b == high))


async def can_dm_directly(db: AsyncSession, a: str, b: str) -> bool:
    if await are_friends(db, a, b):
        return True
    if await existing_dm(db, a, b) is not None:
        return True
    return bool(await shared_server_ids(db, a, b))


async def block_row(db: AsyncSession, owner: str, target: str) -> Block | None:
    return await db.get(Block, (owner, target))
