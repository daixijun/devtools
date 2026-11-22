//! Toast 通知（成功/错误提示）。
//!
//! 经 gpui-component 的 WindowExt 在当前窗口推送 Notification，右上角自动消失。

use gpui::{App, SharedString, Window};
use gpui_component::notification::Notification;
use gpui_component::WindowExt as _;

pub fn toast_success(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::success(message), cx);
}

pub fn toast_error(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::error(message), cx);
}
