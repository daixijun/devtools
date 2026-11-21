//! 应用配置全局状态（包装 devtools_core::config::AppConfig）。

use devtools_core::config::AppConfig;
use gpui::{App, Global};

/// 配置全局单例。写入配置的统一入口是 [`update`]（内存修改 + 持久化）。
pub struct ConfigState {
    pub config: AppConfig,
}

impl Global for ConfigState {}

pub fn init(config: AppConfig, cx: &mut App) {
    cx.set_global(ConfigState { config });
}

pub fn get(cx: &App) -> &AppConfig {
    &cx.global::<ConfigState>().config
}

/// 修改并持久化配置。持久化失败仅记录日志，不阻塞 UI 操作。
pub fn update(cx: &mut App, f: impl FnOnce(&mut AppConfig)) {
    let config = &mut cx.global_mut::<ConfigState>().config;
    f(config);
    if let Err(e) = config.save() {
        eprintln!("保存配置失败: {}", e);
    }
}
