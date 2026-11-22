//! 工具页面布局与表单辅助。
//!
//! 视觉规范（macOS 分组视图语言）：页面底色与卡片靠背景色深浅分层
//! （light：灰页白卡；dark：近黑页 + 抬升卡），**边框只留给输入控件与
//! 按钮**——卡片、面板、代码块、横条、徽章一律无边框；分隔线只出现在
//! 列表行与行之间（末行无线）。排版五档：11（注记）/12（正文小）/
//! 13（正文）/14（小标题）/17（大数值），圆角统一走 `theme.radius`/
//! `radius_lg`，等宽统一 Menlo。

use gpui::{
    div, px, App, AppContext as _, Div, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
};
use gpui_base::SelectableText;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    input::{Editor, EditorState, InputState, Textarea, TextareaState},
    spinner::Spinner,
    tab::{Tab, TabBar},
    ActiveTheme as _, IconNamed, Sizable as _,
};

// ---------------------------------------------------------------------------
// 输入状态构造（单行 / 多行 / 代码编辑器）
// ---------------------------------------------------------------------------

/// 单行输入。
pub fn new_input(window: &mut gpui::Window, cx: &mut App) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx))
}

/// 多行文本域。
pub fn new_textarea(window: &mut gpui::Window, cx: &mut App) -> Entity<TextareaState> {
    cx.new(|cx| TextareaState::new(window, cx))
}

/// 代码编辑器状态（gpui-component Editor，syntect 语法高亮）。
pub fn new_editor(
    window: &mut gpui::Window,
    cx: &mut App,
) -> Entity<gpui_component::input::EditorState> {
    cx.new(|cx| gpui_component::input::EditorState::new(window, cx))
}

// ---------------------------------------------------------------------------
// 页面骨架
// ---------------------------------------------------------------------------

/// 工具页面根容器：纵向布局 + 页面留白。
///
/// 高度为内容自然高（不设 size_full）：页面滚动由 ToolWindow 的内容区宿主
/// 统一承担（overflow_y_scroll + items_start）。若在此设满视口高度，flex 子项
/// 默认 flex_shrink=1 会被压缩而不是溢出，滚动容器将失去可滚动量（页面滚不动）。
///
/// 工具名/描述由 ToolWindow 页头展示，页面内不再重复标题（原 themed_header）。
pub fn tool_page(id: &'static str) -> gpui::Stateful<Div> {
    div().id(id).flex().flex_col().w_full().p_4().gap_4()
}

/// 卡片容器（macOS 分组卡：无边框，靠 popover 底色与页面分层）。
pub fn card(cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded(theme.radius_lg)
        .bg(theme.colors.popover)
}

/// 卡片标题行（标题 + 可选右侧操作区）。
pub fn card_header(title: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div().flex().items_center().justify_between().child(
        div()
            .text_size(px(14.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.colors.foreground)
            .child(title.to_string()),
    )
}

/// 表单字段：label 在上、控件在下。
pub fn field(label: &str, control: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child(label.to_string()),
        )
        .child(control)
}

/// 水平字段：label 在左、控件在右（紧凑行）。
pub fn hfield(label: &str, control: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.colors.foreground)
                .child(label.to_string()),
        )
        .child(control)
}

// ---------------------------------------------------------------------------
// 工具条 / 子面板 / 统计块 / 文件行
// ---------------------------------------------------------------------------

/// 无边框工具条行（横向控件行，可换行）。
///
/// 统一原先「有的带框有的不带」的页内工具条：筛选、格式切换、操作按钮行等。
pub fn toolbar() -> Div {
    div().flex().flex_wrap().items_center().gap_2()
}

/// 卡内无框子面板（muted 底）。
///
/// 承接所有「卡内再套边框」的子块：过滤条、示例块、漏洞卡、密码展示框、
/// 路径框等。仅在卡片（popover 底）内部使用——muted 底与页面同层，直接
/// 放在页面上会失去边界感。
pub fn panel(cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded(theme.radius)
        .bg(theme.colors.muted)
}

/// 统计块：大数值（17px semibold）+ 小标签（11px muted），无边框。
pub fn stat_tile(value: &str, label: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            div()
                .text_size(px(17.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.colors.foreground)
                .child(value.to_string()),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.colors.muted_foreground)
                .child(label.to_string()),
        )
}

/// 只读路径行：等宽小字路径（muted 底、无边框），占满剩余宽度。
///
/// 操作按钮（浏览/清空等）由调用方链式 `.child()` 追加在右侧。
pub fn file_row(path: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div().flex().items_center().gap_2().child(
        div()
            .flex_1()
            .min_w(px(0.))
            .px_3()
            .py_1p5()
            .rounded(theme.radius)
            .bg(theme.colors.muted)
            .font_family("Menlo")
            .text_size(px(12.))
            .text_color(theme.colors.foreground)
            .overflow_hidden()
            .truncate()
            .child(path.to_string()),
    )
}

