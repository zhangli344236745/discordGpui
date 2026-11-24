from httpx import AsyncClient

from app.db import SessionLocal
from app.ids import new_id
from app.models import AutoModKeyword, AutoModRule
from tests.conftest import auth_headers


async def _create_server(client: AsyncClient, headers: dict, template: str | None = None) -> dict:
    payload: dict = {"name": "测试服务器", "is_public": True}
    if template:
        payload["template_key"] = template
    resp = await client.post("/api/v1/servers", headers=headers, json=payload)
    assert resp.status_code == 201, resp.text
    return resp.json()


async def test_server_provisioning_default(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    assert server["member_count"] == 1

    tree = await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)
    assert tree.status_code == 200
    body = tree.json()
    assert len(body["categories"]) == 2
    names = [c["name"] for c in body["channels"]]
    assert "一般" in names and "大厅" in names
    kinds = {c["name"]: c["kind"] for c in body["channels"]}
    assert kinds["大厅"] == "voice"


async def test_server_provisioning_with_template(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers, template="design")
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    names = {c["name"] for c in tree["channels"]}
    assert {"作品展示", "灵感", "作品反馈"} <= names
    forum = [c for c in tree["channels"] if c["kind"] == "forum"]
    assert forum and forum[0]["name"] == "作品反馈"


async def test_create_channel_and_duplicate(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)

    resp = await client.post(
        f"/api/v1/servers/{server['id']}/channels",
        headers=headers,
        json={"name": "新品频道"},
    )
    assert resp.status_code == 201
    channel = resp.json()
    assert channel["kind"] == "text"

    dup = await client.post(
        f"/api/v1/servers/{server['id']}/channels",
        headers=headers,
        json={"name": "新品频道"},
    )
    assert dup.status_code == 409


async def test_cannot_delete_last_text_channel(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    text_channels = [c for c in tree["channels"] if c["kind"] == "text"]
    assert len(text_channels) == 1

    resp = await client.delete(f"/api/v1/channels/{text_channels[0]['id']}", headers=headers)
    assert resp.status_code == 409


async def test_send_and_list_messages(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    sent = await client.post(
        f"/api/v1/channels/{channel_id}/messages",
        headers=headers,
        json={"content": "第一条消息"},
    )
    assert sent.status_code == 201, sent.text
    msg = sent.json()
    assert msg["content"] == "第一条消息"
    assert msg["author"]["name"] == "a_u"

    listed = await client.get(f"/api/v1/channels/{channel_id}/messages", headers=headers)
    assert listed.status_code == 200
    items = listed.json()["items"]
    assert len(items) == 1
    assert items[0]["id"] == msg["id"]


async def test_message_nonce_is_idempotent(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    payload = {"content": "重试一次", "client_nonce": "nonce-1"}
    first = await client.post(f"/api/v1/channels/{channel_id}/messages", headers=headers, json=payload)
    second = await client.post(f"/api/v1/channels/{channel_id}/messages", headers=headers, json=payload)
    assert first.json()["id"] == second.json()["id"]

    listed = await client.get(f"/api/v1/channels/{channel_id}/messages", headers=headers)
    assert len(listed.json()["items"]) == 1


async def test_automod_blocks_message(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    async with SessionLocal() as db:
        rule = AutoModRule(id=new_id(), server_id=server["id"], name="禁词", enabled=True, action="block")
        db.add(rule)
        await db.flush()
        db.add(AutoModKeyword(rule_id=rule.id, keyword="违禁词"))
        await db.commit()

    blocked = await client.post(
        f"/api/v1/channels/{channel_id}/messages",
        headers=headers,
        json={"content": "这里有违禁词"},
    )
    assert blocked.status_code == 422
    error = blocked.json()["error"]
    assert error["code"] == "AUTOMOD_BLOCKED"
    assert error["matched_keyword"] == "违禁词"

    listed = await client.get(f"/api/v1/channels/{channel_id}/messages", headers=headers)
    assert listed.json()["items"] == []


async def test_outsider_cannot_see_server(client: AsyncClient) -> None:
    owner_headers = await auth_headers(client, "owner@example.com")
    server = await _create_server(client, owner_headers)

    outsider_headers = await auth_headers(client, "outsider@example.com")
    resp = await client.get(f"/api/v1/servers/{server['id']}", headers=outsider_headers)
    assert resp.status_code == 404


async def test_non_admin_cannot_create_channel(client: AsyncClient) -> None:
    owner_headers = await auth_headers(client, "owner@example.com")
    server = await _create_server(client, owner_headers, template="study")

    # 让第二个人通过邀请之外的方式加入（走 discover join）
    member_headers = await auth_headers(client, "member@example.com")
    join = await client.post(f"/api/v1/discover/servers/{server['id']}/join", headers=member_headers)
    assert join.status_code == 200
    assert join.json()["welcome"]["require_rules"] is True

    denied = await client.post(
        f"/api/v1/servers/{server['id']}/channels",
        headers=member_headers,
        json={"name": "偷偷建的"},
    )
    assert denied.status_code == 403
    assert denied.json()["error"]["code"] == "MISSING_PERMISSION"


async def test_edit_message_writes_history(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    msg = (
        await client.post(f"/api/v1/channels/{channel_id}/messages", headers=headers, json={"content": "原文"})
    ).json()
    edited = await client.patch(f"/api/v1/messages/{msg['id']}", headers=headers, json={"content": "改后"})
    assert edited.status_code == 200
    assert edited.json()["content"] == "改后"
    assert edited.json()["edited_at"] is not None

    history = await client.get(f"/api/v1/messages/{msg['id']}/edits", headers=headers)
    assert [h["content"] for h in history.json()] == ["原文"]


async def test_upload_and_attach(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await _create_server(client, headers)
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    init = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "note.txt", "size": 5, "mime": "text/plain"},
    )
    assert init.status_code == 201
    file_id = init.json()["file_id"]

    put = await client.put(f"/api/v1/uploads/{file_id}/content", headers=headers, content=b"hello")
    assert put.status_code == 204

    sent = await client.post(
        f"/api/v1/channels/{channel_id}/messages",
        headers=headers,
        json={"content": "", "attachment_ids": [file_id]},
    )
    assert sent.status_code == 201
    body = sent.json()
    assert body["type"] == "attachment"
    assert body["attachments"][0]["filename"] == "note.txt"


async def test_stub_endpoint_returns_501(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    resp = await client.get("/api/v1/friends", headers=headers)
    assert resp.status_code == 501
    assert resp.json()["error"]["code"] == "NOT_IMPLEMENTED"
