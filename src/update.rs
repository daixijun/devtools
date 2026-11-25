//! 「软件更新」窗口（app 菜单 → 检查更新…）。
//!
//! 状态机渲染：Checking / UpToDate / Available / Updating / ReadyToRestart /
//! Failed。检查与下载安装均在业务层 `devtools::utils::updater`（GitHub
//! Releases），经 core_rt 提交到后台 tokio Runtime：检查是单值结果流
//! （submit），下载安装是多值进度流（submit_multi），结果回流本视图更新
//! 状态（与工具页异步模式同构）。Changelog 用 Markdown 渲染并可滚动。

use devtools::i18n::t;
use devtools::utils::updater::{self, ReleaseInfo, UpdateEvent};
use futures::StreamExt as _;
use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, Render, Window};
use gpui_component::{
    progress::Progress, scroll::ScrollableElement as _, text::TextView, ActiveTheme as _,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{components::ui, core_rt};

/// 启动时自动更新（设置页「自动更新」开启时由 main 调用）。
///
/// 静默检查 → 有新版本则自动下载安装（进度不展示，失败不打扰启动）→
/// 成功后打开更新窗口展示「更新已就绪」，由用户确认重启。
pub fn auto_update_on_launch(cx: &mut gpui::App) {
    if !crate::config_state::get(cx).auto_update {
        return;
    }
    let mut check = core_rt::submit(cx, updater::fetch_latest_release());
    cx.spawn(async move |cx| {
        let Some(Ok(info)) = check.next().await else {
            return; // 网络失败 / 尚无发布 / 已是最新——均静默结束
        };
        if !updater::is_newer_version(&info.version, updater::current_version()) {
            return;
        }

        // 有新版本：静默下载安装，只消费终态事件
        let cancel = Arc::new(AtomicBool::new(false));
        let info_for_install = info.clone();
        let mut events = cx.update(|cx| {
            core_rt::submit_multi(cx, move |tx| async move {
                updater::download_and_install(info_for_install, tx, cancel).await
            })
        });
        while let Some(event) = events.next().await {
            match event {
                UpdateEvent::Finished => {
                    cx.update(|cx| crate::windows::open_update_ready(cx, &info));
                    break;
                }
                UpdateEvent::Failed(msg) => {
                    // 静默失败：仅在日志留痕，不弹窗打扰
                    eprintln!("自动更新失败: {msg}");
                    break;
                }
                _ => {}
            }
        }
    })
    .detach();
}

pub struct UpdateView {
    state: UpdateState,
    /// 检查到的可用更新（取消下载后回退展示、失败重试时复用）
    available: Option<ReleaseInfo>,
    /// 取消标志：下载循环逐块检查；关窗钩子同样置位
    cancel: Arc<AtomicBool>,
}

enum UpdateState {
    /// 检查中
    Checking,
    /// 已是最新
    UpToDate,
    /// 有新版本
    Available(ReleaseInfo),
    /// 下载安装中（installing=true 后进度条隐藏、仅显示安装文案）
    Updating {
        version: String,
        progress: f32,
        installing: bool,
    },
    /// 安装完成，等待用户重启
    ReadyToRestart { version: String },
    /// 检查或更新失败（错误消息）
    Failed(String),
}

impl UpdateView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            state: UpdateState::Checking,
            available: None,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        this.start_check(cx);
        this
    }

    /// 自动更新完成后的窗口入口：跳过检查，直接显示「更新已就绪」。
    pub fn new_ready(version: String, _cx: &mut Context<Self>) -> Self {
        Self {
            state: UpdateState::ReadyToRestart { version },
            available: None,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 已有窗口时切换到「更新已就绪」状态（自动更新完成通知复用窗口）。
    pub fn show_ready(&mut self, version: &str, cx: &mut Context<Self>) {
        self.state = UpdateState::ReadyToRestart {
            version: version.to_string(),
        };
        cx.notify();
    }

    /// 发起一次更新检查（窗口创建与菜单重复触发共用）。
    pub fn start_check(&mut self, cx: &mut Context<Self>) {
        self.state = UpdateState::Checking;
        let mut rx = core_rt::submit(cx, updater::fetch_latest_release());
        cx.spawn(async move |this, cx| {
            let Some(result) = rx.next().await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(info)
                        if updater::is_newer_version(&info.version, updater::current_version()) =>
                    {
                        this.available = Some(info.clone());
                        UpdateState::Available(info)
                    }
                    Ok(_) => UpdateState::UpToDate,
                    Err(e) => UpdateState::Failed(e.to_string()),
                };
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 开始下载并安装 `info`（「立即更新」入口）。
    fn start_update(&mut self, info: &ReleaseInfo, cx: &mut Context<Self>) {
        let version = info.version.clone();
        // 新一轮下载重置取消标志
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let info = info.clone();
        self.state = UpdateState::Updating {
            version: version.clone(),
            progress: 0.0,
            installing: false,
        };
        cx.notify();

        let mut rx = core_rt::submit_multi(cx, move |tx| async move {
            updater::download_and_install(info, tx, cancel).await
        });
        cx.spawn(async move |this, cx| {
            while let Some(event) = rx.next().await {
                // 取消后的残余事件一律忽略（UI 已回到可重试状态）
                if this
                    .update(cx, |this, cx| {
                        if this.cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        match event {
                            UpdateEvent::Progress(pct) => {
                                if let UpdateState::Updating { progress, .. } = &mut this.state {
                                    *progress = pct;
                                    cx.notify();
                                }
                            }
                            UpdateEvent::Verifying | UpdateEvent::Installing => {
                                if let UpdateState::Updating { installing, .. } = &mut this.state {
                                    *installing = true;
                                    cx.notify();
                                }
                            }
                            UpdateEvent::Finished => {
                                let version = match &this.state {
                                    UpdateState::Updating { version, .. } => version.clone(),
                                    _ => String::new(),
                                };
                                this.state = UpdateState::ReadyToRestart { version };
                                cx.notify();
                            }
                            UpdateEvent::Failed(msg) => {
                                this.state = UpdateState::Failed(msg);
                                cx.notify();
                            }
                        }
                    })
                    .is_err()
                {
                    // 实体已释放（窗口关闭）：停止消费
                    break;
                }
            }
        })
        .detach();
    }

    /// 取消当前下载（关窗钩子调用；不回退状态——窗口即将销毁）。
    pub fn request_cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 取消当前下载（回退到「发现新版本」状态等待用户决定）。
    fn cancel_download(&mut self, cx: &mut Context<Self>) {
        self.cancel.store(true, Ordering::Relaxed);
        if let UpdateState::Updating { .. } = self.state {
            if let Some(info) = self.available.clone() {
                self.state = UpdateState::Available(info);
            } else {
                self.state = UpdateState::Checking;
                self.start_check(cx);
            }
            cx.notify();
        }
    }
}

impl Render for UpdateView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let current = updater::current_version();

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(match &self.state {
                UpdateState::Checking => div()
                    .text_size(px(13.))
                    .text_color(theme.colors.muted_foreground)
                    .child(t("正在检查更新…")),
                UpdateState::UpToDate => div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t("已是最新版本")),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!("{} v{current}", t("当前版本"))),
                    ),
                UpdateState::Available(info) => {
                    let url = info.url.clone();
                    let notes = info.notes.trim().to_string();
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .px_6()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("{} v{}", t("发现新版本"), info.version)),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(format!("{} v{current}", t("当前版本"))),
                        )
                        .when(!notes.is_empty(), |el| {
                            el.child(
                                ui::panel(cx)
                                    .id("update-notes")
                                    .w_full()
                                    .max_h_56()
                                    .overflow_y_scrollbar()
                                    .child(TextView::markdown("update-changelog", notes)),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(ui::primary_button(
                                    "update-download",
                                    t("立即更新"),
                                    cx.listener(|this, _, _, cx| {
                                        if let Some(info) = this.available.clone() {
                                            this.start_update(&info, cx);
                                        }
                                    }),
                                ))
                                .child(ui::secondary_button(
                                    "update-open-page",
                                    t("前往下载页"),
                                    move |_, _, cx| {
                                        cx.open_url(&url);
                                    },
                                )),
                        )
                }
                UpdateState::Updating {
                    version,
                    progress,
                    installing,
                } => {
                    let pct = (progress * 100.0).round() as u32;
                    let installing = *installing;
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .px_6()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("{} v{version}", t("正在更新"))),
                        )
                        .when(installing, |el| {
                            el.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(t("正在安装新版本，请稍候…")),
                            )
                        })
                        .when(!installing, |el| {
                            el.child(ui::panel(cx).w_full().child(
                                div().flex().flex_col().gap_1().child(
                                    Progress::new("update-progress").value(*progress * 100.0),
                                ),
                            ))
                        })
                        .when(!installing, |el| {
                            el.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(format!("{} {pct}%", t("下载中"))),
                            )
                        })
                        .when(!installing, |el| {
                            el.child(ui::secondary_button(
                                "update-cancel",
                                t("取消"),
                                cx.listener(|this, _, _, cx| this.cancel_download(cx)),
                            ))
                        })
                }
                UpdateState::ReadyToRestart { version } => div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{} v{version}", t("更新已就绪"))),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(t("重启 DevTools 即可使用新版本")),
                    )
                    .child(ui::primary_button(
                        "update-relaunch",
                        t("立即重启"),
                        |_, _, _| {
                            if let Err(e) = updater::relaunch() {
                                // 拉起新进程失败：罕见（可执行被杀软占用等），仅上报日志
                                eprintln!("重启失败: {e}");
                            }
                        },
                    )),
                UpdateState::Failed(message) => {
                    let message = message.clone();
                    let has_update = self.available.is_some();
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .px_6()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t("更新失败")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(message),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .when(has_update, |el| {
                                    el.child(ui::primary_button(
                                        "update-retry-download",
                                        t("重试更新"),
                                        cx.listener(|this, _, _, cx| {
                                            if let Some(info) = this.available.clone() {
                                                this.start_update(&info, cx);
                                            }
                                        }),
                                    ))
                                })
                                .when(!has_update, |el| {
                                    el.child(ui::secondary_button(
                                        "update-retry-check",
                                        t("重试"),
                                        cx.listener(|this, _, _, cx| this.start_check(cx)),
                                    ))
                                })
                                .child(ui::secondary_button(
                                    "update-open-page",
                                    t("前往下载页"),
                                    |_, _, cx| {
                                        cx.open_url(&updater::releases_page_url());
                                    },
                                )),
                        )
                }
            })
    }
}
