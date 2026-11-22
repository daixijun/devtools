//! AES 解密。
//!
//! 独立解密工具（与 AES 加密对称）：Base64 密文 + 密钥 + IV（非 ECB）→
//! UTF-8 明文。「加载示例」按当前配置生成真实可解密的密文。

use devtools::crypto::{aes_decrypt_to_bytes, aes_encrypt, AesOptions};
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

pub struct AesDecrypt {
    text: Entity<TextareaState>,
    key: Entity<InputState>,
    iv: Entity<InputState>,
    result: Entity<TextareaState>,
    error: Option<String>,
    mode: SharedString,
    key_len: SharedString,
    padding: SharedString,
    mode_sel: Entity<SelectState<SimpleSelectDelegate>>,
    key_len_sel: Entity<SelectState<SimpleSelectDelegate>>,
    padding_sel: Entity<SelectState<SimpleSelectDelegate>>,
}

impl AesDecrypt {
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
            text: cx.new(|cx| TextareaState::new(window, cx).placeholder("请输入要解密的文本")),
            result: cx.new(|cx| TextareaState::new(window, cx).placeholder("解密结果将显示在这里")),
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

    /// 解密：前置校验 → aes_decrypt_to_bytes → UTF-8 解码（空串视为失败）。
    fn run_decrypt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text.read(cx).value().to_string();
        let key = self.key.read(cx).value().to_string();
        let iv = self.iv.read(cx).value().to_string();
        let mode: &str = &self.mode;

        if text.is_empty() {
            self.error = Some("请输入要解密的文本".to_string());
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
            let iv_arg = if mode == "ECB" { "" } else { iv.as_str() };
            match aes_decrypt_to_bytes(&text, &key, iv_arg, &opts) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    // 严格 UTF-8 校验：非法序列走异常分支
                    Ok(decrypted) if !decrypted.is_empty() => {
                        self.result
                            .update(cx, |state, cx| state.set_value(decrypted, window, cx));
                    }
                    Ok(_) => {
                        self.error = Some("解密失败，请检查密钥、IV和密文是否正确".to_string());
                    }
                    Err(msg) => {
                        self.error = Some(format!("解密过程中发生错误: {msg}"));
                    }
                },
                Err(msg) => {
                    self.error = Some(format!("解密过程中发生错误: {msg}"));
                }
            }
        }
        cx.notify();
    }

    /// 加载示例：重置配置为默认（CBC/128/PKCS#7），并用该配置加密示例文本
    /// 生成**真实可解密**的密文——点击「解密」立即成功，完整体验流程。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = SharedString::from("CBC");
        self.key_len = SharedString::from("128");
        self.padding = SharedString::from("Pkcs7");
        self.mode_sel.update(cx, |state, cx| {
            state.set_selected_index(Some(gpui_base::IndexPath::new(0)), window, cx);
        });
        self.key_len_sel.update(cx, |state, cx| {
            state.set_selected_index(Some(gpui_base::IndexPath::new(0)), window, cx);
        });
        self.padding_sel.update(cx, |state, cx| {
            state.set_selected_index(Some(gpui_base::IndexPath::new(0)), window, cx);
        });
        self.sync_key_placeholder(window, cx);

        let opts = AesOptions {
            mode: aes_mode("CBC"),
            key_size: 128,
            padding: aes_padding("Pkcs7"),
        };
        let cipher = aes_encrypt(
            "这是一段需要解密的示例文本",
            "my-secret-key-16",
            "initialization-ve",
            &opts,
        )
        .unwrap_or_default();
        self.text
            .update(cx, |state, cx| state.set_value(cipher, window, cx));
        self.key.update(cx, |state, cx| {
            state.set_value("my-secret-key-16", window, cx)
        });
        self.iv.update(cx, |state, cx| {
            state.set_value("initialization-ve", window, cx)
        });
        self.error = None;
        cx.notify();
    }
}

impl Render for AesDecrypt {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_iv: bool = { self.mode.as_ref() != "ECB" };
        let text = self.text.read(cx).value().to_string();
        let result = self.result.read(cx).value().to_string();
        let bits = key_len_bits(&self.key_len);

        let input_row = div()
            .flex()
            .gap_4()
            .child(ui::field("要解密的文本", ui::edit_area(&self.text, 96.), cx).flex_1())
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
                                    .child("解密结果"),
                            )
                            .when(!result.is_empty(), |el| {
                                el.child(ui::copy_result_button(
                                    "aesdecrypt-copy",
                                    "复制结果",
                                    result.clone(),
                                ))
                            }),
                    )
                    .child(ui::result_area(&self.result, 96.)),
            );

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
                            "aesdecrypt-iv-gen",
                            "随机生成",
                            cx.listener(|this, _, window, cx| {
                                let iv = devtools::crypto::generate_random_iv_hex();
                                this.iv
                                    .update(cx, |state, cx| state.set_value(iv, window, cx));
                                cx.notify();
                            }),
                        )),
                    cx,
                )
                .child(ui::hint_text("IV长度必须为16字符 (128位)", cx))
                .flex_1(),
            );
        }

        let mut page = ui::tool_page("aesdecrypt")
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
                        "aesdecrypt-run",
                        "解密",
                        cx.listener(|this, _, window, cx| this.run_decrypt(window, cx)),
                    )
                    .disabled(text.is_empty()),
                )
                .child(ui::secondary_button(
                    "aesdecrypt-example",
                    "加载示例",
                    cx.listener(|this, _, window, cx| this.load_example(window, cx)),
                )),
        )
    }
}
