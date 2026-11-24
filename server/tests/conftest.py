import os
import tempfile
from pathlib import Path

import pytest
from httpx import ASGITransport, AsyncClient

_TMP = Path(tempfile.mkdtemp(prefix="dgpui-test-"))
os.environ["DATABASE_URL"] = f"sqlite+aiosqlite:///{_TMP / 'test.db'}"
os.environ["UPLOAD_DIR"] = str(_TMP / "uploads")
os.environ["JWT_SECRET"] = "test-secret-key-0123456789abcdef"

from sqlalchemy import select  # noqa: E402

from app.db import SessionLocal, reset_db  # noqa: E402
from app.ids import new_id  # noqa: E402
from app.main import app  # noqa: E402
from app.models import DmChannel, MemberRole, Role, ServerMember  # noqa: E402
from app.services import ratelimit  # noqa: E402


@pytest.fixture
async def client():
    ratelimit.reset()
    await reset_db()
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c


async def register(client: AsyncClient, email: str = "a@example.com") -> dict:
    resp = await client.post(
        "/api/v1/auth/register",
        json={"email": email, "password": "password123", "username": f"{email.split(chr(64))[0]}_u"},
    )
    assert resp.status_code == 201, resp.text
    return resp.json()


async def auth_headers(client: AsyncClient, email: str = "a@example.com") -> dict:
    tokens = await register(client, email)
    return {"Authorization": f"Bearer {tokens['access_token']}"}


async def me(client: AsyncClient, headers: dict) -> dict:
    resp = await client.get("/api/v1/users/@me", headers=headers)
    assert resp.status_code == 200, resp.text
    return resp.json()


async def create_server(
    client: AsyncClient,
    headers: dict,
    name: str = "测试服务器",
    *,
    template_key: str | None = None,
    is_public: bool = True,
) -> dict:
    payload: dict = {"name": name, "is_public": is_public}
    if template_key:
        payload["template_key"] = template_key
    resp = await client.post("/api/v1/servers", headers=headers, json=payload)
    assert resp.status_code == 201, resp.text
    return resp.json()


async def create_channel(
    client: AsyncClient, headers: dict, server_id: str, name: str = "一般", kind: str = "text"
) -> dict:
    resp = await client.post(
        f"/api/v1/servers/{server_id}/channels", headers=headers, json={"name": name, "kind": kind}
    )
    assert resp.status_code == 201, resp.text
    return resp.json()


async def first_channel(client: AsyncClient, headers: dict, server_id: str, kind: str = "text") -> dict:
    resp = await client.get(f"/api/v1/servers/{server_id}/channels", headers=headers)
    assert resp.status_code == 200, resp.text
    return next(c for c in resp.json()["channels"] if c["kind"] == kind)


async def make_dm(user_a: str, user_b: str) -> str:
    """直接落库建 DM，规范化 (user_a < user_b)；B1 的端点实现后仍可复用做测试铺垫。"""
    a, b = sorted((user_a, user_b))
    async with SessionLocal() as db:
        dm = DmChannel(id=new_id(), user_a=a, user_b=b)
        db.add(dm)
        await db.commit()
        return dm.id


async def add_member(server_id: str, user_id: str, nickname: str = "") -> None:
    """直接落库加入服务器并给 @everyone；用于不需要走邀请流程的测试铺垫。"""
    async with SessionLocal() as db:
        if await db.get(ServerMember, (server_id, user_id)) is None:
            db.add(ServerMember(server_id=server_id, user_id=user_id, nickname=nickname))
        everyone = await db.scalar(
            select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True))
        )
        if everyone is not None and await db.get(MemberRole, (server_id, user_id, everyone.id)) is None:
            db.add(MemberRole(server_id=server_id, user_id=user_id, role_id=everyone.id))
        await db.commit()


@pytest.fixture
async def world(client: AsyncClient) -> dict:
    """两名用户 + 一个服务器 + 一个文字频道 + 一个 DM。"""
    alice_tokens = await register(client, "alice@example.com")
    bob_tokens = await register(client, "bob@example.com")
    alice = {"Authorization": f"Bearer {alice_tokens['access_token']}"}
    bob = {"Authorization": f"Bearer {bob_tokens['access_token']}"}
    alice_user = await me(client, alice)
    bob_user = await me(client, bob)
    server = await create_server(client, alice, "世界")
    channel = await first_channel(client, alice, server["id"])
    dm_id = await make_dm(alice_user["id"], bob_user["id"])
    return {
        "alice": alice,
        "bob": bob,
        "alice_user": alice_user,
        "bob_user": bob_user,
        "server": server,
        "channel": channel,
        "dm_id": dm_id,
    }