// ---------------------------------------------------------------------------
// 横幅（错误 / 成功 / 警告 / 信息）
// ---------------------------------------------------------------------------

fn banner(text: &str, bg: gpui::Hsla, fg: gpui::Hsla, radius: gpui::Pixels) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(radius)
        .bg(bg)
        .text_size(px(13.))
        .text_color(fg)
        .child(text.to_string())
}

/// 错误横条（内联，无边框）。
pub fn error_banner(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    banner(
        text,
        theme.colors.danger.opacity(0.1),
        theme.colors.danger,
        theme.radius,
    )
}

/// 成功横条。
pub fn success_banner(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    banner(
        text,
        theme.colors.success.opacity(0.1),
        theme.colors.success,
        theme.radius,
    )
}

/// 警告横条。
pub fn warning_banner(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    banner(
        text,
        theme.colors.warning.opacity(0.1),
        theme.colors.warning,
        theme.radius,
    )
}

/// 信息横条。
pub fn info_banner(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    banner(
        text,
        theme.colors.info.opacity(0.1),
        theme.colors.info,
        theme.radius,
    )
}

/// 富内容横条：与四色横条同视觉（无框浅底 + 同色文字），但返回容器允许
/// 调用方链式 `.child()` 追加操作按钮、明细行等富内容。
pub fn rich_banner(text: &str, color: gpui::Hsla, cx: &App) -> Div {
    banner(text, color.opacity(0.1), color, cx.theme().radius)
}

// ---------------------------------------------------------------------------
// 徽章 / 键值行 / 代码块 / 表格 / 加载态
// ---------------------------------------------------------------------------

/// 小徽章（状态标签）：浅色底 + 同色文字，胶囊形。
pub fn badge(text: &str, color: gpui::Hsla) -> Div {
    div()
        .px_2()
        .py_0p5()
        .rounded_full()
        .bg(color.opacity(0.12))
        .text_size(px(11.))
        .text_color(color)
        .child(text.to_string())
}

/// 键值行（证书/SSL 详情的 label: value 展示）。
///
/// 行本身无边框；行间 hairline 分隔（末行无线）由 [`info_rows`] 容器处理。
pub fn info_row(label: &str, value: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_start()
        .gap_3()
        .py_1()
        .child(
            div()
                .min_w(px(120.))
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(13.))
                .font_family("Menlo")
                .overflow_hidden()
                .child(value.into()),
        )
}

/// 键值行分组容器：行与行之间渲染 hairline 分隔线（末行无线），macOS 分组列表。
pub fn info_rows(rows: impl IntoIterator<Item = Div>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    let separator = theme.colors.border.opacity(0.5);
    let rows: Vec<Div> = rows.into_iter().collect();
    let last = rows.len().saturating_sub(1);
    let mut container = div().flex().flex_col();
    for (i, row) in rows.into_iter().enumerate() {
        if i < last {
            container = container.child(row.border_b_1().border_color(separator));
        } else {
            container = container.child(row);
        }
    }
    container
}

// -- 轻量表格（手工 div 表格的统一样式；不引入重型 Table 组件） --

/// 表头行容器：单元格由调用方链式 `.child()` 填入（列宽自行控制）。
pub fn table_head(cx: &App) -> Div {
    let _ = cx;
    div().flex().items_center()
}

/// 表头单元格（11px muted 小字）。
pub fn table_head_cell(text: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .text_size(px(11.))
        .text_color(theme.colors.muted_foreground)
        .child(text.to_string())
}

/// 表格数据行：水平 flex 行（列宽/单元格内容由调用方控制）。
pub fn table_row() -> Div {
    div().flex().items_center().py_1p5()
}

/// 表格数据行容器：行间 hairline 分隔（末行无线）。
pub fn table_rows(rows: impl IntoIterator<Item = Div>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    let separator = theme.colors.border.opacity(0.5);
    let rows: Vec<Div> = rows.into_iter().collect();
    let last = rows.len().saturating_sub(1);
    let mut container = div().flex().flex_col();
    for (i, row) in rows.into_iter().enumerate() {
        if i < last {
            container = container.child(row.border_b_1().border_color(separator));
        } else {
            container = container.child(row);
        }
    }
    container
}

