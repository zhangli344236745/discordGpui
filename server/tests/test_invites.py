from datetime import UTC, datetime, timedelta

from httpx import AsyncClient

from app.db import SessionLocal
from app.ids import new_id
from app.models import Invite
from tests.conftest import me, register


async def test_invite_lifecycle(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    tokens = await register(client, "carol@example.com")
    carol = {"Authorization": f"Bearer {tokens['access_token']}"}
    carol_user = await me(client, carol)

    created = await client.post(
        f"/api/v1/channels/{world['channel']['id']}/invites",
        headers=alice,
        json={"expiry": "1h", "max_uses": 2},
    )
    assert created.status_code == 201, created.text
    code = created.json()["code"]
    assert created.json()["valid"] is True

    preview = await client.get(f"/api/v1/invites/{code}", headers=carol)
    assert preview.status_code == 200
    assert preview.json()["server"]["name"] == world["server"]["name"]
    assert preview.json()["valid"] is True
    assert preview.json()["joined"] is False

    joined = await client.post(f"/api/v1/invites/{code}/accept", headers=carol)
    assert joined.status_code == 200, joined.text
    assert joined.json()["server"]["member_count"] == 2
    assert joined.json()["invite"]["uses"] == 1
    members = (await client.get(f"/api/v1/servers/{world['server']['id']}/members", headers=alice)).json()
    assert carol_user["id"] in [m["user_id"] for m in members]

    # 重复接受：幂等，不增加 uses
    again = await client.post(f"/api/v1/invites/{code}/accept", headers=carol)
    assert again.status_code == 200
    assert again.json()["invite"]["uses"] == 1

    # 人数上限
    bob = world["bob"]
    second = await client.post(f"/api/v1/invites/{code}/accept", headers=bob)
    assert second.status_code == 200 and second.json()["invite"]["uses"] == 2

    third_tokens = await register(client, "dave@example.com")
    dave = {"Authorization": f"Bearer {third_tokens['access_token']}"}
    exhausted = await client.get(f"/api/v1/invites/{code}", headers=dave)
    assert exhausted.json()["valid"] is False and exhausted.json()["exhausted"] is True
    denied = await client.post(f"/api/v1/invites/{code}/accept", headers=dave)
    assert denied.status_code == 404

    listed = (await client.get(f"/api/v1/servers/{world['server']['id']}/invites", headers=alice)).json()
    assert listed[0]["code"] == code
    assert (await client.get(f"/api/v1/servers/{world['server']['id']}/invites", headers=bob)).status_code == 403


async def test_invite_expired_and_revoked(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    channel_id = world["channel"]["id"]
    server_id = world["server"]["id"]

    created = await client.post(f"/api/v1/channels/{channel_id}/invites", headers=alice, json={"expiry": "never"})
    code = created.json()["code"]

    # 非创建者、无 MANAGE_SERVER 的成员不能撤销
    assert (await client.delete(f"/api/v1/invites/{code}", headers=bob)).status_code == 404
    revoked = await client.delete(f"/api/v1/invites/{code}", headers=alice)
    assert revoked.status_code == 200
    assert (await client.get(f"/api/v1/invites/{code}", headers=bob)).json()["valid"] is False
    assert (await client.post(f"/api/v1/invites/{code}/accept", headers=bob)).status_code == 404

    expired_code = "expired1"
    async with SessionLocal() as db:
        db.add(
            Invite(
                code=expired_code,
                server_id=server_id,
                channel_id=channel_id,
                created_by=world["alice_user"]["id"],
                expires_at=datetime.now(UTC) - timedelta(hours=1),
            )
        )
        await db.commit()

    preview = await client.get(f"/api/v1/invites/{expired_code}", headers=bob)
    assert preview.status_code == 200 and preview.json()["expired"] is True
    assert (await client.post(f"/api/v1/invites/{expired_code}/accept", headers=bob)).status_code == 404


async def test_invite_requires_permission(client: AsyncClient, world: dict) -> None:
    bob = world["bob"]
    # bob 尚未加入，无法为频道创建邀请
    resp = await client.post(f"/api/v1/channels/{world['channel']['id']}/invites", headers=bob, json={})
    assert resp.status_code == 404

    unknown = await client.get(f"/api/v1/invites/{new_id()}", headers=bob)
    assert unknown.status_code == 404
