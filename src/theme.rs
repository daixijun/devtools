//! 主题初始化与运行中跟随系统切换（M4）。
//!
//! 将 core 的三态主题（light/dark/system）映射到 gpui-component 的
//! `ThemeMode`。system 模式下每个窗口订阅外观变化事件实时跟随；
//! light/dark 固定主题不受系统影响。
//!
//! 在内置默认主题之上施加应用级「macOS 分组视图」覆盖：
//!
//! 1. 底色分层：默认主题两种模式下 background 与 popover 同色（白/白、
//!    #0a0a0a/#0a0a0a），卡片只能靠描边区分层级。覆盖后 light 页面转中性
//!    浅灰、卡片保持白色，dark 页面保持近黑、卡片抬升为 neutral-900。
//! 2. 主色：默认主题 primary 为近黑的 neutral-900（shadcn Web 风格），
//!    覆盖为 macOS 系统蓝（light #2563eb 系、dark #3b82f6 系），主按钮、
//!    开关、进度条等统一原生观感。
//! 3. 形状：radius 8 / radius_lg 10，关闭控件聚焦光圈。
//!
//! 注意：`Theme::change` 会重新载入注册主题冲掉覆盖，因此覆盖必须在
//! 每次切换之后重放（本文件三条切换路径均已收口）。

use devtools::config::ThemeMode;
use gpui::{px, App, Global, Subscription, Window};
use gpui_component::theme::{
    blue_300, blue_400, blue_500, blue_600, blue_700, blue_800, neutral_100, neutral_200,
    neutral_800, neutral_900, white, Theme, ThemeMode as ComponentThemeMode,
};

/// 外观订阅保活表：Subscription drop 即取消订阅，必须持有才能持续跟随。
struct AppearanceSubscriptions(Vec<Subscription>);

impl Global for AppearanceSubscriptions {}

/// 应用「macOS 分组视图」视觉覆盖（见模块注释）。
///
/// gpui-component 文档要求：切模式与改颜色需分两次 `Theme::update`——
/// 本函数仅在 change 之后调用。
fn apply_overrides(cx: &mut App) {
    Theme::update(cx, |t| {
        let (primary, hover, active) = if t.mode.is_dark() {
            // dark：页面保持近黑，卡片（popover）抬升一层形成分层
            t.colors.popover = neutral_900();
            (blue_500(), blue_400(), blue_300())
        } else {
            // light：页面转中性浅灰，卡片（popover）保持白色形成分层
            t.colors.background = neutral_100();
            (blue_600(), blue_700(), blue_800())
        };
        // 主色系（Switch/Checkbox/Progress/Select 等控件直接读 primary）
        t.colors.primary = primary;
        t.colors.primary_hover = hover;
        t.colors.primary_active = active;
        t.colors.primary_foreground = white();
        // Button 组件读 button_primary 家族——其默认值在主题载入时从
        // primary fallback 定型，不会跟随上述覆盖，需一并写入
        t.colors.button_primary = primary;
        t.colors.button_primary_hover = hover;
        t.colors.button_primary_active = active;
        t.colors.button_primary_foreground = white();
        // 分段 TabBar：激活 tab 底色 = background（light 白 / dark 近黑），
        // bar 底与激活底差距小（#f5f5f5/#171717）导致激活态几乎看不出。
        // 调深 bar 一档（light neutral-200、dark neutral-800）拉开色差。
        t.colors.tab_bar_segmented = if t.mode.is_dark() {
            neutral_800()
        } else {
            neutral_200()
        };
        t.radius = px(8.);
        t.radius_lg = px(10.);
        t.focus_ring = false;
    });
}

pub fn apply_initial(mode: ThemeMode, cx: &mut App) {
    match mode {
        ThemeMode::Light => Theme::change(ComponentThemeMode::Light, None, cx),
        ThemeMode::Dark => Theme::change(ComponentThemeMode::Dark, None, cx),
        // system：启动时默认 light，窗口创建时由 observe_window_appearance 修正
        ThemeMode::System => Theme::change(ComponentThemeMode::Light, None, cx),
    }
    apply_overrides(cx);
}

/// 按窗口当前外观同步主题（system 模式下在窗口创建后调用）。
pub fn sync_with_window_appearance(window: &Window, cx: &mut App) {
    let mode = ComponentThemeMode::from(window.appearance());
    Theme::change(mode, None, cx);
    apply_overrides(cx);
}

/// 写共享配置并即时切换主题模式（侧栏主题图标与设置弹窗共用）。
///
/// system：按当前窗口外观解析；各窗口创建时已注册 observe_window_appearance，
/// 配置写入后系统外观变化会自动跟随。
pub fn set_mode(mode: ThemeMode, window: &mut Window, cx: &mut App) {
    crate::config_state::update(cx, |cfg| cfg.theme = mode);
    match mode {
        ThemeMode::Light => Theme::change(ComponentThemeMode::Light, None, cx),
        ThemeMode::Dark => Theme::change(ComponentThemeMode::Dark, None, cx),
        ThemeMode::System => Theme::sync_system_appearance(Some(window), cx),
    }
    apply_overrides(cx);
}

/// 订阅窗口外观变化：仅当配置为 system 模式时跟随（M4 运行中主题切换）。
pub fn observe_window_appearance(window: &mut Window, cx: &mut App) {
    if !cx.has_global::<AppearanceSubscriptions>() {
        cx.set_global(AppearanceSubscriptions(Vec::new()));
    }
    let is_system = crate::config_state::get(cx).theme == ThemeMode::System;
    if is_system {
        sync_with_window_appearance(window, cx);
    }
    let sub = window.observe_window_appearance(|window, cx| {
        if crate::config_state::get(cx).theme == ThemeMode::System {
            sync_with_window_appearance(window, cx);
        }
    });
    cx.global_mut::<AppearanceSubscriptions>().0.push(sub);
}
