//! CSR 查看。
//!
//! 解析 PKCS#10 证书签名请求：文件选择（rfd 后台线程）或粘贴文本二选一，
//! 可选私钥校验；输出卡片化（基本信息 / 主题 / 公钥 / 私钥校验 / 扩展 / 原始 PEM）。

use futures::StreamExt as _;
use gpui::{
    div, prelude::*, px, App, Context, Entity, FontWeight, IntoElement, Render, SharedString,
    Window,
};
use gpui_component::{
    input::{InputEvent, TextareaState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::{notify, ui};
use devtools::tools::csr_parser::{self, CsrInfo};

/// 文件保存结果（rfd save_file + std::fs::write，均在 core_rt 后台任务中执行）。
enum SaveOutcome {
    Cancelled,
    Saved,
    Failed(String),
}

pub struct CsrViewer {
    /// 粘贴的 CSR 文本（PEM / Base64）。
    csr_text: Entity<TextareaState>,
    /// 可选私钥（PKCS#8 / PKCS#1 / SEC1），用于校验与 CSR 公钥是否匹配。
    key_text: Entity<TextareaState>,
    /// 公钥 PEM（SPKI）只读展示。
    spki_view: Entity<TextareaState>,
    /// 原始 CSR PEM 只读展示。
    pem_view: Entity<TextareaState>,
    /// 已选择文件的名称（显示用）。
    file_name: Option<String>,
    /// 已选择文件的原始字节（解析时按内容判定 PEM 文本或 DER）。
    file_data: Option<Vec<u8>>,
    result: Option<CsrInfo>,
    loading: bool,
    error: Option<String>,
}

impl CsrViewer {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let csr_text =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("粘贴 PEM 格式的 CSR 文件…"));
        let key_text = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----")
        });
        let spki_view = cx.new(|cx| TextareaState::new(window, cx));
        let pem_view = cx.new(|cx| TextareaState::new(window, cx));

        // 手动输入文本时清除已选文件（文本优先，对齐历史版本行为）
        cx.subscribe(&csr_text, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let has_text = !this.csr_text.read(cx).value().trim().is_empty();
                if has_text && this.file_data.is_some() {
                    this.file_data = None;
                    this.file_name = None;
                    cx.notify();
                }
            }
        })
        .detach();

        Self {
            csr_text,
            key_text,
            spki_view,
            pem_view,
            file_name: None,
            file_data: None,
            result: None,
            loading: false,
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // 交互
    // ------------------------------------------------------------------

    /// 选择 CSR 文件：rfd 对话框与文件读取都在 core_rt 后台任务中完成。
    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rx = crate::core_rt::submit(cx, async move {
            let path = rfd::FileDialog::new()
                .add_filter("CSR 文件", &["csr", "pem", "txt"])
                .pick_file()?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "未命名文件".to_string());
            match std::fs::read(&path) {
                Ok(data) => Some(Ok((name, data))),
                Err(e) => Some(Err(format!("读取文件失败: {e}"))),
            }
        });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(picked) = rx.next().await {
                let _ = this.update_in(cx, |this, window, cx| match picked {
                    None => {}
                    Some(Ok((name, data))) => this.set_file(name, data, window, cx),
                    Some(Err(message)) => {
                        this.error = Some(message);
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    /// 记录已选文件并清空粘贴文本与上一次结果。
    fn set_file(
        &mut self,
        name: String,
        data: Vec<u8>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.csr_text
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.file_name = Some(name);
        self.file_data = Some(data);
        self.result = None;
        self.error = None;
        cx.notify();
    }

    /// 解析 CSR：文件按内容判定（含 `-----BEGIN` 按文本，否则按 DER 字节），后台任务中调用 core。
    fn parse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let csr_text = self.csr_text.read(cx).value().to_string();
        let has_file = self.file_data.is_some();

        if !has_file && csr_text.trim().is_empty() {
            self.error = Some("请先粘贴 PEM 格式的 CSR 内容或上传 CSR 文件".to_string());
            cx.notify();
            return;
        }

        let file_data = self.file_data.clone();
        let key_text = self.key_text.read(cx).value().trim().to_string();
        let private_key = (!key_text.is_empty()).then_some(key_text);

        self.loading = true;
        self.error = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            let (content, der) = match file_data {
                Some(data) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    if text.contains("-----BEGIN") {
                        (Some(text), None)
                    } else {
                        (None, Some(data))
                    }
                }
                None => (Some(csr_text), None),
            };
            csr_parser::parse_csr(content, der, private_key)
        });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, window, cx| this.on_parsed(result, window, cx));
            }
        })
        .detach();
    }

    fn on_parsed(
        &mut self,
        result: Result<CsrInfo, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(info) => {
                let spki = info.public_key.spki_pem.clone();
                let pem = info.pem.clone();
                self.spki_view
                    .update(cx, |state, cx| state.set_value(spki, window, cx));
                self.pem_view
                    .update(cx, |state, cx| state.set_value(pem, window, cx));
                self.result = Some(info);
                self.error = None;
            }
            Err(message) => {
                self.error = Some(message);
                self.result = None;
            }
        }
        cx.notify();
    }

    /// 清空输入、文件与结果。
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.csr_text
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.key_text
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.spki_view
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.pem_view
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.file_name = None;
        self.file_data = None;
        self.result = None;
        self.error = None;
        cx.notify();
    }

    /// 下载原始 CSR（.csr）：保存对话框与写文件都在后台任务中完成。
    fn download_pem(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(info) = &self.result else { return };
        let content = info.pem.clone();
        let mut rx = crate::core_rt::submit(cx, async move {
            save_text(content, "certificate_request.csr", "csr", "CSR 文件").await
        });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(outcome) = rx.next().await {
                let _ = this.update_in(cx, |_this, window, cx| report_save(outcome, window, cx));
            }
        })
        .detach();
    }
}

