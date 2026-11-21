//! AES 加密/解密。
//!
//! 顶部「加密/解密」双 Tab，两页 text/key/iv/result/error 状态独立；
//! 模式 / 密钥长度 / 填充三项 AES 配置两页共享。加密走
//! `devtools_core::crypto::aes_encrypt` 输出 Base64；解密走
//! `aes_decrypt_to_bytes` + UTF-8 解码。错误一律内联红条展示。

use devtools_core::crypto::{
    AesMode, AesOptions, AesPadding, aes_decrypt_to_bytes, aes_encrypt, generate_random_iv_hex,
};
use gpui::{
    div, px, prelude::*, App, Context, Entity, IntoElement, Render, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputState, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _, Selectable as _,
};

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::ui;

const TITLE: &str = "AES 加密/解密工具";
const SUBTITLE: &str = "使用 AES 算法对文本进行加密和解密，支持多种模式和配置";

/// 模式下拉选项（value 为历史契约字符串）。
const MODE_OPTIONS: &[(&str, &str)] = &[
    ("CBC", "CBC"),
    ("ECB", "ECB"),
    ("CFB", "CFB"),
    ("OFB", "OFB"),
    ("CTR", "CTR"),
];

/// 密钥长度下拉选项。
const KEY_LEN_OPTIONS: &[(&str, &str)] = &[("128", "128 位"), ("192", "192 位"), ("256", "256 位")];

/// 填充下拉选项（value 为历史契约字符串）。
const PADDING_OPTIONS: &[(&str, &str)] = &[
    ("Pkcs7", "PKCS#7"),
    ("Iso97971", "ISO/IEC 9797-1"),
    ("AnsiX923", "ANSI X9.23"),
    ("Iso10126", "ISO 10126"),
    ("ZeroPadding", "Zero Padding"),
    ("NoPadding", "No Padding"),
];

/// 历史契约 AesMode 字符串 → core 枚举。
fn aes_mode(value: &str) -> AesMode {
    match value {
        "ECB" => AesMode::Ecb,
        "CFB" => AesMode::Cfb,
        "OFB" => AesMode::Ofb,
        "CTR" => AesMode::Ctr,
        _ => AesMode::Cbc,
    }
}

