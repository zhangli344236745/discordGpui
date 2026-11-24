"""私信语音/视频呼叫端点。房间内音频由 LiveKit 负责，这里只管状态与 token。"""

from fastapi import APIRouter, Depends
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db, utcnow
from app.deps import current_user
from app.errors import conflict, forbidden, not_found
from app.livekit import token as livekit_token
from app.models import CallSession, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.dm import CallIn
from app.services import calls, social
from app.services import dm as dm_service

router = APIRouter(tags=["calls"])


def _token_for(call: CallSession, user: User) -> str:
    return livekit_token.build_token(user.id, user.display_name, livekit_token.dm_room(call.id))


def _ring_payload(call: CallSession, caller: User) -> dict:
    data = calls.call_out(call)
    data["caller"] = {
        "id": caller.id,
        "display_name": caller.display_name,
        "avatar_color": caller.avatar_color,
    }
    return data


@router.post("/dms/{dm_id}/call")
async def start_call(
    dm_id: str,
    body: CallIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    dm = await dm_service.require_dm(db, dm_id, user.id)
    partner_id = dm_service.partner_id(dm, user.id)
    if await social.blocked(db, user.id, partner_id):
        raise forbidden("DIRECT_MESSAGE")

    await calls.expire_ringing()
    existing = await calls.active_call(db, dm.id)
    if existing is not None:
        if existing.state == "ringing" and existing.caller_id == user.id:
            return {
                **calls.call_out(existing),
                "token": _token_for(existing, user),
                "url": settings.livekit_url,
            }
        raise conflict("当前会话已有进行中的通话", "state")

    call = CallSession(
        dm_id=dm.id,
        kind=body.kind,
        state="ringing",
        outcome="",
        caller_id=user.id,
        callee_id=partner_id,
    )
    db.add(call)
    await db.flush()
    outbox.queue(db, "user", partner_id, Event.CALL_RING, _ring_payload(call, user))
    await db.commit()
    return {**calls.call_out(call), "token": _token_for(call, user), "url": settings.livekit_url}


@router.post("/dms/{dm_id}/call/accept")
async def accept_call(
    dm_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await dm_service.require_dm(db, dm_id, user.id)
    await calls.expire_ringing()
    call = await db.scalar(
        select(CallSession)
        .where(
            CallSession.dm_id == dm_id,
            CallSession.callee_id == user.id,
            CallSession.state == "ringing",
        )
        .order_by(CallSession.created_at.desc())
        .limit(1)
    )
    if call is None:
        raise conflict("没有待接听的呼叫", "state")

    call.state = "active"
    call.outcome = "accepted"
    payload = calls.call_out(call)
    outbox.queue(db, "user", call.caller_id, Event.CALL_ACCEPT, payload)
    await db.commit()
    return {**payload, "token": _token_for(call, user), "url": settings.livekit_url}


@router.post("/dms/{dm_id}/call/decline")
async def decline_call(
    dm_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await dm_service.require_dm(db, dm_id, user.id)
    call = await db.scalar(
        select(CallSession)
        .where(
            CallSession.dm_id == dm_id,
            CallSession.callee_id == user.id,
            CallSession.state == "ringing",
        )
        .order_by(CallSession.created_at.desc())
        .limit(1)
    )
    if call is None:
        raise conflict("没有待接听的呼叫", "state")
    call.state = "ended"
    call.outcome = "declined"
    call.ended_at = utcnow()
    payload = calls.call_out(call)
    outbox.queue(db, "user", call.caller_id, Event.CALL_DECLINE, payload)
    await db.commit()
    return payload


@router.post("/dms/{dm_id}/call/hangup")
async def hangup_call(
    dm_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await dm_service.require_dm(db, dm_id, user.id)
    call = await db.scalar(calls.find_for_participant_stmt(dm_id, user.id))
    if call is None:
        ended = await db.scalar(
            select(CallSession)
            .where(
                CallSession.dm_id == dm_id,
                CallSession.state == "ended",
                (CallSession.caller_id == user.id) | (CallSession.callee_id == user.id),
            )
            .order_by(CallSession.created_at.desc())
            .limit(1)
        )
        if ended is not None:
            return calls.call_out(ended)
        raise not_found("通话")

    call.state = "ended"
    call.outcome = "hangup"
    call.ended_at = utcnow()
    payload = calls.call_out(call)
    outbox.queue(db, "user", calls.other_party(call, user.id), Event.CALL_END, payload)
    await db.commit()
    return payload


__all__ = ["router"]
