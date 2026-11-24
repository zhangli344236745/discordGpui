"""端到端冒烟：真实起服务后跑一遍「注册 → 建服 → 加入 → 实时消息 → 语音 token」。

用法：
    uv run uvicorn app.main:app --port 8099 &
    uv run python scripts/smoke.py http://127.0.0.1:8099
"""

import asyncio
import json
import sys
import time

import httpx
import websockets

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8099"
API = f"{BASE}/api/v1"
STAMP = str(int(time.time()))


async def register(client: httpx.AsyncClient, name: str) -> tuple[str, dict]:
    resp = await client.post(
        f"{API}/auth/register",
        json={
            "email": f"{name}{STAMP}@example.com",
            "password": "password123",
            "username": f"{name}{STAMP}"[:32],
        },
    )
    resp.raise_for_status()
    tokens = resp.json()
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}
    me = (await client.get(f"{API}/users/@me", headers=headers)).json()
    return tokens["access_token"], {"headers": headers, "me": me}


async def main() -> None:
    async with httpx.AsyncClient(timeout=10) as client:
        health = (await client.get(f"{API}/health")).json()
        print(f"1. health            {health}")

        _, a = await register(client, "alice")
        _, b = await register(client, "bob")
        print(f"2. register          alice={a['me']['id']} bob={b['me']['id']}")

        server = (
            await client.post(
                f"{API}/servers",
                headers=a["headers"],
                json={"name": f"冒烟服{STAMP}", "template_key": "study", "is_public": True},
            )
        ).json()
        print(f"3. create server     {server['name']} members={server['member_count']}")

        tree = (await client.get(f"{API}/servers/{server['id']}/channels", headers=a["headers"])).json()
        channels = {c["name"]: c for c in tree["channels"]}
        text_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")
        voice_id = channels["专注自习"]["id"]
        print(f"4. channel tree      {len(tree['categories'])} 分类 / {len(tree['channels'])} 频道")

        join = (await client.post(f"{API}/discover/servers/{server['id']}/join", headers=b["headers"])).json()
        print(f"5. join (discover)   welcome={join['welcome']['headline']} 规则 {len(join['welcome']['rules'])} 条")

        # 实时：B 连 Gateway，A 发消息，B 应在 2 秒内收到
        ws_url = BASE.replace("http", "ws") + f"/ws/gateway?token={b['headers']['Authorization'].split()[1]}"
        async with websockets.connect(ws_url) as ws:
            ready = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
            print(f"6. gateway READY     servers={len(ready['data']['servers'])} dms={len(ready['data']['dms'])}")

            sent = await client.post(
                f"{API}/channels/{text_id}/messages",
                headers=a["headers"],
                json={"content": "实时冒烟消息", "client_nonce": f"n-{STAMP}"},
            )
            sent.raise_for_status()
            event = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
            assert event["event"] == "MESSAGE_CREATE", event
            print(f"7. MESSAGE_CREATE    {event['data']['author']['name']}: {event['data']['content']}")

            typing = await client.post(f"{API}/channels/{text_id}/typing", headers=a["headers"])
            assert typing.status_code == 202
            print("8. typing            ok (202)")

        voice = (await client.post(f"{API}/channels/{voice_id}/voice/join", headers=b["headers"])).json()
        print(f"9. voice join        room={voice['room']} token={voice['token'][:24]}…")
        print(f"   voice participants {[p['name'] for p in voice['participants']]}")

        dup = await client.post(
            f"{API}/servers",
            headers=a["headers"],
            json={"name": f"冒烟服{STAMP}"},
        )
        print(f"10. duplicate server {dup.status_code} {dup.json()['error']['code']}")

        stub = await client.get(f"{API}/friends", headers=a["headers"])
        print(f"11. pending endpoint {stub.status_code} {stub.json()['error']['code']}")

    print("\n冒烟通过。")


if __name__ == "__main__":
    asyncio.run(main())
