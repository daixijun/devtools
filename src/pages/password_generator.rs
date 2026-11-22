//! 密码生成器。
//!
//! 挂载及任意选项变化自动生成；长度用 gpui-component Slider（4–64）；
//! 勾选「特殊符号」时展开自定义字符区（快捷 chips + 自定义输入）。
//! 契约见 docs/plans/tool-specs/batch1.md。

use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, ClipboardItem, IntoElement,
    ParentElement as _, Render, Styled as _, Window,
};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_component::{ActiveTheme as _, Disableable as _};

use crate::components::{notify, ui};

use devtools::i18n::t;
use devtools::text_tools::password as pw;

const MIN_LENGTH: f32 = 4.;
const MAX_LENGTH: f32 = 64.;
const DEFAULT_LENGTH: f32 = 16.;
/// 快捷 chips（与规格一致）：id、名称、字符集。
const CHIP_GROUPS: &[(&str, &str, &str)] = &[
    ("chip-common", "常用", "!@#$%^&*()_+-="),
    ("chip-bracket", "括号", "[]{}()<>"),
    ("chip-punct", "标点", ".,:;?!"),
    ("chip-math", "数学", "+-*/=<>"),
    ("chip-currency", "货币", "$€£¥¢"),
    ("chip-other", "其他", "|~`'\""),
];

pub struct PasswordGenerator {
    /// 当前生成的密码；全不选时为提示文案 [`pw::NO_CHARSET_MSG`]。
    password: String,
    /// `password` 是否为「请至少选择一种字符类型」提示（禁用复制、隐藏强度）。
    no_charset: bool,
    strength: Option<pw::Strength>,
    length: usize,
    include_uppercase: bool,
    include_lowercase: bool,
    include_numbers: bool,
    include_symbols: bool,
    custom_symbols: String,
    custom_input: gpui::Entity<InputState>,
    slider: gpui::Entity<SliderState>,
}

impl PasswordGenerator {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let slider = cx.new(|_| {
            SliderState::new()
                .min(MIN_LENGTH)
                .max(MAX_LENGTH)
                .step(1.)
                .default_value(DEFAULT_LENGTH)
        });
        cx.subscribe(&slider, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                let v = match value {
                    SliderValue::Single(v) => *v,
                    SliderValue::Range(start, _) => *start,
                };
                let v = (v as usize).clamp(MIN_LENGTH as usize, MAX_LENGTH as usize);
                if v != this.length {
                    this.length = v;
                    this.regenerate(cx);
                }
            }
        })
        .detach();

        let custom_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(pw::DEFAULT_CUSTOM_SYMBOLS)
                .placeholder(t("输入您想要使用的特殊字符..."))
        });
        cx.subscribe(&custom_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.custom_symbols = this.custom_input.read(cx).value().to_string();
                this.regenerate(cx);
            }
        })
        .detach();

        let mut this = Self {
            password: String::new(),
            no_charset: false,
            strength: None,
            length: DEFAULT_LENGTH as usize,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
            custom_symbols: pw::DEFAULT_CUSTOM_SYMBOLS.to_string(),
            custom_input,
            slider,
        };
        // 页面创建即自动生成一次
        this.regenerate(cx);
        this
    }

    // ------------------------------------------------------------------
    // 状态操作
    // ------------------------------------------------------------------

    /// 按当前选项生成密码并更新强度；全不选时展示提示文案。
    fn regenerate(&mut self, cx: &mut gpui::Context<Self>) {
        match pw::generate(
            self.length,
            self.include_uppercase,
            self.include_lowercase,
            self.include_numbers,
            self.include_symbols,
            &self.custom_symbols,
        ) {
            Some(password) => {
                self.strength = Some(pw::strength(&password));
                self.no_charset = false;
                self.password = password;
            }
            None => {
                self.strength = None;
                self.no_charset = true;
                self.password = pw::NO_CHARSET_MSG.to_string();
            }
        }
        cx.notify();
    }

    fn set_chip(&mut self, value: &str, window: &mut Window, cx: &mut gpui::Context<Self>) {
        // set_value 不发 InputEvent::Change，这里手动同步并生成
        self.custom_symbols = value.to_string();
        self.custom_input
            .update(cx, |state, cx| state.set_value(value, window, cx));
        self.regenerate(cx);
    }

    fn reset_options(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.length = DEFAULT_LENGTH as usize;
        self.include_uppercase = true;
        self.include_lowercase = true;
        self.include_numbers = true;
        self.include_symbols = true;
        self.custom_symbols = pw::DEFAULT_CUSTOM_SYMBOLS.to_string();
        self.slider
            .update(cx, |state, cx| state.set_value(DEFAULT_LENGTH, window, cx));
        self.custom_input.update(cx, |state, cx| {
            state.set_value(pw::DEFAULT_CUSTOM_SYMBOLS, window, cx)
        });
        self.regenerate(cx);
    }
}

impl Render for PasswordGenerator {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let password = self.password.clone();
        let char_count = self.custom_symbols.chars().count();
        let can_copy = !self.no_charset && !password.is_empty();

