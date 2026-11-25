"""事件总线抽象：业务层只依赖 EventBus，不关心单机内存还是 Redis Pub/Sub。

- 默认 `MemoryEventBus`：单进程开发/测试。
- 配置 `REDIS_URL` 后切换 `RedisEventBus`：发布走 Redis，订阅任务把消息回放到本进程 Hub。
"""

from __future__ import annotations

import asyncio
import contextlib
import json
import logging
from typing import Any, Protocol

import redis.asyncio as redis

from app.config import settings
from app.realtime.hub import Hub
from app.realtime.hub import hub as _hub

log = logging.getLogger("server.bus")

CHANNEL = "discord-gateway-events"


class EventBus(Protocol):
    @property
    def size(self) -> int: ...

    async def to_user(self, user_id: str, event: str, data: dict, event_id: str | None = None) -> None: ...

    async def to_server(self, server_id: str, event: str, data: dict, event_id: str | None = None) -> None: ...

    async def to_dm(self, dm_id: str, event: str, data: dict, event_id: str | None = None) -> None: ...

    async def dispatch(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None = None,
    ) -> None: ...

    async def disconnect_user(self, user_id: str) -> None: ...


class MemoryEventBus:
    """单进程实现：直接复用现有 Hub 的连接池与订阅过滤。"""

    def __init__(self, delegate: Hub) -> None:
        self._delegate = delegate

    @property
    def size(self) -> int:
        return self._delegate.size

    async def to_user(self, user_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._delegate.to_user(user_id, event, data, event_id)

    async def to_server(self, server_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._delegate.to_server(server_id, event, data, event_id)

    async def to_dm(self, dm_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._delegate.to_dm(dm_id, event, data, event_id)

    async def dispatch(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None = None,
    ) -> None:
        await self._delegate.dispatch(scope_kind, scope_id, event, data, event_id)

    async def disconnect_user(self, user_id: str) -> None:
        self._delegate.close_user(user_id)


class RedisEventBus:
    """多实例实现：发布到 Redis，由各实例的订阅任务回放到本进程 Hub。"""

    def __init__(self, client: redis.Redis) -> None:
        self._client = client

    @property
    def size(self) -> int:
        return _hub.size

    async def _publish(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None,
    ) -> None:
        payload = json.dumps(
            {
                "scope_kind": scope_kind,
                "scope_id": scope_id,
                "event": event,
                "data": data,
                "event_id": event_id,
            },
            ensure_ascii=False,
            default=str,
        )
        await self._client.publish(CHANNEL, payload)

    async def to_user(self, user_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._publish("user", user_id, event, data, event_id)

    async def to_server(self, server_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._publish("server", server_id, event, data, event_id)

    async def to_dm(self, dm_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self._publish("dm", dm_id, event, data, event_id)

    async def dispatch(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None = None,
    ) -> None:
        await self._publish(scope_kind, scope_id, event, data, event_id)

    async def disconnect_user(self, user_id: str) -> None:
        # 控制消息广播到所有实例，各实例摘除本地连接
        await self._publish("control", user_id, "DISCONNECT_USER", {}, None)


class EventBusProxy:
    """保持导入引用不变，允许运行时切换内存/Redis 实现。"""

    def __init__(self, delegate: EventBus) -> None:
        self.delegate = delegate

    @property
    def size(self) -> int:
        return self.delegate.size

    async def to_user(self, user_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self.delegate.to_user(user_id, event, data, event_id)

    async def to_server(self, server_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self.delegate.to_server(server_id, event, data, event_id)

    async def to_dm(self, dm_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        await self.delegate.to_dm(dm_id, event, data, event_id)

    async def dispatch(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None = None,
    ) -> None:
        await self.delegate.dispatch(scope_kind, scope_id, event, data, event_id)

    async def disconnect_user(self, user_id: str) -> None:
        await self.delegate.disconnect_user(user_id)


bus = EventBusProxy(MemoryEventBus(_hub))
_subscriber_task: asyncio.Task[None] | None = None
_redis_client: redis.Redis | None = None


async def init_bus() -> None:
    """配置了 REDIS_URL 时切换到 Redis 广播；否则保持内存实现。"""
    global _redis_client, _subscriber_task

    if not settings.redis_url:
        bus.delegate = MemoryEventBus(_hub)
        return

    client = redis.from_url(settings.redis_url, decode_responses=True)
    await client.ping()
    _redis_client = client
    bus.delegate = RedisEventBus(client)

    async def replay() -> None:
        pubsub = client.pubsub()
        await pubsub.subscribe(CHANNEL)
        try:
            async for message in pubsub.listen():
                if message.get("type") != "message":
                    continue
                await _replay(message.get("data", ""))
        finally:
            with contextlib.suppress(Exception):
                await pubsub.unsubscribe(CHANNEL)
                await pubsub.close()

    _subscriber_task = asyncio.create_task(replay())
    log.info("redis event bus enabled: %s", settings.redis_url)


async def close_bus() -> None:
    global _redis_client, _subscriber_task

    if _subscriber_task is not None:
        _subscriber_task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await _subscriber_task
        _subscriber_task = None
    if _redis_client is not None:
        with contextlib.suppress(Exception):
            await _redis_client.aclose()
        _redis_client = None
    bus.delegate = MemoryEventBus(_hub)


async def _replay(raw: str) -> None:
    try:
        payload: dict[str, Any] = json.loads(raw)
    except json.JSONDecodeError:
        log.warning("invalid redis event payload")
        return
    if payload.get("scope_kind") == "control":
        if payload.get("event") == "DISCONNECT_USER":
            _hub.close_user(str(payload.get("scope_id", "")))
        return
    await _hub.dispatch(
        str(payload.get("scope_kind", "")),
        str(payload.get("scope_id", "")),
        str(payload.get("event", "")),
        payload.get("data") or {},
        payload.get("event_id"),
    )


__all__ = ["EventBus", "EventBusProxy", "MemoryEventBus", "RedisEventBus", "bus", "close_bus", "init_bus"]