/// 等宽代码/结果块（文本可拖选复制）。`id` 须在页面内唯一（GPUI element id 去重）。
///
/// 文本用 `SelectableText`（gpui-base）：纯 div 文本无法被鼠标拖选，
/// 选中高亮取自主题 `selection` 色。块本身无边框，muted 实色底。
pub fn mono_block(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    cx: &App,
) -> gpui::Stateful<Div> {
    let theme = cx.theme().clone();
    let block_id = id.into();
    let text_id = gpui::ElementId::from(SharedString::from(format!("{block_id}-text")));
    let text_style = gpui::TextStyleRefinement {
        font_family: Some("Menlo".into()),
        font_size: Some(px(12.).into()),
        color: Some(theme.colors.foreground),
        ..Default::default()
    };
    div()
        .id(block_id)
        .p_3()
        .rounded(theme.radius)
        .bg(theme.colors.muted)
        .overflow_y_scroll()
        .overflow_x_scroll()
        .child(
            SelectableText::new(text_id, text)
                .text_style(text_style)
                .selection_color(theme.colors.selection),
        )
}

/// 加载行（spinner + 文案）。
pub fn loading_row(text: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(13.))
        .text_color(theme.colors.muted_foreground)
        .child(Spinner::new().small())
        .child(text.to_string())
}

/// 一键复制按钮（「复制/已复制 ✓」两态）。
pub fn copy_button(id: &'static str, value: impl Into<SharedString>) -> Clipboard {
    Clipboard::new(id).value(value).label("复制")
}

// ---------------------------------------------------------------------------
// 按钮
// ---------------------------------------------------------------------------

/// 主操作按钮。
pub fn primary_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id).label(label).primary().on_click(on_click)
}

/// 次级按钮。
pub fn secondary_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id).label(label).secondary().on_click(on_click)
}

/// 危险按钮（清空/删除等）。
pub fn danger_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id).label(label).danger().on_click(on_click)
}

/// 幽灵按钮（轻量操作）。
pub fn ghost_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id).label(label).ghost().on_click(on_click)
}

// ---------------------------------------------------------------------------
// 页内 Tab（macOS 分段控件）
// ---------------------------------------------------------------------------

/// 页内 Tab 栏：gpui-component `TabBar` 的 segmented 变体（macOS 分段控件）。
///
/// `selected` 为当前选中下标；`on_select` 收到被点击的下标（TabBar 层统一
/// 接管点击，无需给每个 Tab 单独挂 on_click）。
pub fn tab_bar(
    id: &'static str,
    items: &[&str],
    selected: usize,
    on_select: impl Fn(&usize, &mut gpui::Window, &mut App) + 'static,
) -> TabBar {
    TabBar::new(id)
        .segmented()
        .selected_index(selected)
        .children(items.iter().map(|label| Tab::new().label(*label)))
        .on_click(on_select)
}

// ---------------------------------------------------------------------------
// 结果区（只读 Textarea，带高度）
// ---------------------------------------------------------------------------

/// 只读多行结果区。`height` 为逻辑像素。
///
/// 用 `readonly` 而非 `disabled`：readonly 保持可聚焦、可选择、可复制，
/// 仅拒绝用户修改（disabled 会连选择/复制一起禁掉，结果无法摘录）。
pub fn result_area(state: &Entity<TextareaState>, height: f32) -> gpui_component::input::Textarea {
    Textarea::new(state).h(px(height)).readonly(true)
}

/// 可编辑多行输入区。
pub fn edit_area(state: &Entity<TextareaState>, height: f32) -> gpui_component::input::Textarea {
    Textarea::new(state).h(px(height))
}

/// 只读代码输出区（代码编辑器模式，带语法高亮；语言由 EditorState 的
/// `.language()` 指定，如 "go"/"json"）。
pub fn code_result_area(state: &Entity<EditorState>, height: f32) -> Editor {
    Editor::new(state).h(px(height)).readonly(true)
}

/// 可编辑代码输入区（代码编辑器模式，带语法高亮与行号）。
pub fn code_edit_area(state: &Entity<EditorState>, height: f32) -> Editor {
    Editor::new(state).h(px(height))
}

/// 占位元素（空态提示行）。
pub fn empty_hint(text: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_center()
        .justify_center()
        .py_6()
        .text_size(px(13.))
        .text_color(theme.colors.muted_foreground)
        .child(text.to_string())
}

/// 图标按钮（仅图标；侧栏底部操作区等轻量入口）。
///
/// `icon` 接受任意 `IconNamed` 实现（`gpui_component::IconName` 组件子集或
/// `gpui_kit_assets::IconName` 完整 Lucide 目录）。
pub fn icon_button(id: &'static str, icon: impl IconNamed) -> Button {
    Button::new(id).icon(icon)
}

/// 供页面状态栏显示的小字说明。
pub fn hint_text(text: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .text_size(px(12.))
        .text_color(theme.colors.muted_foreground)
        .child(text.to_string())
}
