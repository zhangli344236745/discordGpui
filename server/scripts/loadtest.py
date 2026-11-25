"""容量压测：REST P95 延迟 + 实时事件扇出滞后。

用法：
    uv run uvicorn app.main:app --port 8099 &
    uv run python scripts/loadtest.py http://127.0.0.1:8099 --users 10 --messages 200

输出：注册耗时、消息 REST P50/P95/P99、Gateway 扇出滞后 P95、吞吐。
扇出滞后 = 监听者收到 MESSAGE_CREATE 的本地时刻 - 发送端 HTTP 响应完成时刻（按 client_nonce 对齐）。
"""

import argparse
import asyncio
import json
import sys
import time

import httpx
import websockets

STAMP = str(int(time.time()))


def percentile(values: list[float], ratio: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, int(round(ratio * (len(ordered) - 1))))
    return ordered[index]


async def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("base", nargs="?", default="http://127.0.0.1:8099")
    parser.add_argument("--users", type=int, default=10, help="并发监听用户数（同时是发送者）")
    parser.add_argument("--messages", type=int, default=200, help="总消息数")
    parser.add_argument("--sender-gap", type=float, default=0.01)
    args = parser.parse_args()

    api = f"{args.base}/api/v1"
    ws_base = args.base.replace("http", "ws")

    register_latencies: list[float] = []
    send_latencies: list[float] = []
    post_done: dict[str, float] = {}
    arrivals: dict[str, float] = {}

    transport = httpx.AsyncHTTPTransport(retries=2)
    async with httpx.AsyncClient(timeout=15, transport=transport) as http:
        tokens = (
            await http.post(
                f"{api}/auth/register",
                json={
                    "email": f"load{STAMP}@example.com",
                    "password": "password123",
                    "username": f"load{STAMP}",
                },
            )
        ).json()
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}
        server = (await http.post(f"{api}/servers", headers=headers, json={"name": "Load", "is_public": True})).json()
        tree = (await http.get(f"{api}/servers/{server['id']}/channels", headers=headers)).json()
        channel = next(c for c in tree["channels"] if c["kind"] == "text")
        # 邀请码：让每个压测用户都成为成员（收事件 + 用自己的令牌发送）
        invite = (
            await http.post(
                f"{api}/channels/{channel['id']}/invites",
                headers=headers,
                json={"expiry": "1d", "max_uses": None},
            )
        ).json()["code"]

        async def register_user(index: int) -> str | None:
            start = time.perf_counter()
            resp = await http.post(
                f"{api}/auth/register",
                json={
                    "email": f"load{STAMP}-{index}@example.com",
                    "password": "password123",
                    "username": f"load{STAMP}u{index}"[:32],
                },
            )
            register_latencies.append(time.perf_counter() - start)
            if resp.status_code != 201:
                return None
            access = resp.json()["access_token"]
            await http.post(f"{api}/invites/{invite}/accept", headers={"Authorization": f"Bearer {access}"})
            return access

        access_list = [t for t in await asyncio.gather(*(register_user(i) for i in range(args.users))) if t]

        async def listener(access: str) -> None:
            try:
                async with websockets.connect(f"{ws_base}/ws/gateway?token={access}") as ws:
                    await asyncio.wait_for(ws.recv(), timeout=5)  # READY
                    while True:
                        raw = await asyncio.wait_for(ws.recv(), timeout=60)
                        event = json.loads(raw)
                        if event["event"] != "MESSAGE_CREATE":
                            continue
                        nonce = event["data"].get("client_nonce")
                        if isinstance(nonce, str) and nonce.startswith(f"load-{STAMP}-"):
                            arrivals.setdefault(nonce, time.perf_counter())
            except (TimeoutError, Exception):  # noqa: BLE001
                return

        listeners = [asyncio.create_task(listener(access)) for access in access_list]
        await asyncio.sleep(1.0)

        per_sender = max(1, args.messages // max(1, len(access_list)))

        sender_headers = [
            {"Authorization": f"Bearer {access}"} for access in access_list
        ]

        async def sender(index: int) -> None:
            mine = sender_headers[index]
            for step in range(per_sender):
                nonce = f"load-{STAMP}-{index}-{step}"
                start = time.perf_counter()
                resp = await http.post(
                    f"{api}/channels/{channel['id']}/messages",
                    headers=mine,
                    json={"content": f"load {index}-{step}", "client_nonce": nonce},
                )
                if resp.status_code == 201:
                    send_latencies.append(time.perf_counter() - start)
                    post_done[nonce] = time.perf_counter()
                elif resp.status_code == 429:
                    # 用户级限流（默认 5 条 / 5 秒）：按提示退避
                    retry = float(resp.json()["error"].get("retry_after", 1.0))
                    await asyncio.sleep(retry)
                await asyncio.sleep(args.sender_gap)

        started = time.perf_counter()
        await asyncio.gather(*(sender(i) for i in range(len(access_list))))
        wall = time.perf_counter() - started
        await asyncio.sleep(3.0)
        for task in listeners:
            task.cancel()

    fanout = [
        max(0.0, arrivals[nonce] - done) for nonce, done in post_done.items() if nonce in arrivals
    ]

    def row(label: str, values: list[float]) -> str:
        if not values:
            return f"{label:<18} n=0"
        return (
            f"{label:<18} n={len(values):<5} p50={percentile(values, 0.5) * 1000:7.2f}ms "
            f"p95={percentile(values, 0.95) * 1000:7.2f}ms p99={percentile(values, 0.99) * 1000:7.2f}ms"
        )

    print(row("register", register_latencies))
    print(row("message rest", send_latencies))
    print(row("gateway fanout", fanout))
    print(f"delivered         {len(arrivals)}/{len(post_done)}")
    print(f"throughput        {len(send_latencies) / max(wall, 0.001):.1f} msg/s (wall {wall:.1f}s)")


if __name__ == "__main__":
    if sys.platform == "win32":  # pragma: no cover
        asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
    asyncio.run(main())
