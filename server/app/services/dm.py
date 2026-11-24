"""私信与消息请求的数据访问与序列化。"""

from __future__ import annotations

from datetime import datetime

from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.errors import not_found
from app.ids import new_id
from app.models import DmChannel, DmMessage, Draft, MessageRequest, User


def iso(value: datetime | None) -> str | None:
    return value.isoformat().replace("+00:00", "Z") if value else None


async def require_dm(db: AsyncSession, dm_id: str, user_id: str) -> DmChannel:
    dm = await db.get(DmChannel, dm_id)
    if dm is None or user_id not in (dm.user_a, dm.user_b):
        raise not_found("会话")
    return dm


def partner_id(dm: DmChannel, viewer_id: str) -> str:
    return dm.user_b if dm.user_a == viewer_id else dm.user_a


def last_read_of(dm: DmChannel, viewer_id: str) -> str | None:
    return dm.last_read_a if dm.user_a == viewer_id else dm.last_read_b


def pinned_of(dm: DmChannel, viewer_id: str) -> bool:
    return dm.pinned_a if dm.user_a == viewer_id else dm.pinned_b


def set_last_read(dm: DmChannel, viewer_id: str, message_id: str | None) -> None:
    if dm.user_a == viewer_id:
        dm.last_read_a = message_id
    else:
        dm.last_read_b = message_id


def set_pinned(dm: DmChannel, viewer_id: str, value: bool) -> None:
    if dm.user_a == viewer_id:
        dm.pinned_a = value
    else:
        dm.pinned_b = value


async def create_or_get_dm(db: AsyncSession, a: str, b: str) -> tuple[DmChannel, bool]:
    low, high = sorted((a, b))
    dm = await db.scalar(select(DmChannel).where(DmChannel.user_a == low, DmChannel.user_b == high))
    if dm is not None:
        if dm.closed:
            dm.closed = False
        return dm, False
    dm = DmChannel(id=new_id(), user_a=low, user_b=high)
    db.add(dm)
    await db.flush()
    return dm, True


async def unread_count(db: AsyncSession, dm: DmChannel, viewer_id: str) -> int:
    stmt = select(func.count()).select_from(DmMessage).where(
        DmMessage.dm_id == dm.id,
        DmMessage.author_id != viewer_id,
        DmMessage.deleted_at.is_(None),
    )
    last_read = last_read_of(dm, viewer_id)
    if last_read:
        stmt = stmt.where(DmMessage.id > last_read)
    return int(await db.scalar(stmt) or 0)


async def message_out(db: AsyncSession, msg: DmMessage) -> dict:
    author = await db.get(User, msg.author_id)
    return {
        "id": msg.id,
        "dm_id": msg.dm_id,
        "author": {
            "id": msg.author_id,
            "name": author.display_name if author else "已注销用户",
            "color": author.avatar_color if author else 0x99AAB5,
        },
        "content": msg.content,
        "type": msg.msg_type,
        "payload": msg.payload,
        "reply_to": msg.reply_to,
        "edited_at": iso(msg.edited_at),
        "created_at": iso(msg.created_at),
    }


async def messages_out(db: AsyncSession, msgs: list[DmMessage]) -> list[dict]:
    if not msgs:
        return []
    author_ids = {m.author_id for m in msgs}
    users = {u.id: u for u in (await db.scalars(select(User).where(User.id.in_(author_ids)))).all()}
    out: list[dict] = []
    for msg in msgs:
        author = users.get(msg.author_id)
        out.append(
            {
                "id": msg.id,
                "dm_id": msg.dm_id,
                "author": {
                    "id": msg.author_id,
                    "name": author.display_name if author else "已注销用户",
                    "color": author.avatar_color if author else 0x99AAB5,
                },
                "content": msg.content,
                "type": msg.msg_type,
                "payload": msg.payload,
                "reply_to": msg.reply_to,
                "edited_at": iso(msg.edited_at),
                "created_at": iso(msg.created_at),
            }
        )
    return out


async def summary(db: AsyncSession, dm: DmChannel, viewer_id: str) -> dict:
    partner = await db.get(User, partner_id(dm, viewer_id))
    last_message = await db.get(DmMessage, dm.last_message_id) if dm.last_message_id else None
    draft = await db.scalar(
        select(func.count())
        .select_from(Draft)
        .where(Draft.user_id == viewer_id, Draft.scope == f"dm:{dm.id}")
    )
    return {
        "id": dm.id,
        "user": {
            "id": partner.id if partner else "",
            "username": partner.username if partner else "",
            "tag": partner.tag if partner else "",
            "display_name": partner.display_name if partner else "已注销用户",
            "avatar_color": partner.avatar_color if partner else 0x99AAB5,
            "status": "offline" if (partner is None or partner.status == "invisible") else partner.status,
        },
        "last_message": {
            "id": last_message.id,
            "author_id": last_message.author_id,
            "content": last_message.content,
            "type": last_message.msg_type,
            "created_at": iso(last_message.created_at),
        }
        if last_message and last_message.deleted_at is None
        else None,
        "unread": await unread_count(db, dm, viewer_id),
        "pinned": pinned_of(dm, viewer_id),
        "closed": dm.closed,
        "has_draft": bool(draft),
        "created_at": iso(dm.created_at),
    }


def request_out(row: MessageRequest, from_user: User, to_user: User, direction: str = "incoming") -> dict:
    return {
        "id": row.id,
        "from_user": {
            "id": from_user.id,
            "username": from_user.username,
            "tag": from_user.tag,
            "display_name": from_user.display_name,
            "avatar_color": from_user.avatar_color,
            "status": "offline" if from_user.status == "invisible" else from_user.status,
        },
        "to_user": {
            "id": to_user.id,
            "username": to_user.username,
            "tag": to_user.tag,
            "display_name": to_user.display_name,
            "avatar_color": to_user.avatar_color,
            "status": "offline" if to_user.status == "invisible" else to_user.status,
        },
        "state": row.state,
        "first_message": row.first_message,
        "direction": direction,
        "created_at": iso(row.created_at),
    }


async def dm_participants(db: AsyncSession, dm: DmChannel, viewer_id: str) -> tuple[User, User]:
    viewer = await db.get(User, viewer_id)
    partner = await db.get(User, partner_id(dm, viewer_id))
    if viewer is None or partner is None:
        raise not_found("用户")
    return viewer, partner


__all__ = [
    "create_or_get_dm",
    "dm_participants",
    "iso",
    "last_read_of",
    "message_out",
    "messages_out",
    "partner_id",
    "pinned_of",
    "request_out",
    "require_dm",
    "set_last_read",
    "set_pinned",
    "summary",
    "unread_count",
]
