//! JWT 解码工具页。
//!
//! Header/Payload 输出为只读 JSON 高亮编辑器（Editor + syntect，跟随主题），
//! Signature 为只读文本框（base64url 原文非 JSON）。JSON pretty(2) 展示不变。
//!
//! 交互：
//! - Token 输入 300ms 防抖自动解码（timer 任务 + 代数计数器淘汰过期任务）。
//! - 解码成功后显示黄色验证卡片（算法联动 HS/RS 密钥输入），验证结果为内联横幅。

use std::time::Duration;

use devtools::i18n::t;
use gpui::div;
use gpui::prelude::*;
use gpui::{px, App, ClickEvent, Context, Entity, IntoElement, Window};
use gpui_component::input::{EditorState, Input, InputEvent, TextareaState};
use gpui_component::{ActiveTheme as _, Disableable as _, Sizable as _};
use serde_json::Value;

use crate::components::ui;

/// 验证结果横幅类型（success | warning | error）。
enum VerifyBanner {
    Success(String),
    Warning(String),
    Error(String),
}

/// `-----BEGIN (RSA )?PUBLIC KEY-----` PEM 公钥校验（无 regex 依赖）。
fn validate_pem(pem: &str, label: &str) -> bool {
    let begin = |rsa: bool| {
        if rsa {
            format!("-----BEGIN RSA {label}-----")
        } else {
            format!("-----BEGIN {label}-----")
        }
    };
    let end = |rsa: bool| {
        if rsa {
            format!("-----END RSA {label}-----")
        } else {
            format!("-----END {label}-----")
        }
    };
    let (begin_tok, end_tok) = if pem.contains(&begin(true)) {
        (begin(true), end(true))
    } else {
        (begin(false), end(false))
    };
    if !pem.contains(begin_tok.as_str()) || !pem.contains(end_tok.as_str()) {
        return false;
    }
    let body = pem
        .replacen(begin_tok.as_str(), "", 1)
        .replacen(end_tok.as_str(), "", 1);
    let content = body.trim();
    if content.is_empty() {
        return false;
    }
    content
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=') || c.is_whitespace())
}

/// 解码 JWT：3 段校验 → base64url 解码 → JSON pretty(2)，签名段保留原文。
fn decode_token(token: &str) -> Result<(String, String, String, String), String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(t("无效的JWT格式：JWT应该包含3个部分，用点号分隔").to_string());
    }
    let header_raw = devtools::crypto::decode_jwt_segment(parts[0])?;
    let header: Value = serde_json::from_str(&header_raw)
        .map_err(|e| format!("{}: {e}", t("Header 不是有效的 JSON")))?;
    let payload_raw = devtools::crypto::decode_jwt_segment(parts[1])?;
    let payload: Value = serde_json::from_str(&payload_raw)
        .map_err(|e| format!("{}: {e}", t("Payload 不是有效的 JSON")))?;
    let algorithm = header
        .get("alg")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    Ok((
        serde_json::to_string_pretty(&header).unwrap_or(header_raw),
        serde_json::to_string_pretty(&payload).unwrap_or(payload_raw),
        parts[2].to_string(),
        algorithm,
    ))
}

/// 只读结果列通用骨架（标题行 + 复制按钮 + 内容主体）。
fn output_column_inner(
    copy_id: &'static str,
    title: &'static str,
    text: String,
    body: impl IntoElement,
    cx: &App,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(ui::card_header(title, cx))
                .when(!text.is_empty(), |el| {
                    el.child(ui::copy_button(copy_id, text))
                }),
        )
        .child(body)
}

/// 只读文本列（Signature：base64url 原文，非 JSON 不做高亮）。
fn output_column(
    copy_id: &'static str,
    title: &'static str,
    state: &Entity<TextareaState>,
    height: f32,
    cx: &App,
) -> gpui::Div {
    let text = state.read(cx).value().to_string();
    output_column_inner(copy_id, title, text, ui::result_area(state, height), cx)
}

/// 只读 JSON 高亮列（Header/Payload：Editor + syntect，颜色跟随主题）。
fn output_column_json(
    copy_id: &'static str,
    title: &'static str,
    state: &Entity<EditorState>,
    height: f32,
    cx: &App,
) -> gpui::Div {
    let text = state.read(cx).value().to_string();
    output_column_inner(
        copy_id,
        title,
        text,
        ui::code_result_area(state, height),
        cx,
    )
}

