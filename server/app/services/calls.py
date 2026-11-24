"""私信语音/视频呼叫状态机。"""

from __future__ import annotations

import asyncio
import contextlib
import logging
from datetime import UTC, datetime, timedelta

from sqlalchemy import or_, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import SessionLocal
from app.models import CallSession
from app.realtime import outbox
from app.realtime.events import Event
from app.services.dm import iso

log = logging.getLogger("server.calls")

ACTIVE_STATES = ("ringing", "active")


def call_out(call: CallSession) -> dict:
    return {
        "id": call.id,
        "dm_id": call.dm_id,
        "kind": call.kind,
        "state": call.state,
        "outcome": call.outcome,
        "caller_id": call.caller_id,
        "callee_id": call.callee_id,
        "room": f"dm:{call.id}",
        "created_at": iso(call.created_at),
        "ended_at": iso(call.ended_at),
    }


async def active_call(db: AsyncSession, dm_id: str) -> CallSession | None:
    return await db.scalar(
        select(CallSession)
        .where(CallSession.dm_id == dm_id, CallSession.state.in_(ACTIVE_STATES))
        .order_by(CallSession.created_at.desc())
        .limit(1)
    )


async def expire_ringing(now: datetime | None = None) -> int:
    """把超过 60 s 仍在响铃的呼叫置为超时结束，并通知双方。"""
    now = now or datetime.now(UTC)
    cutoff = now - timedelta(seconds=settings.call_ring_timeout_seconds)
    expired = 0
    async with SessionLocal() as db:
        rows = list(
            (await db.scalars(select(CallSession).where(CallSession.state == "ringing"))).all()
        )
        for call in rows:
            created = call.created_at
            if created.tzinfo is None:
                created = created.replace(tzinfo=UTC)
            if created >= cutoff:
                continue
            call.state = "ended"
            call.outcome = "timeout"
            call.ended_at = now
            payload = call_out(call)
            for user_id in (call.caller_id, call.callee_id):
                outbox.queue(db, "user", user_id, Event.CALL_END, payload)
            expired += 1
        if expired:
            await db.commit()
    return expired


async def _watchdog() -> None:
    while True:
        try:
            await expire_ringing()
        except asyncio.CancelledError:
            raise
        except Exception:  # pragma: no cover
            log.exception("call watchdog failed")
        await asyncio.sleep(settings.call_watchdog_interval_seconds)


def start() -> asyncio.Task:
    return asyncio.create_task(_watchdog())


async def stop(task: asyncio.Task) -> None:
    task.cancel()
    with contextlib.suppress(asyncio.CancelledError):
        await task


def other_party(call: CallSession, user_id: str) -> str:
    return call.callee_id if call.caller_id == user_id else call.caller_id


def is_participant(call: CallSession, user_id: str) -> bool:
    return user_id in (call.caller_id, call.callee_id)


def find_for_participant_stmt(dm_id: str, user_id: str):  # noqa: ANN201
    return (
        select(CallSession)
        .where(
            CallSession.dm_id == dm_id,
            or_(CallSession.caller_id == user_id, CallSession.callee_id == user_id),
            CallSession.state.in_(ACTIVE_STATES),
        )
        .order_by(CallSession.created_at.desc())
        .limit(1)
    )
