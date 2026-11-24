import asyncio
from datetime import UTC, datetime, timedelta

import pytest
from httpx import AsyncClient
from sqlalchemy import select, text

from app.db import SessionLocal
from app.errors import ApiError
from app.ids import new_id
from app.models import (
    Attachment,
    ChannelOverride,
    DmMessage,
    GatewayEvent,
    Invite,
    Message,
    Role,
    ServerMember,
)
from app.realtime import outbox
from app.services import retention
from app.services.permissions import assert_can_act, effective_permissions, visible_channel_ids


async def test_fts_index_tracks_channel_and_dm_messages(client: AsyncClient, world: dict) -> None:
    headers = world["alice"]
    channel_id = world["channel"]["id"]

    created = await client.post(
        f"/api/v1/channels/{channel_id}/messages",
        headers=headers,
        json={"content": "今晚讨论部署方案"},
    )
    assert created.status_code == 201, created.text
    message_id = created.json()["id"]

    async with SessionLocal() as db:
        hit = (await db.execute(text("SELECT content FROM messages_fts WHERE messages_fts MATCH '部署方'"))).all()
    assert [row[0] for row in hit] == ["今晚讨论部署方案"]

    edited = await client.patch(
        f"/api/v1/messages/{message_id}", headers=headers, json={"content": "改成复盘安排"}
    )
    assert edited.status_code == 200, edited.text
    async with SessionLocal() as db:
        assert not (await db.execute(text("SELECT 1 FROM messages_fts WHERE messages_fts MATCH '部署方'"))).all()
        assert (await db.execute(text("SELECT 1 FROM messages_fts WHERE messages_fts MATCH '复盘安'"))).all()

    async with SessionLocal() as db:
        message = await db.get(Message, message_id)
        assert message is not None
        await db.delete(message)
        await db.commit()
        assert not (await db.execute(text("SELECT 1 FROM messages_fts WHERE messages_fts MATCH '复盘安'"))).all()

    dm_id = world["dm_id"]
    async with SessionLocal() as db:
        db.add(DmMessage(id=new_id(), dm_id=dm_id, author_id=world["alice_user"]["id"], content="私信里的决策记录"))
        await db.commit()
        hit = (await db.execute(text("SELECT content FROM dm_messages_fts WHERE dm_messages_fts MATCH '决策记'"))).all()
    assert [row[0] for row in hit] == ["私信里的决策记录"]


async def test_outbox_broadcasts_only_after_commit(monkeypatch: pytest.MonkeyPatch) -> None:
    calls: list[tuple] = []

    async def fake_dispatch(scope_kind, scope_id, event, data, event_id=None):  # noqa: ANN001, ANN202
        calls.append((scope_kind, scope_id, event, data, event_id))

    monkeypatch.setattr(outbox.hub, "dispatch", fake_dispatch)

    async with SessionLocal() as db:
        event_id = outbox.queue(db, "user", "u-commit", "TEST_EVENT", {"n": 1})
        await db.commit()
    await asyncio.sleep(0)
    assert calls == [("user", "u-commit", "TEST_EVENT", {"n": 1}, event_id)]

    async with SessionLocal() as db:
        row = await db.get(GatewayEvent, event_id)
        assert row is not None and row.event == "TEST_EVENT"

    async with SessionLocal() as db:
        outbox.queue(db, "user", "u-rollback", "TEST_EVENT", {"n": 2})
        await db.rollback()
    await asyncio.sleep(0)
    assert len(calls) == 1


async def test_channel_override_drives_visibility(client: AsyncClient, world: dict) -> None:
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    alice_id = world["alice_user"]["id"]
    bob_id = world["bob_user"]["id"]

    async with SessionLocal() as db:
        db.add(ServerMember(server_id=server_id, user_id=bob_id, nickname=""))
        await db.commit()

        assert "VIEW_CHANNEL" in await effective_permissions(db, server_id, bob_id, channel_id)

        everyone = await db.scalar(select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True)))
        assert everyone is not None
        db.add(ChannelOverride(channel_id=channel_id, role_id=everyone.id, permission="VIEW_CHANNEL", state="deny"))
        await db.commit()

        assert "VIEW_CHANNEL" not in await effective_permissions(db, server_id, bob_id, channel_id)
        assert channel_id not in await visible_channel_ids(db, server_id, bob_id)
        # owner 不受频道覆盖影响
        assert channel_id in await visible_channel_ids(db, server_id, alice_id)


def test_role_hierarchy_guard() -> None:
    assert_can_act(10, 5)
    with pytest.raises(ApiError) as exc:
        assert_can_act(5, 5, "踢出")
    assert exc.value.status_code == 403


async def test_retention_purges_expired_rows(client: AsyncClient, world: dict) -> None:
    now = datetime.now(UTC)
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    alice_id = world["alice_user"]["id"]

    async with SessionLocal() as db:
        db.add(
            GatewayEvent(
                id=new_id(),
                scope_kind="server",
                scope_id=server_id,
                event="OLD",
                payload={},
                created_at=now - timedelta(hours=25),
            )
        )
        db.add(
            GatewayEvent(
                id=new_id(),
                scope_kind="server",
                scope_id=server_id,
                event="NEW",
                payload={},
                created_at=now - timedelta(hours=1),
            )
        )
        db.add(
            Invite(
                code="oldinvite",
                server_id=server_id,
                channel_id=channel_id,
                created_by=alice_id,
                expires_at=now - timedelta(days=8),
            )
        )
        db.add(
            Invite(
                code="newinvite",
                server_id=server_id,
                channel_id=channel_id,
                created_by=alice_id,
                expires_at=now + timedelta(days=1),
            )
        )
        db.add(
            Attachment(
                id=new_id(),
                owner_id=alice_id,
                filename="old.png",
                mime="image/png",
                size=1,
                storage_key="missing-old.png",
                uploaded=True,
                created_at=now - timedelta(days=2),
            )
        )
        await db.commit()

    counts = await retention.purge_once(now)
    assert counts == {"gateway_events": 1, "invites": 1, "attachments": 1}

    async with SessionLocal() as db:
        assert (await db.scalar(select(GatewayEvent).where(GatewayEvent.event == "NEW"))) is not None
        assert (await db.scalar(select(Invite).where(Invite.code == "newinvite"))) is not None
