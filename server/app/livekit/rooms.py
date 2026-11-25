"""LiveKit RoomService 客户端（Twirp HTTP API）。

真实部署时服务器侧静音 / 踢人不再只改本地投影，而是调用 RoomService，
由 LiveKit webhook 回写最终状态；调用失败或未启用时回退 Demo 投影。
"""

from __future__ import annotations

import logging
import time
from typing import Any

import httpx
import jwt

from app.config import settings

log = logging.getLogger("server.livekit.rooms")

_TIMEOUT = 5.0


def _http_base() -> str:
    # wss://host → http://host；已配置显式 HTTP 地址则直接使用
    url = settings.livekit_http_url or settings.livekit_url
    return url.replace("wss://", "https://").replace("ws://", "http://").rstrip("/")


def _admin_token(room: str) -> str:
    now = int(time.time())
    payload = {
        "iss": settings.livekit_api_key,
        "sub": "server",
        "iat": now,
        "exp": now + 300,
        "video": {"roomAdmin": True, "room": room},
    }
    return jwt.encode(payload, settings.livekit_api_secret, algorithm="HS256")


def enabled() -> bool:
    return bool(settings.livekit_roomservice_enabled)


async def _call(room: str, method: str, payload: dict[str, Any], transport: httpx.AsyncBaseTransport | None) -> bool:
    """调用 Twirp 接口；返回是否成功。失败只记日志，不抛异常。"""
    url = f"{_http_base()}/twirp/livekit.RoomService/{method}"
    headers = {"Authorization": f"Bearer {_admin_token(room)}"}
    try:
        client = (
            httpx.AsyncClient(timeout=_TIMEOUT, transport=transport)
            if transport
            else httpx.AsyncClient(timeout=_TIMEOUT)
        )
        try:
            resp = await client.post(url, json=payload, headers=headers)
        finally:
            await client.aclose()
    except httpx.HTTPError as error:
        log.info("roomservice %s unreachable: %s", method, error)
        return False
    if resp.status_code != 200:
        log.info("roomservice %s -> %s %s", method, resp.status_code, resp.text[:200])
        return False
    return True


async def mute_published_track(
    room: str,
    identity: str,
    muted: bool,
    transport: httpx.AsyncBaseTransport | None = None,
) -> bool:
    return await _call(
        room,
        "MutePublishedTrack",
        {"room": room, "identity": identity, "muted": muted, "kind": "audio"},
        transport,
    )


async def remove_participant(
    room: str,
    identity: str,
    transport: httpx.AsyncBaseTransport | None = None,
) -> bool:
    return await _call(
        room,
        "RemoveParticipant",
        {"room": room, "identity": identity},
        transport,
    )


__all__ = ["enabled", "mute_published_track", "remove_participant"]
