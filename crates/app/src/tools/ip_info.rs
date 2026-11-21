//! IP 地址信息查询。
//!
//! 挂载即自动查询一次（空 IP → 查询本机出口 IP）；输入 IP（Enter 触发）后
//! IPv4/IPv6 校验，经 core_rt 异步调用 core `query_ip_info`（ipinfo.io +
//! ip-api.com 双源），每来源一张信息卡，可切换显示格式化原始 JSON。

use devtools_core::tools::ip_info::{SourceInfo, query_ip_info};
use futures::StreamExt as _;
use gpui::{
    App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div, px, prelude::*,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Sizable as _,
    clipboard::Clipboard,
    input::{Input, InputEvent, InputState},
};

use crate::components::ui;

pub struct IpInfo {
    ip_input: Entity<InputState>,
    infos: Vec<SourceInfo>,
    raw_json: String,
    show_raw: bool,
    loading: bool,
    error: Option<String>,
    /// 查询序列号：丢弃过期查询的结果。
    query_seq: u64,
}

impl IpInfo {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ip_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("请输入IP地址，留空则查询当前IP")
        });

        let mut this = Self {
            ip_input,
            infos: Vec::new(),
            raw_json: String::new(),
            show_raw: false,
            loading: false,
            error: None,
            query_seq: 0,
        };

        // Enter 触发查询
        cx.subscribe_in(
            &this.ip_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.query(window, cx);
                }
            },
        )
        .detach();

        // 挂载即自动查询一次（空 IP → 查询本机出口 IP）
        this.query(window, cx);

        this
    }

    fn query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.error = None;

        let ip = self.ip_input.read(cx).value().trim().to_string();
        if !ip.is_empty() && !is_valid_ip(&ip) {
            self.error = Some("请输入有效的IP地址".to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.query_seq += 1;
        let seq = self.query_seq;
        cx.notify();

        let ip_opt = if ip.is_empty() { None } else { Some(ip) };
        let mut rx = crate::core_rt::submit(cx, async move { query_ip_info(ip_opt).await });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = rx.next().await else {
                return;
            };
            let _ = this.update_in(cx, |this, _window, cx| {
                if this.query_seq != seq {
                    return; // 已过期
                }
                this.loading = false;
                match result {
                    Ok(resp) => {
                        this.raw_json = serde_json::to_string_pretty(&resp).unwrap_or_default();
                        this.infos = resp.infos;
                        this.error = resp.error;
                    }
                    Err(e) => {
                        this.error = Some(format!("查询出错: {}", e));
                        this.infos.clear();
                        this.raw_json.clear();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn source_card(&self, idx: usize, info: &SourceInfo, cx: &App) -> Div {
        let copy_id = if idx == 0 { "ipinfo-copy-ip-0" } else { "ipinfo-copy-ip-1" };

        let mut ip_row = ui::info_row("IP 地址", opt_or_na(&info.ip), cx);
        if let Some(ip) = &info.ip {
            // IP 带复制小图标（Clipboard 组件，icon-only）
            ip_row = ip_row.child(
                Clipboard::new(copy_id)
                    .value(ip.clone())
                    .xsmall()
                    .tooltip("复制 IP"),
            );
        }

        ui::card(cx)
            .flex_1()
            .min_w(px(0.))
            .child(ui::card_header(&format!("IP 信息 ({})", info.source), cx))
            .child(ip_row)
            .child(ui::info_row("国家", opt_or_na(&info.country), cx))
            .child(ui::info_row("城市", opt_or_na(&info.city), cx))
            .child(ui::info_row("地区", opt_or_na(&info.region), cx))
            .child(ui::info_row("运营商", opt_or_na(&info.org), cx))
            .child(ui::info_row("时区", opt_or_na(&info.timezone), cx))
    }
}

/// Option<String> → 值或 N/A。
fn opt_or_na(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "N/A".to_string())
}

/// IP 校验：IPv4 / IPv6。
fn is_valid_ip(ip: &str) -> bool {
    is_valid_ipv4(ip) || is_valid_ipv6(ip)
}

fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 3
                && p.chars().all(|c| c.is_ascii_digit())
                && p.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
        })
}

fn is_valid_ipv6(s: &str) -> bool {
    // "::" 只能出现一次
    if s.matches("::").count() > 1 {
        return false;
    }
    let parse_groups = |part: &str| -> Option<Vec<u16>> {
        if part.is_empty() {
            return Some(Vec::new());
        }
        part.split(':')
            .map(|g| {
                if g.is_empty() || g.len() > 4 || !g.chars().all(|c| c.is_ascii_hexdigit()) {
                    None
                } else {
                    u16::from_str_radix(g, 16).ok()
                }
            })
            .collect()
    };

    match s.split_once("::") {
        None => parse_groups(s).map(|g| g.len() == 8).unwrap_or(false),
        Some((head, tail)) => {
            let head_groups = match parse_groups(head) {
                Some(g) => g,
                None => return false,
            };
            let tail_groups = match parse_groups(tail) {
                Some(g) => g,
                None => return false,
            };
            // "::" 至少压缩掉 1 组
            head_groups.len() + tail_groups.len() < 8
        }
    }
}

/// loading 且无数据时的骨架卡（简化为 muted 背景卡 + loading_row）。
fn skeleton_card(cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.))
        .h(px(200.))
        .items_center()
        .justify_center()
        .rounded(theme.radius)
        .bg(theme.colors.muted.opacity(0.4))
        .child(ui::loading_row("正在查询 IP 信息...", cx))
}

