#!/bin/bash
# 构建平台产物入口（跨平台）：
# - macOS：DevTools.app bundle（含 Info.plist 与 icns 图标）
# - Windows / Linux：仅 cargo build 出裸二进制（打包由 CI/release.yml 完成）
# 用法：scripts/build-app.sh [release]
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
# 应用版本（Cargo.toml package version，写入 Info.plist）
CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
PROFILE="debug"
CARGO_FLAGS=""
if [ "${1:-}" = "release" ]; then
  PROFILE="release"
  CARGO_FLAGS="--release"
fi

echo "▶ cargo build $CARGO_FLAGS"
cargo build $CARGO_FLAGS

if [ "$(uname)" = "Darwin" ]; then
  BIN="$ROOT/target/$PROFILE/devtools"
  APP="$ROOT/target/DevTools.app"
  ICONS="$ROOT/assets"

  echo "▶ 组装 $APP"
  rm -rf "$APP"
  mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

  cp "$BIN" "$APP/Contents/MacOS/DevTools"
  cp "$ICONS/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

  cat > "$APP/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>DevTools</string>
    <key>CFBundleDisplayName</key>
    <string>DevTools</string>
    <key>CFBundleIdentifier</key>
    <string>devtools.linuxyunwei.com</string>
    <key>CFBundleVersion</key>
    <string>${CARGO_VERSION?}</string>
    <key>CFBundleShortVersionString</key>
    <string>${CARGO_VERSION?}</string>
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

  # 嵌入 Homebrew 动态库（libheif/pcre2 等绝对路径在用户机器不存在，
  # 不嵌入会启动即 "Library not loaded"；脚本内含 ad-hoc 重签）
  scripts/embed-dylibs.sh

  # 最小化二进制（release 时已 strip；debug 保留）
  echo "✅ DevTools.app 构建完成：$APP"
  echo "   运行：open $APP"
else
  BIN_NAME="devtools"
  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) BIN_NAME="devtools.exe" ;;
  esac
  BIN="$ROOT/target/$PROFILE/$BIN_NAME"
  [ -f "$BIN" ] || { echo "❌ 未找到 $BIN"; exit 1; }
  echo "✅ 二进制构建完成：$BIN"
fi
