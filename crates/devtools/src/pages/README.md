# 工具页实现指南

每个工具一个文件，实现模式统一。新增工具时：在 `crates/devtools/src/registry.rs` 注册元数据，
在 `mod.rs` 增加模块声明与 `create_tool_view` 分发，并按 AGENTS.md 的流程执行。

## 文件骨架

```rust
//! 工具名。

use gpui::{App, prelude::*, IntoElement, Window};
use gpui_component::input::{InputEvent, InputState};
use gpui_component::ActiveTheme as _;

use crate::components::{notify, ui};

pub struct JsonFormatter {
    input: gpui::Entity<gpui_component::input::TextareaState>,
    output: gpui::Entity<gpui_component::input::TextareaState>,
    error: Option<String>,
}

impl JsonFormatter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input = ui::new_textarea(window, cx);
        // 订阅输入变化（自动转换场景）
        cx.subscribe(&input, |this, _, event: &gpui_component::input::InputEvent, cx| {
            if matches!(event, InputEvent::Change) { this.convert(cx); }
        }).detach();
        Self { input, output: ui::new_textarea(window, cx), error: None }
    }

    fn convert(&mut self, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        // 调业务层（devtools lib）逻辑...
        cx.notify();
    }

    fn set_output(&mut self, text: &str, window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.output.update(cx, |out, cx| out.set_value(text, window, cx));
    }
}

impl gpui::Render for JsonFormatter {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        // 工具名/描述由 ToolWindow 页头统一展示，页面内不再重复标题
        ui::tool_page("jsonformatter")
            .child(ui::card(cx)
                .child(ui::field("输入", ui::edit_area(&self.input, 180.), cx))
                .child(ui::error_banner(&self.error.clone().unwrap_or_default(), cx).when_some(self.error.clone(), |el, e| el))
            )
    }
}
```

## 视觉规范（macOS 分组视图语言）

- **边框只留给输入控件与按钮**（gpui-component 原生控件自带）；卡片/面板/代码块/横条/徽章一律无边框，层级靠背景色深浅：页面底 = `background`（浅灰/近黑），卡片 = `popover`（白/抬升层）
- 状态色块 = 浅底（0.1）+ 同色文字；行间分隔用 hairline（`border.opacity(0.5)`）且末行无线
- 字号五档：11（注记）/12（正文小）/13（正文）/14（小标题）/17（页头/大数值）；圆角只用 `theme.radius`/`theme.radius_lg`（badge 用 rounded_full）；等宽统一 Menlo
- 卡内子块用 `ui::panel`（muted 底），不得「卡内再套边框」

## 组件层 API（crates/devtools/src/components/ui.rs）

- `tool_page(id) -> Stateful<Div>`：页面根（纵向、p_4、gap_4；整页滚动由 ToolWindow 内容区承担）
- `card(cx)`（无边框 popover 底分组卡）、`card_header(title, cx)`
- `panel(cx)`（卡内无框子块，muted 底）、`toolbar()`（无边框工具条行）、`file_row(path, cx)`（只读路径行，按钮链式追加）、`stat_tile(value, label, cx)`
- `field(label, control, cx)` / `hfield(label, control, cx)`
- `new_input(window, cx) -> Entity<InputState>`（单行）、`new_textarea`、`new_editor`
- `edit_area(&state, height)` / `result_area(&state, height)`（只读）
- `code_edit_area(&state, height)` / `code_result_area(&state, height)`（Editor 语法高亮）
- `primary_button(id, label, |ev, window, cx| ...)` / `secondary_button` / `danger_button` / `ghost_button` / `icon_button(id, icon)`
- `error_banner/warning_banner/success_banner/info_banner(text, cx)`；`rich_banner(text, color, cx)`（可链式加内容的横条）
- `info_row(label, value, cx)` + `info_rows(rows, cx)`（行间 hairline、末行无线）；表格：`table_head(cx)` + `table_head_cell(text, cx)`、`table_row()` + `table_rows(rows, cx)`
- `tab_bar(id, items, selected, on_select)`（macOS 分段控件，on_select 收 &usize）
- `badge(text, color)`、`mono_block(id, text, cx)`、`empty_hint(text, cx)`、`loading_row(text, cx)`、`hint_text(text, cx)`
- `copy_button(id, value)`（gpui-component Clipboard，点击即复制）

## 通知

`notify::toast_success(window, cx, "已复制")` / `toast_error` / `toast_warning` / `toast_info`

## 下拉选择（Select）

