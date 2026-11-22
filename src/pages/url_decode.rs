//! URL 解码。
//!
//! 独立解码工具：手动触发，将 URL 编码的文本还原；异常把 `错误: <msg>`
//! 写入输出框。

use gpui::{div, IntoElement, ParentElement as _, Render, Styled as _, Window};
use gpui_component::input::TextareaState;
use gpui_component::ActiveTheme as _;

use crate::components::ui;

pub struct UrlDecode {
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
}

impl UrlDecode {
    pub fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            input: ui::new_textarea(window, cx),
            output: ui::new_textarea(window, cx),
        }
    }

    /// 手动触发解码；异常把 `错误: <msg>` 写入输出框。
    fn process(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let result = match devtools::text_tools::url::decode_component(&text) {
            Ok(decoded) => decoded,
            Err(msg) => format!("错误: {msg}"),
        };
        self.output
            .update(cx, |state, cx| state.set_value(result.as_str(), window, cx));
        cx.notify();
    }
}

impl Render for UrlDecode {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        ui::tool_page("urldecode")
            .child(ui::card(cx).child(ui::field("输入:", ui::edit_area(&self.input, 128.), cx)))
            .child(div().flex().justify_center().child(ui::primary_button(
                "process",
                "解码",
                cx.listener(|this, _ev, window, cx| this.process(window, cx)),
            )))
            .child(ui::card(cx).child(ui::field("输出:", ui::result_area(&self.output, 128.), cx)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_size(gpui::px(13.))
                    .text_color(theme.colors.muted_foreground)
                    .child(div().child("说明:"))
                    .child(div().child("解码: 将 URL 编码的文本还原为原始格式")),
            )
    }
}
