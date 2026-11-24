from httpx import AsyncClient

from tests.conftest import me, register


async def _new_user(client: AsyncClient, email: str) -> tuple[dict, dict]:
    tokens = await register(client, email)
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}
    return headers, await me(client, headers)


async def test_friend_request_lifecycle(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    bob_user = world["bob_user"]

    created = await client.post(
        "/api/v1/friends/requests",
        headers=alice,
        json={"username": "bob_u", "tag": bob_user["tag"], "message": "一起玩"},
    )
    assert created.status_code == 201, created.text
    request_id = created.json()["id"]
    assert created.json()["state"] == "pending"

    duplicate = await client.post(
        "/api/v1/friends/requests",
        headers=alice,
        json={"username": "bob_u", "tag": bob_user["tag"]},
    )
    assert duplicate.status_code == 409

    inbox = (await client.get("/api/v1/friends/requests", headers=bob)).json()
    assert inbox["count"] == 1
    assert inbox["items"][0]["from_user"]["id"] == world["alice_user"]["id"]

    accepted = await client.post(f"/api/v1/friends/requests/{request_id}/accept", headers=bob)
    assert accepted.status_code == 200, accepted.text
    assert accepted.json()["state"] == "accepted"

    alice_friends = (await client.get("/api/v1/friends", headers=alice)).json()
    bob_friends = (await client.get("/api/v1/friends", headers=bob)).json()
    assert [f["id"] for f in alice_friends] == [world["bob_user"]["id"]]
    assert [f["id"] for f in bob_friends] == [world["alice_user"]["id"]]

    again = await client.post(f"/api/v1/friends/requests/{request_id}/accept", headers=bob)
    assert again.status_code == 409

    removed = await client.delete(f"/api/v1/friends/{world['bob_user']['id']}", headers=alice)
    assert removed.status_code == 200
    assert (await client.get("/api/v1/friends", headers=alice)).json() == []


async def test_friend_request_self_and_reverse(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    alice_user = world["alice_user"]

    self_request = await client.post(
        "/api/v1/friends/requests",
        headers=alice,
        json={"username": "alice_u", "tag": alice_user["tag"]},
    )
    assert self_request.status_code in (400, 404)

    first = await client.post(
        "/api/v1/friends/requests", headers=alice, json={"username": "bob_u", "tag": world["bob_user"]["tag"]}
    )
    assert first.status_code == 201
    reverse = await client.post(
        "/api/v1/friends/requests", headers=bob, json={"username": "alice_u", "tag": alice_user["tag"]}
    )
    assert reverse.status_code == 409

    rejected_id = first.json()["id"]
    rejected = await client.post(f"/api/v1/friends/requests/{rejected_id}/reject", headers=bob)
    assert rejected.status_code == 200
    assert rejected.json()["state"] == "rejected"
    assert (await client.get("/api/v1/friends", headers=alice)).json() == []


async def test_friend_groups_and_move(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    bob_id = world["bob_user"]["id"]

    # 先成为好友
    created = await client.post(
        "/api/v1/friends/requests", headers=alice, json={"username": "bob_u", "tag": world["bob_user"]["tag"]}
    )
    await client.post(f"/api/v1/friends/requests/{created.json()['id']}/accept", headers=bob)

    group = await client.post("/api/v1/friend-groups", headers=alice, json={"name": "同事"})
    assert group.status_code == 201, group.text
    group_id = group.json()["id"]

    duplicate = await client.post("/api/v1/friend-groups", headers=alice, json={"name": "同事"})
    assert duplicate.status_code == 409

    moved = await client.patch(f"/api/v1/friends/{bob_id}", headers=alice, json={"group_id": group_id})
    assert moved.status_code == 200
    groups = (await client.get("/api/v1/friend-groups", headers=alice)).json()
    assert groups[0]["count"] == 1

    bad_order = await client.patch("/api/v1/friend-groups/order", headers=alice, json={"ids": ["missing"]})
    assert bad_order.status_code == 400

    second = await client.post("/api/v1/friend-groups", headers=alice, json={"name": "朋友"})
    second_id = second.json()["id"]
    ordered = await client.patch(
        "/api/v1/friend-groups/order", headers=alice, json={"ids": [second_id, group_id]}
    )
    assert ordered.status_code == 200
    assert [g["id"] for g in ordered.json()] == [second_id, group_id]

    renamed = await client.patch(f"/api/v1/friend-groups/{group_id}", headers=alice, json={"name": "同事组"})
    assert renamed.status_code == 200 and renamed.json()["name"] == "同事组"

    deleted = await client.delete(f"/api/v1/friend-groups/{group_id}", headers=alice)
    assert deleted.status_code == 200
    friends = (await client.get("/api/v1/friends", headers=alice)).json()
    assert friends[0]["group_id"] is None


async def test_block_effects(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    alice_id, bob_id = world["alice_user"]["id"], world["bob_user"]["id"]
    dm_id = world["dm_id"]

    # 先成为好友，才能观察屏蔽后的列表过滤
    request = await client.post(
        "/api/v1/friends/requests", headers=alice, json={"username": "bob_u", "tag": world["bob_user"]["tag"]}
    )
    await client.post(f"/api/v1/friends/requests/{request.json()['id']}/accept", headers=bob)

    blocked = await client.post("/api/v1/blocks", headers=bob, json={"user_id": alice_id, "reason": "测试"})
    assert blocked.status_code == 201, blocked.text
    assert blocked.json()["blocked"] is True

    hidden = (await client.get("/api/v1/friends", headers=bob)).json()
    assert hidden == []

    dm_blocked = await client.post(f"/api/v1/dms/{dm_id}/messages", headers=alice, json={"content": "hi"})
    assert dm_blocked.status_code == 403

    request_blocked = await client.post(
        "/api/v1/friends/requests", headers=alice, json={"username": "bob_u", "tag": world["bob_user"]["tag"]}
    )
    assert request_blocked.status_code == 404

    duplicate = await client.post("/api/v1/blocks", headers=bob, json={"user_id": alice_id})
    assert duplicate.status_code == 201 and duplicate.json()["blocked"] is True

    listed = (await client.get("/api/v1/blocks", headers=bob)).json()
    assert [b["user_id"] for b in listed] == [alice_id]

    self_block = await client.post("/api/v1/blocks", headers=bob, json={"user_id": bob_id})
    assert self_block.status_code == 400

    await client.delete(f"/api/v1/blocks/{alice_id}", headers=bob)
    assert (await client.get("/api/v1/blocks", headers=bob)).json() == []
    restored = (await client.get("/api/v1/friends", headers=bob)).json()
    assert [f["id"] for f in restored] == [alice_id]
