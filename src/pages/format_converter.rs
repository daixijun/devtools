//! 格式转换器工具页。
//!
//! JSON / YAML / TOML 双向转换：两个 select + 左右两个编辑器（grid 2 列），
//! 输入或格式任一变化**立即自动转换（无防抖）**，结果写入右侧只读编辑器。
//!
//! Editor API 说明：EditorState（gpui_base）提供 `value()` 与
//! `set_value(text, window, cx)`，可直接读写，无需降级为 Textarea。

use gpui::{prelude::*, App, Context, Entity, IntoElement, Render, Window};
use gpui_component::input::{EditorState, InputEvent};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    Disableable as _, Sizable as _,
};

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::{notify, ui};
use devtools::text_tools::format::{self, Format};

/// 示例（美化 JSON）。
const EXAMPLE_JSON: &str = r#"{
  "name": "开发者工具箱",
  "version": "1.0.0",
  "description": "一个功能强大的开发者工具集合",
  "features": [
    "JSON格式化",
    "Base64编码/解码",
    "JWT解析",
    "时间戳转换"
  ],
  "config": {
    "theme": "dark",
    "language": "zh-CN",
    "autoSave": true
  }
}"#;

/// 示例（YAML，与 EXAMPLE_JSON 同一份数据）。
const EXAMPLE_YAML: &str = r#"name: 开发者工具箱
version: 1.0.0
description: 一个功能强大的开发者工具集合
features:
  - JSON格式化
  - Base64编码/解码
  - JWT解析
  - 时间戳转换
config:
  theme: dark
  language: zh-CN
  autoSave: true
"#;

/// 示例（TOML，与 EXAMPLE_JSON 同一份数据）。
const EXAMPLE_TOML: &str = r#"name = "开发者工具箱"
version = "1.0.0"
description = "一个功能强大的开发者工具集合"
features = ["JSON格式化", "Base64编码/解码", "JWT解析", "时间戳转换"]

[config]
theme = "dark"
language = "zh-CN"
autoSave = true
"#;

/// 编辑器高度（左右两栏，逻辑像素）。
const EDITOR_HEIGHT: f32 = 500.0;

const FORMAT_OPTIONS: &[(&str, &str)] = &[("json", "JSON"), ("yaml", "YAML"), ("toml", "TOML")];

pub struct FormatConverter {
    input: Entity<EditorState>,
    output: Entity<EditorState>,
    input_format: Format,
    output_format: Format,
    input_sel: Entity<SelectState<SimpleSelectDelegate>>,
    output_sel: Entity<SelectState<SimpleSelectDelegate>>,
    error: Option<String>,
}