/// 历史契约 AesPadding 字符串 → core 枚举。
fn aes_padding(value: &str) -> AesPadding {
    match value {
        "Iso97971" => AesPadding::Iso97971,
        "AnsiX923" => AesPadding::AnsiX923,
        "Iso10126" => AesPadding::Iso10126,
        "ZeroPadding" => AesPadding::Zero,
        "NoPadding" => AesPadding::NoPadding,
        _ => AesPadding::Pkcs7,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Encrypt,
    Decrypt,
}

/// 单个页面的独立状态（text/key/iv/result/error 各一套）。
struct PageState {
    text: Entity<TextareaState>,
    key: Entity<InputState>,
    iv: Entity<InputState>,
    result: Entity<TextareaState>,
    error: Option<String>,
}

impl PageState {
    fn new(is_encrypt: bool, window: &mut Window, cx: &mut Context<AesCrypto>) -> Self {
        let (text_placeholder, result_placeholder) = if is_encrypt {
            ("请输入要加密的文本", "加密结果将显示在这里")
        } else {
            ("请输入要解密的文本", "解密结果将显示在这里")
        };
        Self {
            text: cx.new(|cx| TextareaState::new(window, cx).placeholder(text_placeholder)),
            result: cx.new(|cx| TextareaState::new(window, cx).placeholder(result_placeholder)),
            // 默认 128 位；密钥长度切换时由 sync_key_placeholder 更新占位文案
            key: cx.new(|cx| {
                InputState::new(window, cx).placeholder("请输入128位密钥 (16 字符)")
            }),
            iv: cx.new(|cx| InputState::new(window, cx).placeholder("请输入16字符的IV (128位)")),
            error: None,
        }
    }

    /// 只写结果框（成功时调用）。
    fn set_result(&self, text: String, window: &mut Window, cx: &mut App) {
        self.result.update(cx, |state, cx| state.set_value(text, window, cx));
    }
}

pub struct AesCrypto {
    active_tab: Tab,
    enc: PageState,
    dec: PageState,
    /// 共享 AES 配置，取值与下拉选项 value 一致：mode="CBC"，key_len="128"，padding="Pkcs7"。
    mode: SharedString,
    key_len: SharedString,
    padding: SharedString,
    mode_sel: Entity<SelectState<SimpleSelectDelegate>>,
    key_len_sel: Entity<SelectState<SimpleSelectDelegate>>,
    padding_sel: Entity<SelectState<SimpleSelectDelegate>>,
}

impl AesCrypto {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let enc = PageState::new(true, window, cx);
        let dec = PageState::new(false, window, cx);

        let mode_sel = select_state(window, cx, MODE_OPTIONS, 0);
        let key_len_sel = select_state(window, cx, KEY_LEN_OPTIONS, 0);
        let padding_sel = select_state(window, cx, PADDING_OPTIONS, 0);

        // 模式切换：仅更新共享配置（结果不重算）
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

        // 密钥长度切换：同步两页密钥输入的占位文案
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
            active_tab: Tab::Encrypt,
            enc,
            dec,
            mode: SharedString::from("CBC"),
            key_len: SharedString::from("128"),
            padding: SharedString::from("Pkcs7"),
            mode_sel,
            key_len_sel,
            padding_sel,
        }
    }

    // ------------------------------------------------------------------
    // 配置
    // ------------------------------------------------------------------

    fn key_len_bits(&self) -> usize {
        self.key_len.parse().unwrap_or(128)
    }

    /// 密钥长度变化后刷新两页密钥输入的占位文案。
    fn sync_key_placeholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let bits = self.key_len_bits();
        let placeholder = format!("请输入{bits}位密钥 ({} 字符)", bits / 8);
        let keys = [self.enc.key.clone(), self.dec.key.clone()];
        for key in keys {
            key.update(cx, |state, cx| {
                state.set_placeholder(placeholder.clone(), window, cx)
            });
        }
    }

    // ------------------------------------------------------------------
    // 交互
    // ------------------------------------------------------------------

    /// 加密：前置校验（空文本/空密钥/非 ECB 空 IV）→ aes_encrypt → Base64 结果。
    fn run_encrypt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.enc.text.read(cx).value().to_string();
        let key = self.enc.key.read(cx).value().to_string();
        let iv = self.enc.iv.read(cx).value().to_string();
        let mode: &str = &self.mode;

        if text.is_empty() {
            self.enc.error = Some("请输入要加密的文本".to_string());
        } else if key.is_empty() {
            self.enc.error = Some("请输入密钥".to_string());
        } else if mode != "ECB" && iv.is_empty() {
            self.enc.error = Some("当前模式需要初始化向量(IV)".to_string());
        } else {
            self.enc.error = None;
            let opts = AesOptions {
                mode: aes_mode(mode),
                key_size: self.key_len_bits(),
                padding: aes_padding(&self.padding),
            };
            // ECB 模式忽略 IV（传空串）
            let iv_arg = if mode == "ECB" { "" } else { iv.as_str() };
            match aes_encrypt(&text, &key, iv_arg, &opts) {
                Ok(encrypted) => self.enc.set_result(encrypted, window, cx),
                Err(msg) => {
                    self.enc.error = Some(format!("加密过程中发生错误: {msg}"));
                }
            }
        }
        cx.notify();
    }

    /// 解密：前置校验 → aes_decrypt_to_bytes → UTF-8 解码（空串视为失败）。
    fn run_decrypt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.dec.text.read(cx).value().to_string();
        let key = self.dec.key.read(cx).value().to_string();
        let iv = self.dec.iv.read(cx).value().to_string();
        let mode: &str = &self.mode;

        if text.is_empty() {
            self.dec.error = Some("请输入要解密的文本".to_string());
        } else if key.is_empty() {
            self.dec.error = Some("请输入密钥".to_string());
        } else if mode != "ECB" && iv.is_empty() {
            self.dec.error = Some("当前模式需要初始化向量(IV)".to_string());
        } else {
            self.dec.error = None;
            let opts = AesOptions {
                mode: aes_mode(mode),
                key_size: self.key_len_bits(),
                padding: aes_padding(&self.padding),
            };
            let iv_arg = if mode == "ECB" { "" } else { iv.as_str() };
            match aes_decrypt_to_bytes(&text, &key, iv_arg, &opts) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    // 严格 UTF-8 校验：非法序列走异常分支
                    Ok(decrypted) if !decrypted.is_empty() => {
                        self.dec.set_result(decrypted, window, cx);
                    }
                    Ok(_) => {
                        self.dec.error =
                            Some("解密失败，请检查密钥、IV和密文是否正确".to_string());
                    }
                    Err(msg) => {
                        self.dec.error = Some(format!("解密过程中发生错误: {msg}"));
                    }
                },
                Err(msg) => {
                    self.dec.error = Some(format!("解密过程中发生错误: {msg}"));
                }
            }
        }
        cx.notify();
    }

    /// 随机生成 IV（32 位小写 hex），填入当前页 IV 输入框。
    fn generate_iv(&mut self, is_encrypt: bool, window: &mut Window, cx: &mut Context<Self>) {
        let iv = generate_random_iv_hex();
        if is_encrypt {
            self.enc.iv.update(cx, |state, cx| state.set_value(iv, window, cx));
        } else {
            self.dec.iv.update(cx, |state, cx| state.set_value(iv, window, cx));
        }
        cx.notify();
    }

    /// 加载示例（一次性填入两页：加密页明文 + 解密页密文）。
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.enc.text.update(cx, |state, cx| {
            state.set_value("这是一段需要加密的示例文本", window, cx)
        });
        self.enc.key.update(cx, |state, cx| {
            state.set_value("my-secret-key-16", window, cx)
        });
        self.enc
            .iv
            .update(cx, |state, cx| state.set_value("initialization-ve", window, cx));
        self.dec.text.update(cx, |state, cx| {
            state.set_value("U2FsdGVkX1+initialization-veexample-encrypted-text", window, cx)
        });
        self.dec.key.update(cx, |state, cx| {
            state.set_value("my-secret-key-16", window, cx)
        });
        self.dec
            .iv
            .update(cx, |state, cx| state.set_value("initialization-ve", window, cx));
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    /// 渲染单个 Tab 页（文本/结果 + 密钥/IV + 错误红条 + 操作按钮）。
    fn render_page(
        &self,
        page: &PageState,
        tab: Tab,
        show_iv: bool,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let is_encrypt = tab == Tab::Encrypt;
        let (action_id, action_label, text_label, result_label, copy_id, example_id, iv_gen_id) =
            if is_encrypt {
                (
                    "aescrypto-enc-run",
                    "加密",
                    "要加密的文本",
                    "加密结果",
                    "aescrypto-enc-copy",
                    "aescrypto-enc-example",
                    "aescrypto-enc-iv-gen",
                )
            } else {
                (
                    "aescrypto-dec-run",
                    "解密",
                    "要解密的文本",
                    "解密结果",
                    "aescrypto-dec-copy",
                    "aescrypto-dec-example",
                    "aescrypto-dec-iv-gen",
                )
            };

        let text = page.text.read(cx).value().to_string();
        let key = page.key.read(cx).value().to_string();
        let result = page.result.read(cx).value().to_string();
        let bits = self.key_len_bits();

        // 文本输入 | 结果（label 行右侧浮动复制按钮，仅结果非空显示）
        let input_row = div()
            .flex()
            .gap_4()
            .child(ui::field(text_label, ui::edit_area(&page.text, 96.), cx).flex_1())
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
                                    .child(result_label),
                            )
                            .when(!result.is_empty(), |el| {
                                el.child(ui::copy_button(copy_id, result.clone()))
                            }),
                    )
                    .child(ui::result_area(&page.result, 96.)),
            );

        // 密钥 | IV（mode≠ECB 时显示）
        let mut key_iv_row = div().flex().gap_4().child(
            ui::field("密钥", Input::new(&page.key), cx)
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
                        .child(div().flex_1().child(Input::new(&page.iv)))
                        .child(ui::secondary_button(
                            iv_gen_id,
                            "随机生成",
                            cx.listener(move |this, _, window, cx| {
                                this.generate_iv(is_encrypt, window, cx)
                            }),
                        )),
                    cx,
                )
                .child(ui::hint_text("IV长度必须为16字符 (128位)", cx))
                .flex_1(),
            );
        }

        let mut page_div = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(input_row)
            .child(key_iv_row);

        if let Some(error) = &page.error {
            page_div = page_div.child(ui::error_banner(error, cx));
        }

        page_div.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    ui::primary_button(
                        action_id,
                        action_label,
                        cx.listener(move |this, _, window, cx| {
                            if is_encrypt {
                                this.run_encrypt(window, cx)
                            } else {
                                this.run_decrypt(window, cx)
                            }
                        }),
                    )
                    .disabled(text.is_empty() || key.is_empty()),
                )
                .child(ui::secondary_button(
                    example_id,
                    "加载示例",
                    cx.listener(|this, _, window, cx| this.load_example(window, cx)),
                )),
        )
    }
}

