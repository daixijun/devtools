#!/bin/bash
# 构建 DevTools.dmg 安装镜像（固定 release 构建；零外部依赖）。
#
# 镜像为标准「拖拽安装」布局：DevTools.app + 指向 /Applications 的符号链接，
# 用户打开 DMG 后把 App 拖进 Applications 即完成安装。
#
# 用法：scripts/build-dmg.sh
#   依赖先由 scripts/build-app.sh release 构建出 target/DevTools.app。
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

APP="$ROOT/target/DevTools.app"
DMG="$ROOT/target/DevTools.dmg"

echo "▶ 构建 DevTools.app (release)"
scripts/build-app.sh release

[ -d "$APP" ] || { echo "❌ 未找到 $APP"; exit 1; }

STAGING="$ROOT/target/.dmg-staging"
echo "▶ 组装 DMG 目录布局"
rm -rf "$STAGING" "$DMG"
mkdir -p "$STAGING"
cp -R "$APP" "$STAGING/"
ln -s /Applications "$STAGING/Applications"

echo "▶ 生成 $DMG"
# macOS 15+ 优先用新的 diskutil image（hdiutil create 已弃用告警）；
# diskutil 在部分系统（如 CI runner）不可用时自动回退 hdiutil
if [ "$(sw_vers -productVersion | cut -d. -f1)" -ge 15 ] 2>/dev/null &&
    diskutil image create from \
        --volumeName "DevTools" \
        --format UDZO \
        "$STAGING" "$DMG"; then
    :
else
    hdiutil create \
        -volname "DevTools" \
        -srcfolder "$STAGING" \
        -ov -format UDZO \
        "$DMG" | tail -1
fi

rm -rf "$STAGING"

SIZE=$(du -h "$DMG" | cut -f1 | tr -d ' ')
echo "✅ DevTools.dmg 构建完成：${DMG}（${SIZE}）"
echo "   验证：hdiutil attach ${DMG}"
