//! AES 加密。
//!
//! 独立加密工具（与 AES 解密对称）：文本 + 密钥 + IV（非 ECB）→ Base64
//! 密文。错误一律内联红条展示；「加载示例」填入可立即加密的示例数据。

use devtools::crypto::{aes_encrypt, generate_random_iv_hex, AesOptions};
use gpui::{div, prelude::*, px, Context, Entity, IntoElement, Render, SharedString, Window};
use gpui_component::{
    input::{Input, InputState, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _,
};

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::ui;
use crate::pages::aes_common::{
    aes_mode, aes_padding, key_len_bits, KEY_LEN_OPTIONS, MODE_OPTIONS, PADDING_OPTIONS,
};

pub struct AesEncrypt {
    text: Entity<TextareaState>,
    key: Entity<InputState>,
    iv: Entity<InputState>,
    result: Entity<TextareaState>,
    error: Option<String>,
    /// 共享 AES 配置，取值与下拉选项 value 一致：mode="CBC"，key_len="128"，padding="Pkcs7"。
    mode: SharedString,
    key_len: SharedString,
    padding: SharedString,
    mode_sel: Entity<SelectState<SimpleSelectDelegate>>,
    key_len_sel: Entity<SelectState<SimpleSelectDelegate>>,
    padding_sel: Entity<SelectState<SimpleSelectDelegate>>,
}

impl AesEncrypt {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode_sel = select_state(window, cx, MODE_OPTIONS, 0);
        let key_len_sel = select_state(window, cx, KEY_LEN_OPTIONS, 0);
        let padding_sel = select_state(window, cx, PADDING_OPTIONS, 0);

        cx.subscribe_in(
            &mode_sel,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.mode = value.clone();
                    cx.notify();
                }
            },
        )
        .detach();

        // 密钥长度切换：同步密钥输入的占位文案
        cx.subscribe_in(
            &key_len_sel,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.key_len = value.clone();
                    this.sync_key_placeholder(window, cx);
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &padding_sel,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.padding = value.clone();
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            text: cx.new(|cx| TextareaState::new(window, cx).placeholder("请输入要加密的文本")),
            result: cx.new(|cx| TextareaState::new(window, cx).placeholder("加密结果将显示在这里")),
            // 默认 128 位；密钥长度切换时由 sync_key_placeholder 更新占位文案
            key: cx.new(|cx| InputState::new(window, cx).placeholder("请输入128位密钥 (16 字符)")),
            iv: cx.new(|cx| InputState::new(window, cx).placeholder("请输入16字符的IV (128位)")),
            error: None,
            mode: SharedString::from("CBC"),
            key_len: SharedString::from("128"),
            padding: SharedString::from("Pkcs7"),
            mode_sel,
            key_len_sel,
            padding_sel,
        }
    }

    fn sync_key_placeholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let bits = key_len_bits(&self.key_len);
        let placeholder = format!("请输入{bits}位密钥 ({} 字符)", bits / 8);
        self.key.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx)
        });
    }

    /// 加密：前置校验（空文本/空密钥/非 ECB 空 IV）→ aes_encrypt → Base64 结果。
    fn run_encrypt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text.read(cx).value().to_string();
        let key = self.key.read(cx).value().to_string();
        let iv = self.iv.read(cx).value().to_string();
        let mode: &str = &self.mode;

        if text.is_empty() {
            self.error = Some("请输入要加密的文本".to_string());
        } else if key.is_empty() {
            self.error = Some("请输入密钥".to_string());
        } else if mode != "ECB" && iv.is_empty() {
            self.error = Some("当前模式需要初始化向量(IV)".to_string());
        } else {
            self.error = None;
            let opts = AesOptions {
                mode: aes_mode(mode),
                key_size: key_len_bits(&self.key_len),
                padding: aes_padding(&self.padding),
            };
            // ECB 模式忽略 IV（传空串）
            let iv_arg = if mode == "ECB" { "" } else { iv.as_str() };
            match aes_encrypt(&text, &key, iv_arg, &opts) {
                Ok(encrypted) => {
                    self.result
                        .update(cx, |state, cx| state.set_value(encrypted, window, cx));
                }
                Err(msg) => {
                    self.error = Some(format!("加密过程中发生错误: {msg}"));
                }
            }
        }
        cx.notify();
    }

    /// 随机生成 IV（32 位小写 hex）填入 IV 输入框。
    fn generate_iv(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let iv = generate_random_iv_hex();
        self.iv
            .update(cx, |state, cx| state.set_value(iv, window, cx));
        cx.notify();
    }

    /// 加载示例（明文 + 密钥 + IV，点击「加密」立即得到结果）。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.text.update(cx, |state, cx| {
            state.set_value("这是一段需要加密的示例文本", window, cx)
        });
        self.key.update(cx, |state, cx| {
            state.set_value("my-secret-key-16", window, cx)
        });
        self.iv.update(cx, |state, cx| {
            state.set_value("initialization-ve", window, cx)
        });
        cx.notify();
    }
}