pub struct JwtDecode {
    /// JWT Token 多行输入。
    token_input: Entity<TextareaState>,
    /// HS* 验证用 Secret Key。
    secret_key: Entity<gpui_component::input::InputState>,
    /// RS* 验证用 PEM 公钥。
    public_key: Entity<TextareaState>,
    /// 解码输出：Header/Payload 为 JSON 高亮编辑器，Signature 为纯文本。
    header_out: Entity<EditorState>,
    payload_out: Entity<EditorState>,
    signature_out: Entity<TextareaState>,
    /// header.alg（解码成功后填充）。
    algorithm: String,
    /// 输入长度（实时更新，非防抖值）。
    input_len: usize,
    decode_error: Option<String>,
    /// 解码成功后显示验证卡片。
    show_verification: bool,
    public_key_error: Option<String>,
    verify_result: Option<VerifyBanner>,
    /// 防抖任务代数：输入变化即自增，过期任务不生效。
    decode_gen: u64,
}

impl JwtDecode {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let token_input = ui::new_textarea(window, cx);
        let secret_key = ui::new_input(window, cx);
        let public_key = ui::new_textarea(window, cx);
        let header_out = ui::new_editor(window, cx);
        let payload_out = ui::new_editor(window, cx);
        let signature_out = ui::new_textarea(window, cx);

        token_input.update(cx, |s, cx| {
            s.set_placeholder(t("请输入JWT令牌..."), window, cx)
        });
        secret_key.update(cx, |s, cx| {
            s.set_placeholder(t("请输入用于签名的Secret Key..."), window, cx)
        });
        public_key.update(cx, |s, cx| {
            s.set_placeholder(t("请输入RSA公钥内容..."), window, cx)
        });
        header_out.update(cx, |s, cx| {
            s.set_placeholder(t("Header 信息将在这里显示..."), window, cx);
            // JSON 语法高亮（syntect 后端，颜色跟随主题）
            s.set_highlighter("json", cx);
        });
        payload_out.update(cx, |s, cx| {
            s.set_placeholder(t("Payload 信息将在这里显示..."), window, cx);
            s.set_highlighter("json", cx);
        });
        signature_out.update(cx, |s, cx| {
            s.set_placeholder(t("Signature 信息将在这里显示..."), window, cx)
        });

