//! 应用单例：Unix domain socket 抢占 + 二次启动激活已有实例。
//!
//! socket 路径 `config_dir/devtools/devtools.sock`：
//! - `acquire_or_notify`（App::run 之前调用）：bind 成功 → 本实例持有，
//!   返回 `true` 继续启动；bind 失败 → 尝试连接并发送 `show` 激活已有
//!   实例后返回 `false`（本进程退出）；连接也失败（陈旧 socket，如上次
//!   崩溃残留）→ 删除后重新 bind。
//! - `init`（主窗口创建后调用）：后台线程 accept 事件泵，经 mpsc 转发
//!   进 gpui executor 前台消费，收到 `show` 即 `windows::show_main`
//!   （与 tray.rs 同构：AsyncApp 非 Send，不能直接被 std::thread 持有）。
//!
//! socket 文件不做退出清理：残留由下一次启动的「connect 失败 → unlink →
//! rebind」路径兜底。

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::OnceLock;

use gpui::App;

/// 线程间消息：socket 收到的激活请求。
enum SingletonEvent {
    Show,
}

/// 持有的 listener（本实例是唯一实例时为 Some）。
static LISTENER: OnceLock<UnixListener> = OnceLock::new();

fn socket_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("devtools")
        .join("devtools.sock")
}

/// 单例抢占入口。返回 `true` 表示本实例继续启动；`false` 表示已成功
/// 激活现有实例，调用方应立即退出进程。
pub fn acquire_or_notify() -> bool {
    let path = socket_path();
    match UnixListener::bind(&path) {
        Ok(listener) => {
            let _ = LISTENER.set(listener);
            true
        }
        Err(_) => {
            // bind 失败：可能已有实例（socket 被占用）或残留陈旧 socket
            match UnixStream::connect(&path) {
                Ok(mut stream) => {
                    // 已有实例：请求其显示主窗口后本进程退出
                    use std::io::Write as _;
                    if stream.write_all(b"show\n").is_ok() {
                        // 略等对端处理，避免退出过早丢失消息
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        return false;
                    }
                    // 写失败：对端异常，按陈旧 socket 处理
                    try_rebind_stale(&path);
                    true
                }
                Err(_) => {
                    // 连接失败：陈旧 socket，删除后重新 bind
                    try_rebind_stale(&path);
                    true
                }
            }
        }
    }
}

/// 删除陈旧 socket 并重新 bind；bind 结果写入 LISTENER。
fn try_rebind_stale(path: &PathBuf) {
    eprintln!("检测到陈旧的实例 socket，已清理: {}", path.display());
    let _ = fs::remove_file(path);
    if let Ok(listener) = UnixListener::bind(path) {
        let _ = LISTENER.set(listener);
    } else {
        eprintln!("单例 socket 重建失败（本次运行不做单例限制）");
    }
}

/// 启动 socket 事件泵（主窗口创建后调用；无 listener 时为空操作）。
///
/// 前台消费必须用 futures 异步流：`cx.spawn` 的任务跑在主线程 executor，
/// `std::sync::mpsc::recv` 这类阻塞调用会把主线程事件循环整个卡死
/// （与 tray.rs 的事件泵同构）。
pub fn init(cx: &mut App) {
    let Some(listener) = LISTENER.get() else {
        return;
    };

    let (tx, mut rx) = futures::channel::mpsc::unbounded::<SingletonEvent>();
    // accept 循环线程：listener 由本线程独占（UnixListener 非 Clone）
    let listener = match listener.try_clone() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("单例 socket 克隆失败: {e}（二次启动将无法激活本实例）");
            return;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            use std::io::Read as _;
            let mut buf = [0u8; 16];
            if stream.read(&mut buf).is_ok()
                && buf.starts_with(b"show")
                && tx.unbounded_send(SingletonEvent::Show).is_err()
            {
                break;
            }
        }
    });

    cx.spawn(async move |cx| {
        while let Some(event) = futures::StreamExt::next(&mut rx).await {
            match event {
                SingletonEvent::Show => {
                    cx.update(crate::windows::show_main);
                }
            }
        }
    })
    .detach();
}