```rust
let sel = crate::components::select_state(window, cx, &[("CBC", "CBC 模式"), ("ECB", "ECB 模式")], 0);
cx.subscribe(&sel, |this, _, event: &gpui_component::select::SelectEvent<crate::components::SimpleSelectDelegate>, cx| {
    if let gpui_component::select::SelectEvent::Confirm(Some(v)) = event {
        this.mode = v.to_string(); // SharedString
        cx.notify();
    }
}).detach();
// render: ui::field("模式", gpui_component::Select::new(&sel), cx)
```

## 复选框 / 开关

```rust
gpui_component::Checkbox::new("id").label("导出字段").checked(true)
    .on_click(gpui::AppContext::listener(cx, |this, checked: &bool, _window, _cx| { ... })) // 或闭包 + Entity 句柄
```
回调签名 `Fn(&bool, &mut Window, &mut App)`；需要改自身状态时用 `cx.listener(...)`（在 Context<Self> 里）：
`cx.listener(move |this, checked: &bool, _window, _cx| { this.exported = *checked; this.convert(); })`
（Checkbox::on_click 的第一参数是 &bool）

设置类布尔项用 `Switch`（macOS 原生开关，受控组件——回调内写状态并 `cx.notify()`）：

```rust
gpui_component::switch::Switch::new("id")
    .checked(this.enabled)
    .on_change(cx.listener(move |this, checked: &bool, _window, cx| {
        this.enabled = *checked;
        cx.notify();
    }))
```

## 异步调用 core（tokio 桥）

```rust
let (sender, rx) = futures::channel::mpsc::unbounded();
let rx2 = crate::core_rt::submit(cx, async move {
    devtools::tools::ssl_checker::check_ssl_info(domain).await
});
cx.spawn_in(window, async move |this, cx| {
    if let Some(result) = futures::StreamExt::next(&mut futures::channel::mpsc::UnboundedReceiverExt::???) ...
});
```

标准写法（推荐）：

```rust
use futures::StreamExt as _;
self.loading = true; cx.notify();
let rx = crate::core_rt::submit(cx, async move { devtools::tools::whois::query_whois(domain).await });
cx.spawn_in(window, async move |this, cx| {
    if let Some(result) = rx.await.ok().and_then(|mut s| futures::StreamExt::next(&mut s)) { ... }
});
```

——**实际上最简**：

```rust
let mut rx = crate::core_rt::submit(cx, async move { ...await });
cx.spawn_in(window, async move |this, cx| {
    if let Some(result) = futures::StreamExt::next(&mut rx).await {
        let _ = this.update_in(cx, |this, window, cx| this.on_result(result, window, cx));
    }
}).detach();
```

`UnboundedReceiver` 实现 Stream；`futures::StreamExt::next(&mut rx).await` 返回 Option<R>。
`update_in` 失败（窗口已关）直接忽略。

## 文件对话框（rfd）

```rust
if let Some(path) = rfd::FileDialog::new()
    .add_filter("图片文件", &["jpg", "png"])
    .pick_file() {
    // std::fs::read_to_string / read
}
if let Some(dir) = rfd::FileDialog::new().pick_folder() { ... }
```
rfd 调用必须在后台线程（阻塞主线程会卡 UI）——放进 `crate::core_rt::submit` 的 Future 里，
或 `cx.background_executor().spawn`。推荐：先 submit 一个返回 `Option<PathBuf>` 的任务。

## 文本读取

文件内容直接 `std::fs::read_to_string(path)`（在 core_rt 的后台任务里做）。

## 状态互斥锁不需要

每个工具窗口独立 Entity，无跨窗口共享状态。

## 渲染细节

- 所有颜色用 `cx.theme()`（ActiveTheme trait）：`theme.colors.foreground` / `muted_foreground` / `border` / `muted` / `danger` / `success` / `warning` / `info` / `popover` / `background`
- 禁止手写 `.border_1()`（边框只属于输入控件与按钮）；透明度只允许语义色横条底 0.1、badge 底 0.12、hairline 0.5
- 布局原语：`div().flex().flex_col().gap_2()` 等；等宽文本用 `.font_family("Menlo")`
- 长列表输出用 `ui::mono_block`（自带滚动、可拖选）；大编辑器用 Editor/Textarea（`.h(px(..))`）
- Enter 提交：订阅 InputEvent::PressEnter
- 按钮 loading 态：`Button::new(id).label(label).loading(loading)`（Disableable 已引入）
