//! 逐窗口显隐控制（跨平台）。
//!
//! gpui 0.3.12 的公开 API 只有 App 级 `App::hide()`（等价 Cmd-H），没有窗口级
//! 的 orderOut/orderFront。Spotlight 显隐切换与 close-to-tray 都需要逐窗口控制，
//! 因此经 raw-window-handle 取原生窗口句柄直调平台 API；Linux 无对应能力
//! （Wayland 安全模型禁止操作他人窗口），降级为 gpui 跨平台的 minimize/activate。
//! 平台 unsafe 集中在 macos / windows 两个 cfg 分支内，接口收窄为两个函数。

use gpui::Window;

// ---------------------------------------------------------------------------
// macOS：NSWindow orderOut / makeKeyAndOrderFront（objc 直调）
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::Window;
    use raw_window_handle::RawWindowHandle;

    /// 显示并聚焦窗口（等价 makeKeyAndOrderFront + activate）。
    pub fn show(window: &Window) {
        with_ns_window(window, |ns_window| unsafe {
            let _: () = objc2::msg_send![ns_window, makeKeyAndOrderFront: std::ptr::null::<objc2::runtime::AnyObject>()];
        });
        window.activate_window();
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
        let ns_window: *mut objc2::runtime::AnyObject =
            unsafe { objc2::msg_send![ns_view, window] };
        if ns_window.is_null() {
            return None;
        }
        Some(f(ns_window))
    }
}

// ---------------------------------------------------------------------------
// Windows：ShowWindow(SW_HIDE/SW_SHOW) + SetForegroundWindow
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod imp {
    use super::Window;
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW,
    };

    /// 显示并聚焦窗口。
    pub fn show(window: &Window) {
        with_hwnd(window, |hwnd| unsafe {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        });
        window.activate_window();
    }

    /// 隐藏窗口（SW_HIDE），不退出应用——托盘驻留依赖此行为。
    pub fn hide(window: &Window) {
        with_hwnd(window, |hwnd| unsafe {
            ShowWindow(hwnd, SW_HIDE);
        });
    }

    fn with_hwnd<R>(
        window: &Window,
        f: impl FnOnce(windows_sys::Win32::Foundation::HWND) -> R,
    ) -> Option<R> {
        let handle = raw_window_handle::HasWindowHandle::window_handle(window).ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(win32) => {
                // raw-window-handle 0.6 的 hwnd 是 NonZero<isize>（Windows HANDLE 本为整数），
                // windows-sys 0.60 的 HWND 是 *mut c_void，经 isize 整数转换衔接
                Some(f(win32.hwnd.get() as *mut std::ffi::c_void))
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Linux：无窗口级隐藏 API（Wayland 安全模型禁止操作他人窗口），降级为
// gpui 跨平台的 minimize/activate——「关闭到托盘」语义变为「最小化」。
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod imp {
    use super::Window;

    pub fn show(window: &Window) {
        // activate_window：聚焦并前置（X11 抬升窗口，Wayland 走 activation 协议）
        window.activate_window();
    }

    pub fn hide(window: &Window) {
        window.minimize_window();
    }
}

pub use imp::{hide, show};
