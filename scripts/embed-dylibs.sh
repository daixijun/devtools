#!/bin/bash
# DevTools.app 动态库嵌入（修复在无 Homebrew 机器上的启动闪退）。
#
# 背景：cargo 产物直接链接 Homebrew 绝对路径 dylib（libheif / pcre2 及其
# 传递依赖），没装 Homebrew 的机器上 dyld 找不到这些路径，启动即
# "Library not loaded" 崩溃。本脚本三步解决：
#   1. 递归收集主二进制的非系统 dylib → Contents/Frameworks/
#   2. 引用统一改写为 @rpath/<basename>，主二进制加
#      LC_RPATH @executable_path/../Frameworks
#   3. ad-hoc 重签——install_name_tool 的改动会使原有签名失效，
#      Apple Silicon 强制所有可执行代码带有效签名，不重签无法加载
#      （ad-hoc 签名不能带 --timestamp，会请求在线时间戳）
#
# 用法：scripts/embed-dylibs.sh
#   由 scripts/build-app.sh 在组装 bundle 后自动调用。
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

APP="$ROOT/target/DevTools.app"
BIN="$APP/Contents/MacOS/DevTools"
FW="$APP/Contents/Frameworks"

[ -f "$BIN" ] || { echo "❌ 未找到 $BIN，先运行 scripts/build-app.sh"; exit 1; }

is_system_lib() {
  case "$1" in
    @rpath/* | @loader_path/* | @executable_path/* | /System/* | /usr/lib/* | /dev/*) return 0 ;;
    *) return 1 ;;
  esac
}

mkdir -p "$FW"

# ---- 1. 递归收集 dylib（按 basename 落位，实体经 cp -L 解析 symlink）----
echo "▶ 收集动态库 → Contents/Frameworks"
PENDING="$BIN"
while [ -n "${PENDING// /}" ]; do
  NEXT=""
  for target in $PENDING; do
    while IFS= read -r dep; do
      is_system_lib "$dep" && continue
      NAME="$(basename "$dep")"
      if [ ! -f "$FW/$NAME" ]; then
        cp -L "$dep" "$FW/$NAME"
        install_name_tool -id "@rpath/$NAME" "$FW/$NAME"
        echo "  + $NAME"
        NEXT="$NEXT $FW/$NAME"
      fi
    done < <(otool -L "$target" | awk 'NR>1 {print $1}')
  done
  PENDING="$NEXT"
done

# ---- 2. 引用改写为 @rpath（主二进制与各 dylib 的非系统依赖逐条替换）----
rewrite_refs() {
  local f="$1" dep
  while IFS= read -r dep; do
    is_system_lib "$dep" || install_name_tool -change "$dep" "@rpath/$(basename "$dep")" "$f"
  done < <(otool -L "$f" | awk 'NR>1 {print $1}')
}
rewrite_refs "$BIN"
for lib in "$FW"/*; do rewrite_refs "$lib"; done
# add_rpath 重复执行会报 already has，吞掉即可
install_name_tool -add_rpath "@executable_path/../Frameworks" "$BIN" 2>/dev/null || true

# ---- 3. ad-hoc 重签（先 dylib 后 bundle）----
echo "▶ 重签名（ad-hoc）"
for lib in "$FW"/*; do codesign --force --sign - "$lib"; done
codesign --force --sign - "$APP"
codesign --verify --strict "$APP" 2>/dev/null
COUNT=$(ls "$FW" | wc -l | tr -d ' ')
echo "✅ 完成：嵌入 $COUNT 个动态库"
