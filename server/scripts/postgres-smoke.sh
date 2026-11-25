#!/usr/bin/env bash
# Postgres 后端冒烟：验证迁移、启动、注册/发消息/搜索在 Postgres 下可用。
# 用法：./scripts/postgres-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PORT=8092
PG_URL="postgresql+asyncpg://discord:discord@127.0.0.1:5433/discord"

cleanup() {
  [ -n "${API_PID:-}" ] && kill "$API_PID" 2>/dev/null || true
  docker compose -f "$ROOT/docker-compose.yml" stop postgres >/dev/null 2>&1 || true
}
trap cleanup EXIT

cd "$ROOT"
docker compose up -d postgres >/dev/null
for _ in $(seq 1 60); do
  if docker compose exec -T postgres pg_isready -U discord -d discord >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

cd "$ROOT/server"
export DATABASE_URL="$PG_URL"
export JWT_SECRET="postgres-smoke-secret-32bytes!!"
export UPLOAD_DIR="$(mktemp -d)/uploads"
mkdir -p "$UPLOAD_DIR"

# 全新库：清掉上一轮残留
docker compose -f "$ROOT/docker-compose.yml" exec -T postgres \
  psql -U discord -d discord -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;" >/dev/null

uv run alembic upgrade head
uv run uvicorn app.main:app --port "$PORT" >/tmp/postgres-smoke.log 2>&1 &
API_PID=$!

for _ in $(seq 1 60); do
  if curl -sf --noproxy '*' "http://127.0.0.1:$PORT/api/v1/health" >/dev/null; then
    break
  fi
  sleep 0.3
done

uv run python - "$PORT" <<'PY'
import json
import sys
import urllib.parse
import urllib.request

port = int(sys.argv[1])
base = f"http://127.0.0.1:{port}/api/v1"


def call(method, path, body=None, token=None):
    req = urllib.request.Request(base + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(req, data=data) as resp:
        return json.loads(resp.read() or b"null")


health = call("GET", "/health")
assert health["db"] is True, health

tokens = call(
    "POST",
    "/auth/register",
    {"email": "pg@example.com", "password": "password123", "username": "pguser"},
)
access = tokens["access_token"]

server = call("POST", "/servers", {"name": "PG", "is_public": True}, access)
tree = call("GET", f"/servers/{server['id']}/channels", token=access)
channel = next(c for c in tree["channels"] if c["kind"] == "text")

sent = call(
    "POST",
    f"/channels/{channel['id']}/messages",
    {"content": "postgres 中文搜索验证", "client_nonce": "pg-1"},
    access,
)
assert sent["id"], sent

query = urllib.parse.urlencode(
    {"q": "中文搜索", "scope": "server", "server_id": server["id"]}
)
found = call("GET", f"/search?{query}", token=access)
messages = found.get("items") or []
assert any(m["message_id"] == sent["id"] for m in messages), found

print("PASS: postgres migrations + message + trigram search")
PY

echo "postgres smoke passed"
