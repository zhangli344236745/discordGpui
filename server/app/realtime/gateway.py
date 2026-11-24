"""Gateway：/ws/gateway?token=<access>&since=<event_id>

只推订阅范围内的 server / dm / user 事件；上行仅 SUBSCRIBE / TYPING / VOICE_STATE / PING。
连接期间 token 过期或 60 s 无上行消息会被关闭。
"""

import asyncio
import contextlib
import json
import time
from datetime import UTC, datetime

import jwt
from fastapi import APIRouter, Query, WebSocket, WebSocketDisconnect
from sqlalchemy import and_, func, or_, select

from app.config import settings
from app.db import SessionLocal
from app.ids import EPOCH_MS, new_id
from app.models import DmChannel, GatewayEvent, ServerMember, User
from app.realtime.events import Event, envelope
from app.realtime.hub import Connection, hub
from app.security import decode_token

router = APIRouter()


async def _authorize(token: str) -> tuple[User, dict] | None:
    try:
        payload = decode_token(token, "access")
    except jwt.InvalidTokenError:
        return None
    async with SessionLocal() as db:
        user = await db.get(User, payload["sub"])
    return (user, payload) if user else None


async def _subscriptions(user_id: str) -> tuple[set[str], set[str]]:
    async with SessionLocal() as db:
        servers = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == user_id))).all())
        dms = set(
            (
                await db.scalars(
                    select(DmChannel.id).where(or_(DmChannel.user_a == user_id, DmChannel.user_b == user_id))
                )
            ).all()
        )
    return servers, dms


async def _event_cursor() -> str | None:
    async with SessionLocal() as db:
        return await db.scalar(select(func.max(GatewayEvent.id)))


def _cursor_expired(since: str) -> bool:
    try:
        event_ms = (int(since) >> 22) + EPOCH_MS
    except (TypeError, ValueError):
        return True
    cutoff = (datetime.now(UTC).timestamp() - settings.gateway_event_retention_hours * 3600) * 1000
    return event_ms < cutoff


async def _events_since(since: str, servers: set[str], dms: set[str], user_id: str) -> list[GatewayEvent]:
    async with SessionLocal() as db:
        rows = list(
            (
                await db.scalars(
                    select(GatewayEvent)
                    .where(
                        GatewayEvent.id > since,
                        or_(
                            and_(GatewayEvent.scope_kind == "server", GatewayEvent.scope_id.in_(servers or {""})),
                            and_(GatewayEvent.scope_kind == "dm", GatewayEvent.scope_id.in_(dms or {""})),
                            and_(GatewayEvent.scope_kind == "user", GatewayEvent.scope_id == user_id),
                        ),
                    )
                    .order_by(GatewayEvent.id)
                    .limit(501)
                )
            ).all()
        )
    return rows


@router.websocket("/ws/gateway")
async def gateway(
    ws: WebSocket,
    token: str = Query(default=""),
    since: str = Query(default=""),
) -> None:
    authorized = await _authorize(token)
    if authorized is None:
        await ws.close(code=4001, reason="unauthorized")
        return
    user, claims = authorized

    await ws.accept()
    servers, dms = await _subscriptions(user.id)
    conn = Connection(ws=ws, user_id=user.id, servers=servers, dms=dms)
    hub.add(conn)

    replayed = False
    if since and not _cursor_expired(since):
        events = await _events_since(since, servers, dms, user.id)
        if len(events) <= 500:
            replayed = True
            # 持锁直接写，保证补发顺序不被并发事件插队
            async with conn.lock:
                for row in events:
                    with contextlib.suppress(Exception):
                        await ws.send_text(envelope(row.event, row.payload or {}, row.id))

    if not replayed:
        await conn.send(
            Event.READY,
            {
                "user": {"id": user.id, "display_name": user.display_name, "status": user.status},
                "servers": sorted(servers),
                "dms": sorted(dms),
                "heartbeat_interval": settings.gateway_heartbeat_timeout_seconds * 1000,
                "event_cursor": await _event_cursor(),
            },
        )

    expires_at = float(claims.get("exp", 0))
    try:
        while True:
            if time.time() >= expires_at:
                await ws.close(code=4001, reason="token expired")
                return
            try:
                raw = await asyncio.wait_for(
                    ws.receive_text(), timeout=settings.gateway_heartbeat_timeout_seconds
                )
            except TimeoutError:
                await ws.close(code=4001, reason="heartbeat timeout")
                return
            try:
                msg = json.loads(raw)
            except json.JSONDecodeError:
                await conn.send(Event.ERROR, {"message": "invalid json"})
                continue

            op = msg.get("op")
            data = msg.get("d") or {}
            if op == "PING":
                await conn.send(Event.PONG, {"t": data.get("t")})
            elif op == "SUBSCRIBE":
                allowed_servers, allowed_dms = await _subscriptions(user.id)
                if isinstance(data.get("servers"), list):
                    conn.servers = {str(s) for s in data["servers"] if str(s) in allowed_servers}
                if isinstance(data.get("dms"), list):
                    conn.dms = {str(d) for d in data["dms"] if str(d) in allowed_dms}
            elif op == "TYPING" and data.get("channel_id"):
                await hub.to_server(
                    str(data.get("server_id", "")),
                    Event.TYPING_START,
                    {"channel_id": data["channel_id"], "user_id": user.id, "nonce": new_id()},
                )
            elif op not in ("VOICE_STATE",):
                await conn.send(Event.ERROR, {"message": f"unsupported op: {op}"})
    except WebSocketDisconnect:
        pass
    finally:
        hub.remove(conn)
        with contextlib.suppress(Exception):
            await ws.close()
