"""Gateway：/ws/gateway?token=<access>

只推订阅范围内的 server / dm 事件；上行仅 SUBSCRIBE / TYPING / PING。
"""

import contextlib
import json

import jwt
from fastapi import APIRouter, Query, WebSocket, WebSocketDisconnect
from sqlalchemy import or_, select

from app.db import SessionLocal
from app.ids import new_id
from app.models import DmChannel, ServerMember, User
from app.realtime.events import Event
from app.realtime.hub import Connection, hub
from app.security import decode_token

router = APIRouter()


async def _authorize(token: str) -> User | None:
    try:
        payload = decode_token(token, "access")
    except jwt.InvalidTokenError:
        return None
    async with SessionLocal() as db:
        return await db.get(User, payload["sub"])


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


@router.websocket("/ws/gateway")
async def gateway(ws: WebSocket, token: str = Query(default="")) -> None:
    user = await _authorize(token)
    if user is None:
        await ws.close(code=4001, reason="unauthorized")
        return

    await ws.accept()
    servers, dms = await _subscriptions(user.id)
    conn = Connection(ws=ws, user_id=user.id, servers=servers, dms=dms)
    hub.add(conn)

    await conn.send(
        Event.READY,
        {
            "user": {"id": user.id, "display_name": user.display_name, "status": user.status},
            "servers": sorted(servers),
            "dms": sorted(dms),
            "heartbeat_interval": 30_000,
        },
    )

    try:
        while True:
            raw = await ws.receive_text()
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
                if isinstance(data.get("servers"), list):
                    conn.servers = {str(s) for s in data["servers"]}
                if isinstance(data.get("dms"), list):
                    conn.dms = {str(d) for d in data["dms"]}
            elif op == "TYPING" and data.get("channel_id"):
                await hub.to_server(
                    str(data.get("server_id", "")),
                    Event.TYPING_START,
                    {"channel_id": data["channel_id"], "user_id": user.id, "nonce": new_id()},
                )
    except WebSocketDisconnect:
        pass
    finally:
        hub.remove(conn)
        with contextlib.suppress(Exception):
            await ws.close()
