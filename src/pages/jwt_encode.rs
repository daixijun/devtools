//! JWT 生成工具页。
//!
//! Header/Payload 以多行文本框承载；JSON 实时校验逻辑保持一致
//! （每次 Change 重新 serde_json 解析，失败红字提示）。
//! 「生成出错」时采用内联错误条约定：红色「生成的JWT」标题 + 隐藏复制按钮。

use std::time::{SystemTime, UNIX_EPOCH};

use devtools::i18n::t;
use gpui::div;
use gpui::prelude::*;
use gpui::{px, App, ClickEvent, Context, Entity, IntoElement, Window};
use gpui_component::input::{Input, InputEvent, NumberInput, TextareaState};
use gpui_component::{ActiveTheme as _, Disableable as _, Sizable as _};
use serde_json::{json, Value};

use crate::components::ui;

/// 当前 Unix 时间戳（秒）。
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `-----BEGIN (RSA )?<label>-----` PEM 格式校验（无 regex 依赖）：
/// 必须含 BEGIN/END 头尾，正文非空且仅由 base64 字符与空白组成。
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

/// 红色错误小字。
fn error_text(text: &str, cx: &App) -> gpui::Div {
    div()
        .text_size(px(12.))
        .text_color(cx.theme().colors.danger)
        .child(text.to_string())
}

/// label 在左（固定宽）、控件在右的紧凑行。
fn field_row(label: &str, control: impl IntoElement, cx: &App) -> gpui::Div {
    let theme = cx.theme().clone();
    div()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .w(px(112.))
                .flex_shrink_0()
                .text_size(px(13.))
                .text_color(theme.colors.foreground)
                .child(label.to_string()),
        )
        .child(control)
}

pub struct JwtEncode {
    /// Header JSON 编辑器。
    header_input: Entity<TextareaState>,
    /// Payload JSON 编辑器。
    payload_input: Entity<TextareaState>,
    /// 有效期（秒）。
    expires_in: Entity<gpui_component::input::InputState>,
    /// HS* 算法的 Secret Key。
    secret_key: Entity<gpui_component::input::InputState>,
    /// RS* 算法的 PEM 私钥（文件或粘贴）。
    private_key: Entity<TextareaState>,
    /// 生成的 JWT（只读输出区）。
    output: Entity<TextareaState>,
    /// 从 header JSON 解析出的 alg（每次 Change 重解析）。
    alg: String,
    header_error: Option<String>,
    payload_error: Option<String>,
    private_key_error: Option<String>,
    /// 输出内容是否为「生成出错」类错误文案。
    output_is_error: bool,
}

impl JwtEncode {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = now_secs();
        let header_input = ui::new_textarea(window, cx);
        let payload_input = ui::new_textarea(window, cx);
        let expires_in = ui::new_input(window, cx);
        let secret_key = ui::new_input(window, cx);
        let private_key = ui::new_textarea(window, cx);
        let output = ui::new_textarea(window, cx);

        // 初始 Header/Payload（pretty 2）；set_value 不触发 Change，状态手动同步。
        let default_header = json!({"alg": "HS256", "typ": "JWT"});
        let default_payload = json!({"sub": "1234567890", "iss": "", "iat": now});
        header_input.update(cx, |s, cx| {
            s.set_value(
                serde_json::to_string_pretty(&default_header)
                    .unwrap_or_default()
                    .as_str(),
                window,
                cx,
            )
        });
        payload_input.update(cx, |s, cx| {
            s.set_value(
                serde_json::to_string_pretty(&default_payload)
                    .unwrap_or_default()
                    .as_str(),
                window,
                cx,
            )
        });
        secret_key.update(cx, |s, cx| s.set_value("mySecretKey", window, cx));
        expires_in.update(cx, |s, cx| s.set_value("3600", window, cx));
        private_key.update(cx, |s, cx| {
            s.set_placeholder(t("请输入RSA私钥内容..."), window, cx)
        });
        secret_key.update(cx, |s, cx| {
            s.set_placeholder(t("输入 Secret Key"), window, cx)
        });
        expires_in.update(cx, |s, cx| s.set_placeholder(t("输入有效期"), window, cx));