impl Render for AesCrypto {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let show_iv = {
            let mode: &str = &self.mode;
            mode != "ECB"
        };

        // 顶部 Tab（自制按钮组，默认加密；两页状态各自保留）
        let tab_button = |id: &'static str, label: &'static str, tab: Tab| {
            Button::new(id)
                .label(label)
                .ghost()
                .selected(self.active_tab == tab)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.active_tab = tab;
                    cx.notify();
                }))
        };
        let tabs = div()
            .flex()
            .items_center()
            .gap_1()
            .pb_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(tab_button("aescrypto-tab-encrypt", "加密", Tab::Encrypt))
            .child(tab_button("aescrypto-tab-decrypt", "解密", Tab::Decrypt));

        // 共享 AES 配置卡片
        let config_card = ui::card(cx)
            .child(ui::card_header("AES 配置", cx))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(ui::field("模式", Select::new(&self.mode_sel), cx).flex_1())
                    .child(ui::field("密钥长度", Select::new(&self.key_len_sel), cx).flex_1())
                    .child(ui::field("填充模式", Select::new(&self.padding_sel), cx).flex_1()),
            );

        let page_content = match self.active_tab {
            Tab::Encrypt => self.render_page(&self.enc, Tab::Encrypt, show_iv, cx),
            Tab::Decrypt => self.render_page(&self.dec, Tab::Decrypt, show_iv, cx),
        };

        ui::tool_page("aescrypto")
            .child(ui::themed_header(TITLE, SUBTITLE, cx))
            .child(tabs)
            .child(config_card)
            .child(page_content)
    }
}
