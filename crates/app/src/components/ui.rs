//! 工具页面布局与表单辅助。
//!
//! 视觉规范：跟随 gpui-component 主题（slate 体系），卡片=边框圆角面板、
//! label=muted 小字、错误=danger 底色横条、代码输出=等宽字体块。

use gpui::{
    div, px, App, AppContext as _, Div, Entity, InteractiveElement as _, StatefulInteractiveElement as _,
    IntoElement, ParentElement as _, SharedString, Styled as _,
};
use gpui_base::SelectableText;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    input::{Editor, EditorState, InputState, Textarea, TextareaState},
    spinner::Spinner,
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
pub fn new_editor(window: &mut gpui::Window, cx: &mut App) -> Entity<gpui_component::input::EditorState> {
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
pub fn tool_page(id: &'static str) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .w_full()
        .p_4()
        .gap_3()
}

/// 页头：标题 + 描述（需在 Render 中用 cx.theme()）。
pub fn themed_header(title: &str, subtitle: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.colors.foreground)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.colors.muted_foreground)
                .child(subtitle.to_string()),
        )
}

/// 卡片容器（边框 + 圆角 + 内边距 + 标题）。
pub fn card(cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded(theme.radius)
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.popover)
}

/// 卡片标题行（标题 + 可选右侧操作区）。
pub fn card_header(title: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
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
// 横幅（错误 / 成功 / 警告 / 信息）
// ---------------------------------------------------------------------------

fn banner(text: &str, bg: gpui::Hsla, fg: gpui::Hsla, border: gpui::Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(px(6.))
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_size(px(13.))
        .text_color(fg)
        .child(text.to_string())
}

/// 错误横条（内联红框）。
pub fn error_banner(text: &str, cx: &App) -> Div {
    let t = cx.theme().colors;
    banner(text, t.danger.opacity(0.1), t.danger, t.danger.opacity(0.4))
}

/// 成功横条。
pub fn success_banner(text: &str, cx: &App) -> Div {
    let t = cx.theme().colors;
    banner(text, t.success.opacity(0.1), t.success, t.success.opacity(0.4))
}

/// 警告横条。
pub fn warning_banner(text: &str, cx: &App) -> Div {
    let t = cx.theme().colors;
    banner(text, t.warning.opacity(0.12), t.warning, t.warning.opacity(0.4))
}

/// 信息横条。
pub fn info_banner(text: &str, cx: &App) -> Div {
    let t = cx.theme().colors;
    banner(
        text,
        t.info.opacity(0.1),
        t.info,
        t.info.opacity(0.4),
    )
}

// ---------------------------------------------------------------------------
// 徽章 / 键值行 / 代码块 / 加载态
// ---------------------------------------------------------------------------

/// 小徽章（状态标签）。
pub fn badge(text: &str, color: gpui::Hsla) -> Div {
    div()
        .px_2()
        .py_0p5()
        .rounded(px(4.))
        .bg(color.opacity(0.15))
        .text_size(px(11.))
        .text_color(color)
        .child(text.to_string())
}

/// 键值行（对齐证书/SSL 详情的 label: value 展示）。
pub fn info_row(label: &str, value: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_start()
        .gap_3()
        .py_1()
        .border_b_1()
        .border_color(theme.colors.border.opacity(0.5))
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

/// 等宽代码/结果块（文本可拖选复制）。`id` 须在页面内唯一（GPUI element id 去重）。
///
/// 文本用 `SelectableText`（gpui-base）：纯 div 文本无法被鼠标拖选，
/// 选中高亮取自主题 `selection` 色。
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
        .rounded(px(6.))
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.muted.opacity(0.4))
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
    Clipboard::new(id)
        .value(value)
        .label("复制")
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
    Button::new(id)
        .label(label)
        .primary()
        .on_click(on_click)
}

/// 次级按钮。
pub fn secondary_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .label(label)
        .secondary()
        .on_click(on_click)
}

/// 危险按钮（清空/删除等）。
pub fn danger_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .label(label)
        .danger()
        .on_click(on_click)
}

/// 幽灵按钮（轻量操作）。
pub fn ghost_button(
    id: &'static str,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .label(label)
        .ghost()
        .on_click(on_click)
}

// ---------------------------------------------------------------------------
// 结果区（只读 Textarea，带高度）
// ---------------------------------------------------------------------------

/// 只读多行结果区。`height` 为逻辑像素。
///
/// 用 `readonly` 而非 `disabled`：readonly 保持可聚焦、可选择、可复制，
/// 仅拒绝用户修改（disabled 会连选择/复制一起禁掉，结果无法摘录）。
pub fn result_area(
    state: &Entity<TextareaState>,
    height: f32,
) -> gpui_component::input::Textarea {
    Textarea::new(state)
        .h(px(height))
        .readonly(true)
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
