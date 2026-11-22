//! Base64 编码。
//!
//! 独立编码工具：文本输入经 200ms 防抖自动编码为 Base64。

use std::time::Duration;

use devtools::i18n::t;
use gpui::{div, ClipboardItem, IntoElement, ParentElement as _, Render, Styled as _, Window};
use gpui_component::input::{InputEvent, TextareaState};
use gpui_component::Disableable as _;

use crate::components::{notify, ui};

/// 「示例」填入的文本。
const EXAMPLE_TEXT: &str = "Hello, World! 你好，世界！";
const DEBOUNCE: Duration = Duration::from_millis(200);

pub struct Base64Encode {
    input: gpui::Entity<TextareaState>,
    output: gpui::Entity<TextareaState>,
    /// 防抖版本号：每次输入变化自增，timer 到期后版本一致才执行转换。
    generation: u64,
}

impl Base64Encode {
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
            generation: 0,
        }
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
        .detach()
    }

    fn convert(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let encoded = if text.is_empty() {
            String::new()
        } else {
            devtools::text_tools::base64::encode(&text)
        };
        self.output.update(cx, |state, cx| {
            state.set_value(encoded.as_str(), window, cx)
        });
        cx.notify();
    }
}

impl Render for Base64Encode {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let input_len = self.input.read(cx).value().chars().count();
        let output_text = self.output.read(cx).value().to_string();
        let has_output = !output_text.is_empty();

        ui::tool_page("base64encode")
            // 操作按钮（无边框工具条）
            .child(
                ui::toolbar()
                    .justify_between()
                    .child(ui::secondary_button(
                        "load-example",
                        t("示例"),
                        cx.listener(|this, _ev, window, cx| {
                            this.input
                                .update(cx, |state, cx| state.set_value(EXAMPLE_TEXT, window, cx));
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
                                .disabled(!has_output),
                            ),
                    )
                    .child(ui::result_area(&self.output, 160.)),
            )
    }
}
