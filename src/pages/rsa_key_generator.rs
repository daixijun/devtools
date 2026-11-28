//! RSA 密钥对生成器。
//!
//! 私钥与公钥共用一套「密钥格式」家族选择：PKCS#8（通用，默认）/
//! PKCS#1（传统 RSA）/ OpenSSH（SSH 场景）——密钥并非都用于 SSH，
//! 因此 OpenSSH 只是其中一个选项且不作为默认。选 OpenSSH 时私钥仍以
//! PKCS#8 PEM 导出（OpenSSH 兼容），公钥为 OpenSSH 格式，注释输入框
//! 仅在该格式下显示。

use futures::StreamExt;
use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, IntoElement,
    ParentElement as _, Render, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _, Sizable as _,
};

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::{file_save, ui};
use devtools::i18n::t;
use devtools::tools::rsa_key_generator::{RsaGenerateRequest, RsaKeyPair};

/// 密钥格式家族：私钥与公钥统一选择。
#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyFormat {
    /// PKCS#8 私钥 + SPKI 公钥（证书 / JWT / TLS 等通用场景）。
    Pkcs8,
    /// PKCS#1 私钥 + PKCS#1 公钥（传统 RSA 专用格式）。
    Pkcs1,
    /// OpenSSH 公钥（私钥仍为 PKCS#8 PEM，OpenSSH 兼容）。
    OpenSsh,
}

impl KeyFormat {
    /// 下拉选项（value 为持久化字符串）。
    const OPTIONS: &[(&str, &str)] = &[
        ("pkcs8", "PKCS#8（通用）"),
        ("pkcs1", "PKCS#1（传统 RSA）"),
        ("openssh", "OpenSSH（SSH）"),
    ];

    fn from_value(value: &str) -> Self {
        match value {
            "pkcs1" => KeyFormat::Pkcs1,
            "openssh" => KeyFormat::OpenSsh,
            _ => KeyFormat::Pkcs8,
        }
    }

    fn id(self) -> &'static str {
        match self {
            KeyFormat::Pkcs8 => "pkcs8",
            KeyFormat::Pkcs1 => "pkcs1",
            KeyFormat::OpenSsh => "openssh",
        }
    }

    /// 私钥 PEM 文本。
    fn private_text(self, pair: &RsaKeyPair) -> &str {
        match self {
            // OpenSSH 家族的私钥统一导出 PKCS#8 PEM（OpenSSH 可直接读取）
            KeyFormat::Pkcs1 => &pair.private_pkcs1_pem,
            _ => &pair.private_pkcs8_pem,
        }
    }

    /// 公钥文本。
    fn public_text(self, pair: &RsaKeyPair) -> &str {
        match self {
            KeyFormat::Pkcs8 => &pair.public_spki_pem,
            KeyFormat::Pkcs1 => &pair.public_pkcs1_pem,
            KeyFormat::OpenSsh => &pair.public_openssh,
        }
    }

    /// 私钥卡说明。
    fn private_hint(self) -> &'static str {
        match self {
            KeyFormat::Pkcs8 => "PKCS#8 PEM —— 证书签发、JWT RS*、TLS 等通用场景的标准格式",
            KeyFormat::Pkcs1 => "PKCS#1 PEM —— 传统 RSA 专用格式（BEGIN RSA PRIVATE KEY）",
            KeyFormat::OpenSsh => "私钥以 PKCS#8 PEM 导出（OpenSSH 兼容读取）",
        }
    }

    /// 公钥卡说明。
    fn public_hint(self) -> &'static str {
        match self {
            KeyFormat::Pkcs8 => "SPKI PEM —— 证书 / TLS / JWT 公钥通用格式",
            KeyFormat::Pkcs1 => "PKCS#1 PEM —— 传统 RSA 公钥格式（BEGIN RSA PUBLIC KEY）",
            KeyFormat::OpenSsh => "可直接粘贴到服务器的 authorized_keys 中",
        }
    }

    /// 公钥下载扩展名（OpenSSH 为 .pub，其余 .pem）。
    fn public_extension(self) -> (&'static str, &'static str) {
        match self {
            KeyFormat::OpenSsh => (t("公钥文件"), "pub"),
            _ => (t("PEM文件"), "pem"),
        }
    }
}

