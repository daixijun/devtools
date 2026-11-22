//! PFX 转 PEM 转换器。
//!
//! PFX 二进制文件选择 + 可选密码 + OpenSSL 参考命令 + 转换结果复制/下载。

use futures::StreamExt;
use gpui::{
    div, px, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render,
    Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputState, TextareaState},
    ActiveTheme as _, Disableable as _, Sizable as _,
};

use crate::components::{notify, ui};

const TIPS: [&str; 3] = [
    "• 选择PFX/P12文件，如有密码请输入",
    "• 点击\"开始转换\"获取PEM格式证书",
    "• 转换后可复制或下载PEM文件",
];

/// OpenSSL 参考命令（4 条）。
const OPENSSL_COMMANDS: [(&str, &str); 4] = [
    (
        "基本转换（无密码）：",
        "openssl pkcs12 -in certificate.pfx -out certificate.pem -nodes",
    ),
    (
        "带密码转换：",
        "openssl pkcs12 -in certificate.pfx -out certificate.pem -nodes -password pass:yourpassword",
    ),
    (
        "仅导出证书（不含私钥）：",
        "openssl pkcs12 -in certificate.pfx -out certificate.pem -nokeys",
    ),
    (
        "仅导出私钥：",
        "openssl pkcs12 -in certificate.pfx -out privatekey.pem -nocerts -nodes",
    ),
];

pub struct PfxToPemConverter {
    file_name: Option<String>,
    file_data: Option<Vec<u8>>,
    /// PFX 密码（可选）。
    password: Entity<InputState>,
    /// 转换结果（只读）。
    result: Entity<TextareaState>,
    error: Option<String>,
    loading: bool,
    show_openssl: bool,
}

impl PfxToPemConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入PFX文件密码（如果有的话）")
                .masked(true)
        });
        let result = ui::new_textarea(window, cx);
        Self {
            file_name: None,
            file_data: None,
            password,
            result,
            error: None,
            loading: false,
            show_openssl: false,
        }
    }

    /// 选择 PFX 文件（对话框 + 读取均在后台线程）。
    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rx = crate::core_rt::submit(cx, async move {
            match rfd::FileDialog::new()
                .add_filter("PFX/P12文件", &["pfx", "p12"])
                .pick_file()
            {
                Some(path) => match std::fs::read(&path) {
                    Ok(data) => {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        Some(Ok((name, data)))
                    }
                    Err(e) => Some(Err(format!("读取文件失败：{}", e))),
                },
                None => None,
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.on_file_picked(result, window, cx);
                });
            }
        })
        .detach();
    }

    fn on_file_picked(
        &mut self,
        picked: Option<Result<(String, Vec<u8>), String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match picked {
            None => {} // 用户取消
            Some(Err(message)) => self.error = Some(message),
            Some(Ok((name, data))) => {
                let lower = name.to_lowercase();
                if !lower.ends_with(".pfx") && !lower.ends_with(".p12") {
                    self.error = Some("请选择PFX或P12格式的文件".to_string());
                    self.file_name = None;
                    self.file_data = None;
                } else {
                    self.error = None;
                    self.file_name = Some(name);
                    self.file_data = Some(data);
                    // 新文件清空旧结果
                    self.result
                        .update(cx, |out, cx| out.set_value("", window, cx));
                }
            }
        }
        cx.notify();
    }

    /// 清除已选文件。
    fn clear_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.file_name = None;
        self.file_data = None;
        self.error = None;
        self.result
            .update(cx, |out, cx| out.set_value("", window, cx));
        cx.notify();
    }

    /// 执行转换。
    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(data) = self.file_data.clone() else {
            self.error = Some("请选择PFX文件".to_string());
            cx.notify();
            return;
        };
        let password = self.password.read(cx).value().trim().to_string();
        self.loading = true;
        self.error = None;
        self.result
            .update(cx, |out, cx| out.set_value("", window, cx));
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            devtools::tools::certificate_converter::convert_pfx_to_pem(
                data,
                if password.is_empty() {
                    None
                } else {
                    Some(password)
                },
            )
            .await
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.on_converted(result, window, cx);
                });
            }
        })
        .detach();
    }

    fn on_converted(
        &mut self,
        result: Result<devtools::tools::certificate_converter::PfxConversionResult, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(res) if res.success => {
                self.error = None;
                let combined = res.combined_pem;
                self.result
                    .update(cx, |out, cx| out.set_value(&combined, window, cx));
            }
            Ok(_) => {
                self.error = Some("转换失败".to_string());
            }
            Err(message) => {
                // 按关键词映射为友好文案
                let mapped = if message.contains("密码") || message.contains("password") {
                    "密码错误或文件损坏。请检查密码是否正确，或尝试使用其他工具验证文件完整性。"
                } else if message.contains("格式") || message.contains("format") {
                    "文件格式错误，可能不是有效的PFX文件。请确认文件未损坏且为正确的PFX/P12格式。"
                } else if message.contains("未找到") || message.contains("not found") {
                    "未在PFX文件中找到证书和私钥。请检查以下几点：\n1. 确保选择的文件是有效的PFX/P12文件\n2. 如果文件有密码保护，请输入正确的密码\n3. 确认文件中确实包含证书和私钥内容"
                } else {
                    message.as_str()
                };
                self.error = Some(mapped.to_string());
            }
        }
        cx.notify();
    }

    /// 下载 PEM（原文件名换 .pem，文本写入后台线程）。
    fn download_pem(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.result.read(cx).value().to_string();
        if content.trim().is_empty() {
            return;
        }
        let default_name = self
            .file_name
            .as_deref()
            .map(|name| {
                let lower = name.to_lowercase();
                if lower.ends_with(".pfx") || lower.ends_with(".p12") {
                    format!("{}.pem", &name[..name.len() - 4])
                } else {
                    name.to_string()
                }
            })
            .unwrap_or_else(|| "certificate.pem".to_string());

        let mut rx = crate::core_rt::submit(cx, async move {
            match rfd::FileDialog::new()
                .add_filter("PEM Files", &["pem"])
                .set_file_name(default_name)
                .save_file()
            {
                Some(path) => match std::fs::write(&path, content.as_bytes()) {
                    Ok(_) => Some(Ok(path.to_string_lossy().to_string())),
                    Err(e) => Some(Err(format!("保存文件失败: {}", e))),
                },
                None => None,
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, window, cx| match result {
                    None => {} // 用户取消
                    Some(Ok(_)) => {
                        this.error = None;
                        notify::toast_success(window, cx, "文件保存成功");
                    }
                    Some(Err(message)) => this.error = Some(message),
                });
            }
        })
        .detach();
    }
}

