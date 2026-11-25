#!/usr/bin/env bash
# 真实语音端到端冒烟：本地 LiveKit + 后端 + 客户端自动加入语音频道。
#
# 前置：
#   - 已构建 voice 版客户端：scripts/build-voice.sh
#   - Docker 可用且已有 livekit/livekit-server 镜像
#   - 与默认配置一致的 LiveKit key/secret（见 server/app/config.py）
#
# 用法：scripts/voice-e2e.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
PORT="${VOICE_E2E_PORT:-8087}"
LK_NAME="dgpui-livekit-voice-e2e"

cleanup() {
  [ -n "${CLIENT_PID:-}" ] && kill "$CLIENT_PID" 2>/dev/null || true
  [ -n "${SERVER_PID:-}" ] && kill "$SERVER_PID" 2>/dev/null || true
  docker rm -f "$LK_NAME" >/dev/null 2>&1 || true
  [ "${KEEP_TMP:-0}" = "1" ] || rm -rf "$TMP"
}
trap cleanup EXIT

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

[ -x "$ROOT/target/debug/discord-gpui" ] || { echo "缺少客户端二进制，先运行 scripts/build-voice.sh" >&2; exit 1; }
docker image inspect livekit/livekit-server:latest >/dev/null 2>&1 || {
  echo "缺少 livekit/livekit-server 镜像：docker pull livekit/livekit-server:latest" >&2
  exit 1
}

docker rm -f "$LK_NAME" >/dev/null 2>&1 || true
docker run -d --name "$LK_NAME" \
  -p 7880:7880 -p 7881:7881 -p 7882:7882/udp \
  --add-host host.docker.internal:host-gateway \
  livekit/livekit-server --dev --bind 0.0.0.0 --node-ip 127.0.0.1 \
  --config-body "keys:
  devkey: devsecret-devsecret-devsecret-1234
webhook:
  api_key: devkey
  urls:
    - http://host.docker.internal:$PORT/api/v1/livekit/webhook
" >/dev/null

if [ "$(docker inspect -f '{{.State.Running}}' "$LK_NAME" 2>/dev/null)" != "true" ]; then
  echo "FAIL voice: LiveKit 容器启动失败" >&2
  docker logs "$LK_NAME" 2>&1 | tail -10 >&2
  exit 1
fi

for _ in $(seq 1 40); do
  curl -sf --noproxy '*' http://127.0.0.1:7880 >/dev/null 2>&1 && break
  sleep 0.25
done

cd "$ROOT/server"
DATABASE_URL="sqlite+aiosqlite:///$TMP/app.db" \
UPLOAD_DIR="$TMP/uploads" \
LIVEKIT_URL="ws://127.0.0.1:7880" \
uv run uvicorn app.main:app --port "$PORT" >"$TMP/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 80); do
  curl -sf --noproxy '*' "http://127.0.0.1:$PORT/api/v1/health" >/dev/null 2>&1 && break
  sleep 0.25
done

python3 - "$PORT" "$TMP" <<'PY'
import json, sys, urllib.request
port, tmp = sys.argv[1], sys.argv[2]
api = f"http://127.0.0.1:{port}/api/v1"

def call(method, path, body=None, token=None):
    req = urllib.request.Request(api + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(req, data=data) as resp:
        return json.loads(resp.read() or b"null")

tokens = call("POST", "/auth/register", {
    "email": "voice-e2e@example.com", "password": "password123", "username": "voicee2e",
})
server = call("POST", "/servers", {"name": "VoiceLab", "is_public": True}, tokens["access_token"])
tree = call("GET", f"/servers/{server['id']}/channels", token=tokens["access_token"])
channel = next(c for c in tree["channels"] if c["kind"] == "voice")
json.dump({"base": f"http://127.0.0.1:{port}", "refresh_token": tokens["refresh_token"]},
          open(f"{tmp}/session.json", "w"))
json.dump({"channel": channel["id"], "token": tokens["access_token"]}, open(f"{tmp}/ctx.json", "w"))
print(channel["id"])
PY
chmod 600 "$TMP/session.json"
CHANNEL="$(python3 -c "import json;print(json.load(open('$TMP/ctx.json'))['channel'])")"
TOKEN="$(python3 -c "import json;print(json.load(open('$TMP/ctx.json'))['token'])")"

DISCORD_API_BASE="http://127.0.0.1:$PORT" \
DISCORD_SESSION_FILE="$TMP/session.json" \
DISCORD_VOICE_AUTOJOIN="$CHANNEL" \
  "$ROOT/target/debug/discord-gpui" >"$TMP/client.log" 2>&1 &
CLIENT_PID=$!

sleep 14
if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
  echo "FAIL voice: 客户端退出"
  tail -20 "$TMP/client.log"
  exit 1
fi
if grep -qiE "panic|Terminating app" "$TMP/client.log" "$TMP/server.log"; then
  echo "FAIL voice: 崩溃"
  tail -20 "$TMP/client.log"
  exit 1
fi

grep -q "room connected" "$TMP/client.log" || { echo "FAIL voice: 未连接房间"; tail -20 "$TMP/client.log"; exit 1; }
grep -q "microphone published" "$TMP/client.log" || { echo "FAIL voice: 麦克风未发布"; tail -20 "$TMP/client.log"; exit 1; }
echo "PASS voice: room + microphone"

for kind in participant_joined track_published; do
  if grep -q "livekit webhook event=$kind" "$TMP/server.log"; then
    echo "PASS voice: webhook $kind"
  else
    echo "FAIL voice: 缺少 webhook $kind"
    tail -20 "$TMP/server.log"
    exit 1
  fi
done

participants="$(curl -s --noproxy '*' -H "Authorization: Bearer $TOKEN" \
  "http://127.0.0.1:$PORT/api/v1/channels/$CHANNEL/voice/participants")"
python3 - "$participants" <<'PY'
import json, sys
rows = json.loads(sys.argv[1])
assert rows and rows[0]["mic"] is True, rows
print("PASS voice: participant projection mic=true")
PY

echo "voice e2e passed"
