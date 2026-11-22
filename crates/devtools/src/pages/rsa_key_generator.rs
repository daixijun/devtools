//! RSA 密钥对生成器。
//!
//! 密钥长度下拉 + OpenSSH 注释输入（Enter 触发）+ 双输出卡（格式 Tab / 复制 / 下载）。

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
use crate::components::{notify, ui};
use devtools::tools::rsa_key_generator::{RsaGenerateRequest, RsaKeyPair};

/// 私钥输出格式。
#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivateFormat {
    Pkcs8,
    Pkcs1,
}

/// 公钥输出格式。
#[derive(Clone, Copy, PartialEq, Eq)]
enum PublicFormat {
    Pem,
    Pkcs1,
    OpenSsh,
}

impl PrivateFormat {
    fn label(self) -> &'static str {
        match self {
            PrivateFormat::Pkcs8 => "PKCS#8",
            PrivateFormat::Pkcs1 => "PKCS#1",
        }
    }

    /// 默认下载文件名中的格式标识。
    fn id(self) -> &'static str {
        match self {
            PrivateFormat::Pkcs8 => "pkcs8",
            PrivateFormat::Pkcs1 => "pkcs1",
        }
    }

    /// 下载文件扩展名。
    fn extension(self) -> &'static str {
        "pem"
    }

    fn all() -> [PrivateFormat; 2] {
        [PrivateFormat::Pkcs8, PrivateFormat::Pkcs1]
    }
}

impl PublicFormat {
    fn label(self) -> &'static str {
        match self {
            PublicFormat::Pem => "PEM",
            PublicFormat::Pkcs1 => "PKCS#1",
            PublicFormat::OpenSsh => "OpenSSH",
        }
    }

    /// 下载文件扩展名（OpenSSH 为 .pub，其余 .pem）。
    fn extension(self) -> &'static str {
        match self {
            PublicFormat::OpenSsh => "pub",
            _ => "pem",
        }
    }

    fn id(self) -> &'static str {
        match self {
            PublicFormat::Pem => "pem",
            PublicFormat::Pkcs1 => "pkcs1",
            PublicFormat::OpenSsh => "openssh",
        }
    }

    fn all() -> [PublicFormat; 3] {
        [
            PublicFormat::Pem,
            PublicFormat::Pkcs1,
            PublicFormat::OpenSsh,
        ]
    }
}

fn private_text(pair: &RsaKeyPair, format: PrivateFormat) -> &str {
    match format {
        PrivateFormat::Pkcs8 => &pair.private_pkcs8_pem,
        PrivateFormat::Pkcs1 => &pair.private_pkcs1_pem,
    }
}

fn public_text(pair: &RsaKeyPair, format: PublicFormat) -> &str {
    match format {
        PublicFormat::Pem => &pair.public_spki_pem,
        PublicFormat::Pkcs1 => &pair.public_pkcs1_pem,
        PublicFormat::OpenSsh => &pair.public_openssh,
    }
}

pub struct RsaKeyGenerator {
    key_size_select: Entity<SelectState<SimpleSelectDelegate>>,
    key_size: u32,
    /// OpenSSH 注释（可选，Enter 触发生成）。
    comment: Entity<InputState>,
    /// 私钥输出卡只读文本。
    private_view: Entity<TextareaState>,
    /// 公钥输出卡只读文本。
    public_view: Entity<TextareaState>,
    result: Option<RsaKeyPair>,
    private_format: PrivateFormat,
    public_format: PublicFormat,
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
                ("2048", "2048 位（推荐）"),
                ("3072", "3072 位"),
                ("4096", "4096 位"),
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

        let comment =
            cx.new(|cx| InputState::new(window, cx).placeholder("如 user@host，留空使用默认值"));
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
            comment,
            private_view: ui::new_textarea(window, cx),
            public_view: ui::new_textarea(window, cx),
            result: None,
            private_format: PrivateFormat::Pkcs8,
            public_format: PublicFormat::Pem,
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
                let private = private_text(&pair, self.private_format).to_string();
                let public = public_text(&pair, self.public_format).to_string();
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

    fn set_private_format(
        &mut self,
        format: PrivateFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.private_format = format;
        if let Some(pair) = &self.result {
            let text = private_text(pair, format).to_string();
            self.private_view
                .update(cx, |out, cx| out.set_value(&text, window, cx));
        }
        cx.notify();
    }

    fn set_public_format(
        &mut self,
        format: PublicFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.public_format = format;
        if let Some(pair) = &self.result {
            let text = public_text(pair, format).to_string();
            self.public_view
                .update(cx, |out, cx| out.set_value(&text, window, cx));
        }
        cx.notify();
    }