// ----------------------------------------------------------------------
// 保存辅助（save 在 core_rt 后台任务内执行，回调回前台 toast）
// ----------------------------------------------------------------------

async fn save_text(
    content: String,
    default_name: &str,
    ext: &'static str,
    filter_name: &str,
) -> SaveOutcome {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(filter_name, &[ext])
        .set_file_name(default_name)
        .save_file()
    else {
        return SaveOutcome::Cancelled;
    };
    match std::fs::write(&path, content) {
        Ok(_) => SaveOutcome::Saved,
        Err(e) => SaveOutcome::Failed(format!("保存文件失败: {e}")),
    }
}

fn report_save(outcome: SaveOutcome, window: &mut Window, cx: &mut App) {
    match outcome {
        SaveOutcome::Cancelled => {}
        SaveOutcome::Saved => notify::toast_success(window, cx, "文件保存成功"),
        SaveOutcome::Failed(message) => notify::toast_error(window, cx, message),
    }
}

// ----------------------------------------------------------------------
// 展示辅助
// ----------------------------------------------------------------------

fn format_der_size(size: usize) -> String {
    if size < 1024 {
        format!("{size} 字节")
    } else {
        format!("{:.2} KB", size as f64 / 1024.0)
    }
}

fn key_line(key_type: &str, key_size: Option<u32>, curve_name: Option<&str>) -> String {
    let mut line = key_type.to_string();
    if let Some(bits) = key_size {
        line.push_str(&format!(" · {bits} 位"));
    }
    if let Some(curve) = curve_name {
        line.push_str(&format!(" · 曲线 {curve}"));
    }
    line
}

/// 键值行，值右侧可附操作按钮（如复制指纹）。
fn action_row(
    label: &str,
    value: impl Into<SharedString>,
    action: impl IntoElement,
    cx: &App,
) -> gpui::Div {
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
        .child(
            div()
                .flex_1()
                .flex()
                .items_start()
                .gap_2()
                .overflow_hidden()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.))
                        .font_family("Menlo")
                        .overflow_hidden()
                        .child(value.into()),
                )
                .child(action),
        )
}

/// 小号灰字（如「OID: 2.5.4.3」或空态提示行）。
fn small_hint(text: &str, color: gpui::Hsla) -> gpui::Div {
    div()
        .text_size(px(11.))
        .text_color(color)
        .child(text.to_string())
}

