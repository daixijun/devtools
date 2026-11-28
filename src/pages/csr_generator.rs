//! CSR 生成。
//!
//! 表单（CN 必填 / RSA·ECC 密钥 / 主题字段 / SAN）→ core `generate_csr`
//! 异步生成（openssl，秒级）；输出 CSR / 私钥 / 公钥三卡，各带复制 + 下载
//! （rfd save_file 后台线程 + std::fs::write，成功 toast）。

use futures::StreamExt as _;
use gpui::{div, prelude::*, px, App, Context, Entity, IntoElement, Render, SharedString, Window};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputEvent, InputState, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::select::SimpleSelectDelegate;
use crate::components::{notify, select_state, ui};
use devtools::i18n::t;
use devtools::tools::csr_generator::{generate_csr, CsrGenerateRequest, GeneratedCsr};

/// 文件保存结果（rfd save_file + std::fs::write，均在 core_rt 后台任务中执行）。
enum SaveOutcome {
    Cancelled,
    Saved,
    Failed(String),
}

pub struct CsrGenerator {
    // 主题字段
    common_name: Entity<InputState>,
    country: Entity<InputState>,
    state: Entity<InputState>,
    locality: Entity<InputState>,
    organization: Entity<InputState>,
    org_unit: Entity<InputState>,
    email: Entity<InputState>,
    /// SAN：每行一个域名或 IP，IP 自动识别。
    san_text: Entity<TextareaState>,

    // 密钥参数（下拉）
    key_type_sel: Entity<SelectState<SimpleSelectDelegate>>,
    rsa_size_sel: Entity<SelectState<SimpleSelectDelegate>>,
    ec_curve_sel: Entity<SelectState<SimpleSelectDelegate>>,
    /// "rsa" | "ec"
    key_type: SharedString,
    rsa_key_size: u32,
    /// "P-256" | "P-384" | "P-521"
    ec_curve: SharedString,

    result: Option<GeneratedCsr>,
    loading: bool,
    error: Option<String>,
}

impl CsrGenerator {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let common_name = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 example.com")));
        let country = cx.new(|cx| InputState::new(window, cx).placeholder(t("两位字母，如 CN")));
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 Shanghai")));
        let locality = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 Shanghai")));
        let organization = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 DevTools")));
        let org_unit = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 Engineering")));
        let email = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 admin@example.com")));
        let san_text = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder("example.com\nwww.example.com\n10.0.0.1")
        });

        let key_type_sel = select_state(window, cx, &[("rsa", "RSA"), ("ec", "ECC")], 0);
        let rsa_size_sel = select_state(
            window,
            cx,
            &[
                ("2048", t("2048 位（推荐）")),
                ("3072", t("3072 位")),
                ("4096", t("4096 位")),
            ],
            0,
        );
        let ec_curve_sel = select_state(
            window,
            cx,
            &[
                ("P-256", t("P-256（prime256v1）")),
                ("P-384", t("P-384（secp384r1）")),
                ("P-521", t("P-521（secp521r1）")),
            ],
            0,
        );

        // CN 输入框回车直接生成
        cx.subscribe(&common_name, |this, _, event: &InputEvent, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.generate(cx);
            }
        })
        .detach();

        cx.subscribe(
            &key_type_sel,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.key_type = v.clone();
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe(
            &rsa_size_sel,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.rsa_key_size = v.parse().unwrap_or(2048);
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe(
            &ec_curve_sel,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.ec_curve = v.clone();
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            common_name,
            country,
            state,
            locality,
            organization,
            org_unit,
            email,
            san_text,
            key_type_sel,
            rsa_size_sel,
            ec_curve_sel,
            key_type: SharedString::from("rsa"),
            rsa_key_size: 2048,
            ec_curve: SharedString::from("P-256"),
            result: None,
            loading: false,
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // 交互
    // ------------------------------------------------------------------

    fn input_value(entity: &Entity<InputState>, cx: &App) -> Option<String> {
        let value = entity.read(cx).value().trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    /// 生成 CSR（openssl 生成秒级，异步走 core_rt）。
    fn generate(&mut self, cx: &mut Context<Self>) {
        let Some(common_name) = Self::input_value(&self.common_name, cx) else {
            self.error = Some(t("请填写通用名称（CN），通常为要申请证书的域名").to_string());
            cx.notify();
            return;
        };

        let sans: Vec<String> = self
            .san_text
            .read(cx)
            .value()
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        let request = CsrGenerateRequest {
            common_name,
            country: Self::input_value(&self.country, cx),
            state: Self::input_value(&self.state, cx),
            locality: Self::input_value(&self.locality, cx),
            organization: Self::input_value(&self.organization, cx),
            org_unit: Self::input_value(&self.org_unit, cx),
            email: Self::input_value(&self.email, cx),
            key_type: self.key_type.to_string(),
            key_size: (self.key_type == "rsa").then_some(self.rsa_key_size),
            curve: (self.key_type == "ec").then(|| self.ec_curve.to_string()),
            sans: (!sans.is_empty()).then_some(sans),
        };

        self.loading = true;
        self.error = None;
        self.result = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move { generate_csr(request).await });

        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update(cx, |this, cx| this.on_generated(result, cx));
            }
        })
        .detach();
    }

    fn on_generated(&mut self, result: Result<GeneratedCsr, String>, cx: &mut Context<Self>) {
        self.loading = false;
        match result {
            Ok(generated) => {
                self.result = Some(generated);
                self.error = None;
            }
            Err(message) => {
                self.error = Some(message);
                self.result = None;
            }
        }
        cx.notify();
    }

    /// 保存单张输出卡的内容：对话框与写文件都在后台任务中完成，成功 toast。
    fn download(
        &mut self,
        content: String,
        default_name: String,
        ext: &'static str,
        filter_name: &'static str,
        // 私钥等敏感内容：Unix 下以 0600 落盘
        private: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut rx = crate::core_rt::submit(cx, async move {
            let Some(path) = rfd::FileDialog::new()
                .add_filter(filter_name, &[ext])
                .set_file_name(default_name)
                .save_file()
            else {
                return SaveOutcome::Cancelled;
            };
            match devtools::utils::fs::write_text_file(&path, &content, private) {
                Ok(_) => SaveOutcome::Saved,
                Err(e) => SaveOutcome::Failed(format!("{}: {}", t("保存文件失败"), e)),
            }
        });

        cx.spawn_in(window, async move |this, cx| {
            if let Some(outcome) = rx.next().await {
                let _ = this.update_in(cx, |_this, window, cx| match outcome {
                    SaveOutcome::Cancelled => {}
                    SaveOutcome::Saved => notify::toast_success(window, cx, t("文件保存成功")),
                    SaveOutcome::Failed(message) => notify::toast_error(window, cx, message),
                });
            }
        })
        .detach();
    }
}

