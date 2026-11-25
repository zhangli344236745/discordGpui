"""定时清理：Gateway 事件保留 24 h、过期邀请、未挂到消息的孤儿附件。

单进程 asyncio 循环即可；多进程部署时应改为独立 worker（见 docs 风险表）。
"""

from __future__ import annotations

import asyncio
import contextlib
import logging
from datetime import UTC, datetime, timedelta

from sqlalchemy import and_, delete, or_, select

from app.config import settings
from app.db import SessionLocal
from app.models import Attachment, GatewayEvent, Invite
from app.services.storage import storage

log = logging.getLogger("server.retention")


async def purge_once(now: datetime | None = None) -> dict[str, int]:
    """执行一次清理，返回各类删除数量；now 可注入便于测试。"""
    now = now or datetime.now(UTC)
    event_cutoff = now - timedelta(hours=settings.gateway_event_retention_hours)
    invite_cutoff = now - timedelta(days=7)
    revoked_cutoff = now - timedelta(days=30)
    orphan_cutoff = now - timedelta(hours=24)

    async with SessionLocal() as db:
        events = await db.execute(delete(GatewayEvent).where(GatewayEvent.created_at < event_cutoff))
        invites = await db.execute(
            delete(Invite).where(
                or_(
                    and_(Invite.expires_at.is_not(None), Invite.expires_at < invite_cutoff),
                    and_(Invite.revoked.is_(True), Invite.created_at < revoked_cutoff),
                )
            )
        )
        orphans = list(
            (
                await db.scalars(
                    select(Attachment).where(
                        Attachment.message_id.is_(None),
                        Attachment.uploaded.is_(True),
                        Attachment.created_at < orphan_cutoff,
                    )
                )
            ).all()
        )
        for attachment in orphans:
            await asyncio.to_thread(storage.delete, attachment.storage_key)
            if attachment.thumb_storage_key:
                await asyncio.to_thread(storage.delete, attachment.thumb_storage_key)
            await db.delete(attachment)
        await db.commit()

    return {
        "gateway_events": events.rowcount or 0,
        "invites": invites.rowcount or 0,
        "attachments": len(orphans),
    }


async def run_forever() -> None:
    while True:
        try:
            counts = await purge_once()
            if any(counts.values()):
                log.info("retention purged: %s", counts)
        except asyncio.CancelledError:
            raise
        except Exception:  # pragma: no cover - 清理失败不应拖垮服务
            log.exception("retention failed")
        await asyncio.sleep(settings.retention_interval_seconds)


def start() -> asyncio.Task:
    return asyncio.create_task(run_forever())


async def stop(task: asyncio.Task) -> None:
    task.cancel()
    with contextlib.suppress(asyncio.CancelledError):
        await task


__all__ = ["purge_once", "run_forever", "start", "stop"]
