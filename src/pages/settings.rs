//! 设置页（唯一设置入口：侧栏底部齿轮 / 欢迎页「偏好设置」卡 / 应用菜单
//! Cmd-, / 托盘「设置」，全部汇入 `pages/mod.rs` 的工具分发渲染完整页）。
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

use devtools::config::ThemeMode;
use devtools::i18n::{t, Language};
use gpui::{prelude::*, px, App, Context, IntoElement, Render, Window};
use gpui_component::{
    button::Button, button::ButtonVariants as _, switch::Switch,
    theme::ThemeMode as ComponentThemeMode, ActiveTheme as _, Disableable as _,
};

use crate::components::{notify, ui};

pub struct Settings {
    /// 主题模式镜像（写共享配置时同步更新）。
    theme: ThemeMode,
    /// 语言镜像（切换后立即生效：写配置 + 刷新托盘/菜单 + 重建窗口）。
    language: Language,
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

    fn new(cx: &mut Context<Self>) -> Self {
        Self::from_config(cx)
    }

    fn from_config(cx: &mut Context<Self>) -> Self {
        let config = crate::config_state::get(cx);
        Self {
            theme: config.theme,
            language: Language::parse(&config.language),
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

    // -----------------------------------------------------------------------
    // 语言
    // -----------------------------------------------------------------------

    /// 切换界面语言（立即生效）。
    ///
    /// 语言是全局渲染属性：写共享配置 → i18n 全局 locale → 刷新托盘菜单与
    /// 应用菜单 → 关闭全部窗口重建主窗口（新窗口以新语言渲染）。
    fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        if self.language == language {
            return;
        }
        crate::config_state::update(cx, |cfg| {
            cfg.language = language.as_config_str().to_string()
        });
        devtools::i18n::set_language(language);
        self.language = language;
        crate::tray::refresh(cx);
        crate::app_menu::apply_menus(cx);
        crate::windows::reopen_all(cx);
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
            crate::config_state::update(cx, |cfg| cfg.tray_enabled = requested);
        } else {
            // 失败回滚：UI 与配置都回写实际状态
            crate::config_state::update(cx, |cfg| cfg.tray_enabled = actual);
            notify::toast_error(window, cx, t("托盘状态切换失败，已回滚"));
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
                notify::toast_error(window, cx, format!("{}: {err}", t("设置开机自启失败")));
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
        crate::config_state::update(cx, |cfg| cfg.start_minimized = requested);
        cx.notify();
    }

    fn toggle_close_to_tray(&mut self, requested: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        // 关闭到托盘：同上，统一写共享配置（windows.rs 关闭钩子实时读取）
        crate::config_state::update(cx, |cfg| cfg.close_to_tray = requested);
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
                            .text_size(px(12.))
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
            (ThemeMode::Light, "theme-light", t("☀️ 浅色")),
            (ThemeMode::Dark, "theme-dark", t("🌙 深色")),
            (ThemeMode::System, "theme-system", t("🖥️ 跟随系统")),
        ] {
            let selected = self.theme == mode;
            // 激活项用 primary（主题蓝实底）与未激活 secondary 拉开对比
            let button = if selected {
                Button::new(id).label(label).primary()
            } else {
                Button::new(id).label(label).secondary()
            };
            theme_buttons = theme_buttons.child(button.on_click(
                cx.listener(move |this, _event, window, cx| this.set_theme(mode, window, cx)),
            ));
        }
        // System 模式下按窗口外观解析实际明暗
        let resolved_dark = match self.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => ComponentThemeMode::from(window.appearance()).is_dark(),
        };
        let appearance_card = ui::card(cx)
            .child(ui::card_header(t("外观设置"), cx))
            .child(ui::field(t("主题模式"), theme_buttons, cx))
            .child(ui::hint_text(
                &format!(
                    "{}: {}",
                    t("当前主题"),
                    if resolved_dark {
                        t("深色")
                    } else {
                        t("浅色")
                    }
                ),
                cx,
            ));

