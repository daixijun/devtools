//! SQL 转 Go 结构体。
//!
//! 顶部配置条（复数表名 / 导出字段 / Go 1.24+ / JSON 处理 select / 标签 checkbox 组）
//! + 左右分栏：SQL 输入编辑器 / Go 结构体只读输出。500ms 防抖自动调 core 转换，
//!   输出侧仅显示第一个表的代码（对齐历史版本行为）。

use std::collections::HashMap;
use std::time::Duration;

use devtools::tools::sql_to_go::{convert_sql_to_go, GoStructOutput, SqlToGoOptions};
use futures::StreamExt as _;
use gpui::{
    div, prelude::*, px, App, Context, Entity, FontWeight, IntoElement, Render, SharedString,
    Window,
};
use gpui_component::{
    checkbox::Checkbox,
    input::{EditorState, InputEvent},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _,
};

use crate::components::{select::select_state, select::SimpleSelectDelegate, ui};

/// 500ms 输入防抖。
const DEBOUNCE: Duration = Duration::from_millis(500);

/// 标签 checkbox 组（顺序对齐历史版本 selectedTags 初始化）。
const TAGS: [&str; 8] = ["json", "gorm", "db", "sql", "yaml", "toml", "env", "ini"];

/// 「示例」DDL：users + posts 两表含外键（对齐历史版本示例）。
const EXAMPLE_SQL: &str = "CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    age INT,
    active BOOLEAN DEFAULT TRUE,
    profile TEXT
);

CREATE TABLE posts (
    id INT PRIMARY KEY AUTO_INCREMENT,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id INT,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);";

/// 历史版本页面初始 SQL（仅 users 单表）。
const DEFAULT_SQL: &str = "CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);";

pub struct SqlToGo {
    sql_input: Entity<EditorState>,
    go_output: Entity<EditorState>,
    json_select: Entity<SelectState<SimpleSelectDelegate>>,
    enable_pluralization: bool,
    exported_fields: bool,
    is_go_124_or_above: bool,
    json_null_handling: String,
    selected_tags: HashMap<String, bool>,
    /// 防抖代次：新一轮输入使旧定时器失效。
    pending: u64,
    error: Option<String>,
}

impl SqlToGo {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sql_input = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("sql")
                .placeholder("输入 CREATE TABLE SQL 语句…")
        });
        let go_output = cx.new(|cx| EditorState::new(window, cx).language("go"));
        let json_select = select_state(window, cx, &Self::json_options(true), 1);

        let mut this = Self {
            sql_input: sql_input.clone(),
            go_output,
            json_select,
            enable_pluralization: true,
            exported_fields: true,
            is_go_124_or_above: true,
            json_null_handling: "omitempty".to_string(),
            selected_tags: SqlToGoOptions::default().selected_tags,
            pending: 0,
            error: None,
        };

        this.observe_json_select(window, cx);

        // 对齐历史版本：页面初始自带 users 单表示例
        sql_input.update(cx, |state, cx| state.set_value(DEFAULT_SQL, window, cx));

        // SQL 输入变化 → 500ms 防抖自动转换
        cx.subscribe_in(
            &sql_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.schedule_convert(window, cx);
                }
            },
        )
        .detach();

        // 初始内容自动转换一次
        this.schedule_convert(window, cx);

        this
    }

    /// JSON 处理下拉选项；omitzero 仅 Go 1.24+ 出现（对齐历史版本 select 渲染）。
    fn json_options(go124: bool) -> Vec<(&'static str, &'static str)> {
        if go124 {
            vec![
                ("none", "无"),
                ("omitempty", "omitempty"),
                ("omitzero", "omitzero"),
            ]
        } else {
            vec![("none", "无"), ("omitempty", "omitempty")]
        }
    }

    /// 按当前 Go 1.24+ 状态重建 JSON 处理下拉；未勾选且当前=omitzero 回退 omitempty。
    fn sync_json_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_go_124_or_above && self.json_null_handling == "omitzero" {
            self.json_null_handling = "omitempty".to_string();
        }
        let options = Self::json_options(self.is_go_124_or_above);
        let selected = options
            .iter()
            .position(|(v, _)| *v == self.json_null_handling)
            .unwrap_or(1);
        self.json_select = select_state(window, cx, &options, selected);
        self.observe_json_select(window, cx);
    }

    /// 订阅 JSON 处理下拉确认事件。
    fn observe_json_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(
            &self.json_select.clone(),
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, window, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.json_null_handling = v.to_string();
                    this.schedule_convert(window, cx);
                }
            },
        )
        .detach();
    }

    /// 500ms 防抖：代次计数使旧定时器失效。
    fn schedule_convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pending += 1;
        let gen = self.pending;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.pending == gen {
                    this.convert(window, cx);
                }
            });
        })
        .detach();
    }

    /// 组装 core 选项。
    fn build_options(&self) -> SqlToGoOptions {
        SqlToGoOptions {
            enable_pluralization: self.enable_pluralization,
            exported_fields: self.exported_fields,
            is_go_124_or_above: self.is_go_124_or_above,
            json_null_handling: self.json_null_handling.clone(),
            selected_tags: self.selected_tags.clone(),
        }
    }

    /// 调 core 转换（经 tokio 桥，避免阻塞 UI）。
    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sql = self.sql_input.read(cx).value().to_string();
        if sql.trim().is_empty() {
            self.error = None;
            self.go_output
                .update(cx, |state, cx| state.set_value("", window, cx));
            cx.notify();
            return;
        }

        let options = self.build_options();
        let mut rx =
            crate::core_rt::submit(
                cx,
                async move { convert_sql_to_go(sql, Some(options)).await },
            );

        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ =
                    this.update_in(cx, |this, window, cx| this.on_converted(result, window, cx));
            }
        })
        .detach();
    }

    fn on_converted(
        &mut self,
        result: Result<GoStructOutput, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(output) => {
                self.error = None;
                // 仅显示第一个表的代码（对齐历史版本）
                let code = output
                    .table_names
                    .first()
                    .and_then(|name| output.outputs.get(name))
                    .cloned()
                    .unwrap_or_default();
                self.go_output
                    .update(cx, |state, cx| state.set_value(code, window, cx));
            }
            Err(message) => {
                // 解析失败：清空输出并显示红条
                self.error = Some(message);
                self.go_output
                    .update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        cx.notify();
    }

    /// 载入示例 DDL（users + posts 含外键）。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sql_input
            .update(cx, |state, cx| state.set_value(EXAMPLE_SQL, window, cx));
        cx.notify();
    }

    /// 清空输入与输出。
    fn clear_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sql_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.go_output
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.error = None;
        self.pending += 1; // 使未决的防抖转换失效
        cx.notify();
    }
}

