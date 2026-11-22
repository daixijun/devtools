//! 证书查看器。
//!
//! 输入：按钮选文件（rfd，后台线程读字节）或粘贴 PEM 文本（可切 Base64 解码），
//! PFX 追加密码输入（Enter 触发解析）。解析走业务层 lib（core_rt 桥），
//! 返回 [`CertificateChainInfo`]（snake_case 字段，直接用 core 类型）。
//!
//! 输出："全屏"结果区（解析成功后隐藏输入区，顶部「← 重新解析」返回）：
//! 链状态卡 + 顺序检测卡（错误时黄条 + 当前/正确顺序 + 修正 PEM 复制）+
//! 页内 Tab（ui::tab_bar 分段控件：证书链概览 + 每证书一页）+ 每证书详情 +
//! 颁发关系图 + 图例。

use std::collections::HashMap;

use devtools::tools::certificate_viewer::{
    parse_pem_certificate, parse_pfx_certificate, CertificateChainInfo, CertificateInfo,
    ChainOrderCheck, ChainOrderEntry, MissingCertificateInfo,
};
use futures::StreamExt as _;
use gpui::{
    div, prelude::*, px, App, ClickEvent, Context, Entity, Hsla, IntoElement, Render, Window,
};
use gpui_component::{
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState, TextareaState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::ui;

// ---------------------------------------------------------------------------
// 文案与常量
// ---------------------------------------------------------------------------

fn cert_role_name(chain_level: usize) -> &'static str {
    match chain_level {
        0 => "终端证书",
        1 => "中间CA证书",
        2 => "根CA证书",
        _ => "未知类型",
    }
}

/// 主题/颁发者中文键的稳定展示顺序（HashMap 直接迭代顺序不稳定）
const NAME_KEY_ORDER: [&str; 7] = [
    "通用名称 (CN)",
    "组织名称 (O)",
    "组织单位 (OU)",
    "国家 (C)",
    "省份 (ST)",
    "城市 (L)",
    "邮箱地址",
];

fn ordered_name_entries(map: &HashMap<String, String>) -> Vec<(&String, &String)> {
    let mut entries: Vec<(&String, &String)> = map.iter().collect();
    entries.sort_by(|a, b| {
        let rank = |key: &String| {
            NAME_KEY_ORDER
                .iter()
                .position(|k| *k == key.as_str())
                .unwrap_or(usize::MAX)
        };
        rank(a.0).cmp(&rank(b.0)).then_with(|| a.0.cmp(b.0))
    });
    entries
}

