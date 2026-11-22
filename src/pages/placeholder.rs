//! 未注册工具的占位页（兜底，正常情况下 32 个工具均有专属页面）。

use gpui::{div, prelude::*, px, App, IntoElement, Render, Window};
use gpui_component::ActiveTheme as _;

use crate::components::ui;

pub struct Placeholder;

impl Placeholder {
    pub fn create_view(_window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|_| Self).into()
    }
}

impl Render for Placeholder {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        ui::tool_page("placeholder").child(
            div()
                .flex()
                .flex_1()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.colors.muted_foreground)
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(theme.colors.foreground)
                        .child("页面不可用"),
                )
                .child(div().text_size(px(13.)).child("未注册的工具 id")),
        )
    }
}
