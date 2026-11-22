//! 工具窗口宿主：页头（工具名/描述/分类徽标）+ 工具页面视图。
//!
//! 页面内容由 `tools::create_tool_view` 按 tool id 分发（M5-M7 逐批实现）。

use devtools::i18n::t;
use devtools::registry::ToolDefinition;
use gpui::{div, prelude::*, px, FocusHandle, Focusable, Render, Window};
use gpui_component::ActiveTheme as _;

pub struct ToolWindow {
    tool: &'static ToolDefinition,
    page: gpui::AnyView,
}

impl ToolWindow {
    pub fn new(tool: &'static ToolDefinition, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = crate::pages::create_tool_view(tool, window, cx);
        Self { tool, page }
    }
}

impl Focusable for ToolWindow {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        // 工具页自持焦点（输入框），宿主不抢占
        cx.focus_handle()
    }
}

impl Render for ToolWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme().clone();
        let tool = self.tool;
        let category = devtools::registry::category_name(tool.category);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(
                // 页头：hairline 下分隔线（macOS 工具栏样式），分类为 muted 小字
                // （原彩色底徽标与工具页无边框语言冲突）
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(17.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(t(tool.name)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(t(tool.description)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(t(category)),
                    ),
            )
            // 内容区：工具页面。滚动在此承担——页面（tool_page）为内容自然高，
            // items_start 阻止交叉轴 stretch 把页面高度定死（否则子项被压缩、无滚动量）
            .child(
                div()
                    .id("tool-page-scroll")
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .overflow_y_scroll()
                    .child(self.page.clone().into_any_element()),
            )
    }
}
