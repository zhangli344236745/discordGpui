"""P3 服务器侧治理：LiveKit RoomService 调用与回退。"""

import httpx
import pytest

from app.livekit import rooms as livekit_rooms
from tests.conftest import auth_headers, create_server, first_channel


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