// ----------------------------------------------------------------------
// 展示辅助
// ----------------------------------------------------------------------

/// 文件名安全化：非 `[a-zA-Z0-9._-]` 替换为 `_`，空则 fallback（照规格）。
fn safe_file_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn key_line(key_type: &str, key_size: Option<u32>, curve_name: Option<&str>) -> String {
    let mut line = key_type.to_string();
    if let Some(bits) = key_size {
        line.push_str(&format!(" · {} {}", bits, t("位")));
    }
    if let Some(curve) = curve_name {
        line.push_str(&format!(" · {} {}", t("曲线"), curve));
    }
    line
}

/// 只读等宽 PEM 展示块（可拖选复制，自带滚动）。
fn pem_block(id: &'static str, text: &str, height: f32, cx: &App) -> gpui::Stateful<gpui::Div> {
    ui::mono_block(id, text, cx).h(px(height))
}

/// 输出卡：标题 + 描述 + 复制/下载按钮 + 只读 PEM 块。
#[allow(clippy::too_many_arguments)]
fn output_card(
    block_id: &'static str,
    copy_id: &'static str,
    download_id: &'static str,
    title: &'static str,
    description: &str,
    content: &str,
    height: f32,
    download_label: &'static str,
    on_download: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> gpui::Div {
    let theme = cx.theme().clone();
    ui::card(cx)
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.foreground)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(description.to_string()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(ui::copy_result_button(
                            copy_id,
                            t("复制"),
                            content.to_string(),
                        ))
                        .child(
                            Button::new(download_id)
                                .label(download_label)
                                .secondary()
                                .on_click(on_download),
                        ),
                ),
        )
        .child(pem_block(block_id, content, height, cx))
}

