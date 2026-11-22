//! 应用单例：抢占唯一通道 + 二次启动激活已有实例。
//!
//! 通道按平台选择：
//! - Unix（macOS/Linux）：domain socket `config_dir/devtools/devtools.sock`
//! - Windows：绑定 `127.0.0.1` 回环固定端口（回环不触发系统防火墙提示）
//!
//! 协议一致：客户端写 `show\n`，服务端收到即 `windows::show_main`。
//!
//! `acquire_or_notify`（App::run 之前调用）：抢占成功 → 本实例持有，返回
//! `true` 继续启动；抢占失败 → 连接既有通道发送 `show` 激活已有实例后返回
//! `false`（本进程退出）；连接也失败（陈旧残留，如上次崩溃）→ 清理后重新
//! 抢占。
//!
//! `init`（主窗口创建后调用）：后台线程 accept 事件泵，经 mpsc 转发进
//! gpui executor 前台消费（AsyncApp 非 Send，不能直接被 std::thread 持有）。
//!
//! 通道文件不做退出清理：残留由下一次启动的「connect 失败 → 清理 →
//! 重新抢占」路径兜底。

use gpui::App;

/// 线程间消息：通道收到的激活请求。
enum SingletonEvent {
    Show,
}

/// accept 循环（平台无关）：逐连接读取首包，`show\n` 即转发激活事件。
/// Unix（UnixStream）与 Windows（TcpStream）的 worker 都收窄到这里。
fn accept_loop<S: std::io::Read>(
    incoming: impl Iterator<Item = std::io::Result<S>>,
    tx: futures::channel::mpsc::UnboundedSender<SingletonEvent>,
) {
    for stream in incoming {
        let Ok(mut stream) = stream else {
            continue;
        };
        let mut buf = [0u8; 16];
        if stream.read(&mut buf).is_ok()
            && buf.starts_with(b"show")
            && tx.unbounded_send(SingletonEvent::Show).is_err()
        {
            break;
        }
    }
}

/// 事件泵骨架：后台线程跑 `worker`（accept 循环 + 读取请求），收到 `show`
/// 经 mpsc 转发，前台任务消费并唤回主窗口（AsyncApp 非 Send，不能直接被
/// std::thread 持有）。
fn pump(
    cx: &App,
    worker: impl FnOnce(futures::channel::mpsc::UnboundedSender<SingletonEvent>) + Send + 'static,
) {
    let (tx, mut rx) = futures::channel::mpsc::unbounded::<SingletonEvent>();
    std::thread::spawn(move || worker(tx));

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

// ---------------------------------------------------------------------------
// Unix（macOS/Linux）：Unix domain socket
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod imp {
    use std::fs;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::OnceLock;

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
    /// `std::sync::mpsc::recv` 这类阻塞调用会把主线程事件循环整个卡死。
    pub fn init(cx: &gpui::App) {
        let Some(listener) = LISTENER.get() else {
            return;
        };

        // accept 循环线程：listener move 进闭包独占（UnixListener 非 Clone）
        let listener = match listener.try_clone() {
            Ok(l) => l,
            Err(e) => {
                eprintln!("单例 socket 克隆失败: {e}（二次启动将无法激活本实例）");
                return;
            }
        };
        super::pump(cx, move |tx| {
            super::accept_loop(listener.incoming(), tx);
        });
    }
}

// ---------------------------------------------------------------------------
// Windows：127.0.0.1 回环固定端口
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use std::net::{TcpListener, TcpStream};
    use std::sync::OnceLock;

    /// 回环单例端口（避开常用服务端口；占用即视为已有实例）。
    const SINGLETON_PORT: u16 = 47613;

    static LISTENER: OnceLock<TcpListener> = OnceLock::new();

    fn addr() -> std::net::SocketAddr {
        std::net::SocketAddr::from(([std::net::Ipv4Addr::LOCALHOST, SINGLETON_PORT]))
    }

    /// 单例抢占入口。返回 `true` 表示本实例继续启动；`false` 表示已成功
    /// 激活现有实例，调用方应立即退出进程。
    pub fn acquire_or_notify() -> bool {
        match TcpListener::bind(addr()) {
            Ok(listener) => {
                let _ = LISTENER.set(listener);
                true
            }
            Err(_) => {
                // 端口被占用：尝试连接既有实例
                match TcpStream::connect(addr()) {
                    Ok(mut stream) => {
                        use std::io::Write as _;
                        if stream.write_all(b"show\n").is_ok() {
                            // 略等对端处理，避免退出过早丢失消息
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            return false;
                        }
                        eprintln!("单例端口写入失败（本次运行不做单例限制）");
                        true
                    }
                    Err(_) => {
                        // 端口被其他应用占用（无应用应答）：本次不做单例限制
                        eprintln!("单例端口被占用且无应用应答（本次运行不做单例限制）");
                        true
                    }
                }
            }
        }
    }

    /// 启动 TCP 事件泵（主窗口创建后调用；无 listener 时为空操作）。
    pub fn init(cx: &gpui::App) {
        let Some(listener) = LISTENER.get() else {
            return;
        };
        super::pump(cx, move |tx| {
            super::accept_loop(listener.incoming(), tx);
        });
    }
}

/// 单例抢占入口。返回 `true` 表示本实例继续启动；`false` 表示已成功激活
/// 现有实例，调用方应立即退出进程。
pub fn acquire_or_notify() -> bool {
    imp::acquire_or_notify()
}

/// 启动通道事件泵（主窗口创建后调用）。
pub fn init(cx: &mut App) {
    imp::init(cx);
}