impl Render for AesEncrypt {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_iv: bool = { self.mode.as_ref() != "ECB" };
        let text = self.text.read(cx).value().to_string();
        let result = self.result.read(cx).value().to_string();
        let bits = key_len_bits(&self.key_len);

        // 文本输入 | 结果（label 行右侧浮动复制按钮，仅结果非空显示）
        let input_row = div()
            .flex()
            .gap_4()
            .child(ui::field("要加密的文本", ui::edit_area(&self.text, 96.), cx).flex_1())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(cx.theme().colors.muted_foreground)
                                    .child("加密结果"),
                            )
                            .when(!result.is_empty(), |el| {
                                el.child(ui::copy_result_button(
                                    "aesencrypt-copy",
                                    "复制结果",
                                    result.clone(),
                                ))
                            }),
                    )
                    .child(ui::result_area(&self.result, 96.)),
            );

        // 密钥 | IV（mode≠ECB 时显示）
        let mut key_iv_row = div().flex().gap_4().child(
            ui::field("密钥", Input::new(&self.key), cx)
                .child(ui::hint_text(
                    &format!("密钥长度必须为 {} 字符 ({} 位)", bits / 8, bits),
                    cx,
                ))
                .flex_1(),
        );
        if show_iv {
            key_iv_row = key_iv_row.child(
                ui::field(
                    "初始化向量 (IV)",
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&self.iv)))
                        .child(ui::secondary_button(
                            "aesencrypt-iv-gen",
                            "随机生成",
                            cx.listener(|this, _, window, cx| this.generate_iv(window, cx)),
                        )),
                    cx,
                )
                .child(ui::hint_text("IV长度必须为16字符 (128位)", cx))
                .flex_1(),
            );
        }

        let mut page = ui::tool_page("aesencrypt")
            // AES 配置卡片
            .child(
                ui::card(cx).child(ui::card_header("AES 配置", cx)).child(
                    div()
                        .flex()
                        .gap_4()
                        .child(ui::field("模式", Select::new(&self.mode_sel), cx).flex_1())
                        .child(ui::field("密钥长度", Select::new(&self.key_len_sel), cx).flex_1())
                        .child(ui::field("填充模式", Select::new(&self.padding_sel), cx).flex_1()),
                ),
            )
            .child(input_row)
            .child(key_iv_row);

        if let Some(error) = &self.error {
            page = page.child(ui::error_banner(error, cx));
        }

        page.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    ui::primary_button(
                        "aesencrypt-run",
                        "加密",
                        cx.listener(|this, _, window, cx| this.run_encrypt(window, cx)),
                    )
                    .disabled(text.is_empty()),
                )
                .child(ui::secondary_button(
                    "aesencrypt-example",
                    "加载示例",
                    cx.listener(|this, _, window, cx| this.load_example(window, cx)),
                )),
        )
    }
}