        // ---- 语言设置卡（跟随系统 / 简体中文 / English）----
        let mut language_buttons = gpui::div().flex().gap_2();
        for (lang, id, label) in [
            (Language::System, "lang-system", t("跟随系统")),
            (Language::ZhCn, "lang-zh", t("简体中文")),
            (Language::En, "lang-en", t("English")),
        ] {
            let selected = self.language == lang;
            let button = if selected {
                Button::new(id).label(label).primary()
            } else {
                Button::new(id).label(label).secondary()
            };
            language_buttons = language_buttons.child(button.on_click(
                cx.listener(move |this, _event, _window, cx| this.set_language(lang, cx)),
            ));
        }
        let language_card = ui::card(cx)
            .child(ui::card_header(t("语言"), cx))
            .child(ui::field(t("语言"), language_buttons, cx));

        // ---- 系统设置卡（4 个开关）----
        let tray_switch = Switch::new("switch-tray")
            .checked(show_tray)
            .disabled(loading)
            .on_change(cx.listener(|this, checked: &bool, window, cx| {
                this.toggle_tray(*checked, window, cx)
            }));
        let autostart_switch = Switch::new("switch-autostart")
            .checked(self.autostart_enabled)
            .disabled(loading)
            .on_change(cx.listener(|this, checked: &bool, window, cx| {
                this.toggle_autostart(*checked, window, cx)
            }));
        let start_minimized = crate::config_state::get(cx).start_minimized;
        let start_minimized_switch = Switch::new("switch-start-minimized")
            .checked(start_minimized)
            .disabled(loading || !show_tray)
            .on_change(cx.listener(|this, checked: &bool, _window, cx| {
                this.toggle_start_minimized(*checked, cx)
            }));
        let close_to_tray = crate::config_state::get(cx).close_to_tray;
        let close_to_tray_switch = Switch::new("switch-close-to-tray")
            .checked(close_to_tray)
            .disabled(loading || !show_tray)
            .on_change(cx.listener(|this, checked: &bool, _window, cx| {
                this.toggle_close_to_tray(*checked, cx)
            }));
        // Linux 无系统托盘（tray.rs stub）：托盘相关开关整行隐藏
        let tray_available = cfg!(not(target_os = "linux"));
        let mut system_card = ui::card(cx).child(ui::card_header(t("系统设置"), cx));
        if tray_available {
            system_card = system_card
                .child(Self::setting_row(
                    t("显示系统托盘图标"),
                    t("在系统托盘中显示应用图标，便于快速访问"),
                    tray_switch,
                    cx,
                ))
                .child(Self::setting_row(
                    t("启动时最小化到托盘"),
                    t("应用程序启动时直接最小化到系统托盘"),
                    start_minimized_switch,
                    cx,
                ))
                .child(Self::setting_row(
                    t("关闭时最小化到托盘"),
                    t("关闭窗口时最小化到系统托盘而非退出程序"),
                    close_to_tray_switch,
                    cx,
                ));
        }
        system_card = system_card.child(Self::setting_row(
            t("开机自动启动"),
            t("系统启动时自动运行此应用程序"),
            autostart_switch,
            cx,
        ));

        // ---- 关于应用卡 ----
        let about_card = ui::card(cx)
            .child(ui::card_header(t("关于应用"), cx))
            .child(ui::info_rows(
                [
                    ui::info_row(t("应用名称"), t("开发者工具箱"), cx),
                    ui::info_row(t("版本"), env!("CARGO_PKG_VERSION"), cx),
                    ui::info_row(t("技术栈"), "GPUI + Rust", cx),
                ],
                cx,
            ));

        ui::tool_page("settings")
            .child(appearance_card)
            .child(language_card)
            .child(system_card)
            .child(about_card)
            .into_any_element()
    }
}
