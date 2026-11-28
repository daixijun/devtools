//! SQL 转 Go Ent ORM Schema。
//!
//! 配置区（复数表名 / 边缘关系 / Mixin / Hooks / Policy / UUID 主键 / 软删除 / 包名）
//! + 左右分栏：SQL 输入编辑器 / 按 table_names 生成的 Tab 页只读输出（`<表名>.go`）。
//!   500ms 防抖自动调 core 转换；出错显示红条（err 或「转换失败」）。

use std::collections::HashMap;
use std::time::Duration;

use devtools::i18n::t;
use devtools::tools::sql_to_ent::{convert_sql_to_ent, EntSchemaOutput, SqlToEntOptions};
use futures::StreamExt as _;
use gpui::{
    div, prelude::*, px, App, Context, Entity, FontWeight, IntoElement, Render, SharedString,
    Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{EditorState, Input, InputEvent, InputState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::ui;

/// 500ms 输入防抖。
const DEBOUNCE: Duration = Duration::from_millis(500);

/// 「示例」/ 页面初始 DDL：users + posts 两表含外键。
const EXAMPLE_SQL: &str = "CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    age INT,
    active BOOLEAN DEFAULT TRUE
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

pub struct SqlToEnt {
    sql_input: Entity<EditorState>,
    /// 当前激活 Tab 的 Schema 文本（只读展示）。
    ent_output: Entity<EditorState>,
    package_input: Entity<InputState>,
    enable_pluralization: bool,
    generate_edges: bool,
    generate_mixin: bool,
    generate_hooks: bool,
    generate_policy: bool,
    use_uuid_primary_key: bool,
    enable_soft_delete: bool,
    package_name: String,
    /// 转换结果的表名顺序（Tab 顺序）与内容。
    table_names: Vec<String>,
    outputs: HashMap<String, String>,
    active_tab: usize,
    /// 防抖代次：新一轮输入使旧定时器失效。
    pending: u64,
    error: Option<String>,
}

impl SqlToEnt {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sql_input = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("sql")
                .placeholder(t("输入 CREATE TABLE SQL 语句…"))
        });
        let ent_output = cx.new(|cx| EditorState::new(window, cx).language("go"));
        let package_input = cx.new(|cx| InputState::new(window, cx).placeholder("schema"));

        let mut this = Self {
            sql_input: sql_input.clone(),
            ent_output,
            package_input: package_input.clone(),
            enable_pluralization: true,
            generate_edges: true,
            generate_mixin: true,
            generate_hooks: false,
            generate_policy: false,
            use_uuid_primary_key: false,
            enable_soft_delete: false,
            package_name: "schema".to_string(),
            table_names: Vec::new(),
            outputs: HashMap::new(),
            active_tab: 0,
            pending: 0,
            error: None,
        };

        // 包名输入变化 → 更新状态并防抖重转换
        cx.subscribe_in(
            &package_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    let name = this.package_input.read(cx).value().trim().to_string();
                    this.package_name = if name.is_empty() {
                        "schema".to_string()
                    } else {
                        name
                    };
                    this.schedule_convert(window, cx);
                }
            },
        )
        .detach();

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

        // 页面初始自带 users + posts 示例并自动转换一次
        sql_input.update(cx, |state, cx| state.set_value(EXAMPLE_SQL, window, cx));
        this.schedule_convert(window, cx);

        this
    }

    /// 组装 core 选项。
    fn build_options(&self) -> SqlToEntOptions {
        SqlToEntOptions {
            generate_edges: self.generate_edges,
            generate_mixin: self.generate_mixin,
            generate_hooks: self.generate_hooks,
            generate_policy: self.generate_policy,
            use_uuid_primary_key: self.use_uuid_primary_key,
            enable_soft_delete: self.enable_soft_delete,
            enable_pluralization: self.enable_pluralization,
            package_name: self.package_name.clone(),
        }
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

    /// 调 core 转换（经 tokio 桥，避免阻塞 UI）。
    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sql = self.sql_input.read(cx).value().to_string();
        if sql.trim().is_empty() {
            self.error = None;
            self.table_names.clear();
            self.outputs.clear();
            self.active_tab = 0;
            self.ent_output
                .update(cx, |state, cx| state.set_value("", window, cx));
            cx.notify();
            return;
        }

        let options = self.build_options();
        let mut rx =
            crate::core_rt::submit(
                cx,
                async move { convert_sql_to_ent(sql, Some(options)).await },
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
        result: Result<EntSchemaOutput, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(output) => {
                self.error = None;
                self.table_names = output.table_names;
                self.outputs = output.outputs;
                self.active_tab = 0;
                self.sync_output(window, cx);
            }
            Err(err) => {
                // 出错：清空结果，红条显示 err（空则回退「转换失败」）
                self.error = Some(if err.trim().is_empty() {
                    t("转换失败").to_string()
                } else {
                    err
                });
                self.table_names.clear();
                self.outputs.clear();
                self.active_tab = 0;
                self.ent_output
                    .update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        cx.notify();
    }

    /// 把当前激活 Tab 的 Schema 写入只读输出区。
    ///
    /// 注：输出区按默认宽度渲染制表符，生成的 Schema 本身已含正确缩进，
    /// 此处不再额外调整缩进宽度。
    fn sync_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let code = self
            .table_names
            .get(self.active_tab)
            .and_then(|name| self.outputs.get(name))
            .cloned()
            .unwrap_or_default();
        self.ent_output
            .update(cx, |state, cx| state.set_value(code, window, cx));
    }

    /// 切换 Tab。
    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.table_names.len() && self.active_tab != index {
            self.active_tab = index;
            self.sync_output(window, cx);
            cx.notify();
        }
    }

    /// 载入示例 DDL（users + posts 含外键）。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sql_input
            .update(cx, |state, cx| state.set_value(EXAMPLE_SQL, window, cx));
        // set_value 不发 Change 事件，显式触发一次转换（与 json_to_go 等页面一致）
        self.schedule_convert(window, cx);
    }

    /// 清空输入、结果与错误。
    fn clear_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sql_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.ent_output
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.table_names.clear();
        self.outputs.clear();
        self.active_tab = 0;
        self.error = None;
        self.pending += 1; // 使未决的防抖转换失效
        cx.notify();
    }
}

