//! 在线 SSL 检测。
//!
//! 两态视图：输入页（域名输入 + https:// 前缀装饰 + 最近记录 chips）→
//! 结果页（概览 / 证书信息 / 证书链 / 加密套件 / CVE 漏洞 / 安全评估 6 Tab，
//! ui::tab_bar 分段控件切换）。检测走业务层 `check_ssl_info`，
//! 经 core_rt tokio 桥异步执行、`update_in` 回填。
//!
//! 最近记录保存为会话内存（不落盘）；
//! 参考资料链接仅展示为文本。

use devtools::tools::ssl_checker::{CertificateChainNode, SecurityVulnerability, SslInfo};
use futures::StreamExt as _;
use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Div, Entity, FontWeight,
    Hsla, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    input::{Input, InputEvent, InputState},
    ActiveTheme as _, Disableable as _, Sizable as _,
};

use crate::components::{notify, ui};

// ---------------------------------------------------------------------------
// 语义色（主题色优先；橙/紫为主题缺失色，沿用历史视觉的固定 Hsla）
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Success,
    Warning,
    Danger,
    Primary,
    Info,
    Orange,
    Purple,
    Foreground,
    Muted,
}

fn tone_color(tone: Tone, cx: &App) -> Hsla {
    let c = &cx.theme().colors;
    match tone {
        Tone::Success => c.success,
        Tone::Warning => c.warning,
        Tone::Danger => c.danger,
        Tone::Primary => c.primary,
        Tone::Info => c.info,
        // 橙 #f97316
        Tone::Orange => gpui::hsla(0.066, 0.95, 0.53, 1.0),
        // 紫 #9333ea
        Tone::Purple => gpui::hsla(0.753, 0.83, 0.56, 1.0),
        Tone::Foreground => c.foreground,
        Tone::Muted => c.muted_foreground,
    }
}

// ---------------------------------------------------------------------------
// Tab 与严重等级过滤
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResultTab {
    Overview,
    Certificate,
    Chain,
    Cipher,
    Cve,
    Security,
}

impl ResultTab {
    const ALL: [ResultTab; 6] = [
        ResultTab::Overview,
        ResultTab::Certificate,
        ResultTab::Chain,
        ResultTab::Cipher,
        ResultTab::Cve,
        ResultTab::Security,
    ];

