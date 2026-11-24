"""服务器日程活动与报名。"""

from datetime import UTC, datetime, timedelta

from fastapi import APIRouter, Depends, Query
from sqlalchemy import delete, func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ServerContext, current_user, server_context
from app.errors import forbidden, not_found
from app.models import Channel, EventRsvp, Server, ServerEvent, ServerMember, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.content import RsvpIn, ServerEventIn, ServerEventPatch
from app.services import content as content_service
from app.services.permissions import effective_permissions

router = APIRouter(tags=["events"])


def _as_utc(value: datetime) -> datetime:
    return value.replace(tzinfo=UTC) if value.tzinfo is None else value


def event_status(event: ServerEvent, now: datetime | None = None) -> str:
    now = now or datetime.now(UTC)
    start = _as_utc(event.starts_at)
    end = start + timedelta(minutes=event.duration_minutes)
    if now < start:
        return "upcoming"
    if now <= end:
        return "live"
    return "ended"


async def _event_out(db: AsyncSession, event: ServerEvent, viewer_id: str) -> dict:
    creator = await db.get(User, event.created_by)
    going_count = int(
        await db.scalar(
            select(func.count()).select_from(EventRsvp).where(EventRsvp.event_id == event.id)
        )
        or 0
    )
    going = await db.get(EventRsvp, (event.id, viewer_id)) is not None
    channel = await db.get(Channel, event.channel_id) if event.channel_id else None
    return {
        "id": event.id,
        "server_id": event.server_id,
        "title": event.title,
        "description": event.description,
        "category": event.category,
        "channel_id": event.channel_id,
        "channel_name": channel.name if channel else None,
        "starts_at": content_service.iso(event.starts_at),
        "duration_minutes": event.duration_minutes,
        "status": event_status(event),
        "created_by": event.created_by,
        "creator": {
            "id": event.created_by,
            "name": creator.display_name if creator else "已注销用户",
            "color": creator.avatar_color if creator else 0x808080,
        },
        "going_count": going_count,
        "going": going,
        "created_at": content_service.iso(event.created_at),
    }


async def _event_scope(
    db: AsyncSession, event_id: str, user: User
) -> tuple[ServerEvent, Server, set[str]]:
    event = await db.get(ServerEvent, event_id)
    if event is None:
        raise not_found("活动")
    server = await db.get(Server, event.server_id)
    if server is None:
        raise not_found("活动")
    if await db.get(ServerMember, (server.id, user.id)) is None:
        raise not_found("活动")
    perms = await effective_permissions(db, server.id, user.id)
    return event, server, perms


@router.get("/servers/{server_id}/events")
async def list_events(
    status: str = Query(default="", pattern="^(|upcoming|live|ended)$"),
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    rows = list(
        (
            await db.scalars(
                select(ServerEvent)
                .where(ServerEvent.server_id == ctx.server.id)
                .order_by(ServerEvent.starts_at)
            )
        ).all()
    )
    items = [await _event_out(db, event, ctx.user.id) for event in rows]
    if status:
        items = [item for item in items if item["status"] == status]
    return items


@router.post("/servers/{server_id}/events", status_code=201)
async def create_event(
    body: ServerEventIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    if body.channel_id is not None:
        channel = await db.get(Channel, body.channel_id)
        if channel is None or channel.server_id != ctx.server.id:
            raise not_found("频道")
    event = ServerEvent(
        server_id=ctx.server.id,
        title=body.title.strip(),
        description=body.description,
        category=body.category,
        channel_id=body.channel_id,
        starts_at=body.starts_at,
        duration_minutes=body.duration_minutes,
        created_by=ctx.user.id,
    )
    db.add(event)
    await db.flush()
    payload = await _event_out(db, event, ctx.user.id)
    outbox.queue(db, "server", ctx.server.id, Event.EVENT_CREATE, payload)
    await db.commit()
    return payload


@router.patch("/events/{event_id}")
async def patch_event(
    event_id: str,
    body: ServerEventPatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    event, server, perms = await _event_scope(db, event_id, user)
    if event.created_by != user.id and "MANAGE_SERVER" not in perms:
        raise forbidden("MANAGE_SERVER")
    if body.channel_id is not None:
        channel = await db.get(Channel, body.channel_id)
        if channel is None or channel.server_id != server.id:
            raise not_found("频道")
        event.channel_id = body.channel_id
    if body.title is not None:
        event.title = body.title.strip()
    if body.description is not None:
        event.description = body.description
    if body.category is not None:
        event.category = body.category
    if body.starts_at is not None:
        event.starts_at = body.starts_at
    if body.duration_minutes is not None:
        event.duration_minutes = body.duration_minutes
    payload = await _event_out(db, event, user.id)
    outbox.queue(db, "server", server.id, Event.EVENT_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/events/{event_id}")
async def delete_event(
    event_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    event, server, perms = await _event_scope(db, event_id, user)
    if event.created_by != user.id and "MANAGE_SERVER" not in perms:
        raise forbidden("MANAGE_SERVER")
    await db.execute(delete(EventRsvp).where(EventRsvp.event_id == event.id))
    await db.delete(event)
    outbox.queue(db, "server", server.id, Event.EVENT_DELETE, {"id": event_id})
    await db.commit()
    return {"deleted": True}


@router.put("/events/{event_id}/rsvp")
async def rsvp_event(
    event_id: str,
    body: RsvpIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    event, server, _perms = await _event_scope(db, event_id, user)
    existing = await db.get(EventRsvp, (event.id, user.id))
    if body.going and existing is None:
        db.add(EventRsvp(event_id=event.id, user_id=user.id))
    elif not body.going and existing is not None:
        await db.delete(existing)
    await db.flush()
    payload = await _event_out(db, event, user.id)
    outbox.queue(db, "server", server.id, Event.EVENT_UPDATE, payload)
    await db.commit()
    return payload


@router.get("/events/{event_id}/attendees")
async def list_attendees(
    event_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> list[dict]:
    event, _server, _perms = await _event_scope(db, event_id, user)
    rows = (
        await db.execute(
            select(EventRsvp, User).join(User, User.id == EventRsvp.user_id).where(EventRsvp.event_id == event.id)
        )
    ).all()
    return [
        {
            "user_id": attendee.user_id,
            "display_name": attendee_user.display_name,
            "avatar_color": attendee_user.avatar_color,
            "status": "offline" if attendee_user.status == "invisible" else attendee_user.status,
            "created_at": content_service.iso(attendee.created_at),
        }
        for attendee, attendee_user in rows
    ]
