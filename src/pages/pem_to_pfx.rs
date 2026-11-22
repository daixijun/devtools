//! PEM 转 PFX 转换器。
//!
//! 证书区/私钥区/密码区 + 格式校验横条 + OpenSSL 参考命令展开 + 转换下载。

use devtools::i18n::t;
use futures::StreamExt;
use gpui::{
    div, px, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render,
    Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, TextareaState},
    Disableable as _, Sizable as _,
};

use crate::components::{notify, ui};

const CERT_HEADER_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const CERT_HEADER_END: &str = "-----END CERTIFICATE-----";

/// 判断内容是否包含完整私钥块（PKCS#8 / PKCS#1 RSA / SEC1 EC）。
fn contains_private_key(content: &str) -> bool {
    let has = |begin: &str, end: &str| content.contains(begin) && content.contains(end);
    has("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----")
        || has(
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----END RSA PRIVATE KEY-----",
        )
        || has(
            "-----BEGIN EC PRIVATE KEY-----",
            "-----END EC PRIVATE KEY-----",
        )
}

/// 判断手输私钥格式是否有效（头尾标记成对）。
fn is_valid_private_key(content: &str) -> bool {
    contains_private_key(content)
}

/// 粗略检测私钥是否被加密：ENCRYPTED 头或 legacy 头内含 ENCRYPTED 关键字。
fn is_encrypted_private_key(content: &str) -> bool {
    if content.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return true;
    }
    let has_legacy_header = content.contains("-----BEGIN RSA PRIVATE KEY-----")
        || content.contains("-----BEGIN EC PRIVATE KEY-----");
    has_legacy_header && content.to_ascii_uppercase().contains("ENCRYPTED")
}

/// 从证书文本中按行提取证书块与私钥块。
fn parse_certificate_content(content: &str) -> (String, Option<String>) {
    let mut cert_lines: Vec<&str> = Vec::new();
    let mut key_lines: Vec<&str> = Vec::new();
    let mut in_cert = false;
    let mut in_key = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains(CERT_HEADER_BEGIN) {
            in_cert = true;
            in_key = false;
            cert_lines.push(trimmed);
        } else if trimmed.contains(CERT_HEADER_END) {
            in_cert = false;
            cert_lines.push(trimmed);
        } else if trimmed.contains("-----BEGIN PRIVATE KEY-----")
            || trimmed.contains("-----BEGIN RSA PRIVATE KEY-----")
            || trimmed.contains("-----BEGIN EC PRIVATE KEY-----")
            || trimmed.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----")
        {
            in_key = true;
            in_cert = false;
            key_lines.push(trimmed);
        } else if trimmed.contains("-----END PRIVATE KEY-----")
            || trimmed.contains("-----END RSA PRIVATE KEY-----")
            || trimmed.contains("-----END EC PRIVATE KEY-----")
            || trimmed.contains("-----END ENCRYPTED PRIVATE KEY-----")
        {
            in_key = false;
            key_lines.push(trimmed);
        } else if in_cert {
            cert_lines.push(trimmed);
        } else if in_key {
            key_lines.push(trimmed);
        }
    }

    let cert = cert_lines.join("\n");
    let key = if key_lines.is_empty() {
        None
    } else {
        Some(key_lines.join("\n"))
    };
    (cert, key)
}

pub struct PemToPfxConverter {
    /// PEM 证书（文件选择或粘贴）。
    cert_area: Entity<TextareaState>,
    /// 手输私钥（仅当证书文本不含私钥块时显示）。
    key_area: Entity<TextareaState>,
    /// 私钥密码（检测到加密私钥才显示）。
    key_password: Entity<InputState>,
    /// PFX 密码（必填）。
    pfx_password: Entity<InputState>,

    // 证书文本实时分析结果
    has_cert: bool,
    has_private_key_in_cert: bool,
    cert_only: String,
    extracted_private_key: Option<String>,

    cert_error: Option<String>,
    key_error: Option<String>,
    form_error: Option<String>,
    pfx_data: Option<Vec<u8>>,
    processing: bool,
    show_openssl: bool,
}