pub struct RsaKeyGenerator {
    key_size_select: Entity<SelectState<SimpleSelectDelegate>>,
    key_size: u32,
    key_format_select: Entity<SelectState<SimpleSelectDelegate>>,
    key_format: KeyFormat,
    /// OpenSSH 注释（可选，Enter 触发生成）——仅 OpenSSH 格式显示。
    comment: Entity<InputState>,
    /// 私钥输出卡只读文本。
    private_view: Entity<TextareaState>,
    /// 公钥输出卡只读文本。
    public_view: Entity<TextareaState>,
    result: Option<RsaKeyPair>,
    loading: bool,
    error: Option<String>,
    /// Enter 触发的生成请求（订阅回调无 window，render 中消费）。
    pending_generate: bool,
}

impl RsaKeyGenerator {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let key_size_select = select_state(
            window,
            cx,
            &[
                ("2048", t("2048 位（推荐）")),
                ("3072", t("3072 位")),
                ("4096", t("4096 位")),
            ],
            0,
        );
        cx.subscribe(
            &key_size_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.key_size = value.parse().unwrap_or(2048);
                }
            },
        )
        .detach();

        // 密钥格式家族（默认 PKCS#8 通用格式）；标签经 i18n 翻译后传入下拉
        let format_options: Vec<(&str, &str)> = KeyFormat::OPTIONS
            .iter()
            .copied()
            .map(|(value, label)| (value, t(label)))
            .collect();
        let key_format_select = select_state(window, cx, &format_options, 0);
        cx.subscribe_in(
            &key_format_select,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.key_format = KeyFormat::from_value(value);
                    // 切换格式后若已有结果，刷新两卡文本
                    if let Some(pair) = this.result.clone() {
                        let private = this.key_format.private_text(&pair).to_string();
                        let public = this.key_format.public_text(&pair).to_string();
                        this.private_view
                            .update(cx, |out, cx| out.set_value(&private, window, cx));
                        this.public_view
                            .update(cx, |out, cx| out.set_value(&public, window, cx));
                    }
                    cx.notify();
                }
            },
        )
        .detach();

        let comment =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("如 user@host，留空使用默认值")));
        // Enter 直接触发生成（回调里无 window，置标记后在 render 消费）
        cx.subscribe(&comment, |this, _, event: &InputEvent, _cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.pending_generate = true;
            }
        })
        .detach();

        Self {
            key_size_select,
            key_size: 2048,
            key_format_select,
            key_format: KeyFormat::Pkcs8,
            comment,
            private_view: ui::new_textarea(window, cx),
            public_view: ui::new_textarea(window, cx),
            result: None,
            loading: false,
            error: None,
            pending_generate: false,
        }
    }

    /// 生成密钥对（CPU 密集，openssl 2048 位约秒级，走 core_rt 后台 tokio）。
    fn generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let comment = self.comment.read(cx).value().trim().to_string();
        let key_size = self.key_size;
        self.loading = true;
        self.error = None;
        self.result = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            devtools::tools::rsa_key_generator::generate_rsa_keypair(RsaGenerateRequest {
                key_size,
                comment: if comment.is_empty() {
                    None
                } else {
                    Some(comment)
                },
            })
            .await
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = StreamExt::next(&mut rx).await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.on_generated(result, window, cx);
                });
            }
        })
        .detach();
    }

    fn on_generated(
        &mut self,
        result: Result<RsaKeyPair, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(pair) => {
                self.error = None;
                let format = self.key_format;
                let private = format.private_text(&pair).to_string();
                let public = format.public_text(&pair).to_string();
                self.private_view
                    .update(cx, |out, cx| out.set_value(&private, window, cx));
                self.public_view
                    .update(cx, |out, cx| out.set_value(&public, window, cx));
                self.result = Some(pair);
            }
            Err(message) => {
                self.error = Some(message);
                self.result = None;
            }
        }
        cx.notify();
    }

    /// 下载私钥（默认名 rsa_private_key_{格式}.pem）。
    fn download_private(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pair) = self.result.clone() else {
            return;
        };
        let format = self.key_format;
        file_save::save_text_file(
            file_save::SaveTextParams {
                content: format.private_text(&pair).to_string(),
                default_name: format!("rsa_private_key_{}.pem", format.id()),
                filter_name: t("PEM文件"),
                extension: "pem",
                // 私钥等敏感内容：Unix 下以 0600 落盘
                private: true,
            },
            window,
            cx,
        );
    }

    /// 下载公钥（默认名 rsa_public_key_{格式}.{ext}，OpenSSH 为 .pub）。
    fn download_public(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pair) = self.result.clone() else {
            return;
        };
        let format = self.key_format;
        let (filter_name, extension) = format.public_extension();
        file_save::save_text_file(
            file_save::SaveTextParams {
                content: format.public_text(&pair).to_string(),
                default_name: format!(
                    "rsa_public_key_{}.{}",
                    format.id(),
                    format.public_extension().1
                ),
                filter_name,
                extension,
                private: false,
            },
            window,
            cx,
        );
    }

    /// 消费 Enter 置位的生成请求（render 开头调用，此时有 window）。
    fn take_pending_generate(&mut self) -> bool {
        std::mem::take(&mut self.pending_generate)
    }
}

