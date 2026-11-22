//! 主窗口：左侧分类导航 + 右侧工具内容区。
//!
//! 侧栏布局（自上而下）：
//! - 搜索框（过滤工具）
//! - 按分类分组的工具列表（分类行带图标，搜索时显示扁平结果）
//! - 底部固定操作区：主题切换图标（循环浅色/深色/跟随系统）+ 设置图标（弹窗）
//!
//! 「设置」不再作为侧栏工具项出现（registry 保留该分类供托盘/兜底分发），
//! 入口统一为底部齿轮图标弹出的设置弹窗。
//! 关闭按钮按 close_to_tray 配置隐藏到托盘；失焦不自动隐藏。

use crate::{components::ui, pages::settings::Settings, tool_window::ToolWindow, windows};
use devtools::config::ThemeMode;
use devtools::registry::{self, ToolDefinition};
use gpui::{
    div, prelude::*, px, App, Context, Div, Entity, FocusHandle, Focusable, FontWeight,
    IntoElement, Render, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::DialogFooter,
    input::{InputEvent, InputState},
    ActiveTheme as _, Icon, Sizable as _, WindowExt as _,
};
// 完整 Lucide 图标目录在 gpui-kit-assets（gpui_component::IconName 仅含组件子集）；
// 其 IconName 实现 IconNamed，可直接用于 Icon::new / Button::icon
use gpui_kit_assets::IconName;

/// 侧栏宽度（逻辑像素）
const SIDEBAR_WIDTH: f32 = 232.;
/// 导航行高
const NAV_ROW_HEIGHT: f32 = 34.;
/// 设置弹窗宽度
const SETTINGS_DIALOG_WIDTH: f32 = 560.;

pub struct MainWindow {
    input: Entity<InputState>,
    input_focus: FocusHandle,
    query: String,
    /// 搜索命中（query 非空时显示扁平列表；设置走侧栏底部入口，不在结果中出现）
    results: Vec<&'static ToolDefinition>,
    /// 当前展示的工具（None = 欢迎页）
    selected_tool: Option<&'static ToolDefinition>,
    /// 待切换工具：open_tool 等无 window 上下文的入口只记录，
    /// 由 render（持有 window）创建工具页视图
    pending_tool: Option<&'static ToolDefinition>,
    /// 右侧内容区（选中工具后创建）
    content: Option<Entity<ToolWindow>>,
    /// 设置弹窗视图（首次打开创建并保持，主题/开关状态跨打开保留）
    settings_view: Option<Entity<Settings>>,
}

