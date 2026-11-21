//! URL 编码/解码工具。
//!
//! 手动触发（无防抖）；模式切换仅更新占位文案；异常把 `错误: <msg>`
//! 写入输出框。契约见 docs/plans/tool-specs/batch1.md。

use gpui::{
    div, App, AppContext as _, IntoElement, ParentElement as _, Render, Styled as _, Window,
};
use gpui_component::input::TextareaState;
use gpui_component::ActiveTheme as _;

use crate::components::ui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Encode,
    Decode,
}

pub struct UrlEncoderDecoder {
    mode: Mode,
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
}

impl UrlEncoderDecoder {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input = ui::new_textarea(window, cx);
        let output = ui::new_textarea(window, cx);
        let this = Self {
            mode: Mode::Encode,
            input,
            output,
        };
        this.sync_input_placeholder(window, cx);
        this
    }

    // ------------------------------------------------------------------
    // 状态操作
    // ------------------------------------------------------------------

    fn input_placeholder(mode: Mode) -> &'static str {
        match mode {
            Mode::Encode => "输入要编码的文本",
            Mode::Decode => "输入要解码的 URL",
        }
    }

    fn sync_input_placeholder(&self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let placeholder = Self::input_placeholder(self.mode);
        self.input.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx)
        });
    }

    fn set_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.sync_input_placeholder(window, cx);
        cx.notify();
    }

    /// 手动触发编码/解码；异常把 `错误: <msg>` 写入输出框。
    fn process(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let result = match self.mode {
            Mode::Encode => devtools_core::text_tools::url::encode_component(&text),
            Mode::Decode => match devtools_core::text_tools::url::decode_component(&text) {
                Ok(decoded) => decoded,
                Err(msg) => format!("错误: {msg}"),
            },
        };
        self.output
            .update(cx, |state, cx| state.set_value(result.as_str(), window, cx));
        cx.notify();
    }

    /// 交换输入/输出并翻转模式（无 disabled 条件）。
    fn swap(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let input_value = self.input.read(cx).value().to_string();
        let output_value = self.output.read(cx).value().to_string();
        self.input.update(cx, |state, cx| {
            state.set_value(output_value.as_str(), window, cx)
        });
        self.output.update(cx, |state, cx| {
            state.set_value(input_value.as_str(), window, cx)
        });
        self.mode = match self.mode {
            Mode::Encode => Mode::Decode,
            Mode::Decode => Mode::Encode,
        };
        self.sync_input_placeholder(window, cx);
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        mode: Mode,
        cx: &mut gpui::Context<Self>,
    ) -> gpui_component::button::Button {
        let listener = cx.listener(move |this, _ev, window, cx| {
            this.set_mode(mode, window, cx);
        });
        if self.mode == mode {
            ui::primary_button(id, label, listener)
        } else {
            ui::secondary_button(id, label, listener)
        }
    }
}

impl Render for UrlEncoderDecoder {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let action_label = match self.mode {
            Mode::Encode => "编码",
            Mode::Decode => "解码",
        };

        ui::tool_page("urlencoderdecoder")
            .child(ui::themed_header(
                "URL 编码/解码",
                "URL 编码和解码工具，支持 URI 组件编码",
                cx,
            ))
            // 模式 + 交换
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::hint_text("操作模式:", cx))
                    .child(self.mode_button("mode-encode", "编码", Mode::Encode, cx))
                    .child(self.mode_button("mode-decode", "解码", Mode::Decode, cx))
                    .child(ui::secondary_button(
                        "swap",
                        "交换输入/输出",
                        cx.listener(|this, _ev, window, cx| this.swap(window, cx)),
                    )),
            )
            // 输入区
            .child(ui::card(cx).child(ui::field("输入:", ui::edit_area(&self.input, 128.), cx)))
            // 居中主按钮（手动触发）
            .child(div().flex().justify_center().child(ui::primary_button(
                "process",
                action_label,
                cx.listener(|this, _ev, window, cx| this.process(window, cx)),
            )))
            // 输出区（只读，无复制按钮）
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
                    .child(div().child("编码: 将文本转换为 URL 安全的格式"))
                    .child(div().child("解码: 将 URL 编码的文本还原为原始格式"))
                    .child(div().child("使用 encodeURIComponent/decodeURIComponent 函数")),
            )
    }
}
