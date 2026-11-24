import asyncio

from httpx import AsyncClient

from tests.conftest import me, register


async def test_concurrent_invite_last_slot(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    invite = (
        await client.post(
            f"/api/v1/channels/{world['channel']['id']}/invites",
            headers=alice,
            json={"expiry": "1h", "max_uses": 1},
        )
    ).json()

    async def join(email: str) -> int:
        tokens = await register(client, email)
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}
        await me(client, headers)
        response = await client.post(f"/api/v1/invites/{invite['code']}/accept", headers=headers)
        return response.status_code

    statuses = await asyncio.gather(join("race1@example.com"), join("race2@example.com"))
    assert sorted(statuses) == [200, 404]

    members = (await client.get(f"/api/v1/servers/{world['server']['id']}/members", headers=alice)).json()
    assert len(members) == 2


async def test_concurrent_same_nonce_is_idempotent(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    channel_id = world["channel"]["id"]

    async def send() -> tuple[int, str]:
        response = await client.post(
            f"/api/v1/channels/{channel_id}/messages",
            headers=alice,
            json={"content": "并发同 nonce", "client_nonce": "race-nonce"},
        )
        return response.status_code, response.json().get("id", "")

    results = await asyncio.gather(send(), send())
    assert all(status == 201 for status, _message_id in results)
    assert len({message_id for _status, message_id in results}) == 1

    history = (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=alice)).json()
    assert [item["content"] for item in history["items"]] == ["并发同 nonce"]
