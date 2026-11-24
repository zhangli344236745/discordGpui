from datetime import UTC, datetime, timedelta

import jwt
from httpx import AsyncClient
from sqlalchemy import select

from app.config import settings
from app.db import SessionLocal
from app.models import CallSession, GatewayEvent
from app.services.calls import expire_ringing
from tests.conftest import register


async def test_call_state_machine_and_events(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    dm_id = world["dm_id"]

    started = await client.post(f"/api/v1/dms/{dm_id}/call", headers=alice, json={"kind": "voice"})
    assert started.status_code == 200, started.text
    call = started.json()
    assert call["state"] == "ringing" and call["kind"] == "voice"
    assert call["room"] == f"dm:{call['id']}"
    assert call["caller_id"] == world["alice_user"]["id"]

    claims = jwt.decode(call["token"], settings.livekit_api_secret, algorithms=["HS256"])
    assert claims["video"]["room"] == call["room"]

    async with SessionLocal() as db:
        ring = await db.scalar(
            select(GatewayEvent).where(
                GatewayEvent.event == "CALL_RING", GatewayEvent.scope_id == world["bob_user"]["id"]
            )
        )
    assert ring is not None and ring.payload["id"] == call["id"]

    accepted = await client.post(f"/api/v1/dms/{dm_id}/call/accept", headers=bob)
    assert accepted.status_code == 200, accepted.text
    assert accepted.json()["state"] == "active"
    assert accepted.json()["outcome"] == "accepted"

    duplicate_accept = await client.post(f"/api/v1/dms/{dm_id}/call/accept", headers=bob)
    assert duplicate_accept.status_code == 409

    ended = await client.post(f"/api/v1/dms/{dm_id}/call/hangup", headers=alice)
    assert ended.status_code == 200 and ended.json()["state"] == "ended"
    assert ended.json()["outcome"] == "hangup"

    idempotent = await client.post(f"/api/v1/dms/{dm_id}/call/hangup", headers=alice)
    assert idempotent.status_code == 200 and idempotent.json()["state"] == "ended"


async def test_call_decline_and_outsider(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    dm_id = world["dm_id"]

    started = await client.post(f"/api/v1/dms/{dm_id}/call", headers=alice, json={"kind": "video"})
    assert started.status_code == 200

    outsider_tokens = await register(client, "carol@example.com")
    outsider = {"Authorization": f"Bearer {outsider_tokens['access_token']}"}
    assert (await client.post(f"/api/v1/dms/{dm_id}/call", headers=outsider, json={})).status_code == 404
    assert (await client.post(f"/api/v1/dms/{dm_id}/call/accept", headers=outsider)).status_code == 404

    declined = await client.post(f"/api/v1/dms/{dm_id}/call/decline", headers=bob)
    assert declined.status_code == 200
    assert declined.json()["state"] == "ended" and declined.json()["outcome"] == "declined"

    second_decline = await client.post(f"/api/v1/dms/{dm_id}/call/decline", headers=bob)
    assert second_decline.status_code == 409


async def test_ring_timeout_closes_call(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    dm_id = world["dm_id"]

    started = await client.post(f"/api/v1/dms/{dm_id}/call", headers=alice, json={"kind": "voice"})
    call_id = started.json()["id"]

    assert await expire_ringing(datetime.now(UTC) + timedelta(seconds=61)) == 1
    assert await expire_ringing(datetime.now(UTC) + timedelta(seconds=61)) == 0

    async with SessionLocal() as db:
        call = await db.get(CallSession, call_id)
        assert call is not None
        assert call.state == "ended" and call.outcome == "timeout"

    # 已结束的呼叫不能再被接听
    bob = world["bob"]
    assert (await client.post(f"/api/v1/dms/{dm_id}/call/accept", headers=bob)).status_code == 409


async def test_call_participant_validation(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    dm_id = world["dm_id"]
    assert (await client.get(f"/api/v1/dms/{dm_id}/messages", headers=bob)).status_code == 200

    # 未开始的会话调用挂断：没有通话记录
    hangup = await client.post(f"/api/v1/dms/{dm_id}/call/hangup", headers=alice)
    assert hangup.status_code == 404