        // Token 输入：长度实时刷新 + 300ms 防抖解码。
        cx.subscribe(&token_input, |this, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            this.input_len = this.token_input.read(cx).value().chars().count();
            this.schedule_decode(cx);
        })
        .detach();

        // RSA 公钥编辑：实时格式校验。
        cx.subscribe(&public_key, |this, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let text = this.public_key.read(cx).value().to_string();
            this.public_key_error = if text.trim().is_empty() || validate_pem(&text, "PUBLIC KEY") {
                None
            } else {
                Some(t("RSA公钥格式无效，请检查或重新选择文件").to_string())
            };
            cx.notify();
        })
        .detach();

        Self {
            token_input,
            secret_key,
            public_key,
            header_out,
            payload_out,
            signature_out,
            algorithm: String::new(),
            input_len: 0,
            decode_error: None,
            show_verification: false,
            public_key_error: None,
            verify_result: None,
            decode_gen: 0,
        }
    }


    /// 300ms 防抖：每次输入重启计时器，代数不一致的到期任务直接丢弃。
    fn schedule_decode(&mut self, cx: &mut Context<Self>) {
        self.decode_gen += 1;
        let gen = self.decode_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.decode_gen != gen {
                    return;
                }
                this.decode_now(window, cx);
            });
        })
        .detach();
    }

    /// 清空三区输出、算法与验证区（decode_error 由调用方决定）。
    fn clear_results(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for out in [&self.header_out, &self.payload_out] {
            out.update(cx, |s, cx| s.set_value("", window, cx));
        }
        self.signature_out
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.algorithm.clear();
        self.show_verification = false;
        self.verify_result = None;
        self.decode_error = None;
    }

    fn decode_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let token = self.token_input.read(cx).value().to_string();
        if token.trim().is_empty() {
            self.clear_results(window, cx);
            cx.notify();
            return;
        }
        self.verify_result = None;
        match decode_token(&token) {
            Ok((header_text, payload_text, signature, algorithm)) => {
                self.decode_error = None;
                self.algorithm = algorithm;
                self.header_out
                    .update(cx, |s, cx| s.set_value(header_text.as_str(), window, cx));
                self.payload_out
                    .update(cx, |s, cx| s.set_value(payload_text.as_str(), window, cx));
                self.signature_out
                    .update(cx, |s, cx| s.set_value(signature.as_str(), window, cx));
                self.show_verification = true;
            }
            Err(msg) => {
                self.clear_results(window, cx);
                self.decode_error = Some(format!("{}: {msg}", t("JWT解码失败")));
            }
        }
        cx.notify();
    }

    /// 清空输入：Token、密钥、输出、验证区全部重置（取消挂起的防抖任务）。
    fn clear_input(&mut self, _ev: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.decode_gen += 1;
        self.token_input
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.secret_key
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.public_key
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.clear_results(window, cx);
        self.public_key_error = None;
        self.input_len = 0;
        cx.notify();
    }

    /// 选择 .pem/.key/.pub 公钥文件（rfd 对话框放后台线程）。
    fn pick_public_key_file(
        &mut self,
        _ev: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut rx = crate::core_rt::submit(cx, async move {
            rfd::FileDialog::new()
                .add_filter(t("密钥文件"), &["pem", "key", "pub"])
                .pick_file()
                .and_then(|path| std::fs::read_to_string(path).ok())
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(content) = futures::StreamExt::next(&mut rx).await {
                let _ =
                    this.update_in(cx, |this, window, cx| this.on_key_file(content, window, cx));
            }
        })
        .detach();
    }

    fn on_key_file(
        &mut self,
        content: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = content else { return };
        self.public_key
            .update(cx, |s, cx| s.set_value(text.as_str(), window, cx));
        self.public_key_error = if validate_pem(&text, "PUBLIC KEY") {
            None
        } else {
            Some(t("RSA公钥格式无效，请检查或重新选择文件").to_string())
        };
        cx.notify();
    }

    /// 验证 JWT 签名：HS → core::verify_jwt；RS → 先校验公钥格式再 verify；
    /// 其他算法 warning。
    fn verify(&mut self, _ev: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let token = self.token_input.read(cx).value().to_string();
        self.verify_result = Some(if token.trim().is_empty() {
            VerifyBanner::Warning(t("请先输入JWT令牌").to_string())
        } else if self.algorithm.is_empty() {
            VerifyBanner::Error(t("无法获取JWT签名算法").to_string())
        } else {
            let alg = self.algorithm.clone();
            if alg.starts_with("HS") {
                let key = self.secret_key.read(cx).value().trim().to_string();
                if key.is_empty() {
                    VerifyBanner::Warning(t("请输入Secret Key").to_string())
                } else {
                    match devtools::crypto::verify_jwt(token.trim(), &alg, &key) {
                        Ok(true) => VerifyBanner::Success(t("JWT签名验证成功").to_string()),
                        Ok(false) => {
                            VerifyBanner::Error(t("JWT签名验证失败：签名不匹配").to_string())
                        }
                        // 此分支异常以错误横幅兜底展示。
                        Err(e) => VerifyBanner::Error(format!("{}: {e}", t("JWT签名验证出错"))),
                    }
                }
            } else if alg.starts_with("RS") {
                let key = self.public_key.read(cx).value().trim().to_string();
                if key.is_empty() {
                    VerifyBanner::Warning(t("请输入RSA公钥").to_string())
                } else if !validate_pem(&key, "PUBLIC KEY") {
                    VerifyBanner::Error(t("RSA公钥格式无效，请检查公钥内容").to_string())
                } else {
                    match devtools::crypto::verify_jwt(token.trim(), &alg, &key) {
                        Ok(true) => VerifyBanner::Success(t("JWT签名验证成功").to_string()),
                        Ok(false) => {
                            VerifyBanner::Error(t("JWT签名验证失败：签名不匹配").to_string())
                        }
                        Err(_) => {
                            VerifyBanner::Error(t("RSA公钥解析失败，请检查公钥格式").to_string())
                        }
                    }
                }
            } else {
                VerifyBanner::Warning(format!("{}{alg}", t("不支持的签名算法：")))
            }
        });
        cx.notify();
    }

    /// 黄色验证卡片（解码成功后显示）。
    fn verification_card(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let alg = self.algorithm.clone();
        let is_hs = alg.starts_with("HS");
        let is_rs = alg.starts_with("RS");

        // 密钥输入按算法联动；验证按钮随输入状态禁用。
        let key_area = if is_hs {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.warning)
                        .child("Secret Key"),
                )
                .child(Input::new(&self.secret_key))
                .child(ui::hint_text(t("💡 输入生成JWT时使用的密钥"), cx))
                .into_any_element()
        } else if is_rs {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.colors.warning)
                        .child("RSA Public Key"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            ui::secondary_button(
                                "jwt-pick-public-key",
                                t("选择文件"),
                                cx.listener(Self::pick_public_key_file),
                            )
                            .small(),
                        )
                        .child(ui::hint_text(
                            t("支持 .pem / .key / .pub 文件，或直接粘贴 PEM 内容"),
                            cx,
                        )),
                )
                .child(ui::edit_area(&self.public_key, 120.))
                .when_some(self.public_key_error.clone(), |el, e| {
                    el.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.danger)
                            .child(e),
                    )
                })
                .child(ui::hint_text(t("💡 输入与签名私钥对应的RSA公钥"), cx))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let verify_disabled = if is_hs {
            self.secret_key.read(cx).value().trim().is_empty()
        } else if is_rs {
            self.public_key.read(cx).value().trim().is_empty() || self.public_key_error.is_some()
        } else {
            false
        };

        let algorithm_row = div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(13.))
            .text_color(theme.colors.warning)
            .child(t("算法:"))
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(alg.clone()),
            );

        // 验证区横条：warning 浅底无框，标题行内嵌算法行、安全提醒子横条、
        // 密钥输入与验证按钮（原双层彩框收敛为横条语言）。
        ui::rich_banner(t("⚠️ JWT签名验证"), theme.colors.warning, cx)
            .flex_col()
            .items_start()
            .gap_2()
            .child(algorithm_row)
            // 固定安全提醒。
            .child(
                ui::rich_banner(t("安全提醒"), theme.colors.warning, cx)
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.warning)
                            .child(t(
                                "JWT签名验证通常在服务端进行。在客户端验证仅用于调试目的，请勿在生产环境中依赖客户端验证结果。",
                            )),
                    ),
            )
            .child(key_area)
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(
                        ui::primary_button(
                            "jwt-verify",
                            t("验证JWT签名"),
                            cx.listener(Self::verify),
                        )
                        .disabled(verify_disabled),
                    ),
            )
    }
}

