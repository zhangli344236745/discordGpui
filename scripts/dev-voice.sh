#!/usr/bin/env bash
# 本地语音联调一键启动：LiveKit + 后端 + 客户端。
#
# 默认端口 8080；被占用时可用 PORT=8090 等指定，客户端需带同样的 DISCORD_API_BASE。
# 用法：
#   scripts/dev-voice.sh                 # 启动 LiveKit + 后端，打印客户端启动命令
#   scripts/dev-voice.sh --client        # 额外拉起客户端（带语音 feature 的构建）
#   PORT=8095 scripts/dev-voice.sh       # 换端口
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${PORT:-8080}"
LK_NAME="${LK_NAME:-dgpui-livekit-dev}"
WEBHOOK="http://host.docker.internal:$PORT/api/v1/livekit/webhook"
WITH_CLIENT=0
[ "${1:-}" = "--client" ] && WITH_CLIENT=1

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

port_busy() {
  python3 - "$1" <<'PY'
import socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    s.bind(("127.0.0.1", int(sys.argv[1])))
    print("0")
except OSError:
    print("1")
finally:
    s.close()
PY
}

if [ "$(port_busy "$PORT")" = "1" ]; then
  echo "端口 ${PORT} 已被占用。换一个端口：PORT=8091 $0" >&2
  exit 1
fi

docker image inspect livekit/livekit-server:latest >/dev/null 2>&1 || {
  echo "缺少 livekit/livekit-server 镜像：docker pull livekit/livekit-server:latest" >&2
  exit 1
}

echo "==> LiveKit（${LK_NAME}，webhook → ${WEBHOOK}）"
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
    - $WEBHOOK
" >/dev/null

for _ in $(seq 1 40); do
  curl -sf http://127.0.0.1:7880 >/dev/null 2>&1 && break
  sleep 0.25
done
if [ "$(docker inspect -f '{{.State.Running}}' "$LK_NAME" 2>/dev/null)" != "true" ]; then
  echo "LiveKit 启动失败：" >&2
  docker logs "$LK_NAME" 2>&1 | tail -10 >&2
  exit 1
fi

echo "==> 后端（http://127.0.0.1:${PORT}）"
cd "$ROOT/server"
LIVEKIT_URL="ws://127.0.0.1:7880" \
  nohup uv run uvicorn app.main:app --host 127.0.0.1 --port "$PORT" \
  >"/tmp/dgpui-server-$PORT.log" 2>&1 &
echo $! >"/tmp/dgpui-server-$PORT.pid"
for _ in $(seq 1 80); do
  curl -sf "http://127.0.0.1:$PORT/api/v1/health" >/dev/null 2>&1 && break
  sleep 0.25
done
curl -sf "http://127.0.0.1:$PORT/api/v1/health" >/dev/null || {
  echo "后端启动失败：/tmp/dgpui-server-$PORT.log" >&2
  exit 1
}
echo "健康检查：$(curl -s "http://127.0.0.1:$PORT/api/v1/health")"

if [ "$WITH_CLIENT" = "1" ]; then
  [ -x "$ROOT/target/debug/discord-gpui" ] || {
    echo "缺少客户端构建：先运行 scripts/build-voice.sh" >&2
    exit 1
  }
  echo "==> 客户端（DISCORD_API_BASE=http://127.0.0.1:${PORT}）"
  DISCORD_API_BASE="http://127.0.0.1:$PORT" \
    nohup "$ROOT/target/debug/discord-gpui" >/tmp/dgpui-client.log 2>&1 &
  echo $! >/tmp/dgpui-client.pid
  echo "客户端 PID $(cat /tmp/dgpui-client.pid)，日志 /tmp/dgpui-client.log"
else
  cat <<MSG

完成。启动客户端：

  DISCORD_API_BASE=http://127.0.0.1:$PORT ./target/debug/discord-gpui

演示账号：demo@example.com / password123
MSG
fi
