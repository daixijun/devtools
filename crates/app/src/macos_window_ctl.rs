//! macOS 逐窗口显隐控制。
//!
//! gpui 0.3.12 的公开 API 只有 App 级 `App::hide()`（等价 Cmd-H），没有窗口级
//! 的 orderOut/orderFront。Spotlight 显隐切换与 close-to-tray 都需要逐窗口控制，
//! 因此通过 raw-window-handle 拿到 NSView，再用 objc 直调 NSWindow。
//! 全部 unsafe 集中在本文件，接口收窄为三个函数。

use gpui::Window;
use raw_window_handle::RawWindowHandle;

/// 显示并聚焦窗口（等价 makeKeyAndOrderFront）。
pub fn show(window: &Window) {
    with_ns_window(window, |ns_window| unsafe {
        let _: () = objc2::msg_send![ns_window, makeKeyAndOrderFront: std::ptr::null::<objc2::runtime::AnyObject>()];
    });
}

/// 隐藏窗口（等价 orderOut），不退出应用。
pub fn hide(window: &Window) {
    with_ns_window(window, |ns_window| unsafe {
        let _: () = objc2::msg_send![ns_window, orderOut: std::ptr::null::<objc2::runtime::AnyObject>()];
    });
}

fn with_ns_window<R>(
    window: &Window,
    f: impl FnOnce(*mut objc2::runtime::AnyObject) -> R,
) -> Option<R> {
    // 显式走 HasWindowHandle trait（Window 的固有 window_handle() 返回 AnyWindowHandle）
    let handle = raw_window_handle::HasWindowHandle::window_handle(window).ok()?;
    let ns_view = match handle.as_raw() {
        RawWindowHandle::AppKit(appkit) => appkit.ns_view,
        _ => return None,
    };
    let ns_view = ns_view.as_ptr() as *mut objc2::runtime::AnyObject;
    // SAFETY: ns_view 由 gpui 持有的合法 NSView；从 NSView 取 NSWindow 是 AppKit 惯用法。
    let ns_window: *mut objc2::runtime::AnyObject = unsafe { objc2::msg_send![ns_view, window] };
    if ns_window.is_null() {
        return None;
    }
    Some(f(ns_window))
}