impl Render for CsrGenerator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = self.loading;
        let error = self.error.clone();
        let show_empty = self.result.is_none() && self.error.is_none();

        // 表单卡片
        let cn_field = ui::field(
            t("通用名称（CN，必填）"),
            gpui_component::input::Input::new(&self.common_name)
                .disabled(loading)
                .w(px(256.)),
            cx,
        );
        let key_type_field = ui::field(
            t("密钥类型"),
            Select::new(&self.key_type_sel).disabled(loading),
            cx,
        );
        let key_param_field = if self.key_type == "ec" {
            ui::field(
                t("ECC 曲线"),
                Select::new(&self.ec_curve_sel).disabled(loading),
                cx,
            )
        } else {
            ui::field(
                t("密钥长度"),
                Select::new(&self.rsa_size_sel).disabled(loading),
                cx,
            )
        };
        let generate_label = if loading {
            t("生成中…")
        } else {
            t("生成 CSR")
        };

        let row1 = div()
            .flex()
            .flex_wrap()
            .items_end()
            .gap_3()
            .child(cn_field)
            .child(key_type_field)
            .child(key_param_field)
            .child(
                ui::primary_button(
                    "csr-gen",
                    generate_label,
                    cx.listener(|this, _, _window, cx| this.generate(cx)),
                )
                .loading(loading)
                .disabled(loading),
            );

        let row2 = div()
            .flex()
            .flex_wrap()
            .items_end()
            .gap_3()
            .child(ui::field(
                t("国家（C）"),
                gpui_component::input::Input::new(&self.country)
                    .disabled(loading)
                    .w(px(128.)),
                cx,
            ))
            .child(ui::field(
                t("省份（ST）"),
                gpui_component::input::Input::new(&self.state)
                    .disabled(loading)
                    .w(px(160.)),
                cx,
            ))
            .child(ui::field(
                t("城市（L）"),
                gpui_component::input::Input::new(&self.locality)
                    .disabled(loading)
                    .w(px(160.)),
                cx,
            ))
            .child(ui::field(
                t("组织（O）"),
                gpui_component::input::Input::new(&self.organization)
                    .disabled(loading)
                    .w(px(176.)),
                cx,
            ))
            .child(ui::field(
                t("部门（OU）"),
                gpui_component::input::Input::new(&self.org_unit)
                    .disabled(loading)
                    .w(px(176.)),
                cx,
            ))
            .child(ui::field(
                t("邮箱"),
                gpui_component::input::Input::new(&self.email)
                    .disabled(loading)
                    .w(px(208.)),
                cx,
            ));

        let san_field = ui::field(
            t("SAN（可选，每行一个域名或 IP，IP 自动识别）"),
            gpui_component::input::Textarea::new(&self.san_text)
                .disabled(loading)
                .h(px(80.)),
            cx,
        );

        let form_card = ui::card(cx)
            .child(ui::card_header(t("参数设置"), cx))
            .child(row1)
            .child(row2)
            .child(san_field);

        let mut page = ui::tool_page("csrgenerator").child(form_card);

        if let Some(e) = error {
            page = page.child(ui::error_banner(&e, cx));
        }

        if show_empty {
            page = page.child(ui::empty_hint(
                t("填写主题信息后点击「生成 CSR」，生成的私钥请妥善保管"),
                cx,
            ));
        }

        if let Some(result) = self.result.clone() {
            page = page.child(self.render_output(&result, cx));
        }

        page
    }
}

impl CsrGenerator {
    /// 结果卡 + 三张输出卡（下载文件名按当前 CN 安全化，照规格规则）。
    fn render_output(&self, result: &GeneratedCsr, cx: &mut Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let container = div().flex().flex_col().gap_3();

        // 生成结果卡
        let mut rows: Vec<gpui::Div> = vec![
            ui::info_row(t("主题"), result.subject.clone(), cx),
            ui::info_row(
                t("密钥"),
                key_line(
                    &result.key_type,
                    result.key_size,
                    result.curve_name.as_deref(),
                ),
                cx,
            ),
        ];
        if !result.sans.is_empty() {
            let badges = div().flex().flex_wrap().gap_1().children(
                result
                    .sans
                    .iter()
                    .map(|san| ui::badge(san.as_str(), theme.colors.primary)),
            );
            rows.push(
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
                            .child("SAN"),
                    )
                    .child(badges),
            );
        }
        let summary = ui::card(cx)
            .child(ui::card_header(t("生成结果"), cx))
            .child(ui::info_rows(rows, cx));
        let container = container.child(summary);

        // 三张输出卡（下载按钮经 listener 捕获内容与文件名）
        let cn = Self::input_value(&self.common_name, cx).unwrap_or_default();
        let base = safe_file_name(&cn, "certificate_request");
        let entity = cx.entity();

        let make_download = |entity: gpui::Entity<Self>,
                             content: String,
                             default_name: String,
                             ext: &'static str,
                             filter_name: &'static str,
                             private: bool| {
            move |_ev: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.download(
                        content.clone(),
                        default_name.clone(),
                        ext,
                        filter_name,
                        private,
                        window,
                        cx,
                    );
                });
            }
        };

        let csr_card = output_card(
            "csr-out",
            "csr-out-copy",
            "csr-out-dl",
            t("证书签名请求（CSR）"),
            t("提交给 CA 签发证书时使用，可安全分享"),
            &result.csr_pem,
            192.,
            t("下载 .csr"),
            make_download(
                entity.clone(),
                result.csr_pem.clone(),
                format!("{base}.csr"),
                "csr",
                t("CSR 文件"),
                false,
            ),
            cx,
        );
        let key_card = output_card(
            "csr-key-out",
            "csr-key-out-copy",
            "csr-key-out-dl",
            t("私钥（PKCS#8）"),
            t("泄露私钥将危及对应证书的安全，切勿提交到代码仓库或聊天工具"),
            &result.private_key_pem,
            192.,
            t("下载 .key"),
            make_download(
                entity.clone(),
                result.private_key_pem.clone(),
                format!("{base}_private_key.key"),
                "key",
                t("私钥文件"),
                true,
            ),
            cx,
        );
        let pub_card = output_card(
            "csr-pub-out",
            "csr-pub-out-copy",
            "csr-pub-out-dl",
            t("公钥（SPKI）"),
            t("可用于核对私钥与 CSR 是否匹配"),
            &result.public_key_pem,
            128.,
            t("下载 .pem"),
            make_download(
                entity,
                result.public_key_pem.clone(),
                format!("{base}_public_key.pem"),
                "pem",
                t("PEM 文件"),
                false,
            ),
            cx,
        );

        container.child(csr_card).child(key_card).child(pub_card)
    }
}