impl Render for SqlToEnt {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // 配置区工具条：7 个 checkbox + 包名
        let mut config_row = ui::toolbar();
        let toggles = [
            ("ent-plural", "复数表名", self.enable_pluralization),
            ("ent-edges", "生成边缘关系", self.generate_edges),
            ("ent-mixin", "生成 Mixin", self.generate_mixin),
            ("ent-hooks", "生成 Hooks", self.generate_hooks),
            ("ent-policy", "生成 Policy", self.generate_policy),
            ("ent-uuid", "UUID 主键", self.use_uuid_primary_key),
            ("ent-soft", "软删除", self.enable_soft_delete),
        ];
        for (id, label, checked) in toggles {
            config_row =
                config_row.child(Checkbox::new(id).label(t(label)).checked(checked).on_click(
                    cx.listener(move |this, checked: &bool, window, cx| {
                        match id {
                            "ent-plural" => this.enable_pluralization = *checked,
                            "ent-edges" => this.generate_edges = *checked,
                            "ent-mixin" => this.generate_mixin = *checked,
                            "ent-hooks" => this.generate_hooks = *checked,
                            "ent-policy" => this.generate_policy = *checked,
                            "ent-uuid" => this.use_uuid_primary_key = *checked,
                            _ => this.enable_soft_delete = *checked,
                        }
                        this.schedule_convert(window, cx);
                    }),
                ));
        }
        let config_row = config_row.child(ui::hfield(
            t("包名:"),
            Input::new(&self.package_input).w(px(160.)),
            cx,
        ));

        // 输出侧 Tab 页按钮组（`<表名>.go`，自制按钮组切换）
        let mut tab_row = ui::toolbar();
        for (index, name) in self.table_names.iter().enumerate() {
            let active = index == self.active_tab;
            let label = format!("{name}.go");
            let id = SharedString::from(format!("ent-tab-{index}"));
            let button = Button::new(id)
                .label(label)
                .when(active, |btn| btn.primary().disabled(true))
                .when(!active, |btn| btn.ghost())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_tab(index, window, cx);
                }));
            tab_row = tab_row.child(button);
        }
        let has_tabs = !self.table_names.is_empty();

        // 左右分栏（共享行结构，与 sql_to_go / json_to_go 一致）：标题一行、
        // （有 Tab 时）Tab 行、编辑器一行，两侧编辑器起始 Y 天然对齐
        let mut editors = div().flex().flex_col().gap_1().child(
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
                                .child(t("SQL 输入")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(ui::secondary_button(
                                    "ent-example",
                                    t("示例"),
                                    cx.listener(|this, _, window, cx| {
                                        this.load_example(window, cx)
                                    }),
                                ))
                                .child(ui::secondary_button(
                                    "ent-clear",
                                    t("清空"),
                                    cx.listener(|this, _, window, cx| this.clear_all(window, cx)),
                                )),
                        ),
                )
                .child(
                    div().flex().flex_1().min_w(px(0.)).items_center().child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.foreground)
                            .child(t("Go Ent Schema 输出")),
                    ),
                ),
        );
        if has_tabs {
            // Tab 行：仅右列上方（左列以空占位对齐）
            editors = editors.child(
                div()
                    .flex()
                    .gap_3()
                    .child(div().flex_1().min_w(px(0.)))
                    .child(div().flex_1().min_w(px(0.)).child(tab_row)),
            );
        }
        let editors = editors.child(
            // 编辑器行：两侧同一行（编辑器不套卡片）
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .child(ui::code_edit_area(&self.sql_input, 440.)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .child(ui::code_result_area(&self.ent_output, 400.)),
                ),
        );

        ui::tool_page("sqltoent")
            .child(config_row)
            .child(editors)
            .when_some(self.error.clone(), |el, err| {
                el.child(ui::error_banner(&err, cx))
            })
    }
}