impl Render for SqlToGo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // 配置区工具条：复数表名 / 导出字段 / Go 1.24+ / JSON 处理 select
        let config_row = ui::toolbar()
            .child(
                Checkbox::new("sqltogo-plural")
                    .label("复数表名")
                    .checked(self.enable_pluralization)
                    .on_click(cx.listener(|this, checked: &bool, window, cx| {
                        this.enable_pluralization = *checked;
                        this.schedule_convert(window, cx);
                    })),
            )
            .child(
                Checkbox::new("sqltogo-exported")
                    .label("导出字段")
                    .checked(self.exported_fields)
                    .on_click(cx.listener(|this, checked: &bool, window, cx| {
                        this.exported_fields = *checked;
                        this.schedule_convert(window, cx);
                    })),
            )
            .child(
                Checkbox::new("sqltogo-go124")
                    .label("Go 1.24+")
                    .checked(self.is_go_124_or_above)
                    .on_click(cx.listener(|this, checked: &bool, window, cx| {
                        this.is_go_124_or_above = *checked;
                        // omitzero 选项随 Go1.24+ 增减；必要时回退 omitempty
                        this.sync_json_select(window, cx);
                        this.schedule_convert(window, cx);
                    })),
            )
            .child(ui::hfield(
                "JSON 处理:",
                Select::new(&self.json_select).w(px(160.)),
                cx,
            ));

        // 配置区第二行：标签 checkbox 组（工具条 + 水平字段）
        let mut tag_checkboxes = div().flex().flex_wrap().items_center().gap_3();
        for tag in TAGS {
            let tag = tag.to_string();
            let checked = self.selected_tags.get(&tag).copied().unwrap_or(false);
            tag_checkboxes = tag_checkboxes.child(
                Checkbox::new(SharedString::from(format!("sqltogo-tag-{tag}")))
                    .label(tag.to_uppercase())
                    .checked(checked)
                    .on_click(cx.listener(move |this, checked: &bool, window, cx| {
                        this.selected_tags.insert(tag.clone(), *checked);
                        this.schedule_convert(window, cx);
                    })),
            );
        }
        let tags_row = ui::toolbar().child(ui::hfield("标签:", tag_checkboxes, cx));

        // 左右分栏（共享行结构，与 json_to_go 一致）：标题一行、编辑器一行，
        // 两侧编辑器起始 Y 天然对齐
        let editors = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                // 标题行：左列标题 + 示例/清空，右列标题
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w(px(0.))
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.colors.foreground)
                                    .child("SQL 输入"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(ui::secondary_button(
                                        "sqltogo-example",
                                        "示例",
                                        cx.listener(|this, _, window, cx| {
                                            this.load_example(window, cx)
                                        }),
                                    ))
                                    .child(ui::secondary_button(
                                        "sqltogo-clear",
                                        "清空",
                                        cx.listener(|this, _, window, cx| {
                                            this.clear_all(window, cx)
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div().flex().flex_1().min_w(px(0.)).items_center().child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.colors.foreground)
                                .child("Go 结构体输出"),
                        ),
                    ),
            )
            .child(
                // 编辑器行：两侧同一行（编辑器不套卡片）
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(ui::code_edit_area(&self.sql_input, 480.)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(ui::code_result_area(&self.go_output, 480.)),
                    ),
            );

        ui::tool_page("sqltogo")
            .child(config_row)
            .child(tags_row)
            .child(editors)
            .when_some(self.error.clone(), |el, err| {
                el.child(ui::error_banner(&err, cx))
            })
    }
}
