import jwt
from httpx import AsyncClient

from app.config import settings
from tests.conftest import auth_headers


async def _voice_channel(client: AsyncClient, headers: dict) -> tuple[str, str]:
    server = (await client.post("/api/v1/servers", headers=headers, json={"name": "语音服"})).json()
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    channel = next(c for c in tree["channels"] if c["kind"] == "voice")
    return server["id"], channel["id"]


async def test_join_voice_returns_livekit_token(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server_id, channel_id = await _voice_channel(client, headers)

    resp = await client.post(f"/api/v1/channels/{channel_id}/voice/join", headers=headers)
    assert resp.status_code == 200, resp.text
    body = resp.json()

    assert body["room"] == f"srv:{server_id}:{channel_id}"
    assert body["url"] == settings.livekit_url
    claims = jwt.decode(body["token"], settings.livekit_api_secret, algorithms=["HS256"])
    assert claims["iss"] == settings.livekit_api_key
    assert claims["video"]["room"] == body["room"]
    assert claims["video"]["roomJoin"] is True
    assert claims["video"]["canPublish"] is True

    assert len(body["participants"]) == 1
    assert body["participants"][0]["mic"] is True


async def test_voice_state_and_leave(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    _, channel_id = await _voice_channel(client, headers)
    room = (await client.post(f"/api/v1/channels/{channel_id}/voice/join", headers=headers)).json()["room"]

    muted = await client.put(f"/api/v1/voice/{room}/me", headers=headers, json={"mic": False})
    assert muted.status_code == 204

    participants = (await client.get(f"/api/v1/channels/{channel_id}/voice/participants", headers=headers)).json()
    assert participants[0]["mic"] is False

    left = await client.post(f"/api/v1/channels/{channel_id}/voice/leave", headers=headers)
    assert left.status_code == 204
    empty = (await client.get(f"/api/v1/channels/{channel_id}/voice/participants", headers=headers)).json()
    assert empty == []


async def test_join_text_channel_as_voice_fails(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = (await client.post("/api/v1/servers", headers=headers, json={"name": "文字服"})).json()
    tree = (await client.get(f"/api/v1/servers/{server['id']}/channels", headers=headers)).json()
    text_channel = next(c["id"] for c in tree["channels"] if c["kind"] == "text")

    resp = await client.post(f"/api/v1/channels/{text_channel}/voice/join", headers=headers)
    assert resp.status_code == 404


async def test_gateway_rejects_bad_token() -> None:
    """WebSocket 鉴权失败时不应拿到用户对象。"""
    from app.realtime.gateway import _authorize

    assert await _authorize("not-a-token") is None
