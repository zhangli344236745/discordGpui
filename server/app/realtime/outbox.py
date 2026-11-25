"""事件 outbox：与业务数据同事务落库，提交后广播，失败回滚则丢弃。

调用方在写操作里只调 `queue(...)`，不要直接碰 `hub`——这样：
1. 事件 id 与事件行在同一个事务里生成，`?since` 才能可靠补齐；
2. 未提交的数据不会被客户端提前看到；
3. 幂等重试命中原记录时不会重复入队。
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

from sqlalchemy import event
from sqlalchemy.orm import Session

from app.ids import new_id
from app.models import GatewayEvent
from app.realtime.bus import bus

try:
    from app.ops import EVENTS_PUBLISHED as _EVENTS_METRIC
    from app.ops import current_traceparent
except Exception:  # pragma: no cover - 缺依赖时不影响广播
    _EVENTS_METRIC = None
    current_traceparent = lambda: ""  # noqa: E731

log = logging.getLogger("server.outbox")

PENDING_KEY = "pending_gateway_events"
SCOPE_SERVER = "server"
SCOPE_DM = "dm"
SCOPE_USER = "user"
SCOPE_KINDS = frozenset({SCOPE_SERVER, SCOPE_DM, SCOPE_USER})


def queue(db, scope_kind: str, scope_id: str, name: str, data: dict[str, Any]) -> str:  # noqa: ANN001
    """登记事件并返回事件 id。真正投递发生在 session.commit() 之后。"""
    if scope_kind not in SCOPE_KINDS:
        raise ValueError(f"unknown scope kind: {scope_kind}")
    event_id = new_id()
    db.add(
        GatewayEvent(
            id=event_id,
            scope_kind=scope_kind,
            scope_id=scope_id,
            event=name,
            payload=data,
        )
    )
    db.info.setdefault(PENDING_KEY, []).append((event_id, scope_kind, scope_id, name, data))
    if _EVENTS_METRIC is not None:
        _EVENTS_METRIC.labels(name).inc()
    log.info(
        "queue event=%s scope=%s:%s id=%s trace=%s",
        name,
        scope_kind,
        scope_id,
        event_id,
        current_traceparent(),
    )
    return event_id


def _after_commit(session: Session) -> None:
    pending = session.info.pop(PENDING_KEY, [])
    if not pending:
        return
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:  # pragma: no cover - 同步脚本里没有事件循环
        log.warning("drop %d gateway events: no running loop", len(pending))
        return
    for event_id, scope_kind, scope_id, name, data in pending:
        loop.create_task(bus.dispatch(scope_kind, scope_id, name, data, event_id))


def _after_rollback(session: Session) -> None:
    session.info.pop(PENDING_KEY, None)


event.listens_for(Session, "after_commit")(_after_commit)
event.listens_for(Session, "after_rollback")(_after_rollback)

__all__ = ["PENDING_KEY", "SCOPE_DM", "SCOPE_KINDS", "SCOPE_SERVER", "SCOPE_USER", "queue"]