impl Render for PfxToPemConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_result = !self.result.read(cx).value().trim().is_empty();
        let openssl_button_label = if self.show_openssl {
            "隐藏OpenSSL命令"
        } else {
            "查看OpenSSL命令"
        };

        let mut page = ui::tool_page("pfxtopem")
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

        // OpenSSL 参考命令
        if self.show_openssl {
            page = page.child(
                ui::card(cx)
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("📋 OpenSSL命令参考"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(OPENSSL_COMMANDS.map(|(title, command)| {
                                ui::field(
                                    title,
                                    ui::mono_block(format!("mono-pfx-{title}"), command, cx),
                                    cx,
                                )
                            })),
                    )
                    .child(ui::hint_text(
                        "💡 此工具为可视化版本，上述命令仅供参考。转换结果与命令行工具完全一致。",
                        cx,
                    )),
            );
        }

        // 使用提示（固定三条）
        page = page.child(
            ui::card(cx)
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("💡 使用提示"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(TIPS.map(|tip| ui::hint_text(tip, cx))),
                ),
        );

        // PFX 文件选择
        let mut file_card = ui::card(cx).child(ui::card_header("PFX文件", cx)).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("pick-file")
                        .label("选择文件")
                        .small()
                        .secondary()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.pick_file(window, cx);
                        })),
                )
                .child(ui::hint_text("支持 .pfx / .p12 格式", cx)),
        );
        if let Some(name) = &self.file_name {
            file_card = file_card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(ui::hint_text(&format!("已选择： {}", name), cx))
                    .child(
                        Button::new("clear-file")
                            .label("清除")
                            .small()
                            .ghost()
                            .text_color(cx.theme().colors.danger)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear_file(window, cx);
                            })),
                    ),
            );
        }
        page = page.child(file_card);

        // PFX 密码（可选）
        page = page.child(ui::hfield(
            "PFX 密码 (可选):",
            Input::new(&self.password).mask_toggle().w_full(),
            cx,
        ));

        // 开始转换（通栏）
        page = page.child(
            div().w_full().child(
                Button::new("convert")
                    .label(if self.loading {
                        "转换中..."
                    } else {
                        "开始转换"
                    })
                    .primary()
                    .large()
                    .w_full()
                    .disabled(self.loading || self.file_data.is_none())
                    .loading(self.loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.convert(window, cx);
                    })),
            ),
        );

        // 错误横条
        if let Some(message) = &self.error {
            page = page.child(ui::error_banner(message, cx));
        }

        // 转换结果
        if has_result {
            let result_text = self.result.read(cx).value().to_string();
            page = page.child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(ui::card_header("转换结果:", cx))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(ui::copy_button("copy-result", result_text))
                                    .child(
                                        Button::new("download-pem")
                                            .label("下载PEM")
                                            .small()
                                            .secondary()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.download_pem(window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(ui::result_area(&self.result, 256.)),
            );
        }

        page
    }
}
