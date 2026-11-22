//! Base64 解码。
//!
//! 独立解码工具：Base64 文本输入经 200ms 防抖自动解码还原；非法输入
//! 直接在输出区提示。

use std::time::Duration;

use devtools::i18n::t;
use gpui::{div, ClipboardItem, IntoElement, ParentElement as _, Render, Styled as _, Window};
use gpui_component::input::{InputEvent, TextareaState};
use gpui_component::Disableable as _;

use crate::components::{notify, ui};

/// 「示例」对应的原文（示例填入其 Base64 编码结果）。
const EXAMPLE_TEXT: &str = "Hello, World! 你好，世界！";
const DEBOUNCE: Duration = Duration::from_millis(200);

pub struct Base64Decode {
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
    error: Option<String>,
    generation: u64,
}

impl Base64Decode {
    pub fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input = ui::new_textarea(window, cx);
        let output = ui::new_textarea(window, cx);
        cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.schedule_convert(cx);
            }
        })
        .detach();
        Self {
            input,
            output,
            error: None,
            generation: 0,
        }
    }

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

    fn convert(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let result = if text.is_empty() {
            Ok(String::new())
        } else {
            devtools::text_tools::base64::decode(&text)
        };

        match result {
            Ok(decoded) => {
                self.error = None;
                self.output.update(cx, |state, cx| {
                    state.set_value(decoded.as_str(), window, cx)
                });
            }
            Err(msg) => {
                let display = format!("{}: {}", t("Base64解码失败"), msg);
                self.error = Some(display.clone());
                self.output.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                    state.set_placeholder(display.as_str(), window, cx);
                });
            }
        }
        cx.notify();
    }
}

impl Render for Base64Decode {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let input_len = self.input.read(cx).value().chars().count();
        let output_text = self.output.read(cx).value().to_string();
        let has_output = !output_text.is_empty();
        let has_error = self.error.is_some();

        ui::tool_page("base64decode")
            .child(
                ui::toolbar()
                    .justify_between()
                    .child(ui::secondary_button(
                        "load-example",
                        t("示例"),
                        cx.listener(|this, _ev, window, cx| {
                            let example = devtools::text_tools::base64::encode(EXAMPLE_TEXT);
                            this.input.update(cx, |state, cx| {
                                state.set_value(example.as_str(), window, cx)
                            });
                            this.convert(window, cx);
                        }),
                    ))
                    .child(
                        ui::secondary_button(
                            "clear",
                            t("清空"),
                            cx.listener(|this, _ev, window, cx| {
                                this.input
                                    .update(cx, |state, cx| state.set_value("", window, cx));
                                this.convert(window, cx);
                            }),
                        )
                        .disabled(input_len == 0),
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
                            .child(ui::hint_text(
                                &format!("{} {}", t("输入长度:"), input_len),
                                cx,
                            )),
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
                                &format!("{} {}", t("输出长度:"), output_text.chars().count()),
                                cx,
                            ))
                            .child(
                                ui::primary_button("copy-result", t("复制结果"), {
                                    let output_for_copy = output_text.clone();
                                    move |_ev, window, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            output_for_copy.clone(),
                                        ));
                                        notify::toast_success(window, cx, t("已复制"));
                                    }
                                })
                                .disabled(!has_output || has_error),
                            ),
                    )
                    .child(ui::result_area(&self.output, 160.)),
            )
            // 内联错误横条（无错误时不渲染）
            .children(self.error.clone().map(|e| ui::error_banner(&e, cx)))
    }
}
