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

# 语音用例必须用带 feature 的二进制；默认 cargo build 会把引擎编译掉
if [ "${VOICE_SKIP_BUILD:-0}" != "1" ]; then
  "$ROOT/scripts/build-voice.sh" --quiet
fi
[ -x "$ROOT/target/debug/discord-gpui" ] || { echo "缺少客户端二进制，先运行 scripts/build-voice.sh" >&2; exit 1; }
docker image inspect livekit/livekit-server:latest >/dev/null 2>&1 || {
  echo "缺少 livekit/livekit-server 镜像：docker pull livekit/livekit-server:latest" >&2
  exit 1
}

docker rm -f "$LK_NAME" >/dev/null 2>&1 || true
docker run -d --name "$LK_NAME" \
  -p 7883:7883 -p 7884:7884 -p 7885:7885/udp \
  --add-host host.docker.internal:host-gateway \
  livekit/livekit-server --dev --bind 0.0.0.0 --node-ip 127.0.0.1 \
  --port 7883 --rtc.tcp_port 7884 --rtc.udp_port.start 7885 --rtc.udp_port.end 7885 \
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
  curl -sf --noproxy '*' http://127.0.0.1:7883 >/dev/null 2>&1 && break
  sleep 0.25
done

cd "$ROOT/server"
DATABASE_URL="sqlite+aiosqlite:///$TMP/app.db" \
UPLOAD_DIR="$TMP/uploads" \
LIVEKIT_URL="ws://127.0.0.1:7883" \
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
DISCORD_VOICE_CAMERA="1" DISCORD_VOICE_SCREEN_FIRST="1" \
  "$ROOT/target/debug/discord-gpui" >"$TMP/client.log" 2>&1 &
CLIENT_PID=$!

wait_until() {
  local seconds=$1
  shift
  for _ in $(seq 1 $((seconds * 4))); do
    if "$@"; then return 0; fi
    sleep 0.25
  done
  return 1
}

wait_until 25 grep -q "room connected" "$TMP/client.log" || { echo "FAIL voice: 未连接房间"; tail -20 "$TMP/client.log"; exit 1; }
wait_until 15 grep -q "microphone published" "$TMP/client.log" || { echo "FAIL voice: 麦克风未发布"; tail -20 "$TMP/client.log"; exit 1; }
kill -0 "$CLIENT_PID" 2>/dev/null || { echo "FAIL voice: 客户端退出"; tail -20 "$TMP/client.log"; exit 1; }
grep -qiE "panic|Terminating app" "$TMP/client.log" "$TMP/server.log" && { echo "FAIL voice: 崩溃"; tail -20 "$TMP/client.log"; exit 1; }
echo "PASS voice: room + microphone"

wait_until 20 grep -q "\[screen\] targets=" "$TMP/client.log" || { echo "FAIL voice: 未枚举到共享目标"; exit 1; }
echo "PASS voice: 共享目标枚举（ScreenCaptureKit）"

wait_until 20 grep -q "camera published" "$TMP/client.log" || { echo "FAIL voice: 摄像头未发布"; exit 1; }
wait_until 20 grep -q "screen published" "$TMP/client.log" || { echo "FAIL voice: 屏幕共享未发布"; exit 1; }
echo "PASS voice: 摄像头与屏幕共享已发布"

wait_until 20 grep -q "first local camera frame" "$TMP/client.log" || { echo "FAIL voice: 无本地摄像头预览"; exit 1; }
wait_until 20 grep -q "first local screen frame" "$TMP/client.log" || { echo "FAIL voice: 无本地屏幕预览"; exit 1; }
echo "PASS voice: 本地预览已生成"

wait_until 20 grep -q "event=track_published .*CAMERA" "$TMP/server.log" || { echo "FAIL voice: 缺少 CAMERA webhook"; exit 1; }
wait_until 20 grep -q "event=track_published .*SCREEN_SHARE" "$TMP/server.log" || { echo "FAIL voice: 缺少 SCREEN_SHARE webhook"; exit 1; }
echo "PASS voice: webhook CAMERA + SCREEN_SHARE"

wait_until 15 grep -q "livekit webhook event=participant_joined" "$TMP/server.log" || { echo "FAIL voice: 缺少 participant_joined"; exit 1; }
echo "PASS voice: webhook participant_joined"

for _ in $(seq 1 40); do
  participants="$(curl -s --noproxy '*' -H "Authorization: Bearer $TOKEN" \
    "http://127.0.0.1:$PORT/api/v1/channels/$CHANNEL/voice/participants")"
  if printf '%s' "$participants" | python3 -c 'import json,sys; rows=json.load(sys.stdin); sys.exit(0 if rows and rows[0]["mic"] is True and rows[0]["camera"] is True and rows[0]["screen"] is True else 1)'; then
    echo "PASS voice: participant projection mic/camera/screen=true"
    break
  fi
  sleep 0.25
done

echo "voice e2e passed"
