#!/usr/bin/env bash
# 客户端端到端冒烟：起临时 server，验证 Demo / 登录页 / 自动登录 / WS / 事件 / 失效 token / 后端不可达。
#
# 用法：
#   ./scripts/client-e2e.sh            # 自动构建并运行全部场景
#   SKIP_BUILD=1 ./scripts/client-e2e.sh
#
# 依赖：server/.venv（uv sync）、target/debug/discord-gpui（cargo build）。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${PORT:-8095}"
BASE="http://127.0.0.1:${PORT}"
API="$BASE/api/v1"
TMP="$(mktemp -d)"
SERVER_PID=""
CLIENT_PID=""

cleanup() {
  [ -n "$CLIENT_PID" ] && kill "$CLIENT_PID" 2>/dev/null || true
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  sleep 0.5
  lsof -ti:"$PORT" 2>/dev/null | xargs kill 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  (cd "$ROOT" && cargo build)
fi
[ -x "$ROOT/target/debug/discord-gpui" ] || {
  echo "missing target/debug/discord-gpui; run cargo build first" >&2
  exit 1
}

if lsof -ti:"$PORT" >/dev/null 2>&1; then
  echo "port $PORT is in use; set PORT=<other>" >&2
  exit 1
fi

(cd "$ROOT/server" && \
  DATABASE_URL="sqlite+aiosqlite:///$TMP/app.db" \
  UPLOAD_DIR="$TMP/uploads" \
  uv run uvicorn app.main:app --port "$PORT" >"$TMP/server.log" 2>&1) &
SERVER_PID=$!
for _ in $(seq 1 80); do
  curl -sf --noproxy '*' "$API/health" >/dev/null 2>&1 && break
  sleep 0.25
done

python3 - "$TMP" "$API" <<'PY'
import json, sys, urllib.request
tmp, api = sys.argv[1], sys.argv[2]
def call(method, path, body=None, token=None):
    req = urllib.request.Request(api + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    try:
        with urllib.request.urlopen(req, data=data) as resp:
            return json.loads(resp.read() or b"null")
    except urllib.error.HTTPError as error:
        print(f"setup failed: {method} {path} -> {error.code} {error.read()!r}", file=sys.stderr)
        raise

a = call("POST", "/auth/register", {"email": "alice-e2e@example.com", "password": "password123", "username": "alice"})
b = call("POST", "/auth/register", {"email": "bob-e2e@example.com", "password": "password123", "username": "bob"})
server = call("POST", "/servers", {"name": "Runtime", "is_public": True}, a["access_token"])
tree = call("GET", f"/servers/{server['id']}/channels", token=a["access_token"])
channel = next(c for c in tree["channels"] if c["kind"] == "text")
call("POST", f"/discover/servers/{server['id']}/join", token=b["access_token"])
alice_me = call("GET", "/users/@me", token=a["access_token"])
dm = call("POST", "/dms", {"user_id": alice_me["id"]}, b["access_token"])["dm"]["id"]

json.dump(
    {"channel": channel["id"], "dm": dm, "alice": a["access_token"], "bob": b["access_token"], "alice_me": alice_me},
    open(f"{tmp}/ctx.json", "w"),
)
json.dump({"base": api[: -len("/api/v1")], "refresh_token": a["refresh_token"]}, open(f"{tmp}/session.json", "w"))
json.dump({"base": api[: -len("/api/v1")], "refresh_token": "invalid-token"}, open(f"{tmp}/bad-session.json", "w"))
PY
chmod 600 "$TMP/session.json" "$TMP/bad-session.json"

start_client() {
  local name=$1
  shift
  env "$@" "$ROOT/target/debug/discord-gpui" >"$TMP/$name.log" 2>&1 &
  CLIENT_PID=$!
}

check_alive() {
  local name=$1
  if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
    echo "FAIL $name: process exited"
    tail -20 "$TMP/$name.log"
    exit 1
  fi
  if grep -qiE "panic|fatal" "$TMP/$name.log"; then
    echo "FAIL $name: panic in log"
    tail -20 "$TMP/$name.log"
    exit 1
  fi
  echo "PASS $name: alive, no panic"
}

gateway_connections() {
  curl -s --noproxy '*' "$API/health" |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["gateway_connections"])'
}

# 1. Demo 模式
start_client demo DISCORD_DEMO=1
sleep 4
check_alive demo
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
CLIENT_PID=""
sleep 0.3

# 2. 在线但无会话：应停在登录页，不建立 WS
start_client login DISCORD_API_BASE="$BASE" DISCORD_SESSION_FILE="$TMP/none.json"
sleep 4
check_alive login
[ "$(gateway_connections)" = "0" ] && echo "PASS login: gateway=0" || { echo "FAIL login: gateway!=0"; exit 1; }
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
CLIENT_PID=""
sleep 0.3

# 3. 有效会话：自动刷新登录 → 首屏 → WS，并注入消息/私信/呼叫/好友请求事件
start_client online DISCORD_API_BASE="$BASE" DISCORD_SESSION_FILE="$TMP/session.json"
sleep 4
python3 - "$TMP" "$API" <<'PY'
import json, sys, urllib.request
tmp, api = sys.argv[1], sys.argv[2]
ctx = json.load(open(f"{tmp}/ctx.json"))
def call(method, path, body=None, token=None):
    req = urllib.request.Request(api + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(req, data=data) as resp:
        return json.loads(resp.read() or b"null")
call("POST", f"/channels/{ctx['channel']}/messages", {"content": "运行时事件消息", "client_nonce": "rt-1"}, ctx["bob"])
call("POST", f"/dms/{ctx['dm']}/messages", {"content": "运行时私信", "client_nonce": "rt-dm-1"}, ctx["bob"])
call("POST", f"/dms/{ctx['dm']}/call", {"kind": "voice"}, ctx["bob"])
call("POST", "/friends/requests", {"username": "alice", "tag": ctx["alice_me"]["tag"]}, ctx["bob"])
PY
sleep 3
check_alive online
[ "$(gateway_connections)" = "1" ] && echo "PASS online: gateway=1" || { echo "FAIL online: gateway!=1"; exit 1; }
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
CLIENT_PID=""
sleep 0.3

# 4. 无效 refresh token：回登录页，不建立 WS
start_client badsession DISCORD_API_BASE="$BASE" DISCORD_SESSION_FILE="$TMP/bad-session.json"
sleep 4
check_alive badsession
[ "$(gateway_connections)" = "0" ] && echo "PASS badsession: gateway=0" || { echo "FAIL badsession: gateway!=0"; exit 1; }
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
CLIENT_PID=""
sleep 0.3

# 5. 后端不可达：回 Demo
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""
start_client down DISCORD_API_BASE="http://127.0.0.1:8099" DISCORD_SESSION_FILE="$TMP/none.json"
sleep 4
check_alive down
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
CLIENT_PID=""

echo "client e2e passed"
