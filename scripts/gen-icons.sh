#!/bin/bash
# 从 macOS 图标源（assets/AppIcon.icns）再生成跨平台图标资产。
# 仅 macOS 上执行（依赖 sips）；产物提交仓库，常规开发无需重跑。
#
# 产物：
# - assets/icon.png        Linux 打包图标（256x256）
# - assets/devtools.ico    Windows exe 嵌入图标（256/128/64/48/32/16 六帧 PNG-in-ICO）
set -euo pipefail

cd "$(dirname "$0")/.."
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

sips -s format png assets/AppIcon.icns --out "$WORK/icon_full.png" >/dev/null
for s in 512 256 128 64 48 32 16; do
  sips -z $s $s "$WORK/icon_full.png" --out "$WORK/icon_${s}.png" >/dev/null
done
cp "$WORK/icon_256.png" assets/icon.png

python3 - "$WORK" <<'EOF'
# PNG-in-ICO 打包（Vista+ 支持 PNG 帧）：ICONDIR + ICONDIRENTRY* + 各帧 PNG 数据
import struct
import sys

work = sys.argv[1]
sizes = [256, 128, 64, 48, 32, 16]
entries, blobs = [], []
offset = 6 + 16 * len(sizes)
for s in sizes:
    data = open(f"{work}/icon_{s}.png", "rb").read()
    w = 0 if s >= 256 else s  # 256 帧尺寸字段写 0
    entries.append(struct.pack("<BBBBHHII", w, w, 0, 0, 1, 32, len(data), offset))
    blobs.append(data)
    offset += len(data)

ico = struct.pack("<HHH", 0, 1, len(sizes)) + b"".join(entries) + b"".join(blobs)
open("assets/devtools.ico", "wb").write(ico)
print(f"✅ assets/devtools.ico（{len(sizes)} 帧）+ assets/icon.png")
EOF