    fn label(self) -> &'static str {
        match self {
            ResultTab::Overview => "概览",
            ResultTab::Certificate => "证书信息",
            ResultTab::Chain => "证书链",
            ResultTab::Cipher => "加密套件",
            ResultTab::Cve => "CVE 漏洞",
            ResultTab::Security => "安全评估",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SeverityFilter {
    All,
    Critical,
    High,
    Medium,
    Low,
}

impl SeverityFilter {
    const ALL: [SeverityFilter; 5] = [
        SeverityFilter::All,
        SeverityFilter::Critical,
        SeverityFilter::High,
        SeverityFilter::Medium,
        SeverityFilter::Low,
    ];

    fn label(self) -> &'static str {
        match self {
            SeverityFilter::All => "全部",
            SeverityFilter::Critical => "严重",
            SeverityFilter::High => "高危",
            SeverityFilter::Medium => "中危",
            SeverityFilter::Low => "低危",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            SeverityFilter::All => "all",
            SeverityFilter::Critical => "critical",
            SeverityFilter::High => "high",
            SeverityFilter::Medium => "medium",
            SeverityFilter::Low => "low",
        }
    }

    fn tone(self) -> Tone {
        match self {
            SeverityFilter::All => Tone::Foreground,
            SeverityFilter::Critical => Tone::Danger,
            SeverityFilter::High => Tone::Orange,
            SeverityFilter::Medium => Tone::Warning,
            SeverityFilter::Low => Tone::Primary,
        }
    }

    fn matches(self, severity: &str) -> bool {
        match self {
            SeverityFilter::All => true,
            SeverityFilter::Critical => severity == "CRITICAL",
            SeverityFilter::High => severity == "HIGH",
            SeverityFilter::Medium => severity == "MEDIUM",
            SeverityFilter::Low => severity == "LOW",
        }
    }
}

// ---------------------------------------------------------------------------
// 主视图
// ---------------------------------------------------------------------------

pub struct SslChecker {
    domain_input: Entity<InputState>,
    loading: bool,
    error: Option<String>,
    info: Option<SslInfo>,
    show_results: bool,
    active_tab: ResultTab,
    severity_filter: SeverityFilter,
    /// 最近检测域名（会话内存，最多 10 个小写域名）
    recent_domains: Vec<String>,
}

impl SslChecker {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let domain_input = ui::new_input(window, cx);
        // Enter 触发检测
        cx.subscribe_in(
            &domain_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.start_check(window, cx);
                }
            },
        )
        .detach();
        Self {
            domain_input,
            loading: false,
            error: None,
            info: None,
            show_results: false,
            active_tab: ResultTab::Overview,
            severity_filter: SeverityFilter::All,
            recent_domains: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // 动作
    // -----------------------------------------------------------------------

    /// 从输入框读取域名并开始检测。
    fn start_check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let domain = self.domain_input.read(cx).value().trim().to_string();
        if domain.is_empty() {
            self.error = Some("请输入域名".to_string());
            cx.notify();
            return;
        }
        if !is_valid_domain(&domain) {
            self.error = Some("请输入有效的域名格式".to_string());
            cx.notify();
            return;
        }

        self.loading = true;
        self.error = None;
        self.info = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            devtools::tools::ssl_checker::check_ssl_info(domain).await
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, window, cx| this.on_result(result, window, cx));
            }
        })
        .detach();
    }

    /// 点击最近记录 chip：填回输入框并立即检测。
    fn check_domain(&mut self, domain: String, window: &mut Window, cx: &mut Context<Self>) {
        self.domain_input
            .update(cx, |state, cx| state.set_value(domain.as_str(), window, cx));
        self.start_check(window, cx);
    }

    /// 检测结果回填（core_rt 异步完成）。
    fn on_result(
        &mut self,
        result: Result<SslInfo, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(info) => {
                self.push_recent(info.domain.clone());
                self.severity_filter = SeverityFilter::All;
                self.active_tab = ResultTab::Overview;
                self.info = Some(info);
                self.show_results = true;
                notify::toast_success(window, cx, "SSL 检测完成");
            }
            Err(e) => {
                // 照规格：错误红条「检测失败: <msg>」（留在输入页）
                self.error = Some(format!("检测失败: {e}"));
            }
        }
        cx.notify();
    }

    fn push_recent(&mut self, domain: String) {
        let d = domain.trim().to_lowercase();
        if d.is_empty() {
            return;
        }
        self.recent_domains.retain(|x| *x != d);
        self.recent_domains.insert(0, d);
        self.recent_domains.truncate(10);
    }

    fn remove_recent(&mut self, domain: &str, cx: &mut Context<Self>) {
        self.recent_domains.retain(|x| x != domain);
        cx.notify();
    }

    fn clear_recent(&mut self, cx: &mut Context<Self>) {
        self.recent_domains.clear();
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 渲染
    // -----------------------------------------------------------------------

    fn render_input(&mut self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let theme = cx.theme().clone();

        let check_button = Button::new("ssl-check-btn")
            .label(if self.loading {
                "检测中..."
            } else {
                "开始检测"
            })
            .primary()
            .loading(self.loading)
            .disabled(self.loading)
            .on_click(cx.listener(|this, _ev, window, cx| this.start_check(window, cx)));

        // 「域名: | https:// | [输入框] | [开始检测]」一行
        let input_row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_none()
                    .text_size(px(14.))
                    .text_color(theme.colors.foreground)
                    .child("域名:"),
            )
            .child(
                div()
                    .flex_none()
                    .px_2p5()
                    .py_2()
                    .text_size(px(13.))
                    .text_color(theme.colors.muted_foreground)
                    .bg(theme.colors.muted)
                    .rounded_l(theme.radius)
                    .child("https://"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(200.))
                    .child(Input::new(&self.domain_input)),
            )
            .child(check_button);

        let hint = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::hint_text(
                "检测域名的 SSL 证书信息、支持的协议版本、加密套件和安全配置，包括 HTTP/2 和 HTTP/3 支持性检测。",
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(ui::hint_text("格式示例:", cx))
                    .child(self.example_chip("example.com", cx))
                    .child("、")
                    .child(self.example_chip("www.example.com", cx))
                    .child("、")
                    .child(self.example_chip("api.example.com", cx)),
            );

        let mut input_card = ui::card(cx)
            .child(input_row)
            .child(hint)
            .when_some(self.error.clone(), |card, err| {
                card.child(ui::error_banner(&err, cx))
            })
            .when(self.loading, |card| {
                card.child(ui::loading_row("正在检测，请稍候…", cx))
            });

        // 最近记录（会话内存 chips：点击直接检测、× 删除）
        if !self.recent_domains.is_empty() {
            let header = ui::card_header("最近检测的域名", cx).child(ui::ghost_button(
                "ssl-clear-recent",
                "清除历史",
                cx.listener(|this, _ev, _window, cx| this.clear_recent(cx)),
            ));

            let accent_hover = theme.colors.accent;
            let danger_fg = theme.colors.danger;
            let mut chips = div().flex().flex_wrap().gap_2();
            for (i, d) in self.recent_domains.iter().enumerate() {
                let d_click = d.clone();
                let d_remove = d.clone();
                chips = chips.child(
                    div()
                        .flex()
                        .items_center()
                        .rounded(theme.radius)
                        .bg(theme.colors.muted)
                        .overflow_hidden()
                        .child(
                            div()
                                .id(SharedString::from(format!("ssl-recent-{i}")))
                                .flex()
                                .items_center()
                                .px_2()
                                .py_1()
                                .cursor_pointer()
                                .text_size(px(12.))
                                .text_color(theme.colors.foreground)
                                .hover(move |s| s.bg(accent_hover))
                                .on_click(cx.listener(move |this, _ev, window, cx| {
                                    this.check_domain(d_click.clone(), window, cx);
                                }))
                                .child(d.clone()),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("ssl-recent-del-{i}")))
                                .px_1p5()
                                .py_1()
                                .cursor_pointer()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .hover(move |s| s.text_color(danger_fg))
                                .on_click(cx.listener(move |this, _ev, _window, cx| {
                                    this.remove_recent(&d_remove, cx);
                                }))
                                .child("×"),
                        ),
                );
            }

            input_card = input_card.child(header).child(chips).child(ui::hint_text(
                "💡 点击域名可直接检测，最多保存 10 个最近检测的域名（仅本次会话内有效）",
                cx,
            ));
        }

        ui::tool_page("sslchecker-input").child(input_card)
    }

    fn render_result(&mut self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let theme = cx.theme().clone();
        let domain = self
            .info
            .as_ref()
            .map(|i| i.domain.clone())
            .unwrap_or_default();

        let header = div()
            .flex()
            .items_center()
            .gap_3()
            .child(ui::ghost_button(
                "ssl-back",
                "← 返回",
                cx.listener(|this, _ev, _window, cx| {
                    this.show_results = false;
                    this.active_tab = ResultTab::Overview;
                    cx.notify();
                }),
            ))
            // 结果页标题是动态域名（数据而非页头重复），14px 中量级
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.foreground)
                    .child(domain),
            );

        let content = match self.active_tab {
            ResultTab::Overview => self.render_overview(cx),
            ResultTab::Certificate => self.render_certificate(cx),
            ResultTab::Chain => self.render_chain(cx),
            ResultTab::Cipher => self.render_cipher(cx),
            ResultTab::Cve => self.render_cve(cx),
            ResultTab::Security => self.render_security(cx),
        };

        ui::tool_page("sslchecker-result")
            .child(header)
            .child(self.render_tab_bar(cx))
            .child(content)
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> gpui_component::tab::TabBar {
        let labels: Vec<&str> = ResultTab::ALL.iter().map(|t| t.label()).collect();
        let selected = ResultTab::ALL
            .iter()
            .position(|t| *t == self.active_tab)
            .unwrap_or(0);
        ui::tab_bar(
            "ssl-result-tabs",
            &labels,
            selected,
            cx.listener(move |this, ix: &usize, _window, cx| {
                this.active_tab = ResultTab::ALL[*ix];
                cx.notify();
            }),
        )
    }

    // -----------------------------------------------------------------------
    // Tab 1：概览
    // -----------------------------------------------------------------------

    fn render_overview(&self, cx: &mut Context<Self>) -> Div {
        let info = match self.info.as_ref() {
            Some(info) => info,
            None => return ui::card(cx).child(ui::empty_hint("暂无检测结果", cx)),
        };
        let theme = cx.theme().clone();

        // ---- 左列：基本信息 ----
        let mut left_rows: Vec<Div> = vec![ui::info_row("域名", info.domain.clone(), cx)
            .child(ui::copy_button("ssl-ov-domain", info.domain.clone()))];

        if let Some(ip) = &info.server_ip {
            left_rows.push(
                ui::info_row("服务器 IP", ip.clone(), cx)
                    .child(ui::copy_button("ssl-ov-ip", ip.clone())),
            );
        }
        if let Some(server_info) = &info.server_info {
            left_rows.push(ui::info_row("服务器版本", server_info.clone(), cx));
        }
        if let Some(score) = info.security_score {
            let color = tone_color(score_tone(score), cx);
            left_rows.push(kv_row_value(
                "安全评分",
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(17.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(format!("{score}/100")),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color)
                            .child(format!("（{}）", score_label(score))),
                    ),
                cx,
            ));
        }
        if let Some(rating) = &info.ssl_labs_rating {
            let grade_color = tone_color(grade_tone(&rating.grade), cx);
            let state_text = if rating.has_errors {
                "有错误"
            } else if rating.has_warnings {
                "有警告"
            } else {
                "无问题"
            };
            left_rows.push(kv_row_value(
                "SSL Labs 评级",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(17.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(grade_color)
                            .child(rating.grade.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(format!("{}/100", rating.score)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(state_text),
                            ),
                    ),
                cx,
            ));
        }
        if let Some(vulns) = &info.cve_vulnerabilities {
            if !vulns.is_empty() {
                let tone = if vulns.iter().any(|v| v.severity == "CRITICAL") {
                    Tone::Danger
                } else if vulns.iter().any(|v| v.severity == "HIGH") {
                    Tone::Orange
                } else {
                    Tone::Warning
                };
                left_rows.push(kv_row_value(
                    "安全发现",
                    ui::badge(&format!("{} 项待处理", vulns.len()), tone_color(tone, cx)),
                    cx,
                ));
            }
        }

        let left = ui::card(cx)
            .child(ui::card_header("基本信息", cx))
            .child(ui::info_rows(left_rows, cx));

        // ---- 右列：协议与特性 ----
        let mut right_rows: Vec<Div> = Vec::new();

        if let Some(versions) = &info.ssl_versions {
            if !versions.is_empty() {
                let mut badges = div().flex().flex_wrap().justify_end().gap_1();
                for v in versions {
                    let deprecated = is_deprecated_version(v);
                    let text = if deprecated {
                        format!("{v} ⚠️")
                    } else {
                        v.clone()
                    };
                    badges = badges.child(ui::badge(
                        &text,
                        tone_color(
                            if deprecated {
                                Tone::Danger
                            } else {
                                Tone::Primary
                            },
                            cx,
                        ),
                    ));
                }
                right_rows.push(kv_row_value("支持的 SSL/TLS 版本", badges, cx));
            }
        }

        right_rows.push(support_row("HTTP/2 支持", info.http2_support, cx));
        right_rows.push(support_row("HTTP/3 支持", info.http3_support, cx));
        right_rows.push(support_row("SPDY 支持", info.spdy_support, cx));

        if let Some(alpn) = &info.alpn_protocols {
            if !alpn.is_empty() {
                let mut badges = div().flex().flex_wrap().justify_end().gap_1();
                for p in alpn {
                    badges = badges.child(ui::badge(p, tone_color(Tone::Purple, cx)));
                }
                right_rows.push(kv_row_value("ALPN 协议", badges, cx));
            }
        }

        if let Some(cert) = &info.certificate {
            right_rows.push(ui::info_row("证书有效期", format_date(&cert.valid_to), cx));
            right_rows.push(ui::info_row("证书颁发者", extract_cn(&cert.issuer), cx));
        }

        let right = ui::card(cx)
            .child(ui::card_header("协议与特性", cx))
            .child(ui::info_rows(right_rows, cx));

        div()
            .flex()
            .gap_3()
            .child(left.flex_1())
            .child(right.flex_1())
    }

    // -----------------------------------------------------------------------
    // Tab 2：证书信息
    // -----------------------------------------------------------------------

    fn render_certificate(&self, cx: &mut Context<Self>) -> Div {
        let cert = match self.info.as_ref().and_then(|i| i.certificate.as_ref()) {
            Some(cert) => cert,
            None => {
                return ui::card(cx).child(ui::empty_hint("未获取到证书信息", cx));
            }
        };

        let key_size_text = cert
            .key_size
            .map(|k| format!("{k} 位"))
            .unwrap_or_else(|| "-".to_string());

        // 前 5 项可复制：主题 / 颁发者 / 有效期开始 / 有效期结束 / 序列号
        let rows = vec![
            kv_row("主题", cert.subject.clone(), true, cx),
            kv_row("颁发者", cert.issuer.clone(), true, cx),
            kv_row("有效期开始", format_date(&cert.valid_from), true, cx),
            kv_row("有效期结束", format_date(&cert.valid_to), true, cx),
            kv_row("序列号", cert.serial_number.clone(), true, cx),
            kv_row("指纹", cert.fingerprint.clone(), false, cx),
            kv_row("签名算法", cert.signature_algorithm.clone(), false, cx),
            kv_row("公钥算法", cert.public_key_algorithm.clone(), false, cx),
            kv_row("密钥长度", key_size_text, false, cx),
        ];
        let mut card = ui::card(cx)
            .child(ui::card_header("证书信息", cx))
            .child(ui::info_rows(rows, cx));

        if let Some(san) = &cert.san_domains {
            if !san.is_empty() {
                let mut badges = div().flex().flex_wrap().gap_2().py_1();
                for d in san {
                    badges = badges.child(ui::badge(d, tone_color(Tone::Info, cx)));
                }
                card = card.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui::hint_text("SAN 域名", cx))
                        .child(badges),
                );
            }
        }

        div().child(card)
    }

    // -----------------------------------------------------------------------
    // Tab 3：证书链
    // -----------------------------------------------------------------------

    fn render_chain(&self, cx: &mut Context<Self>) -> Div {
        let info = match self.info.as_ref() {
            Some(info) => info,
            None => return ui::card(cx).child(ui::empty_hint("未获取到证书链信息", cx)),
        };
        let chain = match info.certificate_chain.as_ref() {
            Some(chain) => chain,
            None => {
                return ui::card(cx)
                    .child(ui::empty_hint("未获取到证书链信息", cx))
                    .child(ui::hint_text("请先运行 SSL 检测以获取完整的证书链信息", cx));
            }
        };

        // ---- 状态卡 ----
        let status_tone = match chain.chain_validation_status.as_str() {
            "valid" => Tone::Success,
            "valid-with-warnings" => Tone::Warning,
            _ => Tone::Danger,
        };
        let status_text = match chain.chain_validation_status.as_str() {
            "valid" => "有效",
            "valid-with-warnings" => "有效(有警告)",
            _ => "无效",
        };
        let status_rows = vec![
            ui::info_row("证书链长度", chain.chain_length.to_string(), cx),
            kv_row_value(
                "链锚定",
                ui::badge(
                    if chain.is_complete {
                        "完整"
                    } else {
                        "不完整"
                    },
                    tone_color(
                        if chain.is_complete {
                            Tone::Success
                        } else {
                            Tone::Danger
                        },
                        cx,
                    ),
                ),
                cx,
            ),
            ui::info_row(
                "根CA",
                chain
                    .root_ca_info
                    .clone()
                    .unwrap_or_else(|| "未知".to_string()),
                cx,
            ),
        ];
        let mut status_card = ui::card(cx)
            .child(
                ui::card_header("证书链状态", cx)
                    .child(ui::badge(status_text, tone_color(status_tone, cx))),
            )
            .child(ui::info_rows(status_rows, cx));

        if !chain.chain_errors.is_empty() {
            let mut errors = div().flex().flex_col().gap_0p5();
            for e in &chain.chain_errors {
                errors = errors.child(div().text_size(px(12.)).child(format!("• {e}")));
            }
            status_card = status_card.child(
                ui::rich_banner("链错误", tone_color(Tone::Danger, cx), cx)
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(errors),
            );
        }

        // ---- 链关系图（垂直节点 + ↓ 箭头）----
        let mut diagram = div().flex().flex_col().gap_2();
        let last = chain.certificates.len().saturating_sub(1);
        for (i, node) in chain.certificates.iter().enumerate() {
            diagram = diagram.child(self.chain_node(i, node, cx));
            if i != last {
                diagram = diagram.child(
                    div()
                        .flex()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(cx.theme().colors.muted_foreground)
                        .child("↓"),
                );
            }
        }

        let mut legend = div().flex().flex_wrap().gap_3().py_1();
        for (label, tone) in [
            ("叶子证书 (服务器)", Tone::Primary),
            ("中间证书", Tone::Warning),
            ("根证书", Tone::Success),
        ] {
            legend = legend.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().size(px(10.)).rounded_full().bg(tone_color(tone, cx)))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(cx.theme().colors.muted_foreground)
                            .child(label),
                    ),
            );
        }

        let diagram_card = ui::card(cx)
            .child(ui::card_header("证书链关系图", cx))
            .child(diagram)
            .child(legend);

        // ---- 分析总结 ----
        let anchor_line = chain.trust_anchor_info.as_ref().map(|anchor| {
            let detail = if chain.root_in_chain {
                "服务器已发送"
            } else {
                "由客户端信任库提供，属标准配置"
            };
            div()
                .flex()
                .gap_1()
                .text_size(px(12.))
                .child(format!("✓ 受信任根 CA：{anchor}（{detail}）"))
        });

        let summary_card = ui::card(cx)
            .child(ui::card_header("证书链分析总结", cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .text_size(px(12.))
                    .text_color(cx.theme().colors.muted_foreground)
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_color(if chain.is_complete {
                                tone_color(Tone::Success, cx)
                            } else {
                                tone_color(Tone::Danger, cx)
                            })
                            .child(format!(
                                "{} 证书链{}，共包含 {} 个证书",
                                if chain.is_complete { "✓" } else { "✗" },
                                if chain.is_complete {
                                    "可验证至受信任的根 CA"
                                } else {
                                    "无法验证到受信任的根 CA"
                                },
                                chain.chain_length
                            )),
                    )
                    .when_some(anchor_line, |col, line| col.child(line))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_color(match chain.chain_validation_status.as_str() {
                                "valid" => tone_color(Tone::Success, cx),
                                "valid-with-warnings" => tone_color(Tone::Warning, cx),
                                _ => tone_color(Tone::Danger, cx),
                            })
                            .child(format!(
                                "证书链验证状态: {}",
                                match chain.chain_validation_status.as_str() {
                                    "valid" => "有效",
                                    "valid-with-warnings" => "有效但有警告",
                                    _ => "无效",
                                }
                            )),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().colors.muted_foreground)
                            .child("建议定期监控证书链完整性和各证书的有效期"),
                    ),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(status_card)
            .child(diagram_card)
            .child(summary_card)
    }

    /// 链关系图单个节点卡片（主题/颁发者/有效期至/签名算法/密钥/验证错误）。
    fn chain_node(&self, index: usize, node: &CertificateChainNode, cx: &App) -> Div {
        let theme = cx.theme().clone();
        let cert = &node.certificate;

        let node_tone = if node.is_leaf {
            Tone::Primary
        } else if node.is_root {
            Tone::Success
        } else {
            Tone::Warning
        };

        // 类型徽章（叶子/中间/根/交叉签名根）
        let type_text = if node.chain_level == 0 {
            "叶子证书 (服务器)"
        } else if node.chain_level == 2 {
            if node.trust_status == "root-ca" {
                "根证书（交叉签名形式）"
            } else {
                "根证书"
            }
        } else {
            "中间CA证书"
        };

        // 信任状态徽章
        let (trust_text, trust_tone) = match node.trust_status.as_str() {
            "end-entity" => ("服务器证书", Tone::Primary),
            "self-signed" => ("自签名", Tone::Orange),
            "root-ca" => ("根CA", Tone::Success),
            "intermediate" => ("中间CA", Tone::Purple),
            other => (other, Tone::Muted),
        };

        let mut col = div().flex().flex_col().gap_1p5();

        // 类型 + 信任状态徽章
        col = col.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(ui::badge(type_text, tone_color(node_tone, cx)))
                .child(ui::badge(trust_text, tone_color(trust_tone, cx))),
        );

        // 主题（显示 CN，复制完整 DN）
        col = col.child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child("主题"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(extract_cn(&cert.subject)),
                ),
        );

        if node.trust_status != "self-signed" {
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.colors.muted_foreground)
                            .child("颁发者"),
                    )
                    .child(div().text_size(px(13.)).child(extract_cn(&cert.issuer))),
            );
        }

        let mut details = div().flex().flex_wrap().gap_3();
        details = details.child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child("有效期至"),
                )
                .child(div().text_size(px(12.)).child(format_date(&cert.valid_to))),
        );
        details = details.child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child("签名算法"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .child(cert.signature_algorithm.clone()),
                ),
        );
        if let Some(key_size) = cert.key_size {
            details = details.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.colors.muted_foreground)
                            .child("密钥"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .child(format!("{} {} 位", cert.public_key_algorithm, key_size)),
                    ),
            );
        }
        col = col.child(details);

        // 节点验证错误
        if !node.validation_errors.is_empty() {
            let mut errs = div().flex().flex_col().gap_0p5();
            for e in &node.validation_errors {
                errs = errs.child(div().text_size(px(11.)).child(format!("• {e}")));
            }
            col = col.child(
                ui::rich_banner("验证错误", tone_color(Tone::Danger, cx), cx)
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(errs),
            );
        }

        // 主题 / 指纹复制
        col = col.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Clipboard::new(SharedString::from(format!("ssl-chain-{index}-subject")))
                        .value(cert.subject.clone())
                        .label("复制主题")
                        .small(),
                )
                .child(
                    Clipboard::new(SharedString::from(format!("ssl-chain-{index}-fp")))
                        .value(cert.fingerprint.clone())
                        .label("复制指纹")
                        .small(),
                ),
        );

        ui::panel(cx).child(col)
    }

    // -----------------------------------------------------------------------
    // Tab 4：加密套件
    // -----------------------------------------------------------------------

    fn render_cipher(&self, cx: &mut Context<Self>) -> Div {
        let info = match self.info.as_ref() {
            Some(info) => info,
            None => return ui::card(cx).child(ui::empty_hint("未获取到加密套件信息", cx)),
        };
        let protocols = match info.protocol_support.as_ref() {
            Some(protocols) => protocols,
            None => {
                return ui::card(cx).child(ui::empty_hint("未获取到加密套件信息", cx));
            }
        };

        let has_groups = protocols
            .iter()
            .any(|p| p.supported && !p.cipher_suites.is_empty());
        if !has_groups {
            return ui::card(cx).child(ui::empty_hint("未获取到加密套件信息", cx));
        }

        let mut page = div().flex().flex_col().gap_3();

        // 服务器加密套件顺序偏好
        if let Some(server_order) = info.server_cipher_order {
            let tone = if server_order {
                Tone::Orange
            } else {
                Tone::Success
            };
            page = page.child(
                ui::card(cx)
                    .child(
                        ui::card_header("服务器加密套件顺序偏好", cx).child(ui::badge(
                            if server_order {
                                "服务器优先"
                            } else {
                                "客户端优先"
                            },
                            tone_color(tone, cx),
                        )),
                    )
                    .child(ui::hint_text(
                        if server_order {
                            "服务器按照自己的配置选择加密套件，忽略客户端的偏好。"
                        } else {
                            "服务器会尊重客户端提供的加密套件顺序。"
                        },
                        cx,
                    )),
            );
        }

        // 按协议分组
        for (pi, protocol) in protocols
            .iter()
            .filter(|p| p.supported && !p.cipher_suites.is_empty())
            .enumerate()
        {
            let (proto_text, proto_tone) = if protocol.version.contains("1.3") {
                ("推荐", Tone::Success)
            } else if protocol.version.contains("1.2") {
                ("安全", Tone::Primary)
            } else if is_deprecated_version(&protocol.version) {
                ("过时", Tone::Warning)
            } else {
                ("不安全", Tone::Danger)
            };

            let mut header_badges = div().flex().flex_wrap().items_center().gap_1();
            header_badges = header_badges.child(ui::badge(proto_text, tone_color(proto_tone, cx)));
            if let Some(h2) = protocol.http2_support {
                header_badges = header_badges.child(ui::badge(
                    &format!("HTTP/2: {}", if h2 { "✓" } else { "✗" }),
                    tone_color(if h2 { Tone::Success } else { Tone::Muted }, cx),
                ));
            }
            if let Some(h3) = protocol.http3_support {
                header_badges = header_badges.child(ui::badge(
                    &format!("HTTP/3: {}", if h3 { "✓" } else { "✗" }),
                    tone_color(if h3 { Tone::Purple } else { Tone::Muted }, cx),
                ));
            }
            if let Some(spdy) = protocol.spdy_support {
                header_badges = header_badges.child(ui::badge(
                    &format!("SPDY: {}", if spdy { "✓" } else { "✗" }),
                    tone_color(if spdy { Tone::Success } else { Tone::Muted }, cx),
                ));
            }

            let mut section = ui::card(cx).child(ui::card_header(
                &format!(
                    "{} 加密套件 ({})",
                    protocol.version,
                    protocol.cipher_suites.len()
                ),
                cx,
            ));

            if let Some(alpn) = &protocol.alpn_protocols {
                if !alpn.is_empty() {
                    let mut alpn_badges = div().flex().flex_wrap().items_center().gap_1();
                    for a in alpn {
                        alpn_badges = alpn_badges.child(ui::badge(a, tone_color(Tone::Purple, cx)));
                    }
                    section = section.child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(ui::hint_text("ALPN:", cx))
                            .child(alpn_badges),
                    );
                }
            }

            for (ci, cipher) in protocol.cipher_suites.iter().enumerate() {
                let (strength_text, strength_tone) = match cipher.strength.as_str() {
                    "HIGH" => ("高强度", Tone::Success),
                    "MEDIUM" => ("中等强度", Tone::Warning),
                    "WEAK" => ("弱加密", Tone::Danger),
                    other => (other, Tone::Muted),
                };
                let mut cipher_meta = div().flex().flex_wrap().items_center().gap_2();
                cipher_meta =
                    cipher_meta.child(ui::badge(strength_text, tone_color(strength_tone, cx)));
                if cipher.server_order {
                    cipher_meta =
                        cipher_meta.child(ui::badge("服务器优先", tone_color(Tone::Purple, cx)));
                }

                section = section.child(
                    ui::panel(cx).child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_family("Menlo")
                                            .child(cipher.name.clone()),
                                    )
                                    .child(cipher_meta),
                            )
                            .child(
                                Clipboard::new(SharedString::from(format!("ssl-cipher-{pi}-{ci}")))
                                    .value(cipher.name.clone())
                                    .label("复制")
                                    .small(),
                            ),
                    ),
                );
            }

            page = page.child(section);
        }

        page
    }

    // -----------------------------------------------------------------------
    // Tab 5：CVE 漏洞
    // -----------------------------------------------------------------------

    fn render_cve(&mut self, cx: &mut Context<Self>) -> Div {
        let vulns = match self
            .info
            .as_ref()
            .and_then(|i| i.cve_vulnerabilities.as_ref())
        {
            Some(vulns) => vulns,
            None => {
                return ui::card(cx)
                    .child(ui::empty_hint("未进行漏洞检测", cx))
                    .child(ui::hint_text(
                        "请先运行 SSL 检测以获取完整的安全检测报告",
                        cx,
                    ));
            }
        };
        if vulns.is_empty() {
            return ui::card(cx).child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .py_6()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(tone_color(Tone::Success, cx))
                            .child("✓ 未发现可确认的安全问题"),
                    )
                    .child(ui::hint_text(
                        "基于真实握手探测，未发现协议、加密套件或证书层面的安全问题",
                        cx,
                    )),
            );
        }

        let count = |f: SeverityFilter| vulns.iter().filter(|v| f.matches(&v.severity)).count();

        // 严重等级过滤按钮组
        let muted_bg = cx.theme().colors.muted;
        let muted_fg = cx.theme().colors.muted_foreground;
        let mut filter_bar = ui::toolbar().child(ui::hint_text("严重等级过滤:", cx));
        for f in SeverityFilter::ALL {
            let selected = self.severity_filter == f;
            let tone = f.tone();
            let (bg, fg) = if selected {
                (tone_color(tone, cx).opacity(0.12), tone_color(tone, cx))
            } else {
                (muted_bg, muted_fg)
            };
            filter_bar = filter_bar.child(
                div()
                    .id(SharedString::from(format!("ssl-sev-{}", f.slug())))
                    .px_2p5()
                    .py_1()
                    .rounded_full()
                    .cursor_pointer()
                    .text_size(px(12.))
                    .bg(bg)
                    .text_color(fg)
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.severity_filter = f;
                        cx.notify();
                    }))
                    .child(format!("{} ({})", f.label(), count(f))),
            );
        }

        let mut list = div().flex().flex_col().gap_3();
        for v in vulns
            .iter()
            .filter(|v| self.severity_filter.matches(&v.severity))
        {
            list = list.child(self.vuln_card(v, cx));
        }

        let mut summary_grid = div().flex().flex_wrap().gap_4();
        for (label, f) in [
            ("严重", SeverityFilter::Critical),
            ("高危", SeverityFilter::High),
            ("中危", SeverityFilter::Medium),
        ] {
            summary_grid = summary_grid.child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(tone_color(f.tone(), cx))
                    .child(format!("{}: {}", label, count(f))),
            );
        }
        summary_grid = summary_grid.child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(tone_color(Tone::Primary, cx))
                .child(format!("当前显示: {}", count(self.severity_filter))),
        );

        let summary_card = ui::card(cx)
            .child(ui::card_header("检测总结", cx))
            .child(summary_grid)
            .child(ui::hint_text(
                "ℹ️ 本列表仅包含可通过远程握手证据确认的安全发现。依赖服务器软件版本的漏洞（如 Heartbleed/CVE-2014-0160）无法远程可靠判定，不在列表中，请以服务器软件补丁状态为准。",
                cx,
            ));

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::hint_text(
                &format!(
                    "基于真实握手探测确认的安全发现共 {} 项，每项均附观测证据",
                    vulns.len()
                ),
                cx,
            ))
            .child(filter_bar)
            .child(list)
            .child(summary_card)
    }

    /// 单条安全发现卡片（severity 徽章 / category / grade_cap / 证据 / 建议 / 参考）。
    fn vuln_card(&self, v: &SecurityVulnerability, cx: &App) -> Div {
        let theme = cx.theme().clone();
        let sev_tone = severity_tone(&v.severity);

        let mut header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(ui::badge(&v.severity, tone_color(sev_tone, cx)))
            .child(ui::badge(
                &category_label(&v.category),
                tone_color(Tone::Purple, cx),
            ))
            .when(!v.grade_cap.is_empty(), |h| {
                h.child(ui::badge(
                    &format!("等级封顶 {}", v.grade_cap),
                    tone_color(Tone::Foreground, cx),
                ))
            });

        let mut title_col = div().flex().flex_col().gap_0p5();
        title_col = title_col.child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(v.name.clone()),
        );
        if !v.cve_ids.is_empty() {
            title_col = title_col.child(
                div()
                    .text_size(px(11.))
                    .font_family("Menlo")
                    .text_color(theme.colors.muted_foreground)
                    .child(v.cve_ids.join(", ")),
            );
        }
        if !v.affected_components.is_empty() {
            title_col = title_col.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.colors.muted_foreground)
                    .child(format!("影响组件: {}", v.affected_components.join(", "))),
            );
        }
        header = header.child(title_col);

        let mut card = ui::panel(cx)
            .child(header)
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.colors.foreground)
                    .child(v.description.clone()),
            )
            .child(ui::warning_banner(
                &format!("🔍 观测证据: {}", v.evidence),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("修复建议"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(v.remediation.clone()),
                    ),
            );

        if !v.references.is_empty() {
            let mut refs = div().flex().flex_col().gap_0p5();
            for r in &v.references {
                // 链接仅展示为文本
                refs = refs.child(
                    div()
                        .text_size(px(11.))
                        .font_family("Menlo")
                        .text_color(theme.colors.link)
                        .child(r.clone()),
                );
            }
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("参考资料"),
                    )
                    .child(refs),
            );
        }

        card
    }

    // -----------------------------------------------------------------------
    // Tab 6：安全评估
    // -----------------------------------------------------------------------

    fn render_security(&self, cx: &mut Context<Self>) -> Div {
        let info = match self.info.as_ref() {
            Some(info) => info,
            None => return ui::card(cx).child(ui::empty_hint("暂无安全评估信息", cx)),
        };

        let mut page = div().flex().flex_col().gap_3();

        // 总分 + 四类分项（证书/协议/密钥交换/套件强度）+ 加权说明
        if let Some(score) = info.security_score {
            let grade_label = match &info.ssl_labs_rating {
                Some(rating) => format!("等级 {} · {}", rating.grade, score_label(score)),
                None => score_label(score).to_string(),
            };
            let mut total = div().flex().flex_col().gap_2().py_2().child(ui::stat_tile(
                &format!("{score}/100"),
                &grade_label,
                cx,
            ));

            if let Some(rating) = &info.ssl_labs_rating {
                let mut cells = div().flex().flex_wrap().gap_2();
                for (label, value) in [
                    ("证书", rating.certificate_score),
                    ("协议", rating.protocol_score),
                    ("密钥交换", rating.key_exchange_score),
                    ("套件强度", rating.cipher_strength_score),
                ] {
                    cells = cells.child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .px_3()
                            .py_2()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().colors.muted)
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(cx.theme().colors.muted_foreground)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(tone_color(score_tone(value), cx))
                                    .child(value.to_string()),
                            ),
                    );
                }
                total = total.child(cells);
                total = total.child(ui::hint_text(
                    "加权权重：证书 30% / 协议 30% / 密钥交换 30% / 套件强度 10%；总分取各类别加权分与封顶上限中的较小值，每个问题只封顶一次。",
                    cx,
                ));
            }

            page = page.child(ui::card(cx).child(total));
        }

        // 等级封顶原因（applied_caps：finding → cap + reason）
        if let Some(rating) = &info.ssl_labs_rating {
            if !rating.applied_caps.is_empty() {
                let mut caps = div().flex().flex_col().gap_2();
                for cap in &rating.applied_caps {
                    caps = caps.child(
                        ui::rich_banner(
                            &format!("{} → 等级上限 {}", cap.finding, cap.cap),
                            tone_color(Tone::Orange, cx),
                            cx,
                        )
                        .flex_col()
                        .items_start()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(cx.theme().colors.muted_foreground)
                                .child(format!("证据：{}", cap.reason)),
                        ),
                    );
                }
                page = page.child(
                    ui::card(cx)
                        .child(ui::card_header(
                            &format!("等级封顶原因 ({})", rating.applied_caps.len()),
                            cx,
                        ))
                        .child(caps),
                );
            }
        }

        // 发现的安全问题
        if let Some(vulns) = &info.vulnerabilities {
            if !vulns.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for v in vulns {
                    items = items.child(ui::error_banner(v, cx));
                }
                page = page.child(
                    ui::card(cx)
                        .child(ui::card_header(
                            &format!("发现的安全问题 ({})", vulns.len()),
                            cx,
                        ))
                        .child(items),
                );
            }
        }

        // 安全配置建议
        if let Some(recs) = &info.recommendations {
            if !recs.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for r in recs {
                    items = items.child(ui::rich_banner(r, tone_color(Tone::Primary, cx), cx));
                }
                page = page.child(
                    ui::card(cx)
                        .child(ui::card_header(
                            &format!("安全配置建议 ({})", recs.len()),
                            cx,
                        ))
                        .child(items),
                );
            }
        }

        let has_content = info.security_score.is_some()
            || info.ssl_labs_rating.is_some()
            || info.vulnerabilities.as_ref().is_some_and(|v| !v.is_empty())
            || info.recommendations.as_ref().is_some_and(|r| !r.is_empty());
        if !has_content {
            return ui::card(cx).child(ui::empty_hint("暂无安全评估信息", cx));
        }

        page
    }

    // -----------------------------------------------------------------------
    // 小部件
    // -----------------------------------------------------------------------

    fn example_chip(&self, text: &str, cx: &App) -> Div {
        ui::badge(text, cx.theme().colors.muted_foreground)
    }
}

