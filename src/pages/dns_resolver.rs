//! DNS 解析工具。
//!
//! Tab「正向解析 / 反向解析」（ui::tab_bar 分段控件切换）：
//! - 正向：DNS 服务器多选（挂载调 core `get_dns_servers()`，默认全选）+ 自定义服务器 +
//!   记录类型 select + 域名输入；多服务器结果 Tab 切换（每 Tab 一台服务器：
//!   latencyMs 徽章 + records 表格），单服务器直接显示，附 IP 地理位置信息卡。
//! - 反向：单 IP（`reverse_dns_lookup`）/ 批量（`batch_reverse_dns_lookup`），
//!   结果为 IP + domains 列表卡。
//!   返回字段为 camelCase serde 名（Rust 侧仍是 snake_case 字段）。

use devtools::i18n::t;
use devtools::tools::dns_resolver::{
    batch_reverse_dns_lookup, get_dns_servers, lookup_dns, reverse_dns_lookup,
    BatchReverseDnsResponse, DnsLookupResponse, DnsServerEntry, ReverseDnsResponse,
};
use futures::StreamExt as _;
use gpui::{div, prelude::*, px, App, Context, Entity, IntoElement, Render, SharedString, Window};
use gpui_component::{
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::{select::select_state, select::SimpleSelectDelegate, ui};

/// 记录类型下拉选项（8 项，中文 label 照规格）。
const RECORD_TYPES: [(&str, &str); 8] = [
    ("ALL", "全部记录"),
    ("A", "A 记录 (IPv4)"),
    ("AAAA", "AAAA 记录 (IPv6)"),
    ("MX", "MX 记录 (邮件交换)"),
    ("TXT", "TXT 记录 (文本)"),
    ("CNAME", "CNAME 记录 (别名)"),
    ("NS", "NS 记录 (域名服务器)"),
    ("SOA", "SOA 记录 (权威开始)"),
];

/// 反向批量示例。
const BATCH_EXAMPLE: &str = "8.8.8.8\n114.114.114.114\n1.1.1.1";

/// 记录类型下拉选项（使用处经 i18n 翻译，常量保持中文 label）。
fn record_type_options() -> [(&'static str, &'static str); 8] {
    RECORD_TYPES.map(|(value, label)| (value, t(label)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DnsTab {
    Forward,
    Reverse,
}

pub struct DnsResolver {
    active_tab: DnsTab,

    // ---- 正向解析 ----
    /// 预设 + 自定义 DNS 服务器（挂载后从 core 加载）。
    dns_servers: Vec<DnsServerEntry>,
    /// 选中的服务器地址（默认全选）。
    selected_servers: Vec<String>,
    servers_loaded: bool,
    custom_dns_input: Entity<InputState>,
    record_select: Entity<SelectState<SimpleSelectDelegate>>,
    record_type: String,
    domain_input: Entity<InputState>,
    lookup: Option<DnsLookupResponse>,
    /// 结果 Tab 当前选中的服务器下标（多服务器结果时 Tab 切换查看）。
    active_result_tab: usize,
    forward_error: Option<String>,

    // ---- 反向解析 ----
    reverse_ip_input: Entity<InputState>,
    reverse_result: Option<ReverseDnsResponse>,
    batch_ips: Entity<TextareaState>,
    batch_result: Option<BatchReverseDnsResponse>,
    reverse_error: Option<String>,

    /// 查询进行中（正向/反向共用，按钮 loading 与禁用）。
    loading: bool,
}

impl DnsResolver {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom_dns_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("例如: 9.9.9.9")));
        let record_select = select_state(window, cx, &record_type_options(), 0);
        let domain_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("例如: www.example.com")));
        let reverse_ip_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("例如: 8.8.8.8")));
        let batch_ips = cx
            .new(|cx| TextareaState::new(window, cx).placeholder(t("输入多个 IP 地址，每行一个")));

        let this = Self {
            active_tab: DnsTab::Forward,
            dns_servers: Vec::new(),
            selected_servers: Vec::new(),
            servers_loaded: false,
            custom_dns_input: custom_dns_input.clone(),
            record_select,
            record_type: "ALL".to_string(),
            domain_input: domain_input.clone(),
            lookup: None,
            active_result_tab: 0,
            forward_error: None,
            reverse_ip_input: reverse_ip_input.clone(),
            reverse_result: None,
            batch_ips: batch_ips.clone(),
            batch_result: None,
            reverse_error: None,
            loading: false,
        };

        // 记录类型下拉
        cx.subscribe_in(
            &this.record_select.clone(),
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _window, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.record_type = v.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        // 自定义 DNS 输入：Enter 直接添加
        cx.subscribe_in(
            &custom_dns_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.add_custom_dns(window, cx);
                }
            },
        )
        .detach();

        // 页面创建：加载预设 DNS 服务器并默认全选
        let mut rx = crate::core_rt::submit(cx, async move { get_dns_servers().await });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, _window, cx| this.on_servers_loaded(result, cx));
            }
        })
        .detach();

        this
    }

    fn on_servers_loaded(
        &mut self,
        result: Result<Vec<DnsServerEntry>, String>,
        cx: &mut Context<Self>,
    ) {
        self.servers_loaded = true;
        match result {
            Ok(entries) => {
                self.selected_servers = entries.iter().map(|e| e.server.clone()).collect();
                self.dns_servers = entries;
            }
            Err(err) => {
                // 预设服务器加载失败静默降级为红条提示
                self.forward_error = Some(format!("{}: {err}", t("加载 DNS 服务器失败")));
            }
        }
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 正向解析
    // ------------------------------------------------------------------

    /// 勾选/取消一台 DNS 服务器。
    fn toggle_server(&mut self, server: String, checked: bool, cx: &mut Context<Self>) {
        if checked {
            if !self.selected_servers.contains(&server) {
                self.selected_servers.push(server);
            }
        } else {
            self.selected_servers.retain(|s| s.as_str() != server);
        }
        cx.notify();
    }

    /// 添加自定义 DNS 服务器（去重、加入即选中）。
    fn add_custom_dns(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let server = self.custom_dns_input.read(cx).value().trim().to_string();
        if server.is_empty() {
            return;
        }
        if !self.dns_servers.iter().any(|e| e.server == server) {
            self.dns_servers.push(DnsServerEntry {
                name: server.clone(),
                server: server.clone(),
            });
        }
        if !self.selected_servers.contains(&server) {
            self.selected_servers.push(server);
        }
        self.custom_dns_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// 正向查询。
    fn run_lookup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let domain = self.domain_input.read(cx).value().trim().to_string();
        if domain.is_empty() {
            self.lookup = None;
            self.forward_error = Some(t("请输入域名").to_string());
            cx.notify();
            return;
        }
        if self.selected_servers.is_empty() {
            self.lookup = None;
            self.forward_error = Some(t("请至少选择一个 DNS 服务器").to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.forward_error = None;
        cx.notify();

        let servers = self.selected_servers.clone();
        let record_type = self.record_type.clone();
        let mut rx = crate::core_rt::submit(cx, async move {
            lookup_dns(domain, Some(servers), record_type).await
        });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, _window, cx| this.on_lookup_done(result, cx));
            }
        })
        .detach();
    }

    fn on_lookup_done(
        &mut self,
        result: Result<DnsLookupResponse, String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(response) => {
                self.lookup = Some(response);
                // 新结果回到第一个服务器 Tab
                self.active_result_tab = 0;
            }
            Err(err) => {
                self.lookup = None;
                self.forward_error = Some(format!("{}: {err}", t("查询失败")));
            }
        }
        cx.notify();
    }

    /// 正向示例：www.example.com / 全部记录。
    fn load_forward_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.domain_input.update(cx, |state, cx| {
            state.set_value("www.example.com", window, cx)
        });
        self.record_type = "ALL".to_string();
        self.record_select = select_state(window, cx, &record_type_options(), 0);
        self.observe_record_select(window, cx);
        cx.notify();
    }

    /// 正向清空。
    fn clear_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.domain_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.lookup = None;
        self.forward_error = None;
        cx.notify();
    }

    /// 重建记录类型下拉后重新订阅（下拉实体被替换时调用）。
    fn observe_record_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(
            &self.record_select.clone(),
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _window, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.record_type = v.to_string();
                    cx.notify();
                }
            },
        )
        .detach();
    }

    // ------------------------------------------------------------------
    // 反向解析
    // ------------------------------------------------------------------

    /// 单 IP 反向查询。
    fn run_reverse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ip = self.reverse_ip_input.read(cx).value().trim().to_string();
        if ip.is_empty() {
            self.reverse_result = None;
            self.reverse_error = Some(t("请输入 IP 地址").to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.reverse_error = None;
        cx.notify();

        // 同步阻塞调用放后台线程，避免卡 UI
        let mut rx = crate::core_rt::submit(cx, async move { reverse_dns_lookup(ip) });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, _window, cx| this.on_reverse_done(result, cx));
            }
        })
        .detach();
    }

    fn on_reverse_done(
        &mut self,
        result: Result<ReverseDnsResponse, String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(response) => self.reverse_result = Some(response),
            Err(err) => {
                self.reverse_result = None;
                self.reverse_error = Some(format!("{}: {err}", t("查询失败")));
            }
        }
        cx.notify();
    }

    /// 批量反向查询（每行一个 IP）。
    fn run_batch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.batch_ips.read(cx).value().to_string();
        let ips: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if ips.is_empty() {
            self.batch_result = None;
            self.reverse_error = Some(t("请输入 IP 地址列表").to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.reverse_error = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move { batch_reverse_dns_lookup(ips).await });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, _window, cx| this.on_batch_done(result, cx));
            }
        })
        .detach();
    }

    fn on_batch_done(
        &mut self,
        result: Result<BatchReverseDnsResponse, String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(response) => self.batch_result = Some(response),
            Err(err) => {
                self.batch_result = None;
                self.reverse_error = Some(format!("{}: {err}", t("批量查询失败")));
            }
        }
        cx.notify();
    }

    /// 反向示例：单 IP 8.8.8.8 / 批量三地址。
    fn load_reverse_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reverse_ip_input
            .update(cx, |state, cx| state.set_value("8.8.8.8", window, cx));
        self.batch_ips
            .update(cx, |state, cx| state.set_value(BATCH_EXAMPLE, window, cx));
        cx.notify();
    }

    /// 反向清空（单 IP + 批量）。
    fn clear_reverse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reverse_ip_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.batch_ips
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.reverse_result = None;
        self.batch_result = None;
        self.reverse_error = None;
        cx.notify();
    }

    /// 服务器展示名：`name (server)`（name 与 server 相同时仅显示地址）。
    fn server_label(&self, server: &str) -> String {
        match self.dns_servers.iter().find(|e| e.server == server) {
            Some(entry) if entry.name != entry.server => {
                format!("{} ({})", entry.name, entry.server)
            }
            _ => server.to_string(),
        }
    }
}

