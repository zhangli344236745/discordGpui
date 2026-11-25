"""P4 机器人与集成：Bot 账号 / Webhook / 斜杠命令。"""

from httpx import AsyncClient

from tests.conftest import auth_headers, create_server, first_channel, register


def _bot_headers(token: str) -> dict:
    return {"Authorization": f"Bot {token}"}


async def test_bot_lifecycle_and_messages(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)

    created = await client.post(
        f"/api/v1/servers/{server['id']}/bots", headers=headers, json={"name": "EchoBot"}
    )
    assert created.status_code == 201, created.text
    bot = created.json()
    assert bot["token"]

    listed = await client.get(f"/api/v1/servers/{server['id']}/bots", headers=headers)
    assert any(item["id"] == bot["id"] for item in listed.json())

    tree = await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)
    channel = next(c for c in tree.json()["channels"] if c["kind"] == "text")

    sent = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=_bot_headers(bot["token"]),
        json={"content": "你好，我是机器人", "client_nonce": "bot-1"},
    )
    assert sent.status_code == 201, sent.text
    assert sent.json()["author"]["bot"] is True
    assert sent.json()["author"]["name"] == "EchoBot"

    # 撤销后 401
    revoked = await client.delete(f"/api/v1/servers/{server['id']}/bots/{bot['id']}", headers=headers)
    assert revoked.status_code == 204
    denied = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=_bot_headers(bot["token"]),
        json={"content": "should fail"},
    )
    assert denied.status_code == 401

    # bot 令牌伪造
    fake = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=_bot_headers("not-a-real-token"),
        json={"content": "x"},
    )
    assert fake.status_code == 401


async def test_member_without_manage_server_cannot_create_bot(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    other_tokens = await register(client, "botless@example.com")
    other = {"Authorization": f"Bearer {other_tokens['access_token']}"}

    # 不是成员直接 404
    resp = await client.post(
        f"/api/v1/servers/{server['id']}/bots", headers=other, json={"name": "Nope"}
    )
    assert resp.status_code == 404


async def test_webhook_external_post_and_revoke(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])

    created = await client.post(
        f"/api/v1/servers/{server['id']}/webhooks",
        headers=headers,
        json={"name": "CI 通知", "channel_id": channel["id"]},
    )
    assert created.status_code == 201, created.text
    webhook = created.json()
    assert webhook["url"].endswith(webhook["token"])

    # 无需登录即可投递
    posted = await client.post(
        f"/api/v1/webhooks/{webhook['id']}/{webhook['token']}",
        json={"content": "构建成功", "username": "CI"},
    )
    assert posted.status_code == 201, posted.text
    assert posted.json()["content"] == "构建成功"
    assert posted.json()["author"]["bot"] is True

    # 错误 token 404，不泄露存在性
    wrong = await client.post(
        f"/api/v1/webhooks/{webhook['id']}/wrong-token",
        json={"content": "x"},
    )
    assert wrong.status_code == 404

    # 撤销后 404
    revoked = await client.delete(
        f"/api/v1/servers/{server['id']}/webhooks/{webhook['id']}", headers=headers
    )
    assert revoked.status_code == 204
    after = await client.post(
        f"/api/v1/webhooks/{webhook['id']}/{webhook['token']}",
        json={"content": "x"},
    )
    assert after.status_code == 404


async def test_slash_commands_register_run_and_permissions(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])

    created = await client.post(
        f"/api/v1/servers/{server['id']}/commands",
        headers=headers,
        json={"name": "deploy", "description": "部署提示", "kind": "custom", "response": "开始部署 {args}"},
    )
    assert created.status_code == 201, created.text

    duplicate = await client.post(
        f"/api/v1/servers/{server['id']}/commands",
        headers=headers,
        json={"name": "deploy", "kind": "custom", "response": "x"},
    )
    assert duplicate.status_code == 409

    listed = await client.get(f"/api/v1/servers/{server['id']}/commands", headers=headers)
    names = {item["name"] for item in listed.json()}
    assert {"deploy", "help", "shrug"} <= names

    run = await client.post(
        f"/api/v1/channels/{channel['id']}/commands/deploy",
        headers=headers,
        json={"args": "prod"},
    )
    assert run.status_code == 200, run.text
    assert run.json()["response"] == "开始部署 prod"

    help_run = await client.post(
        f"/api/v1/channels/{channel['id']}/commands/help", headers=headers, json={"args": ""}
    )
    assert help_run.status_code == 200
    assert "/deploy" in help_run.json()["response"]

    shrug = await client.post(
        f"/api/v1/channels/{channel['id']}/commands/shrug",
        headers=headers,
        json={"args": "就这样"},
    )
    assert shrug.status_code == 200
    assert shrug.json()["response"].endswith("¯\\_(ツ)_/¯")

    unknown = await client.post(
        f"/api/v1/channels/{channel['id']}/commands/nope", headers=headers, json={"args": ""}
    )
    assert unknown.status_code == 404