impl MainWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(SharedString::from("搜索工具…")));
        let input_focus = input.focus_handle(cx);

        cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, _window, cx| match event {
                InputEvent::Change => {
                    let query = this.input.read(cx).value().to_string();
                    this.query = query.clone();
                    this.results = if query.is_empty() {
                        Vec::new()
                    } else {
                        registry::search_tools(&query)
                            .into_iter()
                            .filter(|tool| tool.category != "settings")
                            .collect()
                    };
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    // 搜索态 Enter：打开第一个命中工具（列表态 Enter 不抢行为）
                    if !this.query.is_empty() {
                        if let Some(tool) = this.results.first().copied() {
                            this.select_tool(tool, cx);
                        }
                    }
                }
                _ => {}
            },
        )
        .detach();

        Self {
            input,
            input_focus,
            query: String::new(),
            results: Vec::new(),
            selected_tool: None,
            pending_tool: None,
            content: None,
            settings_view: None,
        }
    }

    /// 切换右侧内容区到指定工具（复用同一窗口，不再每工具开新窗）。
    ///
    /// 仅记录待切换工具并触发重渲染；工具页视图由 render 在持有 window
    /// 的上下文中创建（可在任意 App 上下文安全调用）。
    pub fn select_tool(&mut self, tool: &'static ToolDefinition, cx: &mut Context<Self>) {
        self.pending_tool = Some(tool);
        cx.notify();
    }

    /// 搜索框聚焦句柄（windows.rs 在持有 window 的上下文中直接 focus）。
    pub fn input_focus_handle(&self) -> &FocusHandle {
        &self.input_focus
    }

    /// 分类图标（壳层视觉映射；registry 分类 id 为权威来源）。
    fn category_icon(id: &str) -> IconName {
        match id {
            "encoding" => IconName::CodeXml,
            "certificate" => IconName::ShieldCheck,
            "network" => IconName::Globe,
            "dataformat" => IconName::Braces,
            "media" => IconName::Clapperboard,
            "developer" => IconName::SquareTerminal,
            "time" => IconName::Clock,
            _ => IconName::Settings,
        }
    }

    /// 当前主题模式对应的底部图标。
    fn theme_icon(mode: ThemeMode) -> IconName {
        match mode {
            ThemeMode::Light => IconName::Sun,
            ThemeMode::Dark => IconName::Moon,
            ThemeMode::System => IconName::Monitor,
        }
    }

    /// 侧栏一行工具导航。
    fn nav_row(
        &self,
        id: &str,
        label: &str,
        note: Option<&str>,
        selected: bool,
        on_click: impl Fn(&mut Window, &mut App) + Clone + 'static,
        cx: &App,
    ) -> gpui::Stateful<Div> {
        let theme = cx.theme().clone();
        let (row_bg, text_color) = if selected {
            (
                theme.colors.sidebar_accent,
                theme.colors.sidebar_accent_foreground,
            )
        } else {
            (gpui::transparent_black(), theme.colors.foreground)
        };
        let note_color = if selected {
            theme.colors.sidebar_accent_foreground.opacity(0.8)
        } else {
            theme.colors.muted_foreground
        };
        let id = SharedString::from(id.to_string());
        let label = label.to_string();
        let note = note.map(str::to_string);
        div()
            .id(id)
            .h(px(NAV_ROW_HEIGHT))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .rounded(theme.radius)
            .bg(row_bg)
            .cursor_pointer()
            // hover 取 sidebar_accent 半透明（accent 在 light 下是近白灰，不可见）
            .hover(|style| style.bg(theme.colors.sidebar_accent.opacity(0.55)))
            .on_click(move |_, window, _cx| {
                // click 处于窗口事件 dispatch 中，此时同窗口 handle.update 会被
                // gpui 以重入为由拒绝（open_tool 内已与窗口操作解耦）：
                // 推迟到下一帧（dispatch 结束后）再打开工具
                let on_click = on_click.clone();
                window.on_next_frame(move |window, cx| on_click(window, cx))
            })
            .child(
                // 选中态只留底色 + 字重两种表达（原竖条指示冗余，已移除）
                div().flex().items_center().min_w_0().child(
                    div()
                        .text_size(px(13.))
                        .font_weight(if selected {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(text_color)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(label),
                ),
            )
            .children(note.map(|note| div().text_size(px(11.)).text_color(note_color).child(note)))
    }

    /// 分类小标题（图标 + 名称；左缩进与工具行文本对齐）。
    fn category_header(&self, category: &registry::ToolCategory, cx: &App) -> Div {
        let theme = cx.theme().clone();
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .pl(px(11.))
            .pr_2()
            .pt_3()
            .pb_1()
            .text_size(px(11.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.colors.muted_foreground)
            .child(
                Icon::new(Self::category_icon(category.id))
                    .xsmall()
                    .text_color(theme.colors.muted_foreground),
            )
            .child(category.name)
    }

    /// 侧栏工具列表（搜索态扁平结果 / 默认按分类分组）。
    fn sidebar_list(&self, cx: &App) -> Div {
        let theme = cx.theme().clone();
        let mut list = div().flex().flex_col().px_2().gap_0p5();

        if self.query.is_empty() {
            for category in registry::CATEGORIES {
                // 设置入口在侧栏底部（齿轮图标 → 弹窗），不再列为工具项
                if category.id == "settings" {
                    continue;
                }
                let tools: Vec<_> = registry::tools_in_category(category.id).collect();
                if tools.is_empty() {
                    continue;
                }
                list = list.child(self.category_header(category, cx));
                for tool in tools {
                    let tool_id = tool.id;
                    let is_selected = self
                        .selected_tool
                        .is_some_and(|selected| selected.id == tool_id);
                    list = list.child(self.nav_row(
                        &format!("nav-{}", tool.id),
                        tool.name,
                        None,
                        is_selected,
                        move |_, cx| windows::open_tool(cx, tool_id),
                        cx,
                    ));
                }
            }
        } else {
            for tool in &self.results {
                let tool_id = tool.id;
                let category = registry::category_name(tool.category);
                let is_selected = self
                    .selected_tool
                    .is_some_and(|selected| selected.id == tool_id);
                list = list.child(self.nav_row(
                    &format!("nav-search-{}", tool.id),
                    tool.name,
                    Some(category),
                    is_selected,
                    move |_, cx| windows::open_tool(cx, tool_id),
                    cx,
                ));
            }
            if self.results.is_empty() {
                list = list.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.colors.muted_foreground)
                        .child("没有匹配的工具"),
                );
            }
        }
        list
    }

    /// 侧栏底部固定操作区：主题切换 + 设置弹窗入口 + 工具数提示。
    fn sidebar_footer(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let theme_mode = crate::config_state::get(cx).theme;

        let theme_button = ui::icon_button("sidebar-theme", Self::theme_icon(theme_mode))
            .ghost()
            .small()
            .on_click(cx.listener(move |this, _, window, cx| {
                // 循环切换：浅色 → 深色 → 跟随系统（图标随当前模式变化）
                let next = match crate::config_state::get(cx).theme {
                    ThemeMode::Light => ThemeMode::Dark,
                    ThemeMode::Dark => ThemeMode::System,
                    ThemeMode::System => ThemeMode::Light,
                };
                crate::theme::set_mode(next, window, cx);
                // 设置弹窗打开期间同步其主题镜像
                if let Some(settings) = &this.settings_view {
                    settings.update(cx, |settings, cx| settings.sync_theme(next, cx));
                }
                cx.notify();
            }));

        let settings_button = ui::icon_button("sidebar-settings", IconName::Settings)
            .ghost()
            .small()
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_settings_dialog(window, cx);
            }));

        div()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1p5()
            .child(theme_button)
            .child(settings_button)
            .child(
                div()
                    .flex_1()
                    .text_size(px(11.))
                    .text_color(theme.colors.muted_foreground)
                    .text_right()
                    .pr_1()
                    .child(format!("{} 个工具", registry::TOOLS.len())),
            )
    }

    /// 打开设置弹窗（侧栏底部齿轮与欢迎页设置卡共用）。
    ///
    /// 实体必须在打开前创建（dialog 层重建时 builder 会重复执行）。
    fn open_settings_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self
            .settings_view
            .get_or_insert_with(|| Settings::create_embedded(cx))
            .clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("设置")
                .w(px(SETTINGS_DIALOG_WIDTH))
                .content({
                    let view = view.clone();
                    move |content, _, _| content.child(view.clone())
                })
                .footer(
                    DialogFooter::new().child(
                        Button::new("settings-done")
                            .label("完成")
                            .primary()
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
        });
        cx.notify();
    }

    /// 欢迎页快捷工具（id 对应 registry；高频入口）。
    const QUICK_TOOL_IDS: &[&str] = &[
        "base64",
        "jsonformatter",
        "timestamp",
        "url",
        "md5crypto",
        "regextester",
    ];

    /// 右侧欢迎页（未选择工具时）：标题 + 分类卡片 + 常用工具 + 快捷键提示。
    fn welcome_page(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let theme = cx.theme().clone();
        let category_count = registry::CATEGORIES
            .iter()
            .filter(|c| c.id != "settings")
            .count();

        // 分类卡片网格（4×2：7 个分类 + 设置入口；点击卡片打开该分类第一个工具）
        let mut category_grid = div().grid().grid_cols(4).gap_2().w_full();
        for category in registry::CATEGORIES {
            if category.id == "settings" {
                continue;
            }
            let count = registry::tools_in_category(category.id).count();
            let first_tool = registry::tools_in_category(category.id).next();
            let icon = Self::category_icon(category.id);
            let card_id = SharedString::from(format!("home-cat-{}", category.id));
            category_grid = category_grid.child(
                div()
                    .id(card_id)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(theme.radius_lg)
                    .bg(theme.colors.popover)
                    .cursor_pointer()
                    // hover 用 secondary（比卡片深一档、比页面更深）——muted 在
                    // light 下与页面背景同为 neutral-100，hover 等于「消失一块」
                    .hover(|style| style.bg(theme.colors.secondary))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        if let Some(tool) = first_tool {
                            this.select_tool(tool, cx);
                        }
                    }))
                    .child(
                        Icon::new(icon)
                            .small()
                            .text_color(theme.colors.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.foreground)
                            .child(category.name),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!("{} 个工具", count)),
                    ),
            );
        }
        // 设置入口卡（与分类卡同款样式，打开设置弹窗）
        category_grid = category_grid.child(
            div()
                .id("home-cat-settings")
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded(theme.radius_lg)
                .bg(theme.colors.popover)
                .cursor_pointer()
                .hover(|style| style.bg(theme.colors.secondary))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_settings_dialog(window, cx);
                }))
                .child(
                    Icon::new(IconName::Settings)
                        .small()
                        .text_color(theme.colors.muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child("偏好设置"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child("主题 · 托盘 · 自启"),
                ),
        );

        // 常用工具快捷入口（点击直接切换右侧内容区）：
        // 胶囊 chip（popover 底 + hover 抬升），ghost Button 无 hover 反馈
        let mut quick_row = div().flex().flex_wrap().gap_2();
        for &tool_id in Self::QUICK_TOOL_IDS {
            if let Some(tool) = registry::get_tool_by_id(tool_id) {
                quick_row = quick_row.child(
                    div()
                        .id(SharedString::from(format!("home-quick-{tool_id}")))
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(theme.colors.popover)
                        .text_size(px(13.))
                        .text_color(theme.colors.foreground)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.colors.secondary))
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.select_tool(tool, cx);
                        }))
                        .child(tool.name),
                );
            }
        }

        div()
            .id("home-scroll")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .overflow_y_scroll()
            .p_8()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .max_w(px(760.))
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.foreground)
                                    .child("DevTools 开发者工具箱"),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(format!(
                                        "{} 个分类 · {} 个工具 · 纯 Rust 原生渲染",
                                        category_count,
                                        registry::TOOLS.len()
                                    )),
                            ),
                    )
                    .child(category_grid)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .w_full()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.colors.muted_foreground)
                                    .child("常用工具"),
                            )
                            .child(quick_row),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child("输入即搜索 · ↑↓ 选择 · Enter 打开 · 关闭窗口可隐藏到托盘"),
                    ),
            )
    }
}

impl Focusable for MainWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.input_focus.clone()
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // 消费待切换工具：创建工具页视图（需要 window，放这里统一处理）
        if let Some(tool) = self.pending_tool.take() {
            self.selected_tool = Some(tool);
            self.content = Some(cx.new(|cx| ToolWindow::new(tool, window, cx)));
        }

        // 左侧导航栏（搜索框 / 工具列表 / 底部操作区）
        // 无右边框：sidebar 底色与内容区底色（background）自然分层
        let sidebar = div()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.colors.sidebar)
            .child(div().px_3().py_3().pb_2().child(self.input.clone()))
            .child(
                div()
                    .id("sidebar-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb_2()
                    .child(self.sidebar_list(cx)),
            )
            .child(self.sidebar_footer(cx));

        // 右侧内容区
        let content = match &self.content {
            Some(tool_view) => div()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_hidden()
                .child(tool_view.clone().into_any_element()),
            None => div()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(self.welcome_page(cx)),
        };

        div()
            .flex()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(sidebar)
            .child(content)
    }
}