impl Render for SslChecker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.show_results && self.info.is_some() {
            self.render_result(cx)
        } else {
            self.render_input(cx)
        }
    }
}

// ---------------------------------------------------------------------------
// 渲染辅助（模块级）
// ---------------------------------------------------------------------------

/// 键值行：label 左、等宽值右侧（可选复制按钮）。
fn kv_row(label: &str, value: String, copy: bool, cx: &App) -> Div {
    let mut row = ui::info_row(label, value.clone(), cx);
    if copy {
        row = row.child(
            Clipboard::new(SharedString::from(format!("ssl-kv-{label}")))
                .value(value)
                .label("复制")
                .small(),
        );
    }
    row
}

/// 键值行：label 左、任意元素右（值区 flex_1，徽章组自带对齐）。
fn kv_row_value(label: &str, value: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_start()
        .gap_3()
        .py_1()
        .child(
            div()
                .min_w(px(120.))
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child(label.to_string()),
        )
        .child(div().flex_1().min_w(px(0.)).child(value))
}

/// 支持/不支持 徽章行（None 显示 -）。
fn support_row(label: &str, supported: Option<bool>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    let value = match supported {
        Some(ok) => ui::badge(
            if ok { "支持" } else { "不支持" },
            tone_color(if ok { Tone::Success } else { Tone::Danger }, cx),
        )
        .into_any_element(),
        None => div()
            .text_size(px(12.))
            .text_color(theme.colors.muted_foreground)
            .child("-")
            .into_any_element(),
    };
    div()
        .flex()
        .items_start()
        .gap_3()
        .py_1()
        .child(
            div()
                .min_w(px(120.))
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child(label.to_string()),
        )
        .child(div().flex_1().child(value))
}

