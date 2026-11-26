from httpx import AsyncClient

from tests.conftest import me, register


async def _new_user(client: AsyncClient, email: str) -> tuple[dict, dict]:
    tokens = await register(client, email)
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}
    return headers, await me(client, headers)


async def test_dm_message_nonce_unread_and_patch(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    dm_id = world["dm_id"]

    first = await client.post(
        f"/api/v1/dms/{dm_id}/messages",
        headers=alice,
        json={"content": "在吗", "client_nonce": "nonce-1"},
    )
    assert first.status_code == 201, first.text
    retry = await client.post(
        f"/api/v1/dms/{dm_id}/messages",
        headers=alice,
        json={"content": "在吗", "client_nonce": "nonce-1"},
    )
    assert retry.status_code == 201 and retry.json()["id"] == first.json()["id"]

    history = (await client.get(f"/api/v1/dms/{dm_id}/messages", headers=bob)).json()
    assert [m["id"] for m in history["items"]] == [first.json()["id"]]

    bob_list = (await client.get("/api/v1/dms", headers=bob)).json()
    assert bob_list[0]["unread"] == 1
    alice_list = (await client.get("/api/v1/dms", headers=alice)).json()
    assert alice_list[0]["unread"] == 0

    read = await client.patch(f"/api/v1/dms/{dm_id}/read", headers=bob, json={})
    assert read.status_code == 200
    assert (await client.get("/api/v1/dms", headers=bob)).json()[0]["unread"] == 0

    pinned = await client.patch(f"/api/v1/dms/{dm_id}", headers=alice, json={"pinned": True})
    assert pinned.status_code == 200 and pinned.json()["pinned"] is True
    assert (await client.get("/api/v1/dms", headers=bob)).json()[0]["pinned"] is False

    closed = await client.patch(f"/api/v1/dms/{dm_id}", headers=alice, json={"closed": True})
    assert closed.json()["closed"] is True
    reopened = await client.post("/api/v1/dms", headers=bob, json={"user_id": world["alice_user"]["id"]})
    assert reopened.status_code in (200, 201)
    assert reopened.json()["dm"]["closed"] is False


async def test_dm_isolation_and_validation(client: AsyncClient, world: dict) -> None:
    dm_id = world["dm_id"]
    carol, _carol_user = await _new_user(client, "carol@example.com")

    outsider = await client.get(f"/api/v1/dms/{dm_id}/messages", headers=carol)
    assert outsider.status_code == 404

    empty = await client.post(f"/api/v1/dms/{dm_id}/messages", headers=world["alice"], json={"content": "   "})
    assert empty.status_code == 400

    self_dm = await client.post(
        "/api/v1/dms", headers=world["alice"], json={"user_id": world["alice_user"]["id"]}
    )
    assert self_dm.status_code == 400


async def test_stranger_message_request_flow(client: AsyncClient, world: dict) -> None:
    bob = world["bob"]
    carol, carol_user = await _new_user(client, "carol@example.com")

    opened = await client.post(
        "/api/v1/dms", headers=carol, json={"user_id": world["bob_user"]["id"], "first_message": "你好，我是 Carol"}
    )
    assert opened.status_code == 201, opened.text
    body = opened.json()
    assert body["kind"] == "request"
    request_id = body["request"]["id"]

    inbox = (await client.get("/api/v1/message-requests", headers=bob)).json()
    assert inbox["count"] == 1
    assert inbox["items"][0]["from_user"]["id"] == carol_user["id"]

    accepted = await client.post(f"/api/v1/message-requests/{request_id}/accept", headers=bob)
    assert accepted.status_code == 200, accepted.text
    dm_id = accepted.json()["dm"]["id"]
    assert accepted.json()["dm"]["last_message"]["content"] == "你好，我是 Carol"

    history = (await client.get(f"/api/v1/dms/{dm_id}/messages", headers=carol)).json()
    assert [m["content"] for m in history["items"]] == ["你好，我是 Carol"]

    again = await client.post(f"/api/v1/message-requests/{request_id}/accept", headers=bob)
    assert again.status_code == 409

    # 已有 DM 后不再是陌生人，直接复用
    direct = await client.post("/api/v1/dms", headers=carol, json={"user_id": world["bob_user"]["id"]})
    assert direct.json()["kind"] == "dm" and direct.json()["dm"]["id"] == dm_id


async def test_message_request_ignore_and_delete(client: AsyncClient, world: dict) -> None:
    bob = world["bob"]
    dave, dave_user = await _new_user(client, "dave@example.com")
    erin, _erin_user = await _new_user(client, "erin@example.com")

    first = await client.post(
        "/api/v1/dms", headers=dave, json={"user_id": world["bob_user"]["id"], "first_message": "hi"}
    )
    request_id = first.json()["request"]["id"]
    ignored = await client.post(f"/api/v1/message-requests/{request_id}/ignore", headers=bob)
    assert ignored.status_code == 200 and ignored.json()["state"] == "ignored"

    ignored_list = (await client.get("/api/v1/message-requests?state=ignored", headers=bob)).json()
    assert [r["id"] for r in ignored_list["items"]] == [request_id]
    assert ignored_list["count"] == 0

    late_accept = await client.post(f"/api/v1/message-requests/{request_id}/accept", headers=bob)
    assert late_accept.status_code == 409

    second = await client.post(
        "/api/v1/dms", headers=erin, json={"user_id": world["bob_user"]["id"], "first_message": "hello"}
    )
    second_id = second.json()["request"]["id"]
    deleted = await client.delete(f"/api/v1/message-requests/{second_id}", headers=bob)
    assert deleted.status_code == 200
    assert (await client.get("/api/v1/message-requests", headers=bob)).json()["items"] == []

    reverse = await client.post(
        "/api/v1/dms", headers=dave, json={"user_id": world["bob_user"]["id"], "first_message": "again"}
    )
    assert reverse.status_code == 201
    # 反向 pending 会被拒绝
    bob_to_dave = await client.post(
        "/api/v1/dms", headers=bob, json={"user_id": dave_user["id"], "first_message": "back"}
    )
    assert bob_to_dave.status_code == 409



async def test_dm_read_broadcasts_event(client: AsyncClient, world: dict) -> None:
    from sqlalchemy import select

    from app.db import SessionLocal
    from app.models import GatewayEvent

    dm_id = world["dm_id"]
    bob = world["bob"]
    sent = await client.post(
        f"/api/v1/dms/{dm_id}/messages",
        headers=bob,
        json={"content": "私信已读", "client_nonce": "dm-read-1"},
    )
    assert sent.status_code == 201

    async with SessionLocal() as db:
        rows = (
            await db.scalars(
                select(GatewayEvent).where(
                    GatewayEvent.scope_kind == "user",
                    GatewayEvent.event == "DM_READ",
                )
            )
        ).all()
    assert any(row.payload.get("dm_id") == dm_id for row in rows)
