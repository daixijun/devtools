//! Base64 编解码综合工具。
//!
//! 编码/解码双模式；输入经 200ms 防抖自动转换；mode 变化与「示例/交换」
//! 等程序化修改立即重算。契约见 docs/plans/tool-specs/batch1.md。

use std::time::Duration;

use gpui::{
    div, App, AppContext as _, ClipboardItem, IntoElement, ParentElement as _, Render, Styled as _,
    Window,
};
use gpui_component::input::{InputEvent, TextareaState};
use gpui_component::Disableable as _;

use crate::components::{notify, ui};

/// 解码模式「示例」填入的文本（示例文本的 Base64 编码）。
const EXAMPLE_TEXT: &str = "Hello, World! 你好，世界！";
const DEBOUNCE: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Encode,
    Decode,
}

pub struct Base64Converter {
    mode: Mode,
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
    error: Option<String>,
    /// 防抖版本号：每次输入变化自增，timer 到期后版本一致才执行转换
    /// （连续输入只有最后一次生效，天然去抖）。
    generation: u64,
}

impl Base64Converter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input = ui::new_textarea(window, cx);
        let output = ui::new_textarea(window, cx);
        cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.schedule_convert(cx);
            }
        })
        .detach();
        let this = Self {
            mode: Mode::Encode,
            input,
            output,
            error: None,
            generation: 0,
        };
        this.sync_input_placeholder(window, cx);
        this
    }

    // ------------------------------------------------------------------
    // 状态操作
    // ------------------------------------------------------------------

    fn input_placeholder(mode: Mode) -> &'static str {
        match mode {
            Mode::Encode => "请输入要编码的文本...",
            Mode::Decode => "请输入要解码的Base64文本...",
        }
    }

    fn output_placeholder(mode: Mode) -> &'static str {
        match mode {
            Mode::Encode => "编码结果将在这里显示...",
            Mode::Decode => "解码结果将在这里显示...",
        }
    }

    fn sync_input_placeholder(&self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let placeholder = Self::input_placeholder(self.mode);
        self.input.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx)
        });
    }

    /// 输入变化：登记新版本后延迟 200ms 转换；期间又有输入则本次作废。
    fn schedule_convert(&mut self, cx: &mut gpui::Context<Self>) {
        self.generation += 1;
        let gen = self.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.generation == gen {
                    this.convert(window, cx);
                }
            });
        })
        .detach();
    }

    fn set_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.sync_input_placeholder(window, cx);
        self.convert(window, cx); // mode 变化立即重算（不防抖）
    }

    fn convert(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let result = if text.is_empty() {
            Ok(String::new())
        } else {
            match self.mode {
                Mode::Encode => Ok(devtools_core::text_tools::base64::encode(&text)),
                Mode::Decode => devtools_core::text_tools::base64::decode(&text),
            }
        };

        match result {
            Ok(value) => {
                self.error = None;
                let placeholder = Self::output_placeholder(self.mode);
                self.output.update(cx, |state, cx| {
                    state.set_value(value.as_str(), window, cx);
                    state.set_placeholder(placeholder, window, cx);
                });
            }
            Err(msg) => {
                let prefix = match self.mode {
                    // 编码路径不会失败，此处仅保持统一的错误前缀
                    Mode::Encode => "Base64编码失败",
                    Mode::Decode => "Base64解码失败",
                };
                let display = format!("{prefix}: {msg}");
                self.error = Some(display.clone());
                self.output.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                    state.set_placeholder(display.as_str(), window, cx);
                });
            }
        }
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
            // 选中高亮：primary 背景
            ui::primary_button(id, label, listener)
        } else {
            ui::secondary_button(id, label, listener)
        }
    }
}

impl Render for Base64Converter {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let (title, subtitle) = match self.mode {
            Mode::Encode => ("Base64 编码", "将文本编码为Base64格式"),
            Mode::Decode => ("Base64 解码", "将Base64格式解码为普通文本"),
        };
        let input_len = self.input.read(cx).value().chars().count();
        let output_text = self.output.read(cx).value().to_string();
        let has_output = !output_text.is_empty();
        let has_error = self.error.is_some();

        // 「示例」：编码模式填示例文本；解码模式填其 Base64
        let example_value = match self.mode {
            Mode::Encode => EXAMPLE_TEXT.to_string(),
            Mode::Decode => devtools_core::text_tools::base64::encode(EXAMPLE_TEXT),
        };
        let output_for_copy = output_text.clone();

        ui::tool_page("base64converter")
            .child(ui::themed_header(title, subtitle, cx))
            // 模式分段按钮 + 操作按钮
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(self.mode_button("mode-encode", "编码", Mode::Encode, cx))
                            .child(self.mode_button("mode-decode", "解码", Mode::Decode, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(ui::secondary_button(
                                "load-example",
                                "示例",
                                cx.listener(move |this, _ev, window, cx| {
                                    this.input.update(cx, |state, cx| {
                                        state.set_value(example_value.as_str(), window, cx)
                                    });
                                    this.convert(window, cx);
                                }),
                            ))
                            .child(
                                ui::secondary_button(
                                    "swap",
                                    "交换输入/输出",
                                    cx.listener(move |this, _ev, window, cx| {
                                        let output = this.output.read(cx).value().to_string();
                                        this.input.update(cx, |state, cx| {
                                            state.set_value(output.as_str(), window, cx)
                                        });
                                        this.mode = match this.mode {
                                            Mode::Encode => Mode::Decode,
                                            Mode::Decode => Mode::Encode,
                                        };
                                        this.sync_input_placeholder(window, cx);
                                        this.convert(window, cx);
                                    }),
                                )
                                .disabled(!has_output || has_error),
                            ),
                    ),
            )
            // 输入区
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(ui::hint_text(&format!("输入长度: {input_len}"), cx))
                            .child(
                                ui::secondary_button(
                                    "clear",
                                    "清空",
                                    cx.listener(|this, _ev, window, cx| {
                                        this.input.update(cx, |state, cx| {
                                            state.set_value("", window, cx)
                                        });
                                        this.convert(window, cx);
                                    }),
                                )
                                .disabled(input_len == 0),
                            ),
                    )
                    .child(ui::edit_area(&self.input, 160.)),
            )
            // 输出区
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(ui::hint_text(
                                &format!("输出长度: {}", output_text.chars().count()),
                                cx,
                            ))
                            .child(
                                ui::primary_button(
                                    "copy-result",
                                    "复制结果",
                                    move |_ev, window, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            output_for_copy.clone(),
                                        ));
                                        notify::toast_success(window, cx, "已复制");
                                    },
                                )
                                .disabled(!has_output || has_error),
                            ),
                    )
                    .child(ui::result_area(&self.output, 160.)),
            )
            // 内联错误横条（无错误时不渲染）
            .children(self.error.clone().map(|e| ui::error_banner(&e, cx)))
    }
}