// ---------------------------------------------------------------------------
// 数据辅助
// ---------------------------------------------------------------------------

/// 域名校验（多段、每段字母数字与中划线、不以 - 开头/结尾、总长 ≤ 253）。
fn is_valid_domain(domain: &str) -> bool {
    let d = domain.trim().to_lowercase();
    if d.is_empty() || d.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = d.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let bytes = label.as_bytes();
        if !bytes[0].is_ascii_alphanumeric() || !bytes[label.len() - 1].is_ascii_alphanumeric() {
            return false;
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return false;
        }
    }
    true
}

/// TLS 1.0 / 1.1（含变体写法）视为已废弃。
fn is_deprecated_version(version: &str) -> bool {
    version.contains("1.0") || version.contains("1.1")
}

/// RFC3339 → 「YYYY-MM-DD HH:MM:SS +08:00」（无小数秒/解析不出时原样返回）。
fn format_date(raw: &str) -> String {
    let s = raw.replace('T', " ");
    match s.find('.') {
        Some(dot) => {
            let head = &s[..dot];
            let tz = s[dot..]
                .trim_start_matches('.')
                .trim_start_matches(|c: char| c.is_ascii_digit());
            if tz.is_empty() {
                head.to_string()
            } else {
                format!("{head}{tz}")
            }
        }
        None => s,
    }
}

