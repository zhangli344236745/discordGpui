from httpx import AsyncClient
from sqlalchemy import select

from app.db import SessionLocal
from app.models import GatewayEvent
from tests.conftest import add_member


async def test_channel_state_collapse_and_read_forward_only(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    channel_id = world["channel"]["id"]
    first = (
        await client.post(f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "一"})
    ).json()
    second = (
        await client.post(f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "二"})
    ).json()

    updated = await client.put(
        f"/api/v1/me/channel-state/{channel_id}",
        headers=alice,
        json={"collapsed": True, "notify_level": "mentions", "last_read_message_id": second["id"]},
    )
    assert updated.status_code == 200, updated.text
    assert updated.json()["collapsed"] is True
    assert updated.json()["last_read_message_id"] == second["id"]

    backwards = await client.put(
        f"/api/v1/me/channel-state/{channel_id}",
        headers=alice,
        json={"last_read_message_id": first["id"]},
    )
    assert backwards.json()["last_read_message_id"] == second["id"]

    states = (await client.get("/api/v1/me/channel-state", headers=alice)).json()
    assert states[0]["channel_id"] == channel_id and states[0]["collapsed"] is True

    invalid = await client.put(
        f"/api/v1/me/channel-state/{channel_id}", headers=alice, json={"notify_level": "sometimes"}
    )
    assert invalid.status_code == 422

    # 无权频道被隐藏
    await add_member(world["server"]["id"], world["bob_user"]["id"])
    private = (
        await client.post(
            f"/api/v1/servers/{world['server']['id']}/channels",
            headers=alice,
            json={"name": "私密状态", "kind": "text"},
        )
    ).json()
    roles = (await client.get(f"/api/v1/servers/{world['server']['id']}/roles", headers=alice)).json()
    everyone = next(role for role in roles if role["is_everyone"])
    await client.put(
        f"/api/v1/channels/{private['id']}/overrides",
        headers=alice,
        json={"items": [{"role_id": everyone["id"], "permission": "VIEW_CHANNEL", "state": "deny"}]},
    )
    denied = await client.put(
        f"/api/v1/me/channel-state/{private['id']}", headers=world["bob"], json={"collapsed": True}
    )
    assert denied.status_code == 404


async def test_drafts_upsert_clear_and_isolation(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    channel_scope = f"channel:{world['channel']['id']}"
    dm_scope = f"dm:{world['dm_id']}"

    saved = await client.put(
        f"/api/v1/me/drafts/{channel_scope}", headers=alice, json={"content": "还没发出去"}
    )
    assert saved.status_code == 200 and saved.json()["cleared"] is False

    dm_saved = await client.put(
        f"/api/v1/me/drafts/{dm_scope}", headers=alice, json={"content": "私信草稿"}
    )
    assert dm_saved.status_code == 200

    drafts = (await client.get("/api/v1/me/drafts", headers=alice)).json()
    assert {draft["scope"] for draft in drafts} == {channel_scope, dm_scope}

    # 跨设备：草稿变更进 Gateway 事件（user scope），其它设备据此同步
    async with SessionLocal() as db:
        rows = (
            await db.scalars(
                select(GatewayEvent).where(
                    GatewayEvent.scope_kind == "user",
                    GatewayEvent.event == "DRAFT_UPDATE",
                )
            )
        ).all()
    assert {row.payload.get("scope") for row in rows} == {channel_scope, dm_scope}
    assert (await client.get("/api/v1/me/drafts", headers=bob)).json() == []

    cleared = await client.put(f"/api/v1/me/drafts/{channel_scope}", headers=alice, json={"content": "   "})
    assert cleared.json()["cleared"] is True
    assert [d["scope"] for d in (await client.get("/api/v1/me/drafts", headers=alice)).json()] == [dm_scope]

    invalid = await client.put("/api/v1/me/drafts/unknown", headers=alice, json={"content": "x"})
    assert invalid.status_code == 400

    outsider = await client.post(
        "/api/v1/auth/register",
        json={"email": "draft_out@example.com", "password": "password123", "username": "draft_out"},
    )
    outsider_headers = {"Authorization": f"Bearer {outsider.json()['access_token']}"}
    assert (
        await client.put(f"/api/v1/me/drafts/{dm_scope}", headers=outsider_headers, json={"content": "x"})
    ).status_code == 404


async def test_channel_read_broadcasts_event(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    channel_id = world["channel"]["id"]
    sent = await client.post(
        f"/api/v1/channels/{channel_id}/messages",
        headers=alice,
        json={"content": "已读同步", "client_nonce": "read-sync-1"},
    )
    assert sent.status_code == 201

    async with SessionLocal() as db:
        rows = (
            await db.scalars(
                select(GatewayEvent).where(
                    GatewayEvent.scope_kind == "user",
                    GatewayEvent.event == "CHANNEL_READ",
                )
            )
        ).all()
    assert any(row.payload.get("channel_id") == channel_id for row in rows)
