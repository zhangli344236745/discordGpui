"""Gateway 事件名与信封格式，与 docs/backend-api.md 第 4 节一致。"""

import json
from datetime import UTC, datetime
from typing import Any

from app.ids import new_id


class Event:
    READY = "READY"
    MESSAGE_CREATE = "MESSAGE_CREATE"
    MESSAGE_UPDATE = "MESSAGE_UPDATE"
    MESSAGE_DELETE = "MESSAGE_DELETE"
    MESSAGE_REACTION_ADD = "MESSAGE_REACTION_ADD"
    MESSAGE_REACTION_REMOVE = "MESSAGE_REACTION_REMOVE"
    TYPING_START = "TYPING_START"
    CHANNEL_CREATE = "CHANNEL_CREATE"
    CHANNEL_UPDATE = "CHANNEL_UPDATE"
    CHANNEL_DELETE = "CHANNEL_DELETE"
    CATEGORY_CREATE = "CATEGORY_CREATE"
    CATEGORY_UPDATE = "CATEGORY_UPDATE"
    CATEGORY_DELETE = "CATEGORY_DELETE"
    SERVER_UPDATE = "SERVER_UPDATE"
    MEMBER_JOIN = "MEMBER_JOIN"
    MEMBER_LEAVE = "MEMBER_LEAVE"
    MEMBER_UPDATE = "MEMBER_UPDATE"
    ROLE_CREATE = "ROLE_CREATE"
    ROLE_UPDATE = "ROLE_UPDATE"
    ROLE_DELETE = "ROLE_DELETE"
    PERMISSIONS_UPDATE = "PERMISSIONS_UPDATE"
    AUDIT_ENTRY_CREATE = "AUDIT_ENTRY_CREATE"
    PRESENCE_UPDATE = "PRESENCE_UPDATE"
    DM_CREATE = "DM_CREATE"
    NOTIFICATION_CREATE = "NOTIFICATION_CREATE"
    VOICE_STATE_UPDATE = "VOICE_STATE_UPDATE"
    CALL_RING = "CALL_RING"
    CALL_ACCEPT = "CALL_ACCEPT"
    CALL_DECLINE = "CALL_DECLINE"
    CALL_END = "CALL_END"
    BOOKMARK_UPDATE = "BOOKMARK_UPDATE"
    PONG = "PONG"
    ERROR = "ERROR"


def envelope(event: str, data: dict[str, Any]) -> str:
    return json.dumps(
        {
            "event": event,
            "id": new_id(),
            "ts": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
            "data": data,
        },
        ensure_ascii=False,
        default=str,
    )
