from httpx import AsyncClient

from tests.conftest import auth_headers, register


async def test_health(client: AsyncClient) -> None:
    resp = await client.get("/api/v1/health")
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


async def test_register_login_and_me(client: AsyncClient) -> None:
    tokens = await register(client)
    assert tokens["token_type"] == "Bearer"
    assert tokens["expires_in"] == 900

    me = await client.get("/api/v1/users/@me", headers={"Authorization": f"Bearer {tokens['access_token']}"})
    assert me.status_code == 200
    body = me.json()
    assert body["username"] == "a_u"
    assert body["email"] == "a@example.com"
    assert len(body["tag"]) == 4

    login = await client.post("/api/v1/auth/login", json={"email": "a@example.com", "password": "password123"})
    assert login.status_code == 200
    assert login.json()["access_token"]


async def test_duplicate_email_conflicts(client: AsyncClient) -> None:
    await register(client)
    resp = await client.post(
        "/api/v1/auth/register",
        json={"email": "a@example.com", "password": "password123", "username": "other"},
    )
    assert resp.status_code == 409
    assert resp.json()["error"]["code"] == "CONFLICT"


async def test_unauthorized_without_token(client: AsyncClient) -> None:
    resp = await client.get("/api/v1/users/@me")
    assert resp.status_code == 401
    assert resp.json()["error"]["code"] == "UNAUTHORIZED"


async def test_settings_roundtrip(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    resp = await client.patch("/api/v1/users/@me/settings", headers=headers, json={"theme": "light", "volume": 42})
    assert resp.status_code == 200
    assert resp.json()["theme"] == "light"
    assert resp.json()["volume"] == 42

    got = await client.get("/api/v1/users/@me/settings", headers=headers)
    assert got.json()["theme"] == "light"
