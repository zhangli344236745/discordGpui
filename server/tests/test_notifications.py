from httpx import AsyncClient

from tests.conftest import add_member


async def _counts(client: AsyncClient, headers: dict) -> dict:
    return (await client.get("/api/v1/notifications/unread-counts", headers=headers)).json()


async def test_dm_and_mention_notifications(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    await add_member(server_id, world["bob_user"]["id"])

    await client.post(f"/api/v1/dms/{world['dm_id']}/messages", headers=alice, json={"content": "私信通知"})
    dm_notifications = (await client.get("/api/v1/notifications?kind=dm", headers=bob)).json()
    assert len(dm_notifications["items"]) == 1
    assert dm_notifications["items"][0]["target_id"] == world["dm_id"]
    assert (await _counts(client, bob))["dms"][0]["unread"] == 1

    await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "@bob_u 来看这个"}
    )
    mention = (await client.get("/api/v1/notifications?kind=mention", headers=bob)).json()
    assert len(mention["items"]) == 1 and mention["items"][0]["target_id"] == channel_id

    counts = await _counts(client, bob)
    assert counts["mentions"] == 1
    assert any(item["channel_id"] == channel_id and item["unread"] >= 1 for item in counts["channels"])

    marked = await client.post("/api/v1/notifications/read", headers=bob, json={"all": True})
    assert marked.status_code == 200 and marked.json()["updated"] >= 2
    assert (await _counts(client, bob))["mentions"] == 0


async def test_friend_and_message_request_notifications(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    carol_tokens = await client.post(
        "/api/v1/auth/register",
        json={"email": "carol_n@example.com", "password": "password123", "username": "carol_n"},
    )
    carol = {"Authorization": f"Bearer {carol_tokens.json()['access_token']}"}

    carol_me = (await client.get("/api/v1/users/@me", headers=carol)).json()
    await client.post(
        "/api/v1/friends/requests",
        headers=alice,
        json={"username": "carol_n", "tag": carol_me["tag"]},
    )
    friend_notifications = (await client.get("/api/v1/notifications?kind=friend_request", headers=carol)).json()
    assert len(friend_notifications["items"]) == 1
    assert (await _counts(client, carol))["friend_requests"] == 1

    request = await client.post(
        "/api/v1/dms",
        headers=carol,
        json={"user_id": world["bob_user"]["id"], "first_message": "想认识一下"},
    )
    assert request.json()["kind"] == "request"
    message_notifications = (await client.get("/api/v1/notifications?kind=message_request", headers=bob)).json()
    assert len(message_notifications["items"]) == 1


async def test_mark_read_specific_and_push_subscribe(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    await add_member(world["server"]["id"], world["bob_user"]["id"])
    await client.post(f"/api/v1/dms/{world['dm_id']}/messages", headers=alice, json={"content": "通知一"})
    await client.post(f"/api/v1/dms/{world['dm_id']}/messages", headers=alice, json={"content": "通知二"})

    items = (await client.get("/api/v1/notifications?kind=dm", headers=bob)).json()["items"]
    assert len(items) == 2
    marked = await client.post("/api/v1/notifications/read", headers=bob, json={"ids": [items[0]["id"]]})
    assert marked.json()["updated"] == 1
    remaining = (await client.get("/api/v1/notifications?kind=dm&unread=true", headers=bob)).json()["items"]
    assert [item["id"] for item in remaining] == [items[1]["id"]]

    crossed = await client.post(
        "/api/v1/notifications/read", headers=alice, json={"ids": [items[1]["id"]]}
    )
    assert crossed.json()["updated"] == 0

    push = await client.post(
        "/api/v1/push/subscribe",
        headers=alice,
        json={"endpoint": "https://push.example.com/sub/1", "keys": {"p256dh": "x"}},
    )
    assert push.status_code == 200 and push.json()["count"] == 1
    duplicate = await client.post(
        "/api/v1/push/subscribe", headers=alice, json={"endpoint": "https://push.example.com/sub/1"}
    )
    assert duplicate.json()["count"] == 1
    invalid = await client.post("/api/v1/push/subscribe", headers=alice, json={"endpoint": "ftp://nope"})
    assert invalid.status_code == 400


async def test_channel_read_clears_unread(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    channel_id = world["channel"]["id"]
    await add_member(world["server"]["id"], world["bob_user"]["id"])

    message = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "未读消息"}
    )
    counts = await _counts(client, bob)
    assert any(item["channel_id"] == channel_id for item in counts["channels"])

    await client.put(
        f"/api/v1/me/channel-state/{channel_id}",
        headers=bob,
        json={"last_read_message_id": message.json()["id"]},
    )
    counts = await _counts(client, bob)
    assert not any(item["channel_id"] == channel_id for item in counts["channels"])
