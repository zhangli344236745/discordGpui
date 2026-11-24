"""通知写入：提及、私信、好友请求、消息请求。"""

from __future__ import annotations

import re

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import utcnow
from app.ids import new_id
from app.models import Channel, DmChannel, Message, Notification, ServerMember, User
from app.realtime import outbox
from app.realtime.events import Event

MENTION_RE = re.compile(r"@([\w\-\u4e00-\u9fff]{1,32})")


def _payload(row: Notification) -> dict:
    return {
        "id": row.id,
        "kind": row.kind,
        "target_kind": row.target_kind,
        "target_id": row.target_id,
        "title": row.title,
        "body": row.body,
        "read": row.read,
        "created_at": row.created_at.isoformat().replace("+00:00", "Z"),
    }


def create(
    db: AsyncSession,
    user_id: str,
    kind: str,
    target_kind: str,
    target_id: str,
    title: str = "",
    body: str = "",
    actor_id: str | None = None,
) -> Notification | None:
    if actor_id and actor_id == user_id:
        return None
    row = Notification(
        id=new_id(),
        user_id=user_id,
        kind=kind,
        target_kind=target_kind,
        target_id=target_id,
        title=title[:190],
        body=body,
        created_at=utcnow(),
    )
    db.add(row)
    outbox.queue(db, "user", user_id, Event.NOTIFICATION_CREATE, _payload(row))
    return row


async def dm_message(db: AsyncSession, message, dm: DmChannel, author: User) -> None:
    partner = dm.user_b if dm.user_a == author.id else dm.user_a
    create(
        db,
        partner,
        "dm",
        "dm",
        dm.id,
        title=author.display_name,
        body=message.content[:190],
        actor_id=author.id,
    )


async def channel_mentions(
    db: AsyncSession,
    message: Message,
    channel: Channel,
    author: User,
    *,
    can_everyone: bool,
) -> None:
    content = message.content
    handles = {handle.lower() for handle in MENTION_RE.findall(content)}
    if not handles and "@everyone" not in content.lower():
        return

    members = (
        await db.execute(
            select(ServerMember.user_id, User)
            .join(User, User.id == ServerMember.user_id)
            .where(ServerMember.server_id == channel.server_id)
        )
    ).all()

    targets: set[str] = set()
    if can_everyone and "@everyone" in content.lower():
        targets.update(user.id for user_id, user in members if user.id != author.id)
    for _user_id, user in members:
        if user.username.lower() in handles and user.id != author.id:
            targets.add(user.id)

    for user_id in targets:
        create(
            db,
            user_id,
            "mention",
            "channel",
            channel.id,
            title=f"#{channel.name}",
            body=f"{author.display_name}: {content[:160]}",
            actor_id=author.id,
        )
