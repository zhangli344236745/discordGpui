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
from app.models import Attachment, Channel, GatewayEvent, Invite, LegalHold, Message, RetentionRule
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
        expired = await _apply_retention_rules(db, now)

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
        "retention_messages": expired,
    }


async def _apply_retention_rules(db, now: datetime) -> int:
    """按服务器保留策略软删过期消息；法务持有优先，跳过被持有的目标。"""
    rules = list((await db.scalars(select(RetentionRule).where(RetentionRule.message_days > 0))).all())
    if not rules:
        return 0

    total = 0
    for rule in rules:
        cutoff = now - timedelta(days=rule.message_days)
        holds = list(
            (
                await db.execute(
                    select(LegalHold.target_kind, LegalHold.target_id).where(
                        LegalHold.server_id == rule.server_id
                    )
                )
            ).all()
        )
        held_messages = {target for kind, target in holds if kind == "message"}
        held_channels = {target for kind, target in holds if kind == "channel"}
        held_users = {target for kind, target in holds if kind == "user"}

        rows = list(
            (
                await db.scalars(
                    select(Message)
                    .join(Channel, Message.channel_id == Channel.id)
                    .where(
                        Channel.server_id == rule.server_id,
                        Message.deleted_at.is_(None),
                        Message.created_at < cutoff,
                    )
                    .limit(5000)
                )
            ).all()
        )
        for message in rows:
            if (
                message.id in held_messages
                or message.channel_id in held_channels
                or message.author_id in held_users
            ):
                continue
            message.deleted_at = now
            total += 1
    if total:
        await db.commit()
    return total


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
