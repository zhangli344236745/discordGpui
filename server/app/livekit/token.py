"""LiveKit AccessToken 签发。

官方 SDK 就是签一个 HS256 JWT，这里直接手写以免多一个依赖：
claims = {iss, sub, exp, nbf, name, metadata, video: {...}}
"""

from datetime import UTC, datetime, timedelta
from typing import Any

import jwt

from app.config import settings


def build_token(
    identity: str,
    name: str,
    room: str,
    metadata: dict[str, Any] | None = None,
    can_publish: bool = True,
    ttl_hours: int | None = None,
) -> str:
    now = datetime.now(UTC)
    ttl = timedelta(hours=ttl_hours or settings.livekit_token_ttl_hours)
    payload = {
        "iss": settings.livekit_api_key,
        "sub": identity,
        "iat": int(now.timestamp()),
        "nbf": int(now.timestamp()) - 5,
        "exp": int((now + ttl).timestamp()),
        "name": name,
        "video": {
            "room": room,
            "roomJoin": True,
            "canPublish": can_publish,
            "canSubscribe": True,
            "canPublishData": True,
            "canUpdateOwnMetadata": True,
        },
    }
    if metadata:
        payload["metadata"] = jwt.encode(metadata, settings.livekit_api_secret, algorithm="HS256")
    return jwt.encode(payload, settings.livekit_api_secret, algorithm="HS256")


def server_room(server_id: str, channel_id: str) -> str:
    return f"srv:{server_id}:{channel_id}"


def dm_room(call_id: str) -> str:
    return f"dm:{call_id}"
