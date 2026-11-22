//! 「软件更新」窗口（app 菜单 → 检查更新…）。
//!
//! 状态机渲染：Checking / UpToDate / Available / Failed。网络检查在业务层
//! `devtools::utils::updater`（GitHub Releases），经 core_rt 提交到后台
//! tokio Runtime，结果回流本视图更新状态（与工具页异步模式同构）。

use devtools::i18n::t;
use devtools::utils::updater::{self, ReleaseInfo};
use futures::StreamExt as _;
use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, Render, Window};
use gpui_component::ActiveTheme as _;

use crate::{components::ui, core_rt};

pub struct UpdateView {
    state: UpdateState,
}

enum UpdateState {
    /// 检查中
    Checking,
    /// 已是最新
    UpToDate,
    /// 有新版本
    Available(ReleaseInfo),
    /// 检查失败（错误消息）
    Failed(String),
}

impl UpdateView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            state: UpdateState::Checking,
        };
        this.start_check(cx);
        this
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
                                    .max_h_24()
                                    .overflow_y_scroll()
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.colors.muted_foreground)
                                            .child(notes),
                                    ),
                            )
                        })
                        .child(ui::primary_button(
                            "update-download",
                            t("前往下载"),
                            move |_, _, cx| {
                                cx.open_url(&url);
                            },
                        ))
                }
                UpdateState::Failed(message) => {
                    let message = message.clone();
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
                                .child(t("检查更新失败")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(message),
                        )
                        .child(ui::secondary_button(
                            "update-retry",
                            t("重试"),
                            cx.listener(|this, _, _, cx| this.start_check(cx)),
                        ))
                }
            })
    }
}