// ----------------------------------------------------------------------
// 渲染
// ----------------------------------------------------------------------

/// 表格单元（固定宽度）。
fn fixed_cell(text: &str, width: f32) -> gpui::Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .text_size(px(13.))
        .child(text.to_string())
}

/// 表格单元（自适应宽度，等宽字体，长值换行）。
fn flex_mono_cell(text: &str) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .text_size(px(13.))
        .font_family("Menlo")
        .child(text.to_string())
}

impl DnsResolver {
    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> gpui_component::tab::TabBar {
        ui::tab_bar(
            "dns-tabs",
            &[t("正向解析"), t("反向解析")],
            match self.active_tab {
                DnsTab::Forward => 0,
                DnsTab::Reverse => 1,
            },
            cx.listener(|this, ix: &usize, _window, cx| {
                this.active_tab = if *ix == 1 {
                    DnsTab::Reverse
                } else {
                    DnsTab::Forward
                };
                cx.notify();
            }),
        )
    }

    /// 正向解析左侧：查询配置。
    fn render_forward_config(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        // DNS 服务器多选
        let mut server_list = div().flex().flex_col().gap_2();
        if self.servers_loaded {
            for entry in &self.dns_servers {
                let server = entry.server.clone();
                let label = self.server_label(&entry.server);
                let checked = self.selected_servers.contains(&entry.server);
                server_list = server_list.child(
                    Checkbox::new(SharedString::from(format!("dns-server-{server}")))
                        .label(label)
                        .checked(checked)
                        .on_click(cx.listener(move |this, checked: &bool, _window, cx| {
                            this.toggle_server(server.clone(), *checked, cx);
                        })),
                );
            }
        } else {
            server_list = server_list.child(ui::loading_row(t("加载预设服务器中..."), cx));
        }

        let config = ui::card(cx)
            .flex_1()
            .min_w_0()
            .child(ui::card_header(t("查询配置"), cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    // DNS 服务器
                    .child(ui::field(
                        t("DNS 服务器（可多选，同时对比查询）"),
                        server_list,
                        cx,
                    ))
                    // 添加自定义 DNS 服务器
                    .child(ui::field(
                        t("添加自定义 DNS 服务器"),
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Input::new(&self.custom_dns_input).flex_1())
                            .child(ui::secondary_button(
                                "dns-add-server",
                                t("添加"),
                                cx.listener(|this, _, window, cx| this.add_custom_dns(window, cx)),
                            )),
                        cx,
                    ))
                    // 记录类型
                    .child(ui::field(t("记录类型"), Select::new(&self.record_select), cx).flex_1())
                    // 域名
                    .child(ui::field(t("域名"), Input::new(&self.domain_input), cx))
                    // 操作按钮
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::primary_button(
                                    "dns-lookup",
                                    if self.loading {
                                        t("查询中...")
                                    } else {
                                        t("查询")
                                    },
                                    cx.listener(|this, _, window, cx| this.run_lookup(window, cx)),
                                )
                                .loading(self.loading)
                                .disabled(self.loading),
                            )
                            .child(ui::secondary_button(
                                "dns-forward-example",
                                t("示例"),
                                cx.listener(|this, _, window, cx| {
                                    this.load_forward_example(window, cx)
                                }),
                            ))
                            .child(ui::secondary_button(
                                "dns-forward-clear",
                                t("清空"),
                                cx.listener(|this, _, window, cx| this.clear_forward(window, cx)),
                            )),
                    ),
            );

        config
    }

    /// records 表格（类型/值/TTL）。
    fn render_records_table(
        &self,
        records: &[devtools::tools::dns_resolver::DnsRecord],
        cx: &Context<Self>,
    ) -> gpui::Div {
        let head = ui::table_head(cx)
            .px_2()
            .gap_2()
            .child(
                ui::table_head_cell(t("类型"), cx)
                    .w(px(56.))
                    .flex_shrink_0(),
            )
            .child(ui::table_head_cell(t("值"), cx).flex_1().min_w_0())
            .child(ui::table_head_cell("TTL", cx).w(px(72.)).flex_shrink_0());

        let mut rows = Vec::new();
        for record in records {
            let ttl = match record.ttl {
                Some(ttl) => ttl.to_string(),
                None => "-".to_string(),
            };
            rows.push(
                ui::table_row()
                    .gap_2()
                    .px_2()
                    .child(fixed_cell(&record.record_type, 56.))
                    .child(flex_mono_cell(&record.value))
                    .child(fixed_cell(&ttl, 72.)),
            );
        }

        div()
            .flex()
            .flex_col()
            .child(head)
            .child(ui::table_rows(rows, cx))
    }

    /// 正向解析右侧：查询结果。
    fn render_forward_result(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let mut pane = ui::card(cx)
            .flex_1()
            .min_w_0()
            .child(ui::card_header(t("查询结果"), cx));

        if let Some(err) = &self.forward_error {
            pane = pane.child(ui::error_banner(err, cx));
        }

        if let Some(response) = &self.lookup {
            pane = pane.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!("{}: {}", t("域名"), response.domain)),
                    )
                    .when_some(response.error.clone(), |el, err| {
                        el.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.colors.danger)
                                .child(err),
                        )
                    })
                    .when(response.error.is_none(), |el| {
                        if response.results.is_empty() {
                            el.child(ui::empty_hint(t("未找到记录"), cx))
                        } else {
                            // 多服务器结果 Tab 化：TabBar 选服务器，下方显示
                            // 该服务器的记录卡（单服务器不渲染 TabBar）
                            let n = response.results.len();
                            let active = self.active_result_tab.min(n - 1);
                            let mut content = div().flex().flex_col().gap_2();
                            if n > 1 {
                                // Tab 标题用服务器地址（name (server) 全名太长，
                                // 多 Tab 放不下；完整名在组卡头部展示）
                                let labels: Vec<String> = response
                                    .results
                                    .iter()
                                    .map(|r| r.dns_server.clone())
                                    .collect();
                                let items: Vec<&str> = labels.iter().map(String::as_str).collect();
                                content = content.child(ui::tab_bar(
                                    "dns-result-tabs",
                                    &items,
                                    active,
                                    cx.listener(|this, ix: &usize, _window, cx| {
                                        this.active_result_tab = *ix;
                                        cx.notify();
                                    }),
                                ));
                            }
                            content = content
                                .child(self.render_server_group(&response.results[active], cx));
                            el.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.colors.foreground)
                                            .child(t("DNS 记录:")),
                                    )
                                    .child(content),
                            )
                        }
                    })
                    .when_some(response.ip_info.clone(), |el, info| {
                        el.child(self.render_ip_info(&info, cx))
                    }),
            );
        } else if self.forward_error.is_none() {
            pane = pane.child(ui::empty_hint(t("输入域名后点击「查询」查看解析结果"), cx));
        }

        pane
    }

    /// 单台服务器分组卡：服务器名 + latencyMs 徽章 + 组错误 + records 表格。
    fn render_server_group(
        &self,
        result: &devtools::tools::dns_resolver::DnsServerLookupResult,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let theme = cx.theme().clone();
        let label = self.server_label(&result.dns_server);

        let mut group = ui::panel(cx).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(label),
                )
                .when_some(result.latency_ms, |el, latency| {
                    el.child(ui::badge(
                        &format!("{latency} ms"),
                        theme.colors.muted_foreground,
                    ))
                }),
        );

        if let Some(err) = &result.error {
            group = group.child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.colors.danger)
                    .child(err.clone()),
            );
        } else if result.records.is_empty() {
            group = group.child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.colors.muted_foreground)
                    .child(t("未找到记录")),
            );
        } else {
            group = group.child(self.render_records_table(&result.records, cx));
        }

        group
    }

    /// IP 地理位置信息卡（2 列，缺失显示 -）。
    fn render_ip_info(
        &self,
        info: &devtools::tools::dns_resolver::IpGeolocationInfo,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let theme = cx.theme().clone();
        let opt = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".to_string());

        let mut grid = div().flex().flex_wrap().gap_x_6().gap_y_2();
        let entries = [
            (t("IP 地址"), info.ip.clone()),
            (t("国家"), opt(&info.country)),
            (t("城市"), opt(&info.city)),
            (t("运营商"), opt(&info.org)),
            (t("地区"), opt(&info.region)),
            (t("时区"), opt(&info.timezone)),
        ];
        for (label, value) in entries {
            grid = grid.child(
                div()
                    .flex()
                    .flex_col()
                    .min_w(px(160.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_family("Menlo")
                            .text_color(theme.colors.foreground)
                            .child(value),
                    ),
            );
        }

        ui::panel(cx)
            .mt_1()
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.foreground)
                    .child(t("IP 地理位置信息")),
            )
            .child(grid)
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.colors.muted_foreground)
                    .child(format!("{}: {}", t("数据来源"), info.source)),
            )
    }

    /// 反向解析左侧：单 IP + 批量配置。
    fn render_reverse_config(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        ui::card(cx)
            .flex_1()
            .min_w_0()
            .child(ui::card_header(t("反向解析配置"), cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    // 单个 IP
                    .child(ui::field(
                        t("单个 IP 反向解析"),
                        Input::new(&self.reverse_ip_input),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::primary_button(
                                    "dns-reverse",
                                    if self.loading {
                                        t("查询中...")
                                    } else {
                                        t("查询")
                                    },
                                    cx.listener(|this, _, window, cx| this.run_reverse(window, cx)),
                                )
                                .loading(self.loading)
                                .disabled(self.loading),
                            )
                            .child(ui::secondary_button(
                                "dns-reverse-example",
                                t("示例"),
                                cx.listener(|this, _, window, cx| {
                                    this.load_reverse_example(window, cx)
                                }),
                            ))
                            .child(ui::secondary_button(
                                "dns-reverse-clear",
                                t("清空"),
                                cx.listener(|this, _, window, cx| this.clear_reverse(window, cx)),
                            )),
                    )
                    // 批量
                    .child(
                        ui::field(
                            t("批量反向解析 (每行一个 IP)"),
                            ui::edit_area(&self.batch_ips, 140.),
                            cx,
                        )
                        .mt_2()
                        .pt_3(),
                    )
                    .child(
                        div().flex().items_center().gap_2().child(
                            ui::primary_button(
                                "dns-batch",
                                if self.loading {
                                    t("查询中...")
                                } else {
                                    t("批量查询")
                                },
                                cx.listener(|this, _, window, cx| this.run_batch(window, cx)),
                            )
                            .loading(self.loading)
                            .disabled(self.loading),
                        ),
                    ),
            )
    }

    /// 反向解析右侧：结果。
    fn render_reverse_result(&self, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let mut pane = ui::card(cx)
            .flex_1()
            .min_w_0()
            .child(ui::card_header(t("查询结果"), cx));

        if let Some(err) = &self.reverse_error {
            pane = pane.child(ui::error_banner(err, cx));
        }

        let mut body = div().flex().flex_col().gap_3();

        // 单个 IP 结果
        if let Some(response) = &self.reverse_result {
            let mut card = ui::panel(cx)
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(t("单个 IP 查询结果")),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!("{}: {}", t("IP 地址"), response.ip)),
                );
            if let Some(err) = &response.error {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.danger)
                        .child(err.clone()),
                );
            } else if response.domains.is_empty() {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.muted_foreground)
                        .child(t("未找到对应的域名记录")),
                );
            } else {
                let mut list = div().flex().flex_col().gap_1();
                for domain in &response.domains {
                    list = list.child(
                        div()
                            .text_size(px(13.))
                            .font_family("Menlo")
                            .text_color(theme.colors.foreground)
                            .child(domain.clone()),
                    );
                }
                card = card.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.colors.foreground)
                                .child(t("对应域名:")),
                        )
                        .child(list),
                );
            }
            body = body.child(card);
        }

        // 批量结果
        if let Some(batch) = &self.batch_result {
            let mut card = ui::panel(cx).child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.foreground)
                    .child(t("批量查询结果")),
            );
            if let Some(err) = &batch.error {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.danger)
                        .child(err.clone()),
                );
            }
            if batch.results.is_empty() {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.muted_foreground)
                        .child(t("未找到查询结果")),
                );
            } else {
                for result in &batch.results {
                    let mut item = ui::panel(cx).child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.foreground)
                            .child(result.ip.clone()),
                    );
                    if let Some(err) = &result.error {
                        item = item.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.danger)
                                .child(err.clone()),
                        );
                    } else if result.domains.is_empty() {
                        item = item.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(t("未找到对应的域名记录")),
                        );
                    } else {
                        for domain in &result.domains {
                            item = item.child(
                                div()
                                    .text_size(px(12.))
                                    .font_family("Menlo")
                                    .text_color(theme.colors.foreground)
                                    .child(domain.clone()),
                            );
                        }
                    }
                    card = card.child(item);
                }
            }
            body = body.child(card);
        }

        if self.reverse_error.is_none()
            && self.reverse_result.is_none()
            && self.batch_result.is_none()
        {
            body = body.child(ui::empty_hint(t("输入 IP 地址后点击「查询」查看结果"), cx));
        }

        pane.child(body)
    }
}

impl Render for DnsResolver {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (left, right) = match self.active_tab {
            DnsTab::Forward => (
                self.render_forward_config(cx),
                self.render_forward_result(cx),
            ),
            DnsTab::Reverse => (
                self.render_reverse_config(cx),
                self.render_reverse_result(cx),
            ),
        };

        ui::tool_page("dnsresolver")
            .child(self.render_tab_bar(cx))
            .child(div().flex().flex_row().gap_2().child(left).child(right))
    }
}
