//! 主题初始化与运行中跟随系统切换（M4）。
//!
//! 将 core 的三态主题（light/dark/system）映射到 gpui-component 的
//! `ThemeMode`。system 模式下每个窗口订阅外观变化事件实时跟随；
//! light/dark 固定主题不受系统影响。

use devtools_core::config::ThemeMode;
use gpui::{App, Global, Subscription, Window};
use gpui_component::theme::{Theme, ThemeMode as ComponentThemeMode};

/// 外观订阅保活表：Subscription drop 即取消订阅，必须持有才能持续跟随。
struct AppearanceSubscriptions(Vec<Subscription>);

impl Global for AppearanceSubscriptions {}

pub fn apply_initial(mode: ThemeMode, cx: &mut App) {
    match mode {
        ThemeMode::Light => Theme::change(ComponentThemeMode::Light, None, cx),
        ThemeMode::Dark => Theme::change(ComponentThemeMode::Dark, None, cx),
        // system：启动时默认 light，窗口创建时由 observe_window_appearance 修正
        ThemeMode::System => Theme::change(ComponentThemeMode::Light, None, cx),
    }
}

/// 按窗口当前外观同步主题（system 模式下在窗口创建后调用）。
pub fn sync_with_window_appearance(window: &Window, cx: &mut App) {
    let mode = ComponentThemeMode::from(window.appearance());
    Theme::change(mode, None, cx);
}

/// 写共享配置并即时切换主题模式（侧栏主题图标与设置弹窗共用）。
///
/// system：按当前窗口外观解析；各窗口创建时已注册 observe_window_appearance，
/// 配置写入后系统外观变化会自动跟随。
pub fn set_mode(mode: ThemeMode, window: &mut Window, cx: &mut App) {
    crate::config::update(cx, |cfg| cfg.theme = mode);
    match mode {
        ThemeMode::Light => Theme::change(ComponentThemeMode::Light, None, cx),
        ThemeMode::Dark => Theme::change(ComponentThemeMode::Dark, None, cx),
        ThemeMode::System => Theme::sync_system_appearance(Some(window), cx),
    }
}

/// 订阅窗口外观变化：仅当配置为 system 模式时跟随（M4 运行中主题切换）。
pub fn observe_window_appearance(window: &mut Window, cx: &mut App) {
    if !cx.has_global::<AppearanceSubscriptions>() {
        cx.set_global(AppearanceSubscriptions(Vec::new()));
    }
    let is_system = crate::config::get(cx).theme == ThemeMode::System;
    if is_system {
        sync_with_window_appearance(window, cx);
    }
    let sub = window.observe_window_appearance(|window, cx| {
        if crate::config::get(cx).theme == ThemeMode::System {
            sync_with_window_appearance(window, cx);
        }
    });
    cx.global_mut::<AppearanceSubscriptions>().0.push(sub);
}

