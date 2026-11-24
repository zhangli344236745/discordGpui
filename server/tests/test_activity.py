from datetime import UTC, datetime, timedelta

from httpx import AsyncClient

from tests.conftest import add_member


async def test_events_crud_rsvp_and_status(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    await add_member(server_id, world["bob_user"]["id"])

    starts = datetime.now(UTC) + timedelta(hours=2)
    created = await client.post(
        f"/api/v1/servers/{server_id}/events",
        headers=alice,
        json={
            "title": "周会",
            "description": "同步进度",
            "category": "meeting",
            "channel_id": channel_id,
            "starts_at": starts.isoformat(),
            "duration_minutes": 60,
        },
    )
    assert created.status_code == 201, created.text
    event = created.json()
    assert event["status"] == "upcoming" and event["going_count"] == 0

    denied = await client.post(
        f"/api/v1/servers/{server_id}/events",
        headers=bob,
        json={"title": "越权", "starts_at": starts.isoformat()},
    )
    assert denied.status_code == 403

    listed = (await client.get(f"/api/v1/servers/{server_id}/events?status=upcoming", headers=bob)).json()
    assert [item["id"] for item in listed] == [event["id"]]

    rsvp = await client.put(f"/api/v1/events/{event['id']}/rsvp", headers=bob, json={"going": True})
    assert rsvp.status_code == 200 and rsvp.json()["going"] is True
    assert rsvp.json()["going_count"] == 1
    duplicate = await client.put(f"/api/v1/events/{event['id']}/rsvp", headers=bob, json={"going": True})
    assert duplicate.json()["going_count"] == 1

    attendees = (await client.get(f"/api/v1/events/{event['id']}/attendees", headers=alice)).json()
    assert [a["user_id"] for a in attendees] == [world["bob_user"]["id"]]

    cancelled = await client.put(f"/api/v1/events/{event['id']}/rsvp", headers=bob, json={"going": False})
    assert cancelled.json()["going_count"] == 0

    patched = await client.patch(f"/api/v1/events/{event['id']}", headers=alice, json={"title": "周会（改）"})
    assert patched.status_code == 200 and patched.json()["title"] == "周会（改）"
    assert (await client.patch(f"/api/v1/events/{event['id']}", headers=bob, json={"title": "x"})).status_code == 403

    deleted = await client.delete(f"/api/v1/events/{event['id']}", headers=alice)
    assert deleted.status_code == 200
    assert (await client.get(f"/api/v1/servers/{server_id}/events", headers=alice)).json() == []


async def test_welcome_config_accept_and_read(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    await add_member(server_id, world["bob_user"]["id"])

    config = (await client.get(f"/api/v1/servers/{server_id}/welcome", headers=bob)).json()
    assert config["welcome"] is not None and config["accepted"] is False
    assert len(config["welcome"]["rules"]) >= 1

    rejected = await client.post(
        f"/api/v1/servers/{server_id}/welcome/accept", headers=bob, json={"rule_agreed": False, "channels": []}
    )
    assert rejected.status_code == 400

    accepted = await client.post(
        f"/api/v1/servers/{server_id}/welcome/accept",
        headers=bob,
        json={"rule_agreed": True, "channels": [channel_id]},
    )
    assert accepted.status_code == 200, accepted.text
    state = (await client.get(f"/api/v1/servers/{server_id}/welcome/state", headers=bob)).json()
    assert state["accepted"] is True

    denied = await client.patch(
        f"/api/v1/servers/{server_id}/welcome", headers=bob, json={"headline": "越权"}
    )
    assert denied.status_code == 403

    patched = await client.patch(
        f"/api/v1/servers/{server_id}/welcome",
        headers=alice,
        json={
            "headline": "新标题",
            "require_rules": False,
            "rules": [{"title": "规则一", "body": "内容"}],
            "recommended": [channel_id],
        },
    )
    assert patched.status_code == 200, patched.text
    assert patched.json()["welcome"]["headline"] == "新标题"
    assert patched.json()["welcome"]["recommended"] == [channel_id]

    message = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "欢迎消息"}
    )
    read = await client.post(f"/api/v1/channels/{channel_id}/welcome-read", headers=bob)
    assert read.status_code == 200 and read.json()["last_read_message_id"] == message.json()["id"]


async def test_boosts_levels_toggle_and_subscribe(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    await add_member(server_id, world["bob_user"]["id"])

    initial = (await client.get(f"/api/v1/servers/{server_id}/boosts", headers=alice)).json()
    assert initial["count"] == 0 and initial["level"] == 0 and initial["boosted"] is False

    tiers = (await client.get(f"/api/v1/servers/{server_id}/boost/tiers", headers=alice)).json()
    assert [tier["threshold"] for tier in tiers] == [2, 7, 14]

    first = await client.put(f"/api/v1/servers/{server_id}/boosts/me", headers=alice, json={"active": True})
    assert first.status_code == 200
    assert first.json()["count"] == 1 and first.json()["boosted"] is True
    again = await client.put(f"/api/v1/servers/{server_id}/boosts/me", headers=alice, json={"active": True})
    assert again.json()["count"] == 1

    second = await client.put(f"/api/v1/servers/{server_id}/boosts/me", headers=bob, json={"active": True})
    assert second.json()["count"] == 2 and second.json()["level"] == 1
    assert {b["user_id"] for b in second.json()["boosters"]} == {
        world["alice_user"]["id"],
        world["bob_user"]["id"],
    }

    removed = await client.put(f"/api/v1/servers/{server_id}/boosts/me", headers=bob, json={"active": False})
    assert removed.json()["count"] == 1 and removed.json()["level"] == 0

    subscribe = await client.post(f"/api/v1/servers/{server_id}/boost/subscribe", headers=alice)
    assert subscribe.status_code == 501
    assert subscribe.json()["error"]["code"] == "NOT_IMPLEMENTED"
