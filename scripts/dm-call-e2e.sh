#!/usr/bin/env bash
# 私信通话真实媒体冒烟：本地 LiveKit + 后端 + 两个客户端（主叫/被叫）。
#
# 前置：scripts/build-voice.sh 构建过带 voice-livekit 的客户端；
#       docker 可用且已有 livekit/livekit-server 镜像。
# 用法：scripts/dm-call-e2e.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
PORT="${DM_E2E_PORT:-8088}"
LK="dgpui-livekit-dm-e2e"
SERVER_PID=""
A_PID=""
B_PID=""

cleanup() {
  [ -n "$A_PID" ] && kill "$A_PID" 2>/dev/null || true
  [ -n "$B_PID" ] && kill "$B_PID" 2>/dev/null || true
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  docker rm -f "$LK" >/dev/null 2>&1 || true
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

docker rm -f "$LK" >/dev/null 2>&1 || true
docker run -d --name "$LK" \
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

for _ in $(seq 1 40); do
  curl -sf --noproxy '*' http://127.0.0.1:7883 >/dev/null 2>&1 && break
  sleep 0.25
done
if [ "$(docker inspect -f '{{.State.Running}}' "$LK" 2>/dev/null)" != "true" ]; then
  echo "FAIL dm-call: LiveKit 容器启动失败" >&2
  docker logs "$LK" 2>&1 | tail -10 >&2
  exit 1
fi

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
curl -sf --noproxy '*' "http://127.0.0.1:$PORT/api/v1/health" >/dev/null || {
  echo "FAIL dm-call: 后端未就绪" >&2
  tail -20 "$TMP/server.log" >&2
  exit 1
}

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


a = call("POST", "/auth/register", {"email": "dm-a@example.com", "password": "password123", "username": "dma"})
b = call("POST", "/auth/register", {"email": "dm-b@example.com", "password": "password123", "username": "dmb"})
am = call("GET", "/users/@me", token=a["access_token"])
bm = call("GET", "/users/@me", token=b["access_token"])
# 必须同服才能直接建私信（否则会走消息请求）
srv = call("POST", "/servers", {"name": "DMLab", "is_public": True}, a["access_token"])
call("POST", f"/discover/servers/{srv['id']}/join", token=b["access_token"])
dm = call("POST", "/dms", {"user_id": am["id"]}, b["access_token"])["dm"]["id"]

json.dump({"base": f"http://127.0.0.1:{port}", "refresh_token": a["refresh_token"]}, open(f"{tmp}/a.json", "w"))
json.dump({"base": f"http://127.0.0.1:{port}", "refresh_token": b["refresh_token"]}, open(f"{tmp}/b.json", "w"))
print(dm)
PY
chmod 600 "$TMP/a.json" "$TMP/b.json"

# B 先上线等待来电；A 后启动并发起呼叫
DISCORD_API_BASE="http://127.0.0.1:$PORT" DISCORD_SESSION_FILE="$TMP/b.json" DISCORD_DM_CALL_AUTO="answer" \
  "$ROOT/target/debug/discord-gpui" >"$TMP/b.log" 2>&1 &
B_PID=$!
sleep 7
DISCORD_API_BASE="http://127.0.0.1:$PORT" DISCORD_SESSION_FILE="$TMP/a.json" \
DISCORD_DM_CALL_AUTO="call" DISCORD_DM_CALL_CAMERA="1" DISCORD_DM_CALL_SCREEN="1" \
DISCORD_VOICE_FORCE_LOW="1" \
  "$ROOT/target/debug/discord-gpui" >"$TMP/a.log" 2>&1 &
A_PID=$!
wait_until() {
  local seconds=$1
  shift
  for _ in $(seq 1 $((seconds * 4))); do
    if "$@"; then return 0; fi
    sleep 0.25
  done
  return 1
}

fail() { echo "FAIL dm-call: $1" >&2; tail -20 "$TMP/a.log" "$TMP/b.log" >&2; exit 1; }

for role in a b; do
  wait_until 30 grep -q "\[voice\] room connected" "$TMP/$role.log" || fail "$role 未连接通话房间"
  wait_until 15 grep -q "microphone published" "$TMP/$role.log" || fail "$role 麦克风未发布"
  kill -0 "$([ "$role" = a ] && echo "$A_PID" || echo "$B_PID")" 2>/dev/null || fail "$role 客户端退出"
  grep -qiE "panic|Terminating app" "$TMP/$role.log" && fail "$role 崩溃"
done
echo "PASS dm-call: 双端连接房间并发布麦克风"

wait_until 20 bash -c "grep -c 'livekit webhook event=participant_joined' '$TMP/server.log' | grep -q '^2$'" ||
  fail "缺少 participant_joined x2"
echo "PASS dm-call: webhook participant_joined x2"

wait_until 20 bash -c "[ \$(grep -c 'livekit webhook event=track_published' '$TMP/server.log') -ge 2 ]" ||
  fail "缺少 track_published x2"
echo "PASS dm-call: webhook track_published x2"

grep -q 'livekit webhook event=participant_joined room=dm:' "$TMP/server.log" ||
  fail "房间名不是 dm: 前缀"
echo "PASS dm-call: 通话房间名 dm: 前缀"

# 摄像头：A 端自动开启，LiveKit 应上报 CAMERA 轨道
wait_until 20 grep -q "camera published" "$TMP/a.log" || fail "摄像头未发布"
wait_until 20 grep -q 'event=track_published .*CAMERA' "$TMP/server.log" ||
  fail "缺少 CAMERA track_published webhook"
echo "PASS dm-call: 摄像头轨道已发布"

# 屏幕共享：A 端自动开启，LiveKit 应上报 SCREEN_SHARE 轨道
wait_until 25 grep -q "screen published" "$TMP/a.log" || fail "屏幕共享未发布"
wait_until 20 grep -q "event=track_published .*SCREEN_SHARE" "$TMP/server.log" ||
  fail "缺少 SCREEN_SHARE track_published webhook"
echo "PASS dm-call: 屏幕共享轨道已发布"

# 本地预览：A 端应把自己的摄像头/屏幕回灌到 UI 管线
wait_until 25 grep -q "first local camera frame" "$TMP/a.log" || fail "A 端无本地摄像头预览"
wait_until 20 grep -q "first local screen frame" "$TMP/a.log" || fail "A 端无本地屏幕预览"
echo "PASS dm-call: 本地摄像头/屏幕预览已生成"

# 远端渲染：B 端应解出 A 的摄像头/屏幕帧（BGRA → RenderImage）
wait_until 25 grep -q "first remote camera frame" "$TMP/b.log" || fail "B 端未渲染远端摄像头"
wait_until 20 grep -q "first remote screen frame .*1280x720" "$TMP/b.log" ||
  fail "B 端屏幕共享未按高码率单层（1280x720）渲染"
echo "PASS dm-call: 远端摄像头/屏幕画面已渲染"

# 码率自适应：强制降档后重新发布成功且进程存活
wait_until 20 grep -q "video quality -> low" "$TMP/a.log" || fail "未触发降档"
kill -0 "$A_PID" 2>/dev/null || fail "降档后客户端退出"
grep -qiE "panic|Terminating app" "$TMP/a.log" && fail "降档后崩溃"
echo "PASS dm-call: 弱网降档重新发布成功"

# 降档后重新发布：订阅端应再次收到轨道（初始 2 路 + 重发 2 路）
wait_until 20 bash -c "[ \$(grep -c 'remote video subscribed' '$TMP/b.log') -ge 4 ]" ||
  fail "降档后未重新订阅轨道"
echo "PASS dm-call: 降档后对端重新订阅"

echo "dm call e2e passed"
