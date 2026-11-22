#!/bin/bash
# 开发模式 watch（make run 使用）。
#
# 生命周期与应用绑定（区别于 cargo-watch 的「监视器常驻」语义）：
#   1. 应用运行中保存源文件 → 自动重编译并重启应用；
#   2. 应用正常退出（Cmd+Q / 退出 DevTools）→ watch 一并结束，回到 shell；
#   3. 应用崩溃或编译失败 → 等待下次保存文件再重启（不空转）。
#
# cargo-watch 无法表达第 2 点（子命令退出后监视器仍驻留，需手动 Ctrl+C），
# 故改用自实现的 mtime 轮询监视（find -newer，零额外依赖）。
set -u
cd "$(dirname "$0")/.."

# mtime 快照标记：早于标记的文件视为旧内容
MARKER="$(mktemp -t devwatch-marker)"
trap 'kill $(jobs -p) 2>/dev/null; rm -f "$MARKER"; exit 130' INT TERM

# 阻塞直到 crates/、Cargo.toml、Cargo.lock 出现比标记新的内容，返回后刷新标记。
watch_changed() {
    while true; do
        if [ -n "$(find crates Cargo.toml Cargo.lock -newer "$MARKER" -print -quit 2>/dev/null)" ]; then
            touch "$MARKER"
            return 0
        fi
        sleep 0.5
    done
}

echo "▶ cargo run -p devtools（watch：运行中保存文件自动重启；退出应用即结束 watch）"
while true; do
    cargo run -p devtools &
    APP_PID=$!
    # 监视线程：文件变化 → 终止正在运行的应用，由主循环重新编译重启
    (
        watch_changed
        kill -TERM "$APP_PID" 2>/dev/null
    ) &
    FS_PID=$!

    STATUS=0
    wait "$APP_PID" 2>/dev/null || STATUS=$?

    if kill -0 "$FS_PID" 2>/dev/null; then
        # 监视线程仍在挂起 → 应用是自行退出（非文件变化触发）
        kill "$FS_PID" 2>/dev/null
        wait "$FS_PID" 2>/dev/null
        if [ "$STATUS" -eq 0 ]; then
            break # 正常退出 → watch 结束
        fi
        echo "▶ 应用异常退出（状态 ${STATUS}）。保存文件后自动重编译重启…"
        watch_changed # 阻塞等待下次保存，避免失败循环空转
    fi
    echo "▶ 检测到文件变化，重新编译并重启…"
done
rm -f "$MARKER"
echo "▶ 应用已退出，watch 结束。"