        // Header 编辑：实时 JSON 校验 + 提取 alg 供密钥区联动。
        cx.subscribe(&header_input, |this, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let text = this.header_input.read(cx).value().to_string();
            match serde_json::from_str::<Value>(&text) {
                Ok(v) => {
                    this.alg = v
                        .get("alg")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    this.header_error = None;
                }
                Err(_) => {
                    this.header_error = Some(t("Header 格式错误: 请输入有效的 JSON").to_string());
                }
            }
            cx.notify();
        })
        .detach();

        // Payload 编辑：实时 JSON 校验。
        cx.subscribe(&payload_input, |this, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let text = this.payload_input.read(cx).value().to_string();
            this.payload_error = match serde_json::from_str::<Value>(&text) {
                Ok(_) => None,
                Err(_) => Some(t("Payload 格式错误: 请输入有效的 JSON").to_string()),
            };
            cx.notify();
        })
        .detach();

        // RSA 私钥编辑：实时格式校验。
        cx.subscribe(&private_key, |this, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let text = this.private_key.read(cx).value().to_string();
            this.private_key_error = if text.trim().is_empty() || validate_pem(&text, "PRIVATE KEY")
            {
                None
            } else {
                Some(t("RSA私钥格式无效，请检查或重新选择文件").to_string())
            };
            cx.notify();
        })
        .detach();

        Self {
            header_input,
            payload_input,
            expires_in,
            secret_key,
            private_key,
            output,
            alg: "HS256".to_string(),
            header_error: None,
            payload_error: None,
            private_key_error: None,
            output_is_error: false,
        }
    }

    /// 把 JSON 值 pretty(2) 写入编辑器（set_value 不触发 Change，错误状态随调用点同步）。
    fn set_editor(
        &self,
        state: &Entity<TextareaState>,
        value: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = serde_json::to_string_pretty(value).unwrap_or_default();
        state.update(cx, |s, cx| s.set_value(text.as_str(), window, cx));
    }

    fn set_output(
        &mut self,
        text: &str,
        is_error: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.output_is_error = is_error;
        self.output
            .update(cx, |s, cx| s.set_value(text, window, cx));
        cx.notify();
    }

    /// 生成 JWT：校验有效期 → payload 注入 exp → core::sign_jwt。
    fn generate(&mut self, _ev: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let header_text = self.header_input.read(cx).value().to_string();
        let payload_text = self.payload_input.read(cx).value().to_string();

        // 有效期必须为正整数，否则报错。
        let expires_raw = self.expires_in.read(cx).value().to_string();
        let expires_in = match expires_raw.trim().parse::<i64>() {
            Ok(v) if v > 0 => v,
            _ => {
                self.set_output(
                    &format!("{}: {}", t("生成出错"), t("有效期必须是正整数")),
                    true,
                    window,
                    cx,
                );
                return;
            }
        };

        // 按钮 disabled 已保证 JSON 有效；此处兜底。
        let header: Value = match serde_json::from_str(&header_text) {
            Ok(v) => v,
            Err(_) => {
                self.set_output(
                    &format!(
                        "{}: {}",
                        t("生成出错"),
                        t("Header 格式错误: 请输入有效的 JSON")
                    ),
                    true,
                    window,
                    cx,
                );
                return;
            }
        };
        let mut payload: Value = match serde_json::from_str(&payload_text) {
            Ok(v) => v,
            Err(_) => {
                self.set_output(
                    &format!(
                        "{}: {}",
                        t("生成出错"),
                        t("Payload 格式错误: 请输入有效的 JSON")
                    ),
                    true,
                    window,
                    cx,
                );
                return;
            }
        };

        let alg = header
            .get("alg")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let is_hs = alg.starts_with("HS");

        // 非 HS 算法（含 RS* 与未知算法）一律先校验私钥格式。
        let private_key = self.private_key.read(cx).value().to_string();
        if !is_hs && !validate_pem(&private_key, "PRIVATE KEY") {
            self.set_output(
                &format!(
                    "{}: {}",
                    t("生成出错"),
                    t("RSA私钥格式无效，请检查或重新选择文件")
                ),
                true,
                window,
                cx,
            );
            return;
        }

        if !payload.is_object() {
            self.set_output(
                &format!("{}: {}", t("生成出错"), t("Payload 必须是 JSON 对象")),
                true,
                window,
                cx,
            );
            return;
        }
        // iat 保持不动，仅注入 exp = now + expiresIn。
        payload["exp"] = json!(now_secs() + expires_in);
        let payload_pretty = serde_json::to_string_pretty(&payload).unwrap_or_default();

        let key = if is_hs {
            self.secret_key.read(cx).value().to_string()
        } else {
            private_key
        };

        match devtools::crypto::sign_jwt(&alg, header, payload, &key) {
            Ok(token) => {
                self.payload_input
                    .update(cx, |s, cx| s.set_value(payload_pretty.as_str(), window, cx));
                self.set_output(&token, false, window, cx);
            }
            Err(e) => self.set_output(&format!("{}: {e}", t("生成JWT失败")), true, window, cx),
        }
    }

    /// 示例：HS256 头 + 带 exp 的载荷 + your-256-bit-secret + 3600 秒。
    fn load_example(&mut self, _ev: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let now = now_secs();
        let header = json!({"alg": "HS256", "typ": "JWT"});
        let payload =
            json!({"sub": "1234567890", "iss": "example.com", "iat": now, "exp": now + 3600});
        self.set_editor(&self.header_input, &header, window, cx);
        self.set_editor(&self.payload_input, &payload, window, cx);
        self.secret_key
            .update(cx, |s, cx| s.set_value("your-256-bit-secret", window, cx));
        self.private_key
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.expires_in
            .update(cx, |s, cx| s.set_value("3600", window, cx));
        self.alg = "HS256".to_string();
        self.header_error = None;
        self.payload_error = None;
        self.private_key_error = None;
        self.set_output("", false, window, cx);
    }

    /// 清空：恢复默认 Header/Payload（空 claims）、清空密钥与输出。
    fn clear_all(&mut self, _ev: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let now = now_secs();
        let header = json!({"alg": "HS256", "typ": "JWT"});
        let payload = json!({"sub": "", "iss": "", "iat": now});
        self.set_editor(&self.header_input, &header, window, cx);
        self.set_editor(&self.payload_input, &payload, window, cx);
        self.secret_key
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.private_key
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.expires_in
            .update(cx, |s, cx| s.set_value("3600", window, cx));
        self.alg = "HS256".to_string();
        self.header_error = None;
        self.payload_error = None;
        self.private_key_error = None;
        self.set_output("", false, window, cx);
    }

    /// 选择 .pem/.key 私钥文件（rfd 对话框放后台线程，读文本后回填校验）。
    fn pick_private_key_file(
        &mut self,
        _ev: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut rx = crate::core_rt::submit(cx, async move {
            rfd::FileDialog::new()
                .add_filter(t("密钥文件"), &["pem", "key"])
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
        self.private_key
            .update(cx, |s, cx| s.set_value(text.as_str(), window, cx));
        self.private_key_error = if validate_pem(&text, "PRIVATE KEY") {
            None
        } else {
            Some(t("RSA私钥格式无效，请检查或重新选择文件").to_string())
        };
        cx.notify();
    }

    /// Header/Payload 编辑器列（标题 + 说明 + 编辑器 + 红字错误）。
    fn editor_column(
        &self,
        title: &'static str,
        description: &str,
        state: &Entity<TextareaState>,
        error: Option<String>,
        cx: &App,
    ) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .gap_1()
            .child(ui::card_header(title, cx))
            .child(ui::hint_text(description, cx))
            .child(ui::edit_area(state, 200.))
            .when_some(error, |el, e| el.child(error_text(&e, cx)))
    }
}

