"""全文搜索：FTS5 trigram + 操作符解析 + 可见性过滤。"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import datetime

from sqlalchemy import bindparam, select, text
from sqlalchemy.ext.asyncio import AsyncSession

from app.errors import not_found
from app.models import Channel, DmChannel, DmMessage, Message, ServerMember, User

OPERATOR_RE = re.compile(r"\b(from|has|after|before):(\S+)", re.IGNORECASE)


@dataclass
class QuerySpec:
    text: str = ""
    author_id: str | None = None
    has: set[str] = field(default_factory=set)
    after: datetime | None = None
    before: datetime | None = None


def parse_query(raw: str) -> QuerySpec:
    spec = QuerySpec()
    for kind, value in OPERATOR_RE.findall(raw):
        kind = kind.lower()
        if kind == "has":
            spec.has.add(value.lower())
        elif kind in ("after", "before"):
            try:
                parsed = datetime.fromisoformat(value)
            except ValueError:
                continue
            if parsed.tzinfo is not None:
                parsed = parsed.replace(tzinfo=None)
            setattr(spec, kind, parsed)
    spec.text = OPERATOR_RE.sub(" ", raw).strip()
    return spec


def escape_fts(value: str) -> str:
    return '"' + value.replace('"', '""') + '"'


def snippet(content: str, needle: str, width: int = 40) -> dict:
    if not needle:
        preview = content[: width * 2]
        return {"preview": preview, "before": preview, "match": "", "after": ""}
    lowered = content.lower()
    index = lowered.find(needle.lower())
    if index < 0:
        preview = content[: width * 2]
        return {"preview": preview, "before": preview, "match": "", "after": ""}
    start = max(0, index - width)
    end = min(len(content), index + len(needle) + width)
    return {
        "preview": content[start:end],
        "before": content[start:index],
        "match": content[index : index + len(needle)],
        "after": content[index + len(needle) : end],
    }


async def resolve_author(db: AsyncSession, handle: str) -> str | None:
    if "#" in handle:
        username, tag = handle.rsplit("#", 1)
        user = await db.scalar(
            select(User).where(User.username == username, User.tag == tag)
        )
        return user.id if user else None
    user = await db.scalar(select(User).where(User.username == handle))
    return user.id if user else None


async def visible_channels(db: AsyncSession, user_id: str) -> dict[str, set[str]]:
    from app.services.permissions import visible_channel_ids

    server_ids = list(
        (await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == user_id))).all()
    )
    return {server_id: await visible_channel_ids(db, server_id, user_id) for server_id in server_ids}


async def _fts_ids(
    db: AsyncSession,
    fts: str,
    table: str,
    column: str,
    scope_ids: set[str],
    q: str,
    before: str | None,
    limit: int,
) -> list[str]:
    if not scope_ids:
        return []
    if len(q) >= 3:
        sql = text(
            f"SELECT m.id FROM {fts} f JOIN {table} m ON m.rowid = f.rowid "
            f"WHERE f.{fts} MATCH :q AND m.deleted_at IS NULL "
            f"AND m.{column} IN :ids"
            + (" AND m.id < :before" if before else "")
            + " ORDER BY m.id DESC LIMIT :cap"
        ).bindparams(bindparam("ids", expanding=True))
        params = {"q": escape_fts(q), "ids": list(scope_ids), "cap": limit}
        if before:
            params["before"] = before
        return [row[0] for row in (await db.execute(sql, params)).all()]

    like = f"%{q}%"
    if table == "messages":
        stmt = select(Message.id).where(
            Message.channel_id.in_(scope_ids),
            Message.deleted_at.is_(None),
            Message.content.like(like),
        )
    else:
        stmt = select(DmMessage.id).where(
            DmMessage.dm_id.in_(scope_ids),
            DmMessage.deleted_at.is_(None),
            DmMessage.content.like(like),
        )
    if before:
        stmt = stmt.where(Message.id < before if table == "messages" else DmMessage.id < before)
    order_column = Message.id if table == "messages" else DmMessage.id
    return list((await db.scalars(stmt.order_by(order_column.desc()).limit(limit))).all())


async def _hydrate_messages(db: AsyncSession, ids: list[str]) -> list[Message]:
    if not ids:
        return []
    rows = {m.id: m for m in (await db.scalars(select(Message).where(Message.id.in_(ids)))).all()}
    return [rows[msg_id] for msg_id in ids if msg_id in rows]


async def _hydrate_dm_messages(db: AsyncSession, ids: list[str]) -> list[DmMessage]:
    if not ids:
        return []
    rows = {m.id: m for m in (await db.scalars(select(DmMessage).where(DmMessage.id.in_(ids)))).all()}
    return [rows[msg_id] for msg_id in ids if msg_id in rows]


def _apply_message_filters(spec: QuerySpec, messages: list, has_columns: bool = True) -> list:
    out = []
    for message in messages:
        if spec.author_id and message.author_id != spec.author_id:
            continue
        if spec.after and message.created_at < spec.after:
            continue
        if spec.before and message.created_at > spec.before:
            continue
        if "poll" in spec.has and message.msg_type != "poll":
            continue
        if "attachment" in spec.has and message.msg_type != "attachment":
            continue
        if "link" in spec.has and "http" not in message.content.lower():
            continue
        out.append(message)
    return out


async def search_messages(
    db: AsyncSession,
    user: User,
    raw_query: str,
    *,
    channel_ids: set[str] | None = None,
    before: str | None = None,
    limit: int = 25,
) -> list[dict]:
    spec = parse_query(raw_query)
    if spec.author_id is None and "from:" in raw_query.lower():
        handle = OPERATOR_RE.search(raw_query)
        if handle is not None:
            spec.author_id = await resolve_author(db, handle.group(2))
            if spec.author_id is None:
                return []
    ids = await _fts_ids(
        db,
        "messages_fts",
        "messages",
        "channel_id",
        channel_ids or set(),
        spec.text,
        before,
        limit * 3,
    )
    messages = _apply_message_filters(spec, await _hydrate_messages(db, ids))
    messages = messages[:limit]

    authors = {
        u.id: u
        for u in (
            await db.scalars(select(User).where(User.id.in_({m.author_id for m in messages} or {""})))
        ).all()
    }
    channels = {
        c.id: c
        for c in (
            await db.scalars(
                select(Channel).where(Channel.id.in_({m.channel_id for m in messages} or {""}))
            )
        ).all()
    }
    return [
        {
            "message_id": message.id,
            "channel_id": message.channel_id,
            "server_id": channels[message.channel_id].server_id if message.channel_id in channels else "",
            "channel_name": channels[message.channel_id].name if message.channel_id in channels else "",
            "author": {
                "id": message.author_id,
                "name": authors[message.author_id].display_name if message.author_id in authors else "已注销用户",
                "color": authors[message.author_id].avatar_color if message.author_id in authors else 0x808080,
            },
            "created_at": message.created_at.isoformat().replace("+00:00", "Z"),
            "content": message.content,
            "highlight": snippet(message.content, spec.text),
            "type": message.msg_type,
        }
        for message in messages
    ]


async def search_dms(
    db: AsyncSession, user: User, raw_query: str, *, dm_ids: set[str], before: str | None, limit: int
) -> list[dict]:
    spec = parse_query(raw_query)
    ids = await _fts_ids(db, "dm_messages_fts", "dm_messages", "dm_id", dm_ids, spec.text, before, limit * 3)
    messages = _apply_message_filters(spec, await _hydrate_dm_messages(db, ids))
    messages = messages[:limit]
    authors = {
        u.id: u
        for u in (
            await db.scalars(select(User).where(User.id.in_({m.author_id for m in messages} or {""})))
        ).all()
    }
    return [
        {
            "message_id": message.id,
            "dm_id": message.dm_id,
            "author": {
                "id": message.author_id,
                "name": authors[message.author_id].display_name if message.author_id in authors else "已注销用户",
                "color": authors[message.author_id].avatar_color if message.author_id in authors else 0x808080,
            },
            "created_at": message.created_at.isoformat().replace("+00:00", "Z"),
            "content": message.content,
            "highlight": snippet(message.content, spec.text),
            "type": message.msg_type,
        }
        for message in messages
    ]


async def dm_ids_for(db: AsyncSession, user_id: str) -> set[str]:
    return set(
        (
            await db.scalars(
                select(DmChannel.id).where((DmChannel.user_a == user_id) | (DmChannel.user_b == user_id))
            )
        ).all()
    )


async def require_visible_channel(db: AsyncSession, user: User, channel_id: str) -> Channel:
    from app.services.permissions import effective_permissions

    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    return channel