    /// 下载私钥（默认名 rsa_private_key_{格式}.pem）。
    fn download_private(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pair) = self.result.clone() else {
            return;
        };
        let text = private_text(&pair, self.private_format).to_string();
        let default_name = format!("rsa_private_key_{}.pem", self.private_format.id());
        self.save_text_file(text, default_name, "PEM Files", "pem", window, cx);
    }

    /// 下载公钥（默认名 rsa_public_key_{格式}.{ext}，OpenSSH 为 .pub）。
    fn download_public(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pair) = self.result.clone() else {
            return;
        };
        let format = self.public_format;
        let text = public_text(&pair, format).to_string();
        let default_name = format!("rsa_public_key_{}.{}", format.id(), format.extension());
        let (filter_name, extension) = if format.extension() == "pub" {
            ("Public Key Files", "pub")
        } else {
            ("PEM Files", "pem")
        };
        self.save_text_file(text, default_name, filter_name, extension, window, cx);
    }

    /// 通用文本保存（rfd save_file + std::fs::write，均在后台线程）。
    fn save_text_file(
        &mut self,
        content: String,
        default_name: String,
        filter_name: &'static str,
        extension: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut rx = crate::core_rt::submit(cx, async move {
            match rfd::FileDialog::new()
                .add_filter(filter_name, &[extension])
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
                let _ = this.update_in(cx, |_this, window, cx| match result {
                    None => {} // 用户取消
                    Some(Ok(_)) => {
                        notify::toast_success(window, cx, "文件保存成功");
                    }
                    Some(Err(message)) => {
                        notify::toast_error(window, cx, message);
                    }
                });
            }
        })
        .detach();
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

        let mut page = ui::tool_page("rsakeygenerator")
            // 参数设置卡
            .child(
                ui::card(cx).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_end()
                        .gap_4()
                        .child(ui::field(
                            "密钥长度",
                            Select::new(&self.key_size_select),
                            cx,
                        ))
                        .child(ui::field(
                            "OpenSSH 注释（可选）",
                            Input::new(&self.comment).w(px(288.)),
                            cx,
                        ))
                        .child(
                            Button::new("generate")
                                .label(if self.loading {
                                    "生成中…"
                                } else {
                                    "生成密钥对"
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
                "选择密钥长度后点击「生成密钥对」，生成的私钥请妥善保管",
                cx,
            ));
        }

        // 私钥输出卡
        if let Some(pair) = self.result.clone() {
            let private_pair_text = private_text(&pair, self.private_format).to_string();
            let public_pair_text = public_text(&pair, self.public_format).to_string();

            // 私钥格式 Tab（自制按钮组）
            let private_tabs = PrivateFormat::all().map(|format| {
                let is_active = format == self.private_format;
                Button::new(match format {
                    PrivateFormat::Pkcs8 => "tab-private-pkcs8",
                    PrivateFormat::Pkcs1 => "tab-private-pkcs1",
                })
                .label(format.label())
                .xsmall()
                .when(is_active, |b| b.primary())
                .when(!is_active, |b| b.secondary())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_private_format(format, window, cx);
                }))
            });

            // 公钥格式 Tab
            let public_tabs = PublicFormat::all().map(|format| {
                let is_active = format == self.public_format;
                Button::new(match format {
                    PublicFormat::Pem => "tab-public-pem",
                    PublicFormat::Pkcs1 => "tab-public-pkcs1",
                    PublicFormat::OpenSsh => "tab-public-openssh",
                })
                .label(format.label())
                .xsmall()
                .when(is_active, |b| b.primary())
                .when(!is_active, |b| b.secondary())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_public_format(format, window, cx);
                }))
            });

            // 私钥卡
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
                                            .child("私钥"),
                                    )
                                    .child(ui::hint_text(
                                        "泄露私钥将危及所有依赖它的系统，切勿提交到代码仓库或聊天工具",
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .children(private_tabs),
                                    )
                                    .child(ui::copy_button(
                                        "copy-private",
                                        private_pair_text,
                                    ))
                                    .child(
                                        Button::new("download-private")
                                            .label(format!(
                                                "下载 .{}",
                                                self.private_format.extension()
                                            ))
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
                                            .child("公钥"),
                                    )
                                    .child(ui::hint_text(
                                        "OpenSSH 格式可直接粘贴到服务器的 authorized_keys 中",
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div().flex().items_center().gap_1().children(public_tabs),
                                    )
                                    .child(ui::copy_button("copy-public", public_pair_text))
                                    .child(
                                        Button::new("download-public")
                                            .label(format!(
                                                "下载 .{}",
                                                self.public_format.extension()
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
