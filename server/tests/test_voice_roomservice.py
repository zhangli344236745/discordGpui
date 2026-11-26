"""P3 服务器侧治理：LiveKit RoomService 调用与回退。"""

import base64
import hashlib
import json
from datetime import UTC, datetime, timedelta

import httpx
import jwt
import pytest

from app.config import settings
from app.livekit import rooms as livekit_rooms
from tests.conftest import auth_headers, create_server, first_channel, me


def _mock_transport(status: int = 200) -> httpx.MockTransport:
    def handler(request: httpx.Request) -> httpx.Response:
        assert request.url.path.endswith("/twirp/livekit.RoomService/MutePublishedTrack")
        assert request.headers["Authorization"].startswith("Bearer ")
        return httpx.Response(status, json={})

    return httpx.MockTransport(handler)


@pytest.mark.asyncio
async def test_mute_calls_roomservice_when_reachable() -> None:
    ok = await livekit_rooms.mute_published_track("srv:s1:c1", "user-1", True, _mock_transport(200))
    assert ok is True


@pytest.mark.asyncio
async def test_mute_falls_back_when_unreachable() -> None:
    def fail(request: httpx.Request) -> httpx.Response:  # noqa: ANN001, ARG001
        return httpx.Response(500, json={})

    ok = await livekit_rooms.mute_published_track("srv:s1:c1", "user-1", True, httpx.MockTransport(fail))
    assert ok is False


async def test_moderate_participant_falls_back_without_roomservice(client: httpx.AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"], kind="voice")
    user = (await client.get("/api/v1/users/@me", headers=headers)).json()
    join = await client.post(f"/api/v1/channels/{channel['id']}/voice/join", headers=headers)
    assert join.status_code == 200, join.text
    room = join.json()["room"]

    muted = await client.patch(
        f"/api/v1/voice/{room}/participants/{user['id']}",
        headers=headers,
        json={"action": "mute"},
    )
    assert muted.status_code == 200, muted.text
    assert muted.json()["ok"] is True

    participants = await client.get(f"/api/v1/channels/{channel['id']}/voice/participants", headers=headers)
    assert participants.json()[0]["mic"] is False


async def test_webhook_tolerates_clock_skew(client: httpx.AsyncClient) -> None:
    """容器与宿主机时钟偏差时 iat 可能在未来，webhook 仍应验签通过。"""
    headers = await auth_headers(client)
    user = await me(client, headers)
    body = json.dumps(
        {
            "event": "participant_joined",
            "room": {"name": "srv:s1:c1"},
            "participant": {"identity": user["id"]},
        }
    ).encode()
    digest = base64.b64encode(hashlib.sha256(body).digest()).decode()
    now = datetime.now(UTC)
    token = jwt.encode(
        {
            "iss": settings.livekit_api_key,
            "iat": now + timedelta(seconds=20),
            "exp": now + timedelta(minutes=5),
            "sha256": digest,
        },
        settings.livekit_api_secret,
        algorithm="HS256",
    )
    resp = await client.post(
        "/api/v1/livekit/webhook",
        content=body,
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
    )
    assert resp.status_code == 200, resp.text


async def test_track_published_updates_projection_by_source(client: httpx.AsyncClient) -> None:
    """LiveKit 的 track.type 是 AUDIO/VIDEO，必须按 source 更新 mic/camera 标记。"""
    headers = await auth_headers(client)
    user = await me(client, headers)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"], kind="voice")
    join = await client.post(f"/api/v1/channels/{channel['id']}/voice/join", headers=headers)
    room = join.json()["room"]

    for kind, source in (("track_published", "MICROPHONE"), ("track_published", "CAMERA")):
        event = {
            "event": kind,
            "room": {"name": room},
            "participant": {"identity": user["id"]},
            "track": {"type": "VIDEO" if source == "CAMERA" else "AUDIO", "source": source},
        }
        body = json.dumps(event).encode()
        digest = base64.b64encode(hashlib.sha256(body).digest()).decode()
        token = jwt.encode(
            {
                "iss": settings.livekit_api_key,
                "exp": datetime.now(UTC) + timedelta(minutes=5),
                "sha256": digest,
            },
            settings.livekit_api_secret,
            algorithm="HS256",
        )
        resp = await client.post(
            "/api/v1/livekit/webhook",
            content=body,
            headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
        )
        assert resp.status_code == 200, resp.text

    participants = await client.get(f"/api/v1/channels/{channel['id']}/voice/participants", headers=headers)
    rows = participants.json()
    assert rows[0]["mic"] is True and rows[0]["camera"] is True, rows
