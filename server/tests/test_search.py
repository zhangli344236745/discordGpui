import pytest
from httpx import AsyncClient

from tests.conftest import add_member


async def _say(client: AsyncClient, headers: dict, channel_id: str, content: str) -> dict:
    resp = await client.post(f"/api/v1/channels/{channel_id}/messages", headers=headers, json={"content": content})
    assert resp.status_code == 201, resp.text
    return resp.json()


async def test_global_search_scope_operators_and_permission(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    channel_id = world["channel"]["id"]
    server_id = world["server"]["id"]
    await add_member(server_id, world["bob_user"]["id"])

    first = await _say(client, alice, channel_id, "搜索测试关键词")
    link = await _say(client, alice, channel_id, "部署记录 https://example.com")
    bob_message = await _say(client, bob, channel_id, "部署记录 由 bob 发")

    found = (await client.get("/api/v1/search?q=搜索测试", headers=bob)).json()
    assert [item["message_id"] for item in found["items"]] == [first["id"]]
    assert found["items"][0]["highlight"]["match"] == "搜索测试"

    by_author = (await client.get("/api/v1/search?q=部署记录%20from:bob_u", headers=alice)).json()
    assert [item["message_id"] for item in by_author["items"]] == [bob_message["id"]]

    with_link = (await client.get("/api/v1/search?q=部署记录%20has:link", headers=alice)).json()
    assert [item["message_id"] for item in with_link["items"]] == [link["id"]]

    # 短查询走 LIKE
    short = (await client.get("/api/v1/search?q=搜索", headers=bob)).json()
    assert any(item["message_id"] == first["id"] for item in short["items"])

    # DM 搜索
    dm_message = (
        await client.post(
            f"/api/v1/dms/{world['dm_id']}/messages", headers=alice, json={"content": "私信搜索词"}
        )
    ).json()
    dm_found = (await client.get("/api/v1/search?q=私信搜索词&scope=dm", headers=alice)).json()
    assert [item["message_id"] for item in dm_found["items"]] == [dm_message["id"]]

    # 频道内搜索与无权频道
    channel_found = (await client.get(f"/api/v1/channels/{channel_id}/search?q=搜索测试", headers=bob)).json()
    assert [item["message_id"] for item in channel_found["items"]] == [first["id"]]

    private = (
        await client.post(
            f"/api/v1/servers/{server_id}/channels", headers=alice, json={"name": "秘密", "kind": "text"}
        )
    ).json()
    roles = (await client.get(f"/api/v1/servers/{server_id}/roles", headers=alice)).json()
    everyone = next(role for role in roles if role["is_everyone"])
    await client.put(
        f"/api/v1/channels/{private['id']}/overrides",
        headers=alice,
        json={"items": [{"role_id": everyone["id"], "permission": "VIEW_CHANNEL", "state": "deny"}]},
    )
    secret = await _say(client, alice, private["id"], "秘密搜索内容")
    hidden = (await client.get("/api/v1/search?q=秘密搜索", headers=bob)).json()
    assert all(item["message_id"] != secret["id"] for item in hidden["items"])
    denied_channel = await client.get(f"/api/v1/channels/{private['id']}/search?q=秘密", headers=bob)
    assert denied_channel.status_code == 404


async def test_search_validation_and_suggest(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    assert (await client.get("/api/v1/search?q=", headers=alice)).status_code == 400
    assert (await client.get("/api/v1/search/suggest?q=", headers=alice)).status_code == 400

    suggestions = (await client.get("/api/v1/search/suggest?q=一", headers=alice)).json()
    assert any(item["type"] == "channel" for item in suggestions)
    assert all(item["type"] in ("channel", "user", "command") for item in suggestions)

    commands = (await client.get("/api/v1/search/suggest?q=/gif", headers=alice)).json()
    assert any(item["type"] == "command" and item["id"] == "/gif" for item in commands)

    # 非成员访问频道搜索
    outsider = await client.post(
        "/api/v1/auth/register",
        json={"email": "search_out@example.com", "password": "password123", "username": "search_out"},
    )
    headers = {"Authorization": f"Bearer {outsider.json()['access_token']}"}
    resp = await client.get(f"/api/v1/channels/{world['channel']['id']}/search?q=测试", headers=headers)
    assert resp.status_code == 404


@pytest.mark.parametrize("q", ["测", "测试"])
async def test_short_query_fallback(client: AsyncClient, world: dict, q: str) -> None:
    headers = world["alice"]
    message = await _say(client, headers, world["channel"]["id"], "测试短查询")
    found = (await client.get(f"/api/v1/search?q={q}", headers=headers)).json()
    assert any(item["message_id"] == message["id"] for item in found["items"])
