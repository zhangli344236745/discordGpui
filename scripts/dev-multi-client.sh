#!/usr/bin/env bash
# 本地多账号联调：创建 3 个测试账号 + 公共服务器 + 互发私信，并拉起客户端。
#
# 用法：scripts/dev-multi-client.sh [--no-launch]
# 依赖：后端已在 8080 运行（scripts/dev-voice.sh），客户端二进制已构建。
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="${DISCORD_API_BASE:-http://127.0.0.1:8080}"
LAUNCH=1
[ "${1:-}" = "--no-launch" ] && LAUNCH=0

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

python3 - "$BASE" <<'PY'
import json
import sys
import time
import urllib.error
import urllib.request

base = sys.argv[1]
api = base + "/api/v1"
PASSWORD = "password123"
USERS = [
    ("alan", "阿岚", "alan.test@example.com"),
    ("ming", "小明", "ming.test@example.com"),
    ("ya", "小雅", "ya.test@example.com"),
]


def call(method, path, body=None, token=None):
    req = urllib.request.Request(api + path, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", "Bearer " + token)
    data = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(req, data=data) as resp:
        return json.loads(resp.read() or b"null")


def login_or_register(email, username, display_name):
    try:
        tokens = call("POST", "/auth/login", {"email": email, "password": PASSWORD})
    except urllib.error.HTTPError:
        tokens = call(
            "POST",
            "/auth/register",
            {
                "email": email,
                "password": PASSWORD,
                "username": username,
                "display_name": display_name,
            },
        )
    me = call("GET", "/users/@me", token=tokens["access_token"])
    return tokens, me


# 演示账号作为主账号（已有客户端）
demo_tokens = call("POST", "/auth/login", {"email": "demo@example.com", "password": PASSWORD})
demo_me = call("GET", "/users/@me", token=demo_tokens["access_token"])

accounts = []
for username, display, email in USERS:
    tokens, me = login_or_register(email, username, display)
    accounts.append(
        {
            "username": username,
            "display": display,
            "email": email,
            "token": tokens["access_token"],
            "refresh": tokens["refresh_token"],
            "id": me["id"],
        }
    )

# 公共服务器：demo 创建（若已存在则复用）
servers = call("GET", "/servers", token=demo_tokens["access_token"])
server = next((s for s in servers if s["name"] == "通话测试"), None)
if server is None:
    server = call(
        "POST",
        "/servers",
        {"name": "通话测试", "is_public": True, "description": "多账号语音/视频联调"},
        demo_tokens["access_token"],
    )

for account in accounts:
    try:
        call("POST", f"/discover/servers/{server['id']}/join", token=account["token"])
    except urllib.error.HTTPError:
        pass

# 两两私信（共享服务器后可直接建）
people = [("demo", demo_tokens["access_token"], demo_me["id"])] + [
    (a["username"], a["token"], a["id"]) for a in accounts
]
for i in range(len(people)):
    for j in range(i + 1, len(people)):
        try:
            call("POST", "/dms", {"user_id": people[j][2]}, people[i][1])
        except urllib.error.HTTPError:
            pass

for account in accounts:
    path = f"/Users/worldunion/.config/discord-gpui/session-{account['username']}.json"
    json.dump({"base": base, "refresh_token": account["refresh"]}, open(path, "w"))
    print(f"session: {path}")

print("server:", server["name"], server["id"])
for account in accounts:
    print(f"account: {account['display']}({account['username']}) / {account['email']} / {PASSWORD}")
PY

chmod 600 ~/.config/discord-gpui/session-*.json 2>/dev/null || true

if [ "$LAUNCH" = "1" ]; then
  for name in alan ming ya; do
    session="$HOME/.config/discord-gpui/session-$name.json"
    [ -f "$session" ] || continue
    DISCORD_API_BASE="$BASE" DISCORD_SESSION_FILE="$session" \
      nohup "$ROOT/target/debug/discord-gpui" >"/tmp/dgpui-client-$name.log" 2>&1 &
    echo "$!" >"/tmp/dgpui-client-$name.pid"
    echo "launched: $name (pid $(cat /tmp/dgpui-client-$name.pid))"
    sleep 2
  done
fi

echo "健康检查：$(curl -s "$BASE/api/v1/health")"