impl Render for IpInfo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let on_query = cx.listener(|this, _ev: &ClickEvent, window, cx| {
            this.query(window, cx);
        });
        let on_toggle_raw = cx.listener(|this, _ev: &ClickEvent, _window, cx| {
            this.show_raw = !this.show_raw;
            cx.notify();
        });

        let query_label = if self.loading { "查询中..." } else { "查询" };
        let raw_label = if self.show_raw { "隐藏原始" } else { "显示原始 JSON" };
        let error = self.error.clone();
        let infos = self.infos.clone();
        let raw_json = self.raw_json.clone();
        let loading = self.loading;
        let show_raw = self.show_raw;

        ui::tool_page("ipinfo")
            .child(ui::themed_header(
                "IP 地址信息查询",
                "查询IP地址的地理位置、运营商和相关信息",
                cx,
            ))
            .child(
                div().flex().flex_row().items_center().gap_2().w_full()
                    .child(
                        div().flex().flex_1().min_w(px(0.))
                            .child(Input::new(&self.ip_input)),
                    )
                    .child(
                        ui::primary_button("ipinfo-query", query_label, on_query)
                            .disabled(loading)
                            .loading(loading),
                    )
                    .child(ui::secondary_button("ipinfo-toggle-raw", raw_label, on_toggle_raw)),
            )
            .children(error.map(|e| ui::error_banner(&e, cx)))
            .child(
                div().flex().flex_row().gap_3()
                    .when(loading && infos.is_empty(), |row| {
                        row.child(skeleton_card(cx)).child(skeleton_card(cx))
                    })
                    .children(
                        infos.iter().enumerate().map(|(idx, info)| self.source_card(idx, info, cx)),
                    ),
            )
            .when(show_raw && !raw_json.is_empty(), |page| {
                page.child(
                    div().flex().flex_col().gap_2()
                        .child(
                            div().flex().items_center().justify_between()
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.colors.foreground)
                                        .child("原始响应"),
                                )
                                .child(ui::copy_button("ipinfo-copy-json", raw_json.clone())),
                        )
                        .child(ui::mono_block("mono-ip-raw", raw_json.clone(), cx).max_h(px(300.))),
                )
            })
    }
}