impl PemToPfxConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cert_area = ui::new_textarea(window, cx);
        let key_area = ui::new_textarea(window, cx);
        let key_password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t("如果私钥需要密码..."))
                .masked(true)
        });
        let pfx_password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t("设置PFX文件密码..."))
                .masked(true)
        });

        // 证书文本变化 → 重新分析（条件渲染私钥区/密码区、校验横条）
        cx.subscribe(&cert_area, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.analyze_cert(cx);
                cx.notify();
            }
        })
        .detach();
        // 私钥文本变化 → 清除字段错误
        cx.subscribe(&key_area, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.key_error = None;
                cx.notify();
            }
        })
        .detach();

        Self {
            cert_area,
            key_area,
            key_password,
            pfx_password,
            has_cert: false,
            has_private_key_in_cert: false,
            cert_only: String::new(),
            extracted_private_key: None,
            cert_error: None,
            key_error: None,
            form_error: None,
            pfx_data: None,
            processing: false,
            show_openssl: false,
        }
    }

    /// 分析证书文本：检测证书块/私钥块，提取纯证书与内嵌私钥。
    fn analyze_cert(&mut self, cx: &Context<Self>) {
        let content = self.cert_area.read(cx).value().to_string();
        if content.trim().is_empty() {
            self.has_cert = false;
            self.has_private_key_in_cert = false;
            self.cert_only.clear();
            self.extracted_private_key = None;
            return;
        }

        let has_cert = content.contains(CERT_HEADER_BEGIN) && content.contains(CERT_HEADER_END);
        if has_cert {
            let has_key = contains_private_key(&content);
            let (cert_only, extracted_key) = parse_certificate_content(&content);
            self.has_cert = true;
            self.has_private_key_in_cert = has_key;
            self.cert_only = cert_only;
            self.extracted_private_key = extracted_key;
        } else {
            self.has_cert = false;
            self.has_private_key_in_cert = false;
            self.cert_only.clear();
            self.extracted_private_key = None;
        }
    }

    /// 当前生效的私钥文本（手输优先，否则取证书内提取的）。
    fn effective_private_key(&self, cx: &Context<Self>) -> String {
        let manual = self.key_area.read(cx).value().trim().to_string();
        if !manual.is_empty() {
            manual
        } else {
            self.extracted_private_key.clone().unwrap_or_default()
        }
    }

    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cert_text = self.cert_area.read(cx).value().trim().to_string();
        let pfx_password = self.pfx_password.read(cx).value().trim().to_string();
        let manual_key = self.key_area.read(cx).value().trim().to_string();
        let key_password = self.key_password.read(cx).value().trim().to_string();

        self.cert_error = None;
        self.key_error = None;
        self.form_error = None;
        self.pfx_data = None;

        if cert_text.is_empty() {
            self.cert_error = Some(t("请提供PEM证书内容").to_string());
            cx.notify();
            return;
        }
        if pfx_password.is_empty() {
            self.form_error = Some(t("请设置PFX密码").to_string());
            cx.notify();
            return;
        }
        // 证书不含私钥块时，手输私钥为必填
        if !self.has_private_key_in_cert && manual_key.is_empty() {
            self.key_error = Some(t("请提供私钥内容").to_string());
            cx.notify();
            return;
        }

        let final_private_key = if !manual_key.is_empty() {
            manual_key
        } else {
            self.extracted_private_key.clone().unwrap_or_default()
        };

        // 检测到加密私钥时必须填写私钥密码
        if !final_private_key.is_empty()
            && is_encrypted_private_key(&final_private_key)
            && key_password.is_empty()
        {
            self.key_error = Some(t("检测到加密私钥，请填写私钥密码").to_string());
            cx.notify();
            return;
        }

        if self.cert_only.trim().is_empty() {
            self.form_error = Some(t("无法提取有效的证书内容").to_string());
            cx.notify();
            return;
        }

        self.processing = true;
        cx.notify();

        // 发送提取后的纯证书（cert_only 已确保非空）
        let clean_cert = if self.cert_only.trim().is_empty() {
            cert_text
        } else {
            self.cert_only.clone()
        };

        let mut rx = crate::core_rt::submit(cx, async move {
            devtools::tools::certificate_converter::convert_pem_to_pfx(
                clean_cert,
                if final_private_key.is_empty() {
                    None
                } else {
                    Some(final_private_key)
                },
                pfx_password,
                if key_password.is_empty() {
                    None
                } else {
                    Some(key_password)
                },
            )
            .await
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, _window, cx| {
                    this.on_converted(result, cx);
                });
            }
        })
        .detach();
    }

    fn on_converted(
        &mut self,
        result: Result<devtools::tools::certificate_converter::PemToPfxResult, String>,
        cx: &mut Context<Self>,
    ) {
        self.processing = false;
        match result {
            Ok(res) if res.success => {
                self.cert_error = None;
                self.key_error = None;
                self.form_error = None;
                self.pfx_data = Some(res.pfx_data);
            }
            Ok(_) => {
                self.form_error = Some(t("转换失败：请检查证书和私钥内容是否匹配").to_string());
                self.pfx_data = None;
            }
            Err(err) => {
                // 关键词分派字段级错误：先按后端原文关键词归类，再取译名展示
                // （后端错误随中文，避免英文译名影响证书/私钥字段的分派）
                let mut message = err;
                let cert_scope = message.contains("证书");
                let key_scope = !cert_scope && message.contains("私钥");
                if message.contains("证书内容不能为空") {
                    message = t("证书内容不能为空，请提供有效的证书内容").to_string();
                } else if message.contains("证书格式不正确") {
                    message = t("证书格式不正确，请确保证书以-----BEGIN CERTIFICATE-----开头")
                        .to_string();
                } else if message.contains("私钥内容不能为空") {
                    message = t("私钥内容不能为空，请提供有效的私钥内容").to_string();
                } else if message.contains("私钥格式不正确") {
                    message = t("私钥格式不正确，请确保私钥包含正确的BEGIN和END标记").to_string();
                } else if message.contains("私钥密码错误") {
                    message = t("私钥密码错误或私钥格式不正确").to_string();
                } else if message.contains("PFX密码不能为空") {
                    message = t("PFX密码不能为空，请设置PFX文件密码").to_string();
                } else if message.contains("PKCS12构建失败") {
                    message = t("PFX文件构建失败：证书和私钥可能不匹配").to_string();
                } else if message.contains("证书解析失败") {
                    message = t("证书解析失败：请检查证书格式是否正确").to_string();
                } else if message.contains("私钥解析失败") {
                    message = t("私钥解析失败：请检查私钥格式和密码是否正确").to_string();
                } else if message == "转换失败" || message.contains("未知错误") {
                    message = t("转换失败：请检查证书和私钥内容是否匹配").to_string();
                }

                if cert_scope {
                    self.cert_error = Some(message);
                } else if key_scope {
                    self.key_error = Some(message);
                } else {
                    self.form_error = Some(message);
                }
                self.pfx_data = None;
            }
        }
        cx.notify();
    }

    /// 下载 PFX（rfd save_file + 二进制写入，均在后台线程）。
    fn download_pfx(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(data) = self.pfx_data.clone() else {
            return;
        };
        let mut rx = crate::core_rt::submit(cx, async move {
            match rfd::FileDialog::new()
                .add_filter(t("PFX文件"), &["pfx", "p12"])
                .set_file_name("certificate.pfx")
                .save_file()
            {
                Some(path) => match std::fs::write(&path, &data) {
                    Ok(_) => Some(Ok(path.to_string_lossy().to_string())),
                    Err(e) => Some(Err(format!("{}{}", t("保存文件失败："), e))),
                },
                None => None,
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, window, cx| match result {
                    None => {} // 用户取消保存
                    Some(Ok(_)) => {
                        this.form_error = None;
                        notify::toast_success(window, cx, t("PFX文件已成功保存！"));
                    }
                    Some(Err(message)) => {
                        this.form_error = Some(message);
                    }
                });
            }
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 渲染辅助
    // ------------------------------------------------------------------

    /// OpenSSL 参考命令卡（4 条 + 💡 说明）。
    fn render_openssl_card(&self, cx: &Context<Self>) -> gpui::Div {
        let commands: [(&str, &str); 4] = [
            (
                "基本转换：",
                "openssl pkcs12 -export -out certificate.pfx -inkey private.key -in certificate.crt",
            ),
            (
                "带私钥密码：",
                "openssl pkcs12 -export -out certificate.pfx -inkey private.key -in certificate.crt -passin pass:私钥密码 -passout pass:PFX密码",
            ),
            (
                "从PEM文件：",
                "openssl pkcs12 -export -out certificate.pfx -inkey key.pem -in cert.pem -certfile chain.pem",
            ),
            (
                "仅证书和私钥：",
                "openssl pkcs12 -export -out certificate.pfx -inkey private.pem -in cert.pem",
            ),
        ];
        ui::card(cx)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(t("📋 OpenSSL命令参考")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(commands.map(|(title, command)| {
                        ui::field(
                            t(title),
                            ui::mono_block(format!("mono-pem-{title}"), command, cx),
                            cx,
                        )
                    })),
            )
            .child(ui::hint_text(
                t("💡 此工具为可视化版本，上述命令仅供参考。转换结果与命令行工具完全一致。"),
                cx,
            ))
    }

    /// 当前是否显示「私钥密码」输入（检测到加密私钥才显示）。
    fn needs_key_password(&self, cx: &Context<Self>) -> bool {
        let key = self.effective_private_key(cx);
        !key.is_empty() && is_encrypted_private_key(&key)
    }
}

impl Render for PemToPfxConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_key_section = self.has_cert && !self.has_private_key_in_cert;
        let needs_key_password = self.needs_key_password(cx);
        let key_format_valid = {
            let manual = self.key_area.read(cx).value().trim().to_string();
            !manual.is_empty() && is_valid_private_key(&manual)
        };

        let convert_disabled = self.processing
            || self.cert_area.read(cx).value().trim().is_empty()
            || self.pfx_password.read(cx).value().trim().is_empty()
            || (!self.has_private_key_in_cert && self.key_area.read(cx).value().trim().is_empty());

        let openssl_button_label = if self.show_openssl {
            t("隐藏OpenSSL命令")
        } else {
            t("查看OpenSSL命令")
        };

        let mut page = ui::tool_page("pemtopfx")
            // 页头 + actions
            .child(
                div().flex().items_start().justify_between().gap_2().child(
                    Button::new("toggle-openssl")
                        .label(openssl_button_label)
                        .small()
                        .secondary()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_openssl = !this.show_openssl;
                            cx.notify();
                        })),
                ),
            );

        if self.show_openssl {
            page = page.child(self.render_openssl_card(cx));
        }

        // PEM 证书输入
        let mut cert_card = ui::card(cx)
            .child(ui::card_header(t("PEM证书"), cx))
            .child(ui::edit_area(&self.cert_area, 140.))
            .child(ui::hint_text(
                t("粘贴PEM格式的证书内容，或以-----BEGIN CERTIFICATE-----开头..."),
                cx,
            ));
        if self.has_cert {
            cert_card = cert_card.child(ui::success_banner(t("证书格式有效"), cx));
            if self.has_private_key_in_cert {
                cert_card = cert_card.child(ui::info_banner(t("证书已包含私钥，无需额外输入"), cx));
            }
        }
        if let Some(message) = &self.cert_error {
            cert_card = cert_card.child(ui::error_banner(message, cx));
        }
        page = page.child(cert_card);

        // 私钥输入（仅当证书不含私钥块时显示）
        if show_key_section {
            let mut key_card = ui::card(cx)
                .child(ui::card_header(t("私钥 *"), cx))
                .child(ui::edit_area(&self.key_area, 120.))
                .child(ui::hint_text(
                    t("请粘贴PEM格式的私钥内容，或以-----BEGIN PRIVATE KEY-----开头..."),
                    cx,
                ));
            if key_format_valid {
                key_card = key_card.child(ui::success_banner(t("私钥格式有效"), cx));
            }
            if let Some(message) = &self.key_error {
                key_card = key_card.child(ui::error_banner(message, cx));
            }
            page = page.child(key_card);
        }

        // 密码设置
        let mut password_card = ui::card(cx).child(ui::card_header(t("密码设置"), cx));
        if needs_key_password {
            password_card = password_card.child(ui::field(
                t("私钥密码 *"),
                Input::new(&self.key_password).mask_toggle().w_full(),
                cx,
            ));
        }
        password_card = password_card
            .child(ui::field(
                t("PFX密码 *"),
                Input::new(&self.pfx_password).mask_toggle().w_full(),
                cx,
            ))
            .child(ui::hint_text(t("用于保护生成的PFX文件"), cx));
        page = page.child(password_card);

        // 转换按钮（通栏）
        page = page.child(
            div().w_full().child(
                Button::new("convert")
                    .label(if self.processing {
                        t("转换中...")
                    } else {
                        t("转换为PFX")
                    })
                    .primary()
                    .large()
                    .w_full()
                    .disabled(convert_disabled)
                    .loading(self.processing)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.convert(window, cx);
                    })),
            ),
        );

        // 表单级错误/提示
        if let Some(message) = &self.form_error {
            page = page.child(ui::error_banner(message, cx));
        }

        // 成功结果卡
        if let Some(data) = &self.pfx_data {
            let size_kb = data.len() as f64 / 1024.0;
            page = page.child(
                ui::card(cx)
                    .child(ui::card_header(t("转换结果"), cx))
                    .child(ui::success_banner(
                        &format!("{}{:.2} KB", t("转换成功！文件大小："), size_kb),
                        cx,
                    ))
                    .child(
                        div().w_full().child(
                            Button::new("download-pfx")
                                .label(t("下载PFX文件"))
                                .small()
                                .secondary()
                                .w_full()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.download_pfx(window, cx);
                                })),
                        ),
                    ),
            );
        }

        page
    }
}
