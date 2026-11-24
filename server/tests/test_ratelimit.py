from httpx import AsyncClient

from tests.conftest import register


async def test_login_rate_limit(client: AsyncClient) -> None:
    await register(client, "login@example.com")
    for _ in range(10):
        ok = await client.post(
            "/api/v1/auth/login", json={"email": "login@example.com", "password": "password123"}
        )
        assert ok.status_code == 200, ok.text

    limited = await client.post(
        "/api/v1/auth/login", json={"email": "login@example.com", "password": "password123"}
    )
    assert limited.status_code == 429
    assert limited.headers["Retry-After"]
    assert limited.json()["error"]["code"] == "RATE_LIMITED"
    assert limited.json()["error"]["retry_after"] > 0


async def test_message_rate_limit(client: AsyncClient, world: dict) -> None:
    headers = world["alice"]
    channel_id = world["channel"]["id"]

    for index in range(5):
        sent = await client.post(
            f"/api/v1/channels/{channel_id}/messages", headers=headers, json={"content": f"m{index}"}
        )
        assert sent.status_code == 201, sent.text

    limited = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=headers, json={"content": "m5"}
    )
    assert limited.status_code == 429
    assert limited.json()["error"]["retry_after"] > 0
    assert limited.headers["Retry-After"]

    history = (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=headers)).json()
    assert [item["content"] for item in history["items"]] == ["m0", "m1", "m2", "m3", "m4"]
