"""单进程 WebSocket Hub。

订阅粒度是 server / dm：客户端连上后只收到自己订阅范围内的事件，
跨服务器的事件不会泄露。多 worker 部署时把 broadcast 换成 Redis pub/sub。
"""

import asyncio
import contextlib
from dataclasses import dataclass, field

from fastapi import WebSocket

from app.ids import new_id
from app.realtime.events import envelope


@dataclass
class Connection:
    ws: WebSocket
    user_id: str
    key: str = field(default_factory=new_id)
    servers: set[str] = field(default_factory=set)
    dms: set[str] = field(default_factory=set)
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)

    async def send(self, event: str, data: dict, event_id: str | None = None) -> None:
        async with self.lock:
            with contextlib.suppress(Exception):
                await self.ws.send_text(envelope(event, data, event_id))


class Hub:
    def __init__(self) -> None:
        self._conns: dict[str, Connection] = {}

    def add(self, conn: Connection) -> None:
        self._conns[conn.key] = conn

    def remove(self, conn: Connection) -> None:
        self._conns.pop(conn.key, None)

    @property
    def size(self) -> int:
        return len(self._conns)

    def close_user(self, user_id: str) -> int:
        """撤销会话时踢掉该用户的全部 Gateway 连接；返回关闭数量。"""
        targets = [conn for conn in self._all() if conn.user_id == user_id]
        for conn in targets:
            self._conns.pop(conn.key, None)
            # WebSocket 关闭由 gateway 的接收循环感知；这里只摘除订阅
        return len(targets)

    def _all(self) -> list[Connection]:
        return list(self._conns.values())

    async def to_user(self, user_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        for conn in self._all():
            if conn.user_id == user_id:
                await conn.send(event, data, event_id)

    async def to_server(self, server_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        for conn in self._all():
            if server_id in conn.servers:
                await conn.send(event, data, event_id)

    async def to_dm(self, dm_id: str, event: str, data: dict, event_id: str | None = None) -> None:
        for conn in self._all():
            if dm_id in conn.dms:
                await conn.send(event, data, event_id)

    async def dispatch(
        self,
        scope_kind: str,
        scope_id: str,
        event: str,
        data: dict,
        event_id: str | None = None,
    ) -> None:
        """统一出口：按 outbox 登记的作用域投递，事件 id 由 outbox 提供。"""
        if scope_kind == "server":
            await self.to_server(scope_id, event, data, event_id)
        elif scope_kind == "dm":
            await self.to_dm(scope_id, event, data, event_id)
        elif scope_kind == "user":
            await self.to_user(scope_id, event, data, event_id)
        else:
            raise ValueError(f"unknown scope: {scope_kind}")


hub = Hub()
