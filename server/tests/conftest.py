import os
import tempfile
from pathlib import Path

import pytest
from httpx import ASGITransport, AsyncClient

_TMP = Path(tempfile.mkdtemp(prefix="dgpui-test-"))
os.environ["DATABASE_URL"] = f"sqlite+aiosqlite:///{_TMP / 'test.db'}"
os.environ["UPLOAD_DIR"] = str(_TMP / "uploads")
os.environ["JWT_SECRET"] = "test-secret-key-0123456789abcdef"

from app.db import Base, engine, init_db  # noqa: E402
from app.main import app  # noqa: E402


@pytest.fixture
async def client():
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await init_db()
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