/// 从 DN 中提取 CN；无 CN 时返回完整 DN。
fn extract_cn(dn: &str) -> String {
    for part in dn.split(',') {
        let t = part.trim();
        if t.to_lowercase().starts_with("cn=") {
            return t[3..].to_string();
        }
    }
    dn.to_string()
}

fn score_label(score: u32) -> &'static str {
    if score >= 90 {
        "优秀"
    } else if score >= 70 {
        "良好"
    } else if score >= 50 {
        "一般"
    } else {
        "较差"
    }
}

fn score_tone(score: u32) -> Tone {
    if score >= 90 {
        Tone::Success
    } else if score >= 70 {
        Tone::Warning
    } else if score >= 50 {
        Tone::Orange
    } else {
        Tone::Danger
    }
}

fn grade_tone(grade: &str) -> Tone {
    match grade {
        "A+" | "A" | "A-" => Tone::Success,
        "B" => Tone::Warning,
        "C" => Tone::Orange,
        "D" | "F" => Tone::Danger,
        _ => Tone::Muted,
    }
}

fn severity_tone(severity: &str) -> Tone {
    match severity {
        "CRITICAL" => Tone::Danger,
        "HIGH" => Tone::Orange,
        "MEDIUM" => Tone::Warning,
        "LOW" => Tone::Primary,
        _ => Tone::Muted,
    }
}

fn category_label(category: &str) -> String {
    match category {
        "protocol" => "协议".to_string(),
        "cipher" => "加密套件".to_string(),
        "certificate" => "证书".to_string(),
        "key-exchange" => "密钥交换".to_string(),
        other => other.to_string(),
    }
}