/// 小号彩色说明文字（如签名验证失败的原因说明）。
fn note_line(text: &str, color: gpui::Hsla) -> gpui::Div {
    div()
        .mt_1()
        .text_size(px(12.))
        .text_color(color)
        .child(text.to_string())
}

/// 结果区全部卡片（基本信息 / 主题 / 公钥 / 私钥校验 / 扩展 / 原始 PEM）。
fn render_result(
    info: &CsrInfo,
    spki_view: &Entity<TextareaState>,
    _pem_view: &Entity<TextareaState>,
    cx: &App,
) -> gpui::Div {
    let theme = cx.theme().clone();

    // 基本信息
    let sig_badge = if info.signature_valid {
        ui::badge("✓ 签名验证通过", theme.colors.success)
    } else {
        ui::badge("✗ 签名验证失败", theme.colors.danger)
    };
    let version_text = if info.version == 0 {
        "v1（版本号 0）".to_string()
    } else {
        format!("{}", info.version)
    };
    let basic_rows = vec![
        ui::info_row("CSR 版本", version_text, cx),
        ui::info_row("签名算法", info.signature_algorithm.clone(), cx),
        ui::info_row("DER 大小", format_der_size(info.der_size), cx),
    ];
    let basic_card = ui::card(cx)
        .child(ui::card_header("基本信息", cx).child(sig_badge))
        .child(ui::info_rows(basic_rows, cx))
        .when(!info.signature_valid, |el| {
            el.child(note_line(
                "CSR 的自签名未通过验证：通常表示内容被篡改，或文件与生成它的私钥不匹配",
                theme.colors.danger,
            ))
        });

    // 主题（Subject）：name 左，value + OID 右
    let mut subject_card = ui::card(cx).child(ui::card_header("主题（Subject）", cx));
    if info.subject.is_empty() {
        subject_card = subject_card.child(small_hint(
            "该 CSR 未包含主题属性",
            theme.colors.muted_foreground,
        ));
    } else {
        let subject_rows: Vec<gpui::Div> = info
            .subject
            .iter()
            .map(|entry| {
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .py_1()
                    .child(
                        div()
                            .min_w(px(140.))
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(entry.name.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_family("Menlo")
                                    .overflow_hidden()
                                    .child(entry.value.clone()),
                            )
                            .child(small_hint(
                                &format!("OID: {}", entry.oid),
                                theme.colors.muted_foreground,
                            )),
                    )
            })
            .collect();
        subject_card = subject_card.child(ui::info_rows(subject_rows, cx));
    }

    // 公钥信息
    let public_key = &info.public_key;
    let fingerprint = public_key.fingerprint_sha256.clone();
    let key_rows = vec![
        ui::info_row(
            "密钥类型",
            key_line(
                &public_key.key_type,
                public_key.key_size,
                public_key.curve_name.as_deref(),
            ),
            cx,
        ),
        action_row(
            "公钥指纹 (SHA-256)",
            fingerprint.clone(),
            ui::copy_button("csr-fp-copy", fingerprint),
            cx,
        ),
    ];
    let key_card = ui::card(cx)
        .child(ui::card_header("公钥信息", cx))
        .child(ui::info_rows(key_rows, cx))
        .child(
            div()
                .mt_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child("公钥 PEM（SPKI 格式，可用于比对私钥是否与 CSR 匹配）"),
                        )
                        .child(ui::copy_button(
                            "csr-spki-copy",
                            public_key.spki_pem.clone(),
                        )),
                )
                .child(ui::result_area(spki_view, 128.)),
        );

    let mut container = div().flex().flex_col().gap_3().child(basic_card);
    container = container.child(subject_card).child(key_card);

    // 私钥校验（仅提供私钥时）
    if let Some(key) = &info.private_key {
        let check_badge = if key.matches_csr {
            ui::badge("✓ 私钥与 CSR 匹配", theme.colors.success)
        } else {
            ui::badge("✗ 私钥与 CSR 不匹配", theme.colors.danger)
        };
        let key_fp = key.fingerprint_sha256.clone();
        let key_rows = vec![
            ui::info_row("私钥格式", key.format.clone(), cx),
            ui::info_row(
                "密钥类型",
                key_line(&key.key_type, key.key_size, key.curve_name.as_deref()),
                cx,
            ),
            action_row(
                "公钥指纹 (SHA-256)",
                key_fp.clone(),
                ui::copy_button("csr-key-fp-copy", key_fp),
                cx,
            ),
        ];
        let mut key_card = ui::card(cx)
            .child(ui::card_header("私钥校验", cx).child(check_badge))
            .child(ui::info_rows(key_rows, cx));
        if !key.matches_csr {
            key_card = key_card.child(note_line(
                "该私钥与 CSR 中的公钥不一致：请确认是否拿错了密钥文件",
                theme.colors.danger,
            ));
        }
        container = container.child(key_card);
    }

    // 请求的扩展
    let mut ext_card = ui::card(cx).child(ui::card_header("请求的扩展", cx));
    if info.requested_extensions.is_empty() {
        ext_card = ext_card.child(small_hint(
            "该 CSR 未包含扩展请求（SAN、Key Usage 等也可能在 CA 签发时补充）",
            theme.colors.muted_foreground,
        ));
    } else {
        for ext in &info.requested_extensions {
            ext_card = ext_card.child(
                ui::panel(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.colors.foreground)
                                    .child(ext.name.clone()),
                            )
                            .when(ext.critical, |el| {
                                el.child(ui::badge("critical", theme.colors.danger))
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_family("Menlo")
                            .text_color(theme.colors.muted_foreground)
                            .child(ext.value.clone()),
                    ),
            );
        }
    }

    // 原始 CSR（PEM）：复制 + 下载 .csr 在 render 里经 listener 追加（见下）
    container.child(ext_card)
}

