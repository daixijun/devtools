//! JSON 格式化工具页。
//!
//! 单编辑器（gpui-component Editor）：格式化/压缩结果**写回同一编辑器**。
//! EditorState（gpui_base）自身提供 `value()` / `set_value(text, window, cx)`，
//! 写回不会循环（set_value 不触发 Change 事件）。

use gpui::{prelude::*, px, App, Context, Entity, IntoElement, Render, Window};
use gpui_component::input::EditorState;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, Disableable as _, Sizable as _,
};

use crate::components::ui;
use devtools::i18n::t;
use devtools::text_tools::format;

/// 示例（最小化 JSON 字符串）。
const EXAMPLE_JSON: &str = r#"{"name":"张三","age":30,"city":"北京","hobbies":["读书","电影","旅游"],"address":{"street":"朝阳区某某路","zipcode":"100000"},"active":true}"#;

pub struct JsonFormatter {
    editor: Entity<EditorState>,
    error: Option<String>,
}

impl JsonFormatter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = ui::new_editor(window, cx);
        editor.update(cx, |state, cx| {
            state.set_placeholder(t("在此输入 JSON 内容..."), window, cx);
            // JSON 语法高亮（syntect 后端，颜色跟随主题）
            state.set_highlighter("json", cx);
        });
        Self {
            editor,
            error: None,
        }
    }

    /// 编辑器全文（每次渲染读取，量级通常不大，可直接物化）。
    fn input_text(&self, cx: &Context<Self>) -> String {
        self.editor.read(cx).value().to_string()
    }

    fn set_editor_value(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }

    /// 通用动作：对当前输入执行 core 变换，成功写回编辑器，失败显示红条。
    fn apply(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input_text(cx);
        match action {
            Action::Example => {
                self.set_editor_value(EXAMPLE_JSON, window, cx);
                self.error = None;
            }
            Action::Clear => {
                self.set_editor_value("", window, cx);
                self.error = None;
            }
            Action::Format | Action::Minify | Action::Unescape => {
                if input.trim().is_empty() {
                    return; // 空输入不动作
                }
                let result = match action {
                    Action::Format => format::json_pretty(&input)
                        .map_err(|e| format!("{}: {e}", t("JSON格式化失败"))),
                    Action::Minify => format::json_minify(&input)
                        .map_err(|e| format!("{}: {e}", t("JSON压缩失败"))),
                    _ => format::json_unescape(&input)
                        .map_err(|e| format!("{}: {e}", t("去除转义失败"))),
                };
                match result {
                    Ok(out) => {
                        self.set_editor_value(&out, window, cx);
                        self.error = None;
                    }
                    Err(msg) => {
                        self.error = Some(msg);
                    }
                }
            }
        }
        cx.notify();
    }

    fn toolbar_button(
        id: &'static str,
        label: &'static str,
        enabled: bool,
        action: Action,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .secondary()
            .small()
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _ev, window, cx| {
                this.apply(action, window, cx);
            }))
    }
}

#[derive(Clone, Copy)]
enum Action {
    Format,
    Minify,
    Example,
    Unescape,
    Clear,
}

impl Render for JsonFormatter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let text = self.input_text(cx);
        let has_input = !text.is_empty();

        ui::tool_page("jsonformatter")
            .child(
                // 工具条：格式化 / 压缩 | 示例 / 去除转义 / 清空 + 长度 + 复制
                ui::toolbar()
                    .child(Self::toolbar_button(
                        "jsonformatter-format",
                        t("格式化"),
                        has_input,
                        Action::Format,
                        cx,
                    ))
                    .child(Self::toolbar_button(
                        "jsonformatter-minify",
                        t("压缩"),
                        has_input,
                        Action::Minify,
                        cx,
                    ))
                    .child(Self::toolbar_button(
                        "jsonformatter-example",
                        t("示例"),
                        true,
                        Action::Example,
                        cx,
                    ))
                    .child(Self::toolbar_button(
                        "jsonformatter-unescape",
                        t("去除转义"),
                        has_input,
                        Action::Unescape,
                        cx,
                    ))
                    .child(Self::toolbar_button(
                        "jsonformatter-clear",
                        t("清空"),
                        has_input,
                        Action::Clear,
                        cx,
                    ))
                    .child(gpui::div().flex_1())
                    .child(
                        gpui::div()
                            .text_size(px(13.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!("{}: {}", t("长度"), text.chars().count())),
                    )
                    .child(ui::copy_result_button(
                        "jsonformatter-copy",
                        t("复制"),
                        text,
                    )),
            )
            // 错误红条在工具条下
            .when_some(self.error.clone(), |el, err| {
                el.child(ui::error_banner(&err, cx))
            })
            // 单编辑器：格式化/压缩结果写回同一编辑器（Editor 自带代码等宽字体）
            .child(ui::code_edit_area(&self.editor, 560.))
    }
}
