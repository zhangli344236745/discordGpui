"""私信会话、私信消息与消息请求。"""

from fastapi import APIRouter, Depends, Query, status
from sqlalchemy import func, or_, select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db
from app.deps import current_user
from app.errors import ApiError, ErrorCode, conflict, forbidden, not_found
from app.ids import new_id
from app.models import DmChannel, DmMessage, MessageRequest, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.dm import DmCreate, DmMessageCreate, DmPatch, DmReadIn
from app.services import dm as dm_service
from app.services import notifications, ratelimit, social

router = APIRouter(tags=["dms"])


@router.get("/dms")
async def list_dms(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    rows = list(
        (
            await db.scalars(
                select(DmChannel).where(or_(DmChannel.user_a == user.id, DmChannel.user_b == user.id))
            )
        ).all()
    )
    items = [await dm_service.summary(db, dm, user.id) for dm in rows]
    items.sort(
        key=lambda item: (
            item["closed"],
            not item["pinned"],
            -(int(item["last_message"]["id"]) if item["last_message"] else 0),
        )
    )
    return items


@router.post("/dms", status_code=status.HTTP_201_CREATED)
async def open_dm(
    body: DmCreate,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if body.user_id == user.id:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "不能和自己建立私信", "user_id")
    target = await social.get_user(db, body.user_id)
    if await social.blocked(db, user.id, target.id):
        raise forbidden("DIRECT_MESSAGE")

    dm = await social.existing_dm(db, user.id, target.id)
    if dm is not None:
        if dm.closed:
            dm.closed = False
            await db.commit()
        return {"kind": "dm", "dm": await dm_service.summary(db, dm, user.id)}

    if await social.can_dm_directly(db, user.id, target.id):
        dm, _created = await dm_service.create_or_get_dm(db, user.id, target.id)
        outbox.queue(db, "user", user.id, Event.DM_CREATE, await dm_service.summary(db, dm, user.id))
        outbox.queue(db, "user", target.id, Event.DM_CREATE, await dm_service.summary(db, dm, target.id))
        await db.commit()
        return {"kind": "dm", "dm": await dm_service.summary(db, dm, user.id)}

    # 陌生人：落到消息请求，不直接建 DM
    pending = await db.scalar(
        select(MessageRequest).where(
            MessageRequest.from_user == user.id,
            MessageRequest.to_user == target.id,
            MessageRequest.state == "pending",
        )
    )
    if pending is not None:
        payload = dm_service.request_out(pending, user, target, "outgoing")
        return {"kind": "request", "request": payload}

    reverse = await db.scalar(
        select(MessageRequest).where(
            MessageRequest.from_user == target.id,
            MessageRequest.to_user == user.id,
            MessageRequest.state == "pending",
        )
    )
    if reverse is not None:
        raise conflict("对方已向你发送消息请求，请到请求列表处理", "user_id")

    request = MessageRequest(
        from_user=user.id,
        to_user=target.id,
        state="pending",
        first_message=(body.first_message or "").strip(),
    )
    db.add(request)
    await db.flush()
    payload = dm_service.request_out(request, user, target, "incoming")
    notifications.create(
        db,
        target.id,
        "message_request",
        "message_request",
        request.id,
        title=user.display_name,
        body=request.first_message[:190],
        actor_id=user.id,
    )
    outbox.queue(db, "user", target.id, Event.MESSAGE_REQUEST_CREATE, payload)
    await db.commit()
    return {"kind": "request", "request": payload}


@router.get("/dms/{dm_id}/messages")
async def list_dm_messages(
    dm_id: str,
    before: str | None = None,
    after: str | None = None,
    limit: int = Query(default=50, le=100),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    dm = await dm_service.require_dm(db, dm_id, user.id)
    stmt = select(DmMessage).where(DmMessage.dm_id == dm.id, DmMessage.deleted_at.is_(None))
    if before:
        stmt = stmt.where(DmMessage.id < before)
    if after:
        stmt = stmt.where(DmMessage.id > after)
    ordering = DmMessage.id.asc() if after else DmMessage.id.desc()
    rows = list((await db.scalars(stmt.order_by(ordering).limit(limit + 1))).all())
    has_more = len(rows) > limit
    rows = rows[:limit]
    if not after:
        rows.reverse()
    return {"items": await dm_service.messages_out(db, rows), "has_more": has_more}


@router.post("/dms/{dm_id}/messages", status_code=status.HTTP_201_CREATED)
async def send_dm_message(
    dm_id: str,
    body: DmMessageCreate,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    dm = await dm_service.require_dm(db, dm_id, user.id)
    partner = dm_service.partner_id(dm, user.id)
    if await social.blocked(db, user.id, partner):
        raise forbidden("DIRECT_MESSAGE")
    ratelimit.check(
        f"msg:{user.id}", settings.rate_limit_messages, settings.rate_limit_window_seconds
    )
    if body.client_nonce:
        existing = await db.scalar(select(DmMessage).where(DmMessage.client_nonce == body.client_nonce))
        if existing is not None:
            return await dm_service.message_out(db, existing)
    if not body.content.strip():
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "消息不能为空", "content")

    message = DmMessage(
        id=new_id(),
        dm_id=dm.id,
        author_id=user.id,
        content=body.content,
        reply_to=body.reply_to,
        msg_type=body.type,
        payload=body.payload,
        client_nonce=body.client_nonce,
    )
    db.add(message)
    dm.last_message_id = message.id
    dm_service.set_last_read(dm, user.id, message.id)
    try:
        await db.flush()
    except IntegrityError:
        await db.rollback()
        existing = (
            await db.scalar(select(DmMessage).where(DmMessage.client_nonce == body.client_nonce))
            if body.client_nonce
            else None
        )
        if existing is None:
            raise
        return await dm_service.message_out(db, existing)
    payload = await dm_service.message_out(db, message)
    await notifications.dm_message(db, message, dm, user)
    outbox.queue(db, "user", dm.user_a, Event.MESSAGE_CREATE, payload)
    outbox.queue(db, "user", dm.user_b, Event.MESSAGE_CREATE, payload)
    try:
        await db.commit()
    except IntegrityError:
        await db.rollback()
        existing = (
            await db.scalar(select(DmMessage).where(DmMessage.client_nonce == body.client_nonce))
            if body.client_nonce
            else None
        )
        if existing is None:
            raise
        return await dm_service.message_out(db, existing)
    return payload


@router.patch("/dms/{dm_id}")
async def patch_dm(
    dm_id: str,
    body: DmPatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    dm = await dm_service.require_dm(db, dm_id, user.id)
    if body.pinned is not None:
        dm_service.set_pinned(dm, user.id, body.pinned)
    if body.closed is not None:
        dm.closed = body.closed
    outbox.queue(db, "user", dm.user_a, Event.DM_CREATE, await dm_service.summary(db, dm, dm.user_a))
    outbox.queue(db, "user", dm.user_b, Event.DM_CREATE, await dm_service.summary(db, dm, dm.user_b))
    await db.commit()
    return await dm_service.summary(db, dm, user.id)


@router.patch("/dms/{dm_id}/read")
async def mark_dm_read(
    dm_id: str,
    body: DmReadIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    dm = await dm_service.require_dm(db, dm_id, user.id)
    latest = body.last_read_message_id
    if latest is None:
        latest = await db.scalar(
            select(DmMessage.id)
            .where(DmMessage.dm_id == dm.id, DmMessage.deleted_at.is_(None))
            .order_by(DmMessage.id.desc())
            .limit(1)
        )
    current = dm_service.last_read_of(dm, user.id)
    if latest and (current is None or latest > current):
        dm_service.set_last_read(dm, user.id, latest)
        await db.commit()
    return {"dm_id": dm.id, "last_read_message_id": dm_service.last_read_of(dm, user.id), "unread": 0}


# ---------------------------------------------------------------------------
# 消息请求
# ---------------------------------------------------------------------------


@router.get("/message-requests")
async def list_message_requests(
    state: str = Query(default="pending", pattern="^(pending|accepted|ignored|all)$"),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    stmt = select(MessageRequest).where(MessageRequest.to_user == user.id)
    if state != "all":
        stmt = stmt.where(MessageRequest.state == state)
    rows = list((await db.scalars(stmt.order_by(MessageRequest.created_at.desc()))).all())
    senders = (
        {
            u.id: u
            for u in (await db.scalars(select(User).where(User.id.in_({row.from_user for row in rows})))).all()
        }
        if rows
        else {}
    )
    items = []
    for row in rows:
        sender = senders.get(row.from_user)
        if sender is not None:
            items.append(dm_service.request_out(row, sender, user, "incoming"))
    pending_total = int(
        await db.scalar(
            select(func.count())
            .select_from(MessageRequest)
            .where(MessageRequest.to_user == user.id, MessageRequest.state == "pending")
        )
        or 0
    )
    return {"items": items, "count": pending_total}


@router.post("/message-requests/{request_id}/accept")
async def accept_message_request(
    request_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    request = await db.get(MessageRequest, request_id)
    if request is None or request.to_user != user.id:
        raise not_found("消息请求")
    if request.state != "pending":
        raise conflict("该请求已处理", "state")
    if await social.blocked(db, request.from_user, request.to_user):
        raise forbidden("DIRECT_MESSAGE")

    dm, _created = await dm_service.create_or_get_dm(db, request.from_user, request.to_user)
    if request.first_message.strip():
        nonce = f"mr:{request.id}"
        if await db.scalar(select(DmMessage.id).where(DmMessage.client_nonce == nonce)) is None:
            message = DmMessage(
                id=new_id(),
                dm_id=dm.id,
                author_id=request.from_user,
                content=request.first_message,
                client_nonce=nonce,
            )
            db.add(message)
            await db.flush()
            dm.last_message_id = message.id
            dm_service.set_last_read(dm, request.from_user, message.id)
            message_payload = await dm_service.message_out(db, message)
            outbox.queue(db, "user", dm.user_a, Event.MESSAGE_CREATE, message_payload)
            outbox.queue(db, "user", dm.user_b, Event.MESSAGE_CREATE, message_payload)
    request.state = "accepted"
    sender = await db.get(User, request.from_user)
    payload = dm_service.request_out(request, sender or user, user, "incoming")
    outbox.queue(db, "user", request.from_user, Event.MESSAGE_REQUEST_ACCEPT, payload)
    outbox.queue(db, "user", request.to_user, Event.MESSAGE_REQUEST_ACCEPT, payload)
    await db.commit()
    return {"request": payload, "dm": await dm_service.summary(db, dm, user.id)}


@router.post("/message-requests/{request_id}/ignore")
async def ignore_message_request(
    request_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    request = await db.get(MessageRequest, request_id)
    if request is None or request.to_user != user.id:
        raise not_found("消息请求")
    if request.state != "pending":
        raise conflict("该请求已处理", "state")
    request.state = "ignored"
    sender = await db.get(User, request.from_user)
    payload = dm_service.request_out(request, sender or user, user, "incoming")
    outbox.queue(db, "user", request.from_user, Event.MESSAGE_REQUEST_IGNORE, payload)
    await db.commit()
    return payload


@router.delete("/message-requests/{request_id}")
async def delete_message_request(
    request_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    request = await db.get(MessageRequest, request_id)
    if request is None or user.id not in (request.from_user, request.to_user):
        raise not_found("消息请求")
    other = request.to_user if user.id == request.from_user else request.from_user
    await db.delete(request)
    outbox.queue(db, "user", other, Event.MESSAGE_REQUEST_DELETE, {"id": request_id})
    await db.commit()
    return {"deleted": True}
