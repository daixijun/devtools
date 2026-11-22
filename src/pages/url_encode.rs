//! URL 编码。
//!
//! 独立编码工具：手动触发，将文本转换为 URL 百分号编码。

use gpui::{div, IntoElement, ParentElement as _, Render, Styled as _, Window};
use gpui_component::input::TextareaState;
use gpui_component::ActiveTheme as _;

use crate::components::ui;

pub struct UrlEncode {
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
}

impl UrlEncode {
    pub fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            input: ui::new_textarea(window, cx),
            output: ui::new_textarea(window, cx),
        }
    }

    /// 手动触发编码。
    fn process(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let encoded = devtools::text_tools::url::encode_component(&text);
        self.output.update(cx, |state, cx| {
            state.set_value(encoded.as_str(), window, cx)
        });
        cx.notify();
    }
}

impl Render for UrlEncode {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        ui::tool_page("urlencode")
            // 输入区
            .child(ui::card(cx).child(ui::field("输入:", ui::edit_area(&self.input, 128.), cx)))
            // 居中主按钮（手动触发）
            .child(div().flex().justify_center().child(ui::primary_button(
                "process",
                "编码",
                cx.listener(|this, _ev, window, cx| this.process(window, cx)),
            )))
            // 输出区（只读）
            .child(ui::card(cx).child(ui::field("输出:", ui::result_area(&self.output, 128.), cx)))
            // 固定说明
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_size(gpui::px(13.))
                    .text_color(theme.colors.muted_foreground)
                    .child(div().child("说明:"))
                    .child(div().child("编码: 将文本转换为 URL 安全的百分号编码格式")),
            )
    }
}