impl Render for RsaKeyGenerator {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Enter 触发的生成请求
        if self.take_pending_generate() {
            self.generate(window, cx);
        }

        let format = self.key_format;
        let mut page = ui::tool_page("rsakeygenerator")
            // 参数设置卡：密钥长度 + 密钥格式（私钥/公钥统一）+ 生成按钮；
            // OpenSSH 注释仅在该格式下显示（密钥并非都用于 SSH）
            .child(
                ui::card(cx).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_end()
                        .gap_4()
                        .child(ui::field(
                            t("密钥长度"),
                            Select::new(&self.key_size_select),
                            cx,
                        ))
                        .child(ui::field(
                            t("密钥格式"),
                            Select::new(&self.key_format_select),
                            cx,
                        ))
                        .when(format == KeyFormat::OpenSsh, |row| {
                            row.child(ui::field(
                                t("OpenSSH 注释（可选）"),
                                Input::new(&self.comment).w(px(288.)),
                                cx,
                            ))
                        })
                        .child(
                            Button::new("generate")
                                .label(if self.loading {
                                    t("生成中…")
                                } else {
                                    t("生成密钥对")
                                })
                                .primary()
                                .loading(self.loading)
                                .disabled(self.loading)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.generate(window, cx);
                                })),
                        ),
                ),
            );

        // 错误横条
        if let Some(message) = &self.error {
            page = page.child(ui::error_banner(message, cx));
        }

        // 空态
        if self.result.is_none() && self.error.is_none() {
            page = page.child(ui::empty_hint(
                t("选择密钥长度后点击「生成密钥对」，生成的私钥请妥善保管"),
                cx,
            ));
        }

        // 私钥输出卡
        if let Some(pair) = self.result.clone() {
            let private_pair_text = format.private_text(&pair).to_string();
            let public_pair_text = format.public_text(&pair).to_string();

            // 私钥卡（格式由顶部统一选择，此处只展示 + 复制/下载）
            page = page.child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
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
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(cx.theme().colors.foreground)
                                            .child(t("私钥")),
                                    )
                                    .child(ui::hint_text(t(format.private_hint()), cx))
                                    .child(ui::hint_text(
                                        t("泄露私钥将危及所有依赖它的系统，切勿提交到代码仓库或聊天工具"),
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_2()
                                    .child(ui::copy_result_button(
                                        "copy-private",
                                        t("复制私钥"),
                                        private_pair_text,
                                    ))
                                    .child(
                                        Button::new("download-private")
                                            .label(t("下载 .pem"))
                                            .small()
                                            .secondary()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.download_private(window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(ui::result_area(&self.private_view, 256.)),
            );

            // 公钥卡
            page = page.child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
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
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(cx.theme().colors.foreground)
                                            .child(t("公钥")),
                                    )
                                    .child(ui::hint_text(t(format.public_hint()), cx)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_2()
                                    .child(ui::copy_result_button(
                                        "copy-public",
                                        t("复制公钥"),
                                        public_pair_text,
                                    ))
                                    .child(
                                        Button::new("download-public")
                                            .label(format!(
                                                "{} .{}",
                                                t("下载"),
                                                format.public_extension().1
                                            ))
                                            .small()
                                            .secondary()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.download_public(window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(ui::result_area(&self.public_view, 160.)),
            );
        }

        page
    }
}
