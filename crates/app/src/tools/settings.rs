//! 设置页。
//!
//! 入口形态（两种渲染）：
//! - 完整页（tool_page + 页头）：工具分发兜底路径保留；
//! - compact 弹窗体（主窗口侧栏底部「设置」图标打开，见 main_window.rs）。
//!
//! 实现要点（壳层能力直调，无 IPC 层）：
//! - 主题：写共享配置 + `Theme::change` / `Theme::sync_system_appearance` 即时生效
//!   （GPUI 全窗口刷新），切换逻辑统一走 `crate::theme::set_mode`。
//! - 托盘：`crate::tray::set_enabled`（运行时显隐，返回实际状态回写）。
//! - 自启：`crate::autostart::set`（LaunchAgent plist 文件操作很快，主线程同步即可，
//!   状态以 `autostart::status()` 为准；配置文件无 autostart 字段，与 start_minimized 无关）。
//! - 启动/关闭最小化到托盘：无前后端分离，统一写共享配置
//!   （windows.rs 关闭钩子在退出时实时读取）。
//!
//! 全局快捷键功能已移除（共享配置中的 hotkey 字段仅为向后兼容保留）。

use devtools_core::config::ThemeMode;
use gpui::{prelude::*, px, App, Context, IntoElement, Render, Window};
use gpui_component::{
    button::Button,
    switch::Switch,
    theme::ThemeMode as ComponentThemeMode,
    ActiveTheme as _, button::ButtonVariants as _, Disableable as _, Selectable as _,
};

use crate::components::{notify, ui};

pub struct Settings {
    /// 弹窗体形态（无页面骨架，嵌入 main_window 的设置弹窗）。
    compact: bool,
    /// 主题模式镜像（写共享配置时同步更新）。
    theme: ThemeMode,
    /// 托盘可见状态（挂载时读运行时，切换后以 set_enabled 返回的实际值为准）。
    show_tray: bool,
    /// 开机自启（挂载时读 autostart::status()，plist 存在即启用）。
    autostart_enabled: bool,
    /// 防抖开关连点（托盘/自启写文件期间禁用全部开关）。
    loading: bool,
}

impl Settings {
    pub fn create_view(_window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(Self::new).into()
    }

