"""通知收件箱、未读计数与推送端点注册。"""

from datetime import UTC, datetime
from urllib.parse import urlparse

from fastapi import APIRouter, Depends, Query
from sqlalchemy import func, select, update
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import current_user
from app.errors import ApiError, ErrorCode
from app.models import (
    ChannelUserState,
    DmChannel,
    FriendRequest,
    Message,
    Notification,
    User,
    UserSettings,
)
from app.schemas.me import NotificationReadIn, PushSubscribeIn
from app.services import dm as dm_service
from app.services import search

router = APIRouter(tags=["notifications"])


def _iso(value) -> str | None:  # noqa: ANN001
    return value.isoformat().replace("+00:00", "Z") if value else None


@router.get("/notifications")
async def list_notifications(
    kind: str = "",
    unread: bool = False,
    before: str | None = None,
    limit: int = Query(default=30, le=100),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    stmt = select(Notification).where(Notification.user_id == user.id)
    if kind:
        stmt = stmt.where(Notification.kind == kind)
    if unread:
        stmt = stmt.where(Notification.read.is_(False))
    if before:
        stmt = stmt.where(Notification.id < before)
    rows = list((await db.scalars(stmt.order_by(Notification.id.desc()).limit(limit + 1))).all())
    has_more = len(rows) > limit
    return {
        "items": [
            {
                "id": row.id,
                "kind": row.kind,
                "target_kind": row.target_kind,
                "target_id": row.target_id,
                "title": row.title,
                "body": row.body,
                "read": row.read,
                "created_at": _iso(row.created_at),
            }
            for row in rows[:limit]
        ],
        "has_more": has_more,
    }


@router.post("/notifications/read")
async def read_notifications(
    body: NotificationReadIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    stmt = update(Notification).where(Notification.user_id == user.id, Notification.read.is_(False))
    if body.all:
        pass
    elif body.ids:
        stmt = stmt.where(Notification.id.in_(body.ids))
    else:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "需要 ids 或 all=true", "ids")
    result = await db.execute(stmt.values(read=True))
    await db.commit()
    return {"updated": result.rowcount or 0}


@router.get("/notifications/unread-counts")
async def unread_counts(
    db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    visible = await search.visible_channels(db, user.id)
    channel_items: list[dict] = []
    server_totals: dict[str, int] = {}

    for server_id, channel_ids in visible.items():
        if not channel_ids:
            continue
        states = {
            row.channel_id: row.last_read_message_id
            for row in (
                await db.scalars(
                    select(ChannelUserState).where(
                        ChannelUserState.user_id == user.id,
                        ChannelUserState.channel_id.in_(channel_ids),
                    )
                )
            ).all()
        }
        for channel_id in channel_ids:
            stmt = select(func.count()).select_from(Message).where(
                Message.channel_id == channel_id,
                Message.deleted_at.is_(None),
                Message.author_id != user.id,
            )
            last_read = states.get(channel_id)
            if last_read:
                stmt = stmt.where(Message.id > last_read)
            count = int(await db.scalar(stmt) or 0)
            if count:
                channel_items.append({"channel_id": channel_id, "server_id": server_id, "unread": count})
                server_totals[server_id] = server_totals.get(server_id, 0) + count

    dm_items: list[dict] = []
    dms = list(
        (
            await db.scalars(
                select(DmChannel).where((DmChannel.user_a == user.id) | (DmChannel.user_b == user.id))
            )
        ).all()
    )
    dm_total = 0
    for dm in dms:
        unread = await dm_service.unread_count(db, dm, user.id)
        if unread:
            dm_items.append({"dm_id": dm.id, "unread": unread})
            dm_total += unread

    mentions = int(
        await db.scalar(
            select(func.count())
            .select_from(Notification)
            .where(
                Notification.user_id == user.id,
                Notification.kind == "mention",
                Notification.read.is_(False),
            )
        )
        or 0
    )
    friend_requests = int(
        await db.scalar(
            select(func.count())
            .select_from(FriendRequest)
            .where(FriendRequest.to_user == user.id, FriendRequest.state == "pending")
        )
        or 0
    )
    notifications = int(
        await db.scalar(
            select(func.count())
            .select_from(Notification)
            .where(Notification.user_id == user.id, Notification.read.is_(False))
        )
        or 0
    )
    return {
        "servers": [{"server_id": sid, "unread": total} for sid, total in server_totals.items()],
        "channels": channel_items,
        "dms": dm_items,
        "mentions": mentions,
        "friend_requests": friend_requests,
        "notifications": notifications,
        "total": sum(server_totals.values()) + dm_total,
    }


@router.post("/push/subscribe")
async def subscribe_push(
    body: PushSubscribeIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    parsed = urlparse(body.endpoint)
    if parsed.scheme not in ("http", "https") or not parsed.netloc:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "endpoint 必须是 http(s) URL", "endpoint")

    settings = await db.get(UserSettings, user.id)
    if settings is None:
        settings = UserSettings(user_id=user.id)
        db.add(settings)
    endpoints = list(settings.push_endpoints or [])
    if not any(item.get("endpoint") == body.endpoint for item in endpoints):
        endpoints.append(
            {
                "endpoint": body.endpoint,
                "keys": body.keys or {},
                "created_at": datetime.now(UTC).isoformat(),
            }
        )
        settings.push_endpoints = endpoints
        await db.commit()
    return {"subscribed": True, "delivery": "demo", "count": len(endpoints)}
