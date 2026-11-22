//! 「关于」窗口（app 菜单 → 关于 DevTools）。

use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, Render, Window};
use gpui_component::ActiveTheme as _;

pub struct AboutView;

impl AboutView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for AboutView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let version = env!("CARGO_PKG_VERSION");

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(
                div()
                    .text_size(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("DevTools 开发者工具箱"),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(theme.colors.muted_foreground)
                    .child(format!("版本 {version} (DevTools)")),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(12.5))
                    .text_color(theme.colors.muted_foreground)
                    .child("面向开发者的 macOS 效率工具箱，内置 31 个常用工具"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .mt_1()
                    .text_color(theme.colors.accent)
                    .child("devtools.linuxyunwei.com"),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(px(11.))
                    .text_color(theme.colors.muted_foreground.opacity(0.7))
                    .child("© DevTools · MIT License"),
            )
    }
}