    /// 设置弹窗体（compact 形态；由 main_window 首次打开弹窗时创建并保持）。
    pub fn create_embedded(cx: &mut App) -> gpui::Entity<Self> {
        cx.new(|cx| Self {
            compact: true,
            ..Self::from_config(cx)
        })
    }

    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            compact: false,
            ..Self::from_config(cx)
        }
    }

    fn from_config(cx: &mut Context<Self>) -> Self {
        let config = crate::config::get(cx);
        Self {
            compact: false,
            theme: config.theme,
            // 托盘以运行时状态为准（图标加载失败时 config 为 true 而实际不可见）
            show_tray: crate::tray::is_enabled(cx),
            // 文件操作很快，挂载时同步读一次 plist 存在性
            autostart_enabled: crate::autostart::status(),
            loading: false,
        }
    }

    // -----------------------------------------------------------------------
    // 主题
    // -----------------------------------------------------------------------

    fn set_theme(&mut self, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.theme == mode {
            return;
        }
        crate::theme::set_mode(mode, window, cx);
        self.theme = mode;
        cx.notify();
    }

    /// 侧栏主题图标切换后的镜像同步（弹窗打开期间保持一致）。
    pub fn sync_theme(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        if self.theme != mode {
            self.theme = mode;
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------
    // 系统开关
    // -----------------------------------------------------------------------

    fn toggle_tray(&mut self, requested: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        // set_enabled 返回实际生效状态（托盘未初始化/系统调用失败时不等于请求值）
        let actual = crate::tray::set_enabled(requested, cx);
        if actual == requested {
            crate::config::update(cx, |cfg| cfg.tray_enabled = requested);
        } else {
            // 失败回滚：UI 与配置都回写实际状态
            crate::config::update(cx, |cfg| cfg.tray_enabled = actual);
            notify::toast_error(window, cx, "托盘状态切换失败，已回滚");
        }
        self.show_tray = actual;
        self.loading = false;
        cx.notify();
    }

    fn toggle_autostart(&mut self, requested: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        match crate::autostart::set(requested) {
            Ok(()) => self.autostart_enabled = requested,
            Err(err) => {
                // 失败回滚为实际状态（plist 可能处于半写入状态）
                self.autostart_enabled = crate::autostart::status();
                notify::toast_error(window, cx, format!("设置开机自启失败: {err}"));
            }
        }
        self.loading = false;
        cx.notify();
    }

    fn toggle_start_minimized(&mut self, requested: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        // 启动最小化：统一写共享配置
        // （启动流程 main.rs 实时读取该值）
        crate::config::update(cx, |cfg| cfg.start_minimized = requested);
        cx.notify();
    }

    fn toggle_close_to_tray(&mut self, requested: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        // 关闭到托盘：同上，统一写共享配置（windows.rs 关闭钩子实时读取）
        crate::config::update(cx, |cfg| cfg.close_to_tray = requested);
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 渲染
    // -----------------------------------------------------------------------

    fn setting_row(label: &str, desc: &str, control: Switch, cx: &App) -> gpui::Div {
        let theme = cx.theme().clone();
        gpui::div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .min_w_0()
                    .child(
                        gpui::div()
                            .text_size(px(13.))
                            .text_color(theme.colors.foreground)
                            .child(label.to_string()),
                    )
                    .child(
                        gpui::div()
                            .text_size(px(11.5))
                            .text_color(theme.colors.muted_foreground)
                            .child(desc.to_string()),
                    ),
            )
            .child(control)
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = self.loading;
        let show_tray = self.show_tray;

        // ---- 外观设置卡 ----
        let mut theme_buttons = gpui::div().flex().gap_2();
        for (mode, id, label) in [
            (ThemeMode::Light, "theme-light", "☀️ 浅色"),
            (ThemeMode::Dark, "theme-dark", "🌙 深色"),
            (ThemeMode::System, "theme-system", "🖥️ 跟随系统"),
        ] {
            let selected = self.theme == mode;
            theme_buttons = theme_buttons.child(
                Button::new(id)
                    .label(label)
                    .secondary()
                    .selected(selected)
                    .on_click(cx.listener(move |this, _event, window, cx| {
                        this.set_theme(mode, window, cx)
                    })),
            );
        }
        // System 模式下按窗口外观解析实际明暗
        let resolved_dark = match self.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => ComponentThemeMode::from(window.appearance()).is_dark(),
        };
        let appearance_card = ui::card(cx)
            .child(ui::card_header("外观设置", cx))
            .child(ui::field("主题模式", theme_buttons, cx))
            .child(ui::hint_text(
                &format!("当前主题: {}", if resolved_dark { "深色" } else { "浅色" }),
                cx,
            ));

        // ---- 系统设置卡（4 个开关）----
        let tray_switch = Switch::new("switch-tray")
            .checked(show_tray)
            .disabled(loading)
            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                this.toggle_tray(*checked, window, cx)
            }));
        let autostart_switch = Switch::new("switch-autostart")
            .checked(self.autostart_enabled)
            .disabled(loading)
            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                this.toggle_autostart(*checked, window, cx)
            }));
        let start_minimized = crate::config::get(cx).start_minimized;
        let start_minimized_switch = Switch::new("switch-start-minimized")
            .checked(start_minimized)
            .disabled(loading || !show_tray)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.toggle_start_minimized(*checked, cx)
            }));
        let close_to_tray = crate::config::get(cx).close_to_tray;
        let close_to_tray_switch = Switch::new("switch-close-to-tray")
            .checked(close_to_tray)
            .disabled(loading || !show_tray)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.toggle_close_to_tray(*checked, cx)
            }));
        let system_card = ui::card(cx)
            .child(ui::card_header("系统设置", cx))
            .child(Self::setting_row(
                "显示系统托盘图标",
                "在系统托盘中显示应用图标，便于快速访问",
                tray_switch,
                cx,
            ))
            .child(Self::setting_row(
                "开机自动启动",
                "系统启动时自动运行此应用程序",
                autostart_switch,
                cx,
            ))
            .child(Self::setting_row(
                "启动时最小化到托盘",
                "应用程序启动时直接最小化到系统托盘",
                start_minimized_switch,
                cx,
            ))
            .child(Self::setting_row(
                "关闭时最小化到托盘",
                "关闭窗口时最小化到系统托盘而非退出程序",
                close_to_tray_switch,
                cx,
            ));

        // ---- 关于应用卡 ----
        let about_card = ui::card(cx)
            .child(ui::card_header("关于应用", cx))
            .child(ui::info_row("应用名称", "开发者工具箱", cx))
            .child(ui::info_row("版本", "1.0.0", cx))
            .child(ui::info_row(
                "技术栈",
                "GPUI + Rust",
                cx,
            ));

        if self.compact {
            // 弹窗体：无页面骨架；限高 + 可滚动（内容超高一屏时兜底）。
            // 卡片 flex_shrink_0：滚动容器限高时 flex 子项默认会被压缩而非溢出
            gpui::div()
                .id("settings-modal-body")
                .flex()
                .flex_col()
                .gap_3()
                .max_h(px(560.))
                .overflow_y_scroll()
                .child(appearance_card.flex_shrink_0())
                .child(system_card.flex_shrink_0())
                .child(about_card.flex_shrink_0())
                .into_any_element()
        } else {
            ui::tool_page("settings")
                .child(ui::themed_header("设置", "配置应用程序偏好设置", cx))
                .child(appearance_card)
                .child(system_card)
                .child(about_card)
                .into_any_element()
        }
    }
}