impl gpui::Render for JwtEncode {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_hs = self.alg.starts_with("HS");
        let is_rs = self.alg.starts_with("RS");
        let output = self.output.read(cx).value().to_string();
        let output_is_error = self.output_is_error && !output.is_empty();
        let private_key_empty = self.private_key.read(cx).value().trim().is_empty();

        // 生成按钮 disabled：任一 JSON 错误，或 RS 且私钥无效/为空。
        let generate_disabled = self.header_error.is_some()
            || self.payload_error.is_some()
            || (is_rs && (self.private_key_error.is_some() || private_key_empty));

        // 密钥区按 header.alg 联动：HS* → Secret Key 单行；其余 → RSA 私钥区。
        let key_area = if is_hs {
            field_row(
                "Secret Key",
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .child(Input::new(&self.secret_key)),
                cx,
            )
            .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::card_header("RSA Private Key", cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            ui::secondary_button(
                                "jwt-pick-private-key",
                                t("选择文件"),
                                cx.listener(Self::pick_private_key_file),
                            )
                            .small(),
                        )
                        .child(ui::hint_text(
                            t("支持 .pem / .key 文件，或直接粘贴 PEM 内容"),
                            cx,
                        )),
                )
                .child(ui::edit_area(&self.private_key, 120.))
                .when_some(self.private_key_error.clone(), |el, e| {
                    el.child(error_text(&e, cx))
                })
                .into_any_element()
        };

        let mut page = ui::tool_page("jwtencode")
            // 页头 + 右侧动作（示例 / 清空）。
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(ui::hint_text(
                        t("配置Header、Payload和签名密钥，生成符合标准的JWT令牌"),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::secondary_button(
                                    "jwt-example",
                                    t("示例"),
                                    cx.listener(Self::load_example),
                                )
                                .small(),
                            )
                            .child(
                                ui::secondary_button(
                                    "jwt-clear",
                                    t("清空"),
                                    cx.listener(Self::clear_all),
                                )
                                .small(),
                            ),
                    ),
            )
            // Header / Payload 并排编辑器。
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.editor_column(
                        "Header",
                        t("包含令牌的元数据，如签名算法(alg)和类型(typ)"),
                        &self.header_input,
                        self.header_error.clone(),
                        cx,
                    ))
                    .child(self.editor_column(
                        "Payload",
                        t("包含声明(claims)，如主题(sub)、签发者(iss)、签发时间(iat)和过期时间(exp)"),
                        &self.payload_input,
                        self.payload_error.clone(),
                        cx,
                    )),
            )
            // 配置区：有效期 + 密钥。
            .child(ui::card(cx).child(field_row(
                t("有效期 (秒)"),
                div().w(px(200.)).child(NumberInput::new(&self.expires_in)),
                cx,
            )))
            .child(key_area)
            // 生成按钮（primary、大号、全宽）。
            .child(
                ui::primary_button(
                    "jwt-generate",
                    t("生成JWT"),
                    cx.listener(Self::generate),
                )
                .large()
                .w_full()
                .disabled(generate_disabled),
            );

        // 输出区：非空才显示；「生成出错」标题标红且无复制按钮（见文件头降级说明）。
        if !output.is_empty() {
            let title_color = if output_is_error {
                theme.colors.danger
            } else {
                theme.colors.foreground
            };
            let title = div()
                .text_size(px(14.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(title_color)
                .child(t("生成的JWT"));
            let mut section = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().flex().items_center().justify_between().child(title));
            if !output_is_error {
                section = section.child(ui::copy_result_button(
                    "jwt-copy-output",
                    t("复制JWT"),
                    output.clone(),
                ));
            }
            page = page
                .child(section)
                .child(ui::result_area(&self.output, 96.));
        }

        page
    }
}
