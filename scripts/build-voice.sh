#!/usr/bin/env bash
# 构建带真实音视频的客户端（voice-livekit feature）。
#
# 首次需要预编译 libwebrtc。脚本会：
#   1. 检查 $DGPUI_WEBRTC_CACHE/mac-x64-release 是否存在（默认 ~/Library/Caches/dgpui-webrtc）
#   2. 不存在且 AUTO_DOWNLOAD=1 时，从镜像下载并解压（约 223MB）
#   3. 对旧版 Command Line Tools 打两个兼容补丁（幂等）
#   4. 用 LK_CUSTOM_WEBRTC 指向缓存目录执行 cargo build
#
# 用法：
#   scripts/build-voice.sh                 # 仅构建
#   AUTO_DOWNLOAD=1 scripts/build-voice.sh # 缓存缺失时自动下载
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE="${DGPUI_WEBRTC_CACHE:-$HOME/Library/Caches/dgpui-webrtc}"
WEBRTC="$CACHE/mac-x64-release"
URL="https://ghfast.top/https://github.com/livekit/rust-sdks/releases/download/webrtc-a545de4/webrtc-mac-x64-release.zip"

if [ ! -d "$WEBRTC" ]; then
  if [ "${AUTO_DOWNLOAD:-0}" != "1" ]; then
    cat >&2 <<MSG
缺少预编译 libwebrtc：$WEBRTC

请任选其一：
  1) AUTO_DOWNLOAD=1 $0
  2) 手动下载并解压到该目录：
     $URL
MSG
    exit 1
  fi
  echo "==> 下载 libwebrtc（223MB，镜像加速）"
  mkdir -p "$CACHE"
  curl -L --fail --retry 5 --retry-delay 2 -o "$CACHE/webrtc.zip" "$URL"
  unzip -q -o "$CACHE/webrtc.zip" -d "$CACHE"
fi

echo "==> 兼容补丁（旧 CLT 的 libc++）"
python3 - "$WEBRTC" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
h = root / "include/rtc_base/buffer.h"
if h.exists():
    s = h.read_text()
    patched = s.replace("using iterator = std::span<T>::iterator;",
                        "using iterator = typename std::span<T>::iterator;") \
               .replace("using const_iterator = std::span<const T>::iterator;",
                        "using const_iterator = typename std::span<const T>::iterator;")
    if patched != s:
        h.write_text(patched)
        print("patched", h)
PY

for cxxh in "$HOME"/.cargo/registry/src/*/cxx-*/include/cxx.h; do
  [ -f "$cxxh" ] || continue
  python3 - "$cxxh" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
s = p.read_text()
if "using element_type = T;" in s:
    raise SystemExit(0)
marker = "  using value_type = T;\n  using difference_type = std::ptrdiff_t;"
if marker in s:
    p.write_text(s.replace(marker, "  using value_type = T;\n  // 旧 libc++：pointer_traits 需要 element_type\n  using element_type = T;\n  using difference_type = std::ptrdiff_t;", 1))
    print("patched", p)
PY
done

echo "==> 构建（feature = voice-livekit）"
cd "$ROOT"
LK_CUSTOM_WEBRTC="$WEBRTC" cargo build --features voice-livekit "$@"
echo "==> 完成：target/debug/discord-gpui（含真实音视频）"