        // 强度徽章颜色：非常强=绿、强=primary、中等=黄、弱=红
        let strength_badge = self.strength.map(|s| {
            let color = match s {
                pw::Strength::VeryStrong => theme.colors.success,
                pw::Strength::Strong => theme.colors.primary,
                pw::Strength::Medium => theme.colors.warning,
                pw::Strength::Weak => theme.colors.danger,
            };
            ui::badge(&format!("{}: {}", t("强度"), s.label()), color)
        });
        let password_for_copy = password.clone();

        // 显示区：密码 / 提示文案 / 空态
        let display = if password.is_empty() {
            ui::empty_hint(t("点击生成按钮创建密码"), cx)
        } else if self.no_charset {
            div()
                .w_full()
                .text_color(theme.colors.danger)
                .child(password.clone())
        } else {
            div()
                .w_full()
                .font_family("Menlo")
                .text_size(px(17.))
                .text_color(theme.colors.foreground)
                .child(password.clone())
        };

        ui::tool_page("passwordgenerator")
            // 生成的密码
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(ui::hint_text(t("生成的密码"), cx))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .children(strength_badge)
                                    .child(
                                        ui::primary_button(
                                            "copy-password",
                                            t("复制密码"),
                                            move |_, window, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    password_for_copy.clone(),
                                                ));
                                                notify::toast_success(window, cx, t("已复制"));
                                            },
                                        )
                                        .disabled(!can_copy),
                                    ),
                            ),
                    )
                    .child(ui::panel(cx).items_center().justify_center().child(display)),
            )
            // 密码长度
            .child(
                ui::card(cx)
                    .child(ui::field(
                        &format!("{}: {}", t("密码长度"), self.length),
                        Slider::new(&self.slider).w_full(),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(ui::hint_text("4", cx))
                            .child(ui::hint_text("64", cx)),
                    ),
            )
            // 字符类型控制
            .child(
                ui::card(cx)
                    .child(ui::card_header(t("字符类型控制"), cx))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_4()
                            .child(
                                Checkbox::new("opt-uppercase")
                                    .label(t("大写字母 (A-Z)"))
                                    .checked(self.include_uppercase)
                                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                        this.include_uppercase = *checked;
                                        this.regenerate(cx);
                                    })),
                            )
                            .child(
                                Checkbox::new("opt-lowercase")
                                    .label(t("小写字母 (a-z)"))
                                    .checked(self.include_lowercase)
                                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                        this.include_lowercase = *checked;
                                        this.regenerate(cx);
                                    })),
                            )
                            .child(
                                Checkbox::new("opt-numbers")
                                    .label(t("数字 (0-9)"))
                                    .checked(self.include_numbers)
                                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                        this.include_numbers = *checked;
                                        this.regenerate(cx);
                                    })),
                            )
                            .child(
                                Checkbox::new("opt-symbols")
                                    .label(t("特殊符号"))
                                    .checked(self.include_symbols)
                                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                        this.include_symbols = *checked;
                                        this.regenerate(cx);
                                    })),
                            ),
                    ),
            )
            // 自定义特殊字符（勾选「特殊符号」时显示）
            .when(self.include_symbols, |el| {
                el.child(
                    ui::card(cx)
                        .child(ui::card_header(t("自定义特殊字符"), cx))
                        .child(ui::hint_text(t("快速选择："), cx))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .children(CHIP_GROUPS.iter().map(|&(id, label, value)| {
                                    let value = value.to_string();
                                    ui::secondary_button(
                                        id,
                                        t(label),
                                        cx.listener(move |this, _ev, window, cx| {
                                            this.set_chip(&value, window, cx);
                                        }),
                                    )
                                })),
                        )
                        .child(ui::field(
                            t("自定义字符（直接输入）："),
                            Input::new(&self.custom_input),
                            cx,
                        ))
                        .child(ui::hint_text(
                            &format!("{}: {}", t("当前字符数"), char_count),
                            cx,
                        )),
                )
            })
            // 操作按钮
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        ui::primary_button(
                            "generate",
                            t("生成新密码"),
                            cx.listener(|this, _ev, _window, cx| this.regenerate(cx)),
                        )
                        .flex_1(),
                    )
                    .child(
                        ui::secondary_button(
                            "reset",
                            t("重置选项"),
                            cx.listener(|this, _ev, window, cx| this.reset_options(window, cx)),
                        )
                        .flex_1(),
                    ),
            )
            // 密码强度说明
            .child(
                ui::card(cx)
                    .child(ui::card_header(t("密码强度说明"), cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_color(theme.colors.muted_foreground)
                            .child(
                                div().child(format!(
                                    "• {}",
                                    t("长度 ≥ 12 且包含 4 种字符类型：非常强")
                                )),
                            )
                            .child(
                                div().child(format!(
                                    "• {}",
                                    t("长度 ≥ 8 且包含 3-4 种字符类型：强")
                                )),
                            )
                            .child(
                                div().child(format!(
                                    "• {}",
                                    t("长度 ≥ 8 且包含 2 种字符类型：中等")
                                )),
                            )
                            .child(div().child(format!("• {}", t("其他情况：弱")))),
                    ),
            )
    }
}
