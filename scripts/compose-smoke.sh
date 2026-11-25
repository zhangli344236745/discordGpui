#!/usr/bin/env bash
# 容器化冒烟：默认 compose（Postgres + Redis + API）启动后验证核心链路。
# 用法：scripts/compose-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="http://127.0.0.1:8081"

cd "$ROOT"
docker compose up -d --build
for _ in $(seq 1 120); do
  if curl -sf --noproxy '*' "$BASE/api/v1/health" >/dev/null; then
    break
  fi
  sleep 1
done

cd "$ROOT/server"
uv run python - "$BASE" <<'PY'
import json
import sys
import time
import urllib.error
import urllib.request

base = sys.argv[1] + "/api/v1"


def call(method, path, body=None, token=None, raw: bytes | None = None, ctype="application/json"):
    req = urllib.request.Request(base + path, method=method)
    req.add_header("Content-Type", ctype)
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    with urllib.request.urlopen(req, data=data) as resp:
        payload = resp.read()
        return json.loads(payload) if payload else None


health = call("GET", "/health")
assert health["status"] == "ok" and health["db"] is True, health

tokens = call(
    "POST",
    "/auth/register",
    {"email": f"compose-{int(time.time())}@example.com", "password": "password123", "username": "compose"},
)
access = tokens["access_token"]

server = call("POST", "/servers", {"name": "Compose", "is_public": True}, access)
tree = call("GET", f"/servers/{server['id']}/channels", token=access)
channel = next(c for c in tree["channels"] if c["kind"] == "text")

sent = call(
    "POST",
    f"/channels/{channel['id']}/messages",
    {"content": "compose 容器消息", "client_nonce": "compose-1"},
    access,
)
assert sent["id"], sent

init = call(
    "POST",
    "/uploads",
    {"filename": "note.txt", "size": 5, "mime": "text/plain"},
    access,
)
call("PUT", f"/uploads/{init['file_id']}/content", token=access, raw=b"hello", ctype="text/plain")
att = call("GET", f"/attachments/{init['file_id']}", token=access)
assert att["filename"] == "note.txt" and att["size"] == 5, att

with urllib.request.urlopen(att["url"]) as resp:
    assert resp.read() == b"hello"

print("PASS: compose stack (postgres + redis + api + uploads)")
PY

echo "compose smoke passed"