fn common_name(map: &HashMap<String, String>, fallback: &str) -> String {
    map.get("通用名称 (CN)")
        .or_else(|| map.get("组织名称 (O)"))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

/// RFC3339 → 日期部分（YYYY-MM-DD），失败时原样返回
fn short_date(date: &str) -> String {
    let trimmed = date.trim();
    if trimmed.len() >= 10 && trimmed.as_bytes()[4] == b'-' {
        trimmed[..10].to_string()
    } else {
        trimmed.to_string()
    }
}

/// 有效期状态：负→红「已过期 N 天」、<30→黄「还有 N 天到期」、否则绿「有效」
enum ExpiryState {
    Expired(u64),
    Warning(u64),
    Valid,
}

fn expiry_state(days_until_expiry: i64) -> ExpiryState {
    if days_until_expiry < 0 {
        ExpiryState::Expired(days_until_expiry.unsigned_abs())
    } else if days_until_expiry < 30 {
        ExpiryState::Warning(days_until_expiry as u64)
    } else {
        ExpiryState::Valid
    }
}

fn expiry_text(state: &ExpiryState) -> String {
    match state {
        ExpiryState::Expired(days) => format!("已过期 {} 天", days),
        ExpiryState::Warning(days) => format!("还有 {} 天到期", days),
        ExpiryState::Valid => "有效".to_string(),
    }
}

fn expiry_color(state: &ExpiryState, cx: &App) -> Hsla {
    let colors = cx.theme().colors;
    match state {
        ExpiryState::Expired(_) => colors.danger,
        ExpiryState::Warning(_) => colors.warning,
        ExpiryState::Valid => colors.success,
    }
}

/// 极简 Base64 解码（标准字母表 + URL-safe 变体，容忍空白/data: 前缀）。
/// app crate 未依赖 base64 crate 且 Cargo.toml 不可改动，故在此自实现；
/// 仅用于「Base64解码」复选框对粘贴文本的预处理。
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let trimmed = input.trim();
    let trimmed = trimmed
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(';').map(|(_, r)| r))
        .and_then(|rest| rest.split_once(',').map(|(_, r)| r))
        .unwrap_or(trimmed);

    let mut values = Vec::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let value = match ch {
            'A'..='Z' => ch as u8 - b'A',
            'a'..='z' => ch as u8 - b'a' + 26,
            '0'..='9' => ch as u8 - b'0' + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            '=' | '\n' | '\r' | ' ' | '\t' => continue,
            other => return Err(format!("Base64解码失败：存在无效字符 '{}'", other)),
        };
        values.push(value);
    }
    if values.is_empty() {
        return Err("Base64解码失败：内容为空".to_string());
    }

    let mut out = Vec::with_capacity(values.len() / 4 * 3 + 3);
    for chunk in values.chunks(4) {
        let n = (chunk[0] as u32) << 18
            | (chunk.get(1).copied().unwrap_or(0) as u32) << 12
            | (chunk.get(2).copied().unwrap_or(0) as u32) << 6
            | (chunk.get(3).copied().unwrap_or(0) as u32);
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 视图状态
// ---------------------------------------------------------------------------

/// 输入模式：文件 / 粘贴文本
#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
    File,
    Text,
}

pub struct CertificateViewer {
    // 输入模式
    input_mode: InputMode,
    // 文件模式状态
    file_name: Option<String>,
    file_data: Option<Vec<u8>>,
    file_is_pfx: bool,
    picking_file: bool,
    // 文本模式状态
    pem_input: Entity<TextareaState>,
    base64_decode: bool,
    // PFX 密码（密码框，Enter 触发解析）
    pfx_password: Entity<InputState>,
    // 解析状态
    loading: bool,
    error: Option<String>,
    // 结果（"全屏"结果区）
    chain: Option<CertificateChainInfo>,
    result_view: bool,
    selected_tab: usize,
}