impl FormatConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = ui::new_editor(window, cx);
        input.update(cx, |state, cx| {
            state.set_placeholder("粘贴 JSON、YAML 或 TOML 内容...", window, cx);
            // 初始即设高亮器（默认源格式 JSON）：否则首次加载示例/粘贴内容不高亮，
            // 只有切换过格式下拉才生效
            state.set_highlighter(Format::Json.as_str(), cx);
        });
        let output = ui::new_editor(window, cx);
        output.update(cx, |state, cx| {
            state.set_placeholder("转换结果将显示在这里...", window, cx);
            state.set_highlighter(Format::Json.as_str(), cx);
        });

        // 两个下拉：默认均为 JSON
        let input_sel = select_state(window, cx, FORMAT_OPTIONS, 0);
        let output_sel = select_state(window, cx, FORMAT_OPTIONS, 0);

        // 输入变化 → 立即自动转换（无防抖）。set_value 写输出时自身不再发 Change，无循环。
        cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::Change) {
                this.convert(window, cx);
            }
        })
        .detach();

        // 格式切换 → 更新对应编辑器高亮语言并立即自动转换
        cx.subscribe_in(
            &input_sel,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    if let Some(f) = Format::from_value(value) {
                        this.input_format = f;
                        this.input.update(cx, |state, cx| {
                            state.set_highlighter(f.as_str(), cx);
                        });
                        this.convert(window, cx);
                    }
                }
            },
        )
        .detach();
        cx.subscribe_in(
            &output_sel,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    if let Some(f) = Format::from_value(value) {
                        this.output_format = f;
                        this.output.update(cx, |state, cx| {
                            state.set_highlighter(f.as_str(), cx);
                        });
                        this.convert(window, cx);
                    }
                }
            },
        )
        .detach();

        Self {
            input,
            output,
            input_format: Format::Json,
            output_format: Format::Json,
            input_sel,
            output_sel,
            error: None,
        }
    }

    fn input_text(&self, cx: &Context<Self>) -> String {
        self.input.read(cx).value().to_string()
    }

    fn output_text(&self, cx: &Context<Self>) -> String {
        self.output.read(cx).value().to_string()
    }

    fn set_output(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.output
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }

    /// 立即自动转换（无防抖）：空输入清输出；格式相同报错；异常包 `转换失败: {msg}`。
    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input_text(cx);
        if input.trim().is_empty() {
            self.error = None;
            self.set_output("", window, cx);
            cx.notify();
            return;
        }
        if self.input_format == self.output_format {
            self.error = Some("输入和输出格式相同，无需转换".to_string());
            self.set_output("", window, cx);
            cx.notify();
            return;
        }
        match format::convert(&input, self.input_format, self.output_format) {
            Ok(result) => {
                self.error = None;
                self.set_output(&result, window, cx);
            }
            Err(msg) => {
                self.error = Some(msg);
                self.set_output("", window, cx);
            }
        }
        cx.notify();
    }

    fn clear_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.error = None;
        self.set_output("", window, cx);
        cx.notify();
    }

    /// 加载当前**源格式**对应的示例（YAML/TOML 源加载 JSON 示例会解析失败），
    /// 加载后立即自动转换。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let example = match self.input_format {
            Format::Json => EXAMPLE_JSON,
            Format::Yaml => EXAMPLE_YAML,
            Format::Toml => EXAMPLE_TOML,
        };
        self.input
            .update(cx, |state, cx| state.set_value(example, window, cx));
        self.convert(window, cx);
    }

    fn copy_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let output = self.output_text(cx);
        if output.is_empty() || self.error.is_some() {
            return;
        }
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(output));
        notify::toast_success(window, cx, "已复制");
    }
}

impl Render for FormatConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = self.input_text(cx);
        let has_input = !input.is_empty();
        let output = self.output_text(cx);
        let can_copy = !output.is_empty() && self.error.is_none();

        ui::tool_page("formatconverter")
            .child(
                // 工具条：输入格式 / 输出格式 + 示例 / 清空 / 复制结果
                ui::toolbar()
                    .child(ui::hfield(
                        "输入格式:",
                        Select::new(&self.input_sel).small(),
                        cx,
                    ))
                    .child(ui::hfield(
                        "输出格式:",
                        Select::new(&self.output_sel).small(),
                        cx,
                    ))
                    .child(gpui::div().flex_1())
                    .child(
                        Button::new("formatconverter-example")
                            .label("示例")
                            .secondary()
                            .small()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.load_example(window, cx);
                            })),
                    )
                    .child(
                        Button::new("formatconverter-clear")
                            .label("清空")
                            .secondary()
                            .small()
                            .disabled(!has_input)
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.clear_input(window, cx);
                            })),
                    )
                    .child(
                        Button::new("formatconverter-copy")
                            .label("复制结果")
                            .primary()
                            .small()
                            .disabled(!can_copy)
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.copy_output(window, cx);
                            })),
                    ),
            )
            .when_some(self.error.clone(), |el, err| {
                el.child(ui::error_banner(&err, cx))
            })
            .child(
                // 左输入右输出，grid 2 列（统一走组件层 code_edit/result_area）
                gpui::div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .child(ui::code_edit_area(&self.input, EDITOR_HEIGHT)),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .child(ui::code_result_area(&self.output, EDITOR_HEIGHT)),
                    ),
            )
    }
}