impl gpui::Render for JwtDecode {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let token_empty = self.token_input.read(cx).value().trim().is_empty();

        let mut page = ui::tool_page("jwtdecode")
            .child(ui::hint_text(
                t("输入JWT令牌以查看其结构化内容，包括算法、有效期等信息"),
                cx,
            ))
            // Token 输入区。
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child("JWT Token"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(ui::hint_text(
                                        &format!("{}: {}", t("输入长度"), self.input_len),
                                        cx,
                                    ))
                                    .child(
                                        ui::secondary_button(
                                            "jwt-clear-input",
                                            t("清空输入"),
                                            cx.listener(Self::clear_input),
                                        )
                                        .small()
                                        .disabled(token_empty),
                                    ),
                            ),
                    )
                    .child(ui::edit_area(&self.token_input, 120.)),
            );

        // 黄色验证卡片（解码成功显示）。
        if self.show_verification && !self.algorithm.is_empty() {
            page = page.child(self.verification_card(cx));
        }

        // 验证结果横幅（✓ / ⚠ / ✕）。
        if let Some(result) = &self.verify_result {
            let banner = match result {
                VerifyBanner::Success(m) => ui::success_banner(&format!("✓ {m}"), cx),
                VerifyBanner::Warning(m) => ui::warning_banner(&format!("⚠ {m}"), cx),
                VerifyBanner::Error(m) => ui::error_banner(&format!("✕ {m}"), cx),
            };
            page = page.child(banner);
        }

        // 解码错误红条（错误: JWT解码失败: ...）。
        if let Some(error) = &self.decode_error {
            page = page.child(ui::error_banner(&format!("{}: {error}", t("错误")), cx));
        }

        // 解码输出：Header/Payload 并排 + Signature。
        // 并排列 flex_1 + min_w(0) 等分窗口宽（同 jwt_encode 的 editor_column；
        // 缺约束时列宽塌缩到内容测量宽，两列挤在左侧）
        page = page
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        output_column_json(
                            "jwt-copy-header",
                            "Header",
                            &self.header_out,
                            200.,
                            cx,
                        )
                        .flex_1()
                        .min_w(px(0.)),
                    )
                    .child(
                        output_column_json(
                            "jwt-copy-payload",
                            "Payload",
                            &self.payload_out,
                            200.,
                            cx,
                        )
                        .flex_1()
                        .min_w(px(0.)),
                    ),
            )
            .child(output_column(
                "jwt-copy-signature",
                "Signature",
                &self.signature_out,
                80.,
                cx,
            ));

        page
    }
}