impl CertificateViewer {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pem_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("请输入PEM证书内容或上传证书文件…\n\n支持格式：\n- PEM文本格式 (.pem, .crt, .cer)\n- PFX二进制格式 (.pfx, .p12)")
        });
        let pfx_password = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("请输入PFX文件密码，如无密码请留空")
        });

        // 文本输入联动：有文本时清除已上传文件；输入与文件均空时重置全部
        cx.subscribe_in(
            &pem_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_text_state(window, cx);
                }
            },
        )
        .detach();

        // PFX 密码框 Enter 触发解析
        cx.subscribe_in(
            &pfx_password,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) && this.can_parse(cx) {
                    this.parse(cx);
                }
            },
        )
        .detach();

        Self {
            input_mode: InputMode::File,
            file_name: None,
            file_data: None,
            file_is_pfx: false,
            picking_file: false,
            pem_input,
            base64_decode: false,
            pfx_password,
            loading: false,
            error: None,
            chain: None,
            result_view: false,
            selected_tab: 0,
        }
    }

    // -----------------------------------------------------------------------
    // 状态联动
    // -----------------------------------------------------------------------

    /// 文本输入联动：
    /// - 有文本且已上传文件 → 清除文件状态（文本优先）；
    /// - 文本与文件均空 → 重置全部结果。
    fn sync_text_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.pem_input.read(cx).value().to_string();
        if !text.trim().is_empty() && self.file_data.is_some() {
            self.clear_file(window, cx);
        }
        if text.trim().is_empty() && self.file_data.is_none() {
            self.reset_result();
            self.error = None;
        }
        cx.notify();
    }

    /// 清除已上传文件状态（含密码输入内容）
    fn clear_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.file_name = None;
        self.file_data = None;
        self.file_is_pfx = false;
        self.pfx_password.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }

    /// 重置解析结果
    fn reset_result(&mut self) {
        self.chain = None;
        self.result_view = false;
        self.selected_tab = 0;
        self.loading = false;
    }

    /// 当前是否可触发解析：非 loading 且有输入（文件或文本）
    fn can_parse(&self, cx: &App) -> bool {
        if self.loading {
            return false;
        }
        if self.file_data.is_some() {
            return true;
        }
        !self.pem_input.read(cx).value().trim().is_empty()
    }

    // -----------------------------------------------------------------------
    // 文件选择（rfd 后台线程）
    // -----------------------------------------------------------------------

    fn pick_file(&mut self, cx: &mut Context<Self>) {
        if self.picking_file {
            return;
        }
        self.picking_file = true;
        self.error = None;
        cx.notify();

        let rx = crate::core_rt::submit(cx, async move {
            let path = rfd::FileDialog::new()
                .set_title("选择证书文件")
                .add_filter("证书文件", &["pem", "crt", "cer", "pfx", "p12"])
                .pick_file();
            match path {
                Some(path) => match std::fs::read(&path) {
                    Ok(data) => Ok(Some((path.to_string_lossy().to_string(), data))),
                    Err(e) => Err(format!("读取文件失败: {}", e)),
                },
                None => Ok(None),
            }
        });

        cx.spawn(async move |this, cx| {
            let mut rx = rx;
            if let Some(result) = rx.next().await {
                let _ = this.update(cx, |this, cx| this.on_file_picked(result, cx));
            }
        })
        .detach();
    }

    fn on_file_picked(
        &mut self,
        result: Result<Option<(String, Vec<u8>)>, String>,
        cx: &mut Context<Self>,
    ) {
        self.picking_file = false;
        match result {
            Ok(Some((path, data))) => {
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(path);
                self.file_is_pfx = Self::detect_pfx(&name, &data);
                self.file_name = Some(name);
                self.file_data = Some(data);
                self.reset_result();
            }
            Ok(None) => {}
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }

    /// PFX 类型检测：优先扩展名（.pfx/.p12），其次内容（PEM 标记），其余按扩展名未知时视为 PFX
    fn detect_pfx(file_name: &str, data: &[u8]) -> bool {
        let lower = file_name.to_lowercase();
        if lower.ends_with(".pfx") || lower.ends_with(".p12") {
            return true;
        }
        if lower.ends_with(".pem") || lower.ends_with(".crt") || lower.ends_with(".cer") {
            return false;
        }
        // 内容检测：含 PEM 标记按 PEM，否则有文件时假定为 PFX
        let head = String::from_utf8_lossy(&data[..data.len().min(4096)]);
        !head.contains("-----BEGIN CERTIFICATE-----")
    }

    // -----------------------------------------------------------------------
    // 解析
    // -----------------------------------------------------------------------

    fn parse(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.error = None;

        // 组装解析来源：文件（PFX/PEM）或粘贴文本（可 Base64 解码）
        enum Source {
            Pfx(Vec<u8>, Option<String>),
            Pem(String),
        }

        let source = if let Some(data) = self.file_data.clone() {
            if self.file_is_pfx {
                let password = self.pfx_password.read(cx).unmask_value().to_string();
                Source::Pfx(data, (!password.is_empty()).then_some(password))
            } else {
                Source::Pem(String::from_utf8_lossy(&data).into_owned())
            }
        } else {
            let text = self
                .pem_input
                .read(cx)
                .value()
                .to_string()
                .trim()
                .to_string();
            if text.is_empty() {
                return;
            }
            if self.base64_decode {
                match base64_decode(&text) {
                    Ok(bytes) => Source::Pem(String::from_utf8_lossy(&bytes).into_owned()),
                    Err(e) => {
                        self.error = Some(e);
                        cx.notify();
                        return;
                    }
                }
            } else {
                Source::Pem(text)
            }
        };

        self.loading = true;
        cx.notify();

        let rx = crate::core_rt::submit(cx, async move {
            match source {
                Source::Pfx(data, password) => parse_pfx_certificate(data, password),
                Source::Pem(pem) => parse_pem_certificate(pem),
            }
        });

        cx.spawn(async move |this, cx| {
            let mut rx = rx;
            if let Some(result) = rx.next().await {
                let _ = this.update(cx, |this, cx| this.on_parsed(result, cx));
            }
        })
        .detach();
    }

    fn on_parsed(&mut self, result: Result<CertificateChainInfo, String>, cx: &mut Context<Self>) {
        self.loading = false;
        match result {
            Ok(chain) => {
                self.chain = Some(chain);
                self.selected_tab = 0;
                self.result_view = true; // 解析成功自动进入"全屏"结果区
            }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 渲染：输入区
    // -----------------------------------------------------------------------

    fn render_input(&self, cx: &Context<Self>) -> gpui::Div {
        // 输入模式切换（macOS 分段控件）
        let mode_bar = ui::tab_bar(
            "cert-input-mode",
            &["选择文件", "粘贴文本"],
            match self.input_mode {
                InputMode::File => 0,
                InputMode::Text => 1,
            },
            cx.listener(|this, ix: &usize, _, cx| {
                this.input_mode = if *ix == 1 {
                    InputMode::Text
                } else {
                    InputMode::File
                };
                cx.notify();
            }),
        );

        let mut card = ui::card(cx).child(mode_bar);

        if self.input_mode == InputMode::File {
            card = card
                .child(self.render_file_mode(cx))
                .when(self.file_is_pfx && self.file_data.is_some(), |c| {
                    c.child(self.render_pfx_password(cx))
                });
        } else {
            card = card
                .child(ui::field(
                    "证书内容",
                    ui::edit_area(&self.pem_input, 200.),
                    cx,
                ))
                .child(self.render_base64_option(cx));
        }

        let can_parse = self.can_parse(cx);
        let parse_button = ui::primary_button(
            "parse-cert",
            if self.loading {
                "解析中..."
            } else {
                "解析证书"
            },
            cx.listener(|this, _: &ClickEvent, _, cx| this.parse(cx)),
        )
        .loading(self.loading)
        .disabled(!can_parse)
        .w_full();

        let mut page = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(card)
            .child(parse_button);

        if let Some(error) = &self.error {
            page = page.child(ui::error_banner(error, cx));
        }

        // 空态提示
        if self.error.is_none() && self.chain.is_none() && !self.loading && !can_parse {
            page = page.child(
                ui::card(cx)
                    .child(ui::empty_hint(
                        "支持 PEM / PFX 格式证书：选择文件或粘贴 PEM 文本后点击「解析证书」",
                        cx,
                    ))
                    .child(ui::hint_text(
                        "支持多证书：自动检测证书链完整性、顺序与缺失的CA证书",
                        cx,
                    )),
            );
        }

        page
    }

    fn render_file_mode(&self, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();

        let pick_button = ui::secondary_button(
            "pick-cert-file",
            if self.picking_file {
                "选择文件…"
            } else {
                "选择证书文件"
            },
            cx.listener(|this, _: &ClickEvent, _, cx| this.pick_file(cx)),
        )
        .loading(self.picking_file)
        .disabled(self.picking_file);

        let mut container = div().flex().flex_col().gap_2().child(pick_button);

        if let Some(name) = &self.file_name {
            let badge_color = if self.file_is_pfx {
                theme.colors.warning
            } else {
                theme.colors.info
            };
            let file_type = if self.file_is_pfx { "PFX" } else { "PEM" };
            container = container.child(
                ui::rich_banner(&format!("已上传文件： {}", name), theme.colors.info, cx)
                    .child(ui::badge(file_type, badge_color)),
            );
        }

        container
    }

    fn render_pfx_password(&self, cx: &Context<Self>) -> gpui::Div {
        ui::field(
            "PFX文件密码（可选）",
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(Input::new(&self.pfx_password))
                .child(ui::hint_text(
                    "如果PFX文件没有密码保护，请留空此字段；输入后按 Enter 直接解析",
                    cx,
                )),
            cx,
        )
    }

    fn render_base64_option(&self, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Checkbox::new("base64-decode")
                    .label("Base64解码")
                    .checked(self.base64_decode)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.base64_decode = *checked;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.colors.muted_foreground)
                    .child("如果输入的证书内容是Base64编码的，请勾选此选项进行解码后再解析"),
            )
    }

    // -----------------------------------------------------------------------
    // 渲染：结果区（"全屏"模式）
    // -----------------------------------------------------------------------

    fn render_result(&self, cx: &Context<Self>) -> gpui::Div {
        let Some(chain) = self.chain.as_ref() else {
            return div();
        };

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(ui::secondary_button(
                        "back-to-input",
                        "← 重新解析",
                        cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.result_view = false;
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cx.theme().colors.foreground)
                            .child("证书详细信息"),
                    ),
            )
            .child(ui::hint_text("全屏模式", cx));

        let mut page = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(header)
            .child(self.render_chain_status(chain, cx));

        if let Some(order_check) = &chain.order_check {
            page = page.child(self.render_order_check(order_check, cx));
        }

        page = page
            .child(self.render_tabs(chain, cx))
            .child(self.render_tab_content(chain, cx));

        page
    }

    /// 链状态卡：绿/红圆点 + 「完整证书链」/「证书链不完整 - 缺少CA证书」+ chain_status
    fn render_chain_status(&self, chain: &CertificateChainInfo, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let (dot_color, title, title_color) = if chain.is_full_chain {
            (theme.colors.success, "完整证书链", theme.colors.success)
        } else {
            (
                theme.colors.danger,
                "证书链不完整 - 缺少CA证书",
                theme.colors.danger,
            )
        };

        ui::card(cx).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().size(px(12.)).rounded_full().bg(dot_color))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(title_color)
                                .child(title),
                        ),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(title_color)
                        .child(chain.chain_status.clone()),
                ),
        )
    }

    /// 顺序检测卡：正确→绿行；错误→黄条 + message + 当前/正确顺序 + 修正 PEM 复制
    fn render_order_check(&self, check: &ChainOrderCheck, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();

        if check.is_correct {
            return ui::card(cx).child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(13.))
                    .text_color(theme.colors.success)
                    .child(check.message.clone()),
            );
        }

        let corrected_copy = check
            .corrected_pem
            .clone()
            .map(|pem| ui::copy_result_button("copy-corrected-pem", "复制正确顺序的证书", pem));

        ui::card(cx).child(
            ui::rich_banner("证书链顺序错误", theme.colors.warning, cx)
                .flex_col()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.warning)
                        .child(check.message.clone()),
                )
                .child(self.render_order_sequence(
                    "当前顺序",
                    &check.actual_order,
                    theme.colors.danger,
                    cx,
                ))
                .child(self.render_order_sequence(
                    "正确顺序",
                    &check.expected_order,
                    theme.colors.success,
                    cx,
                ))
                .children(corrected_copy),
        )
    }

    /// 顺序序列芯片行（当前顺序=红 / 正确顺序=绿）
    fn render_order_sequence(
        &self,
        label: &str,
        entries: &[ChainOrderEntry],
        color: Hsla,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let muted = cx.theme().colors.muted_foreground;
        let mut row = div().flex().flex_wrap().items_center().gap_1().child(
            div()
                .text_size(px(12.))
                .text_color(muted)
                .child(label.to_string()),
        );

        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                row = row.child(div().text_size(px(12.)).text_color(muted).child("→"));
            }
            let chip_text = if entry.subject_cn.is_empty() {
                format!("{}. {}", index + 1, entry.role)
            } else {
                format!("{}. {}（{}）", index + 1, entry.role, entry.subject_cn)
            };
            row = row.child(ui::badge(&chip_text, color));
        }

        row
    }

    /// Tab 栏（macOS 分段控件）：证书链概览 + 每证书一页。
    fn render_tabs(
        &self,
        chain: &CertificateChainInfo,
        cx: &Context<Self>,
    ) -> gpui_component::tab::TabBar {
        let mut labels: Vec<String> = vec!["证书链概览".to_string()];
        for (index, cert) in chain.certificates.iter().enumerate() {
            labels.push(format!(
                "证书 {} - {}",
                index + 1,
                cert_role_name(cert.chain_level)
            ));
        }
        let items: Vec<&str> = labels.iter().map(String::as_str).collect();
        ui::tab_bar(
            "cert-viewer-tabs",
            &items,
            self.selected_tab,
            cx.listener(|this, ix: &usize, _, cx| {
                this.selected_tab = *ix;
                cx.notify();
            }),
        )
    }

    fn render_tab_content(&self, chain: &CertificateChainInfo, cx: &Context<Self>) -> gpui::Div {
        if self.selected_tab == 0 {
            self.render_overview(chain, cx)
        } else {
            let index = self.selected_tab - 1;
            match chain.certificates.get(index) {
                Some(cert) => self.render_certificate_detail(cert, cx),
                None => div(),
            }
        }
    }

    /// 概览页：颁发关系图（含缺失证书红卡 + 箭头 + 图例）+ 链级信息（下载建议/缺失CA信息）
    fn render_overview(&self, chain: &CertificateChainInfo, cx: &Context<Self>) -> gpui::Div {
        let mut card = ui::card(cx)
            .child(ui::card_header("证书颁发关系图", cx))
            .child(self.render_chain_graph(chain, cx))
            .child(self.render_graph_legend(cx));

        // 链级信息：缺失CA说明
        if let Some(info) = &chain.missing_ca_info {
            card = card.child(ui::warning_banner(info, cx));
        }

        // 链级信息：CA 下载建议（仅链不完整时）
        if !chain.is_full_chain && !chain.ca_download_urls.is_empty() {
            let mut download = div().flex().flex_col().gap_1().child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(cx.theme().colors.foreground)
                    .child("CA证书下载建议"),
            );
            for (index, url) in chain.ca_download_urls.iter().enumerate() {
                download = download.child(ui::info_row(
                    &format!("下载链接 {}", index + 1),
                    url.clone(),
                    cx,
                ));
            }
            card = card.child(download);
        }

        card
    }

    /// 颁发关系图：证书 + 缺失证书按链级别从高到低合并，节点间「颁发给 ↓」
    fn render_chain_graph(&self, chain: &CertificateChainInfo, cx: &Context<Self>) -> gpui::Div {
        // 合并节点并按链级别降序（稳定排序保持核心输出的根在前顺序）
        #[allow(clippy::large_enum_variant)]
        enum ChainNode {
            Cert(CertificateInfo),
            Missing(MissingCertificateInfo),
        }

        let level_of = |node: &ChainNode| match node {
            ChainNode::Cert(cert) => cert.chain_level,
            ChainNode::Missing(missing) => missing.chain_level,
        };

        let mut nodes: Vec<ChainNode> = chain
            .certificates
            .iter()
            .cloned()
            .map(ChainNode::Cert)
            .chain(
                chain
                    .missing_certificates
                    .iter()
                    .cloned()
                    .map(ChainNode::Missing),
            )
            .collect();
        nodes.sort_by_key(|a| std::cmp::Reverse(level_of(a)));

        let mut graph = div().flex().flex_col().items_center().w_full();

        let total = nodes.len();
        for (index, node) in nodes.iter().enumerate() {
            match node {
                ChainNode::Cert(cert) => graph = graph.child(self.render_cert_node(cert, cx)),
                ChainNode::Missing(missing) => {
                    graph = graph.child(self.render_missing_node(missing, cx))
                }
            }

            // 节点间箭头：下一节点缺失时用红色「应该颁发给 ↓」
            if index + 1 < total {
                let (arrow, color) = match &nodes[index + 1] {
                    ChainNode::Missing(_) => ("应该颁发给 ↓", cx.theme().colors.danger),
                    ChainNode::Cert(_) => ("颁发给 ↓", cx.theme().colors.muted_foreground),
                };
                graph = graph.child(
                    div()
                        .py_2()
                        .text_size(px(12.))
                        .text_color(color)
                        .child(arrow),
                );
            }
        }

        graph
    }

    fn level_color(chain_level: usize, cx: &Context<Self>) -> Hsla {
        let colors = cx.theme().colors;
        match chain_level {
            2 => colors.danger,
            1 => colors.warning,
            _ => colors.info,
        }
    }

    /// 现有证书节点卡（层级色用左侧小圆点表达）
    fn render_cert_node(&self, cert: &CertificateInfo, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let color = Self::level_color(cert.chain_level, cx);
        let expiry = expiry_state(cert.validity.days_until_expiry);

        ui::panel(cx)
            .w_full()
            .max_w(px(680.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(10.)).rounded_full().bg(color))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.foreground)
                            .child(cert_role_name(cert.chain_level)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_size(px(13.))
                    .child(
                        div()
                            .text_color(theme.colors.muted_foreground)
                            .child("颁发给:"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_color(theme.colors.foreground)
                            .child(common_name(&cert.subject, "未知")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_size(px(13.))
                    .child(
                        div()
                            .text_color(theme.colors.muted_foreground)
                            .child("颁发者:"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_color(theme.colors.muted_foreground)
                            .child(common_name(&cert.issuer, "未知颁发者")),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .font_family("Menlo")
                    .text_color(theme.colors.muted_foreground)
                    .child(format!("序列号: {}", cert.serial_number)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pt_1()
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.colors.muted_foreground)
                                            .child("开始时间"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.colors.foreground)
                                            .child(short_date(&cert.validity.not_before)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.colors.muted_foreground)
                                            .child("结束时间"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.colors.foreground)
                                            .child(short_date(&cert.validity.not_after)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child("证书状态"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(expiry_color(&expiry, cx))
                                    .child(expiry_text(&expiry)),
                            ),
                    ),
            )
    }

    /// 缺失证书红卡（无框浅底 + 同色文字）
    fn render_missing_node(
        &self,
        missing: &MissingCertificateInfo,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let theme = cx.theme().clone();
        let color = theme.colors.danger;

        ui::rich_banner(
            &format!("{} (缺失)", cert_role_name(missing.chain_level)),
            color,
            cx,
        )
        .flex_col()
        .items_start()
        .gap_1()
        .w_full()
        .max_w(px(680.))
        .child(
            div()
                .flex()
                .gap_2()
                .text_size(px(13.))
                .child(div().child("缺失证书:"))
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_color(theme.colors.foreground)
                        .child(missing.subject_name.clone()),
                ),
        )
        .child(div().text_size(px(12.)).child(missing.description.clone()))
        .child(ui::badge("此证书缺失可能导致证书链验证失败", color))
    }

    /// 图例：根CA / 中间CA / 终端 / 缺失证书
    fn render_graph_legend(&self, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let items = [
            ("根CA证书", theme.colors.danger),
            ("中间CA证书", theme.colors.warning),
            ("终端证书", theme.colors.info),
        ];

        let mut legend = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap_4();
        for (label, color) in items {
            legend = legend.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().size(px(8.)).rounded_full().bg(color))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(label),
                    ),
            );
        }
        legend = legend.child(ui::badge("缺失证书", theme.colors.danger));
        legend
    }

    /// 单证书详情：主题/颁发者/有效期/详细信息/公钥/SAN
    fn render_certificate_detail(&self, cert: &CertificateInfo, cx: &Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let expiry = expiry_state(cert.validity.days_until_expiry);

        let mut card = ui::card(cx);

        // 主题信息
        let subject_rows: Vec<gpui::Div> = ordered_name_entries(&cert.subject)
            .into_iter()
            .map(|(key, value)| ui::info_row(key, value.clone(), cx))
            .collect();
        card = card
            .child(ui::card_header("主题信息", cx))
            .child(ui::info_rows(subject_rows, cx));

        // 颁发者信息
        let issuer_rows: Vec<gpui::Div> = ordered_name_entries(&cert.issuer)
            .into_iter()
            .map(|(key, value)| ui::info_row(key, value.clone(), cx))
            .collect();
        card = card
            .child(ui::card_header("颁发者信息", cx))
            .child(ui::info_rows(issuer_rows, cx));

        // 有效期信息
        let validity_rows = vec![
            ui::info_row("开始时间", cert.validity.not_before.clone(), cx),
            ui::info_row("结束时间", cert.validity.not_after.clone(), cx),
            // 状态行需要着色，info_row 不支持自定义颜色，这里手写同结构行
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
                        .child("证书状态"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(expiry_color(&expiry, cx))
                        .child(expiry_text(&expiry)),
                ),
        ];
        card = card
            .child(ui::card_header("有效期信息", cx))
            .child(ui::info_rows(validity_rows, cx));

        // 证书详细信息
        let mut detail_rows: Vec<gpui::Div> = Vec::new();
        if cert.chain_level == 0 {
            if let Some(cert_type) = &cert.certificate_type {
                detail_rows.push(ui::info_row("证书类型", cert_type.clone(), cx));
            }
            if let Some(brand) = &cert.brand {
                detail_rows.push(ui::info_row("证书品牌", brand.clone(), cx));
            }
        }
        detail_rows.push(ui::info_row("序列号", cert.serial_number.clone(), cx));
        detail_rows.push(ui::info_row(
            "签名算法",
            cert.signature_algorithm.clone(),
            cx,
        ));
        if let Some(sha1) = &cert.sha1_fingerprint {
            detail_rows.push(ui::info_row("SHA1指纹", sha1.clone(), cx));
        }
        if let Some(sha256) = &cert.sha256_fingerprint {
            detail_rows.push(ui::info_row("SHA256指纹", sha256.clone(), cx));
        }
        card = card
            .child(ui::card_header("证书详细信息", cx))
            .child(ui::info_rows(detail_rows, cx));

        // 公钥信息
        let mut pubkey_rows: Vec<gpui::Div> = vec![ui::info_row(
            "类型",
            cert.public_key_info.key_type.clone(),
            cx,
        )];
        if let Some(key_size) = cert.public_key_info.key_size {
            pubkey_rows.push(ui::info_row("密钥长度", format!("{} bits", key_size), cx));
        }
        pubkey_rows.push(ui::info_row(
            "算法",
            cert.public_key_info.algorithm.clone(),
            cx,
        ));
        card = card
            .child(ui::card_header("公钥信息", cx))
            .child(ui::info_rows(pubkey_rows, cx));

        // SAN（仅终端证书）
        if cert.chain_level == 0 && !cert.sans.is_empty() {
            let san_rows: Vec<gpui::Div> = cert
                .sans
                .iter()
                .enumerate()
                .map(|(index, san)| ui::info_row(&format!("域名 {}", index + 1), san.clone(), cx))
                .collect();
            card = card
                .child(ui::card_header("域名清单 (SAN)", cx))
                .child(ui::info_rows(san_rows, cx));
        }

        card
    }
}

impl Render for CertificateViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.result_view && self.chain.is_some() {
            self.render_result(cx)
        } else {
            self.render_input(cx)
        };

        ui::tool_page("certificate").child(body)
    }
}
