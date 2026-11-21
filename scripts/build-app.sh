#!/bin/bash
# 构建 DevTools.app（GPUI 壳打包）
# 用法：scripts/build-app.sh [release]
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
PROFILE="debug"
CARGO_FLAGS=""
if [ "${1:-}" = "release" ]; then
  PROFILE="release"
  CARGO_FLAGS="--release"
fi

echo "▶ cargo build -p devtools-app $CARGO_FLAGS"
cargo build -p devtools-app $CARGO_FLAGS

BIN="$ROOT/target/$PROFILE/devtools-gpui"
APP="$ROOT/target/DevTools.app"
ICONS="$ROOT/crates/app/assets"

echo "▶ 组装 $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/DevTools"
cp "$ICONS/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>DevTools</string>
    <key>CFBundleDisplayName</key>
    <string>开发者工具箱</string>
    <key>CFBundleIdentifier</key>
    <string>devtools.linuxyunwei.com</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleExecutable</key>
    <string>DevTools</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
PLIST

# 最小化二进制（release 时已 strip；debug 保留）
echo "✅ DevTools.app 构建完成：$APP"
echo "   运行：open $APP"
