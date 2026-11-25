#!/usr/bin/env bash
# 双实例实时冒烟：验证 Redis Pub/Sub 跨进程广播。
# 用法：./scripts/multi-instance-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP="$(mktemp -d)"
A_PORT=8090
B_PORT=8091

cleanup() {
  [ -n "${A_PID:-}" ] && kill "$A_PID" 2>/dev/null || true
  [ -n "${B_PID:-}" ] && kill "$B_PID" 2>/dev/null || true
  docker compose -f "$ROOT/docker-compose.yml" stop redis >/dev/null 2>&1 || true
  if [ "${KEEP_TMP:-0}" != "1" ]; then rm -rf "$TMP"; fi
}
trap cleanup EXIT

cd "$ROOT"
docker compose up -d redis >/dev/null
for _ in $(seq 1 30); do
  if python3 -c "import socket; socket.create_connection(('127.0.0.1', 6380), 1)" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

cd "$ROOT/server"
export UPLOAD_DIR="$TMP/uploads"
mkdir -p "$UPLOAD_DIR"

DATABASE_URL="sqlite+aiosqlite:///$TMP/app.db" \
REDIS_URL="redis://127.0.0.1:6380/0" \
JWT_SECRET="multi-instance-smoke-secret-32bytes!!" \
uv run uvicorn app.main:app --port "$A_PORT" >"$TMP/a.log" 2>&1 &
A_PID=$!

# 等 A 完成建表与迁移，避免两实例同时迁移 SQLite 造成锁竞争
for _ in $(seq 1 60); do
  if curl -sf --noproxy '*' "http://127.0.0.1:$A_PORT/api/v1/health" >/dev/null; then
    break
  fi
  sleep 0.2
done

DATABASE_URL="sqlite+aiosqlite:///$TMP/app.db" \
REDIS_URL="redis://127.0.0.1:6380/0" \
JWT_SECRET="multi-instance-smoke-secret-32bytes!!" \
uv run uvicorn app.main:app --port "$B_PORT" >"$TMP/b.log" 2>&1 &
B_PID=$!

for port in "$A_PORT" "$B_PORT"; do
  for _ in $(seq 1 60); do
    if curl -sf --noproxy '*' "http://127.0.0.1:$port/api/v1/health" >/dev/null; then
      break
    fi
    sleep 0.2
  done
done

uv run python - "$A_PORT" "$B_PORT" <<'PY'
import asyncio
import json
import sys
import urllib.request

import websockets

a_port, b_port = int(sys.argv[1]), int(sys.argv[2])
base_a = f"http://127.0.0.1:{a_port}/api/v1"
base_b = f"http://127.0.0.1:{b_port}"


def call(method, path, body=None, token=None, base=base_a):
    req = urllib.request.Request(base + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(req, data=data) as resp:
        return json.loads(resp.read() or b"null")


async def main() -> None:
    tokens = call("POST", "/auth/register", {
        "email": "multi@example.com",
        "password": "password123",
        "username": "multi",
    })
    access = tokens["access_token"]
    server = call("POST", "/servers", {"name": "Multi", "is_public": True}, access)
    tree = call("GET", f"/servers/{server['id']}/channels", token=access)
    channel = next(c for c in tree["channels"] if c["kind"] == "text")

    ws_url = base_b.replace("http", "ws") + f"/ws/gateway?token={access}"
    async with websockets.connect(ws_url) as ws:
        ready = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
        assert ready["event"] == "READY", ready

        sent = call(
            "POST",
            f"/channels/{channel['id']}/messages",
            {"content": "cross instance", "client_nonce": "multi-1"},
            access,
        )
        assert sent["id"], sent

        while True:
            event = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
            if event["event"] == "MESSAGE_CREATE" and event["data"].get("id") == sent["id"]:
                print("PASS: B instance received MESSAGE_CREATE from A instance")
                return


asyncio.run(main())
PY

echo "multi-instance smoke passed"
