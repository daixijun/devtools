//! tokio 运行时桥（core 的异步逻辑基于 tokio/reqwest，GPUI 有自己的 executor）。
//!
//! 启动时在后台线程建一个 tokio Runtime（常驻），UI 通过 `submit` 把
//! `devtools_core` 的 Future 提交进去，结果经 futures mpsc 回流到
//! `cx.spawn` 前台任务（AsyncApp 非 Send，与 tray/hotkey 事件泵同构）。
//!
//! 用法（工具视图内）：
//! ```ignore
//! let rx = core_rt::submit(cx, async move { check_ssl_info(domain).await });
//! cx.spawn_in(window, async move |this, cx| {
//!     if let Some(result) = rx.next().await {
//!         this.update_in(cx, |this, window, cx| this.on_result(result, window, cx)).ok();
//!     }
//! }).detach();
//! ```

use gpui::App;
use tokio::runtime::Handle;

struct CoreHandle(Handle);

impl gpui::Global for CoreHandle {}

/// 启动后台 tokio Runtime（在独立线程 block_on 保活，Handle 存入 Global）。
pub fn init(cx: &mut App) {
    let (tx, rx) = std::sync::mpsc::channel::<Handle>();
    std::thread::Builder::new()
        .name("core-tokio-rt".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("初始化 core tokio Runtime 失败: {e}");
                    return;
                }
            };
            let _ = tx.send(runtime.handle().clone());
            // 保活：Runtime 在线程内永远 pending，worker 线程随 block_on 运行
            runtime.block_on(async {
                futures::future::pending::<()>().await;
            });
        })
        .expect("启动 core tokio 线程");

    let handle = rx.recv().expect("core tokio Runtime 启动失败");
    cx.set_global(CoreHandle(handle));
}

/// 提交一个 core Future 到 tokio Runtime，返回单值结果流。
pub fn submit<R, F>(cx: &App, fut: F) -> futures::channel::mpsc::UnboundedReceiver<R>
where
    R: Send + 'static,
    F: std::future::Future<Output = R> + Send + 'static,
{
    let handle = cx
        .try_global::<CoreHandle>()
        .map(|h| h.0.clone())
        .expect("core_rt::init 未调用");
    let (tx, rx) = futures::channel::mpsc::unbounded::<R>();
    handle.spawn(async move {
        let result = fut.await;
        let _ = tx.unbounded_send(result);
    });
    rx
}