impl Render for CsrViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let error = self.error.clone();
        let show_empty = self.result.is_none() && self.error.is_none();

        // 输入卡片
        let parse_label = if self.loading {
            "解析中…"
        } else {
            "解析 CSR"
        };
        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(ui::secondary_button(
                "csr-pick",
                "选择文件",
                cx.listener(|this, _, window, cx| this.pick_file(window, cx)),
            ))
            .child(
                ui::primary_button(
                    "csr-parse",
                    parse_label,
                    cx.listener(|this, _, window, cx| this.parse(window, cx)),
                )
                .loading(self.loading)
                .disabled(self.loading),
            )
            .child(ui::secondary_button(
                "csr-clear",
                "清空",
                cx.listener(|this, _, window, cx| this.clear(window, cx)),
            ));

        let input_card = ui::card(cx)
            .child(ui::card_header("CSR 内容", cx).child(actions))
            .when_some(self.file_name.clone(), |el, name| {
                el.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!("已选择文件： {name}")),
                )
            })
            .child(ui::field(
                "CSR 文件内容",
                ui::edit_area(&self.csr_text, 120.),
                cx,
            ))
            .child(ui::field(
                "私钥（可选）— 提供 PKCS#8 / PKCS#1 / SEC1 私钥，校验其与 CSR 公钥是否匹配",
                ui::edit_area(&self.key_text, 96.),
                cx,
            ));

        let mut page = ui::tool_page("csrviewer").child(input_card);

        if let Some(e) = error {
            page = page.child(ui::error_banner(&e, cx));
        }

        if show_empty {
            page = page.child(ui::empty_hint(
                "上传或粘贴 CSR 文件（PEM / Base64 / DER 格式）后点击「解析 CSR」",
                cx,
            ));
        }

        if let Some(info) = self.result.clone() {
            let download = ui::secondary_button(
                "csr-pem-dl",
                "下载 .csr",
                cx.listener(|this, _, window, cx| this.download_pem(window, cx)),
            );
            let pem = info.pem.clone();
            let mut cards = render_result(&info, &self.spki_view, &self.pem_view, cx);
            cards = cards.child(
                ui::card(cx)
                    .child(
                        ui::card_header("原始 CSR（PEM）", cx).child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(ui::copy_button("csr-pem-copy", pem))
                                .child(download),
                        ),
                    )
                    .child(ui::result_area(&self.pem_view, 160.)),
            );
            page = page.child(cards);
        }

        page
    }
}
