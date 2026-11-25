#!/usr/bin/env bash
# macOS 发布产物：release 构建 + .app bundle + ad-hoc 签名 + zip。
# 用法：scripts/release-macos.sh [输出目录]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${1:-$ROOT/dist}"
APP_NAME="Discord GPUI"
BUNDLE_ID="workbuddy.discord-gpui"

cd "$ROOT"
echo "==> cargo build --release"
cargo build --release
BIN="$ROOT/target/release/discord-gpui"
[ -x "$BIN" ] || { echo "missing $BIN" >&2; exit 1; }

APP_ROOT="$OUT_DIR/$APP_NAME.app"
CONTENTS="$APP_ROOT/Contents"
rm -rf "$APP_ROOT"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cp "$BIN" "$CONTENTS/MacOS/discord-gpui"

cat >"$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key><string>discord-gpui</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>语音频道与私信通话需要使用麦克风。</string>
  <key>NSCameraUsageDescription</key><string>视频通话需要使用摄像头。</string>
  <key>NSAppTransportSecurity</key>
  <dict><key>NSAllowsLocalNetworking</key><true/></dict>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP_ROOT" >/dev/null 2>&1 || echo "codesign skipped (无签名环境)"

STAMP="$(date -u +%Y%m%d%H%M%S)"
ZIP="$OUT_DIR/discord-gpui-macos-$STAMP.zip"
ditto -c -k --keepParent "$APP_ROOT" "$ZIP" 2>/dev/null || zip -r -q "$ZIP" "$APP_ROOT"

echo "==> 完成"
echo "bundle: $APP_ROOT"
echo "archive: $ZIP"
echo "验证：全新环境解压后双击启动 → 登录页可登录（DISCORD_API_BASE 指向后端）。"
