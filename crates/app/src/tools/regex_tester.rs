//! 正则表达式测试器。
//!
//! 左栏「配置与控制」（引擎/模式/正则/替换/标志 + 测试文本），右栏「匹配/替换结果」。
//! 正则输入 300ms 防抖调用 core `validate_regex` 实时显示 ✓/✗；
//! 「测试匹配 / 执行替换」按钮手动触发 core `test_regex` / `replace_regex`（经 core_rt 异步）。

use std::time::Duration;

use devtools_core::tools::regex_tester::{
    RegexFlags, RegexReplaceResult, RegexTestResult, replace_regex, test_regex, validate_regex,
};
use futures::StreamExt as _;
use gpui::{
    Context, Div, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
    prelude::*, App, ClickEvent,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _,
    button::{Toggle, ToggleVariants as _},
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState, TextareaState},
};

use crate::components::ui;

/// 正则引擎选项（值 = core 支持的 engine 字符串）。
const ENGINES: &[(&str, &str)] = &[
    ("rust", "Rust"),
    ("re2", "RE2"),
    ("pcre", "PCRE"),
    ("golang", "Golang"),
    ("javascript", "JavaScript"),
];

/// 操作模式。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Test,
    Replace,
}

pub struct RegexTester {
    /// 正则表达式（单行，mono）。
    pattern_input: Entity<InputState>,
    /// 替换文本（replace 模式显示）。
    replacement_input: Entity<InputState>,
    /// 测试文本（大 textarea）。
    text_area: Entity<TextareaState>,

    engine: &'static str,
    mode: Mode,
    replace_all: bool,
    flags: RegexFlags,

    test_result: Option<RegexTestResult>,
    replace_result: Option<RegexReplaceResult>,
    /// None=未验证（空），Some(true)=✓，Some(false)=✗。
    pattern_valid: Option<bool>,
    /// 防抖序列号：过期验证结果直接丢弃。
    validate_seq: u64,
    loading: bool,
    error: Option<String>,
}

impl RegexTester {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pattern_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入正则表达式..."));
        let replacement_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入替换文本..."));
        let text_area = cx.new(|cx| TextareaState::new(window, cx).placeholder("输入要测试的文本..."));

        let this = Self {
            pattern_input,
            replacement_input,
            text_area,
            engine: "rust",
            mode: Mode::Test,
            replace_all: true,
            flags: RegexFlags::default(), // unicode 默认 true，其余 false
            test_result: None,
            replace_result: None,
            pattern_valid: None,
            validate_seq: 0,
            loading: false,
            error: None,
        };

        // 正则输入变化 → 300ms 防抖验证
        cx.subscribe_in(
            &this.pattern_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule_validate(window, cx);
                }
            },
        )
        .detach();

        this
    }

    /// 300ms 防抖验证正则（防抖在 tokio 任务内做，过期序列号的结果回填时丢弃）。
    fn schedule_validate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pattern = self.pattern_input.read(cx).value().to_string();
        self.validate_seq += 1;
        let seq = self.validate_seq;

        if pattern.is_empty() {
            self.pattern_valid = None;
            cx.notify();
            return;
        }

        let engine = self.engine.to_string();
        let mut rx = crate::core_rt::submit(cx, async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            validate_regex(pattern, engine).await
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = rx.next().await else {
                return;
            };
            let _ = this.update_in(cx, |this, _window, cx| {
                if this.validate_seq != seq {
                    return; // 已过期（期间用户又输入了）
                }
                this.pattern_valid = match result {
                    Ok(valid) => Some(valid),
                    Err(_) => Some(false),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// 手动触发：测试匹配 / 执行替换（随模式），经 core_rt 异步调用 core。
    fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pattern = self.pattern_input.read(cx).value().to_string();
        let text = self.text_area.read(cx).value().to_string();
        if pattern.is_empty() || text.is_empty() || self.loading {
            return;
        }

        self.loading = true;
        self.error = None;
        cx.notify();

        let flags = self.flags.clone();
        let engine = self.engine.to_string();

        match self.mode {
            Mode::Test => {
                let mut rx = crate::core_rt::submit(cx, async move {
                    test_regex(pattern, text, flags, engine).await
                });
                cx.spawn_in(window, async move |this, cx| {
                    if let Some(result) = rx.next().await {
                        let _ = this.update_in(cx, |this, _window, cx| {
                            this.loading = false;
                            match result {
                                Ok(r) => this.test_result = Some(r),
                                Err(e) => {
                                    this.error = Some(e);
                                    this.test_result = None;
                                }
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
            Mode::Replace => {
                let replacement = self.replacement_input.read(cx).value().to_string();
                let replace_all = self.replace_all;
                let mut rx = crate::core_rt::submit(cx, async move {
                    replace_regex(pattern, text, replacement, flags, engine, replace_all).await
                });
                cx.spawn_in(window, async move |this, cx| {
                    if let Some(result) = rx.next().await {
                        let _ = this.update_in(cx, |this, _window, cx| {
                            this.loading = false;
                            match result {
                                Ok(r) => this.replace_result = Some(r),
                                Err(e) => {
                                    this.error = Some(e);
                                    this.replace_result = None;
                                }
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
        }
    }

    // -----------------------------------------------------------------------
    // 渲染
    // -----------------------------------------------------------------------

    fn render_left(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let pattern = self.pattern_input.read(cx).value().to_string();
        let text = self.text_area.read(cx).value().to_string();
        let disabled = pattern.is_empty() || text.is_empty() || self.loading;
        let (run_label, running_label) = match self.mode {
            Mode::Test => ("测试匹配", "测试中..."),
            Mode::Replace => ("执行替换", "替换中..."),
        };
        let on_run = cx.listener(move |this, _ev: &ClickEvent, window, cx| {
            this.run(window, cx);
        });

        // 引擎按钮组（自制 radio：outline Toggle，checked=选中）
        let engine_row = div().flex().flex_row().flex_wrap().gap_2().children(
            ENGINES.iter().map(|(value, label)| {
                let value = *value;
                Toggle::new(SharedString::from(format!("rt-engine-{value}")))
                    .label(*label)
                    .outline()
                    .checked(self.engine == value)
                    .on_click(cx.listener(move |this, _checked: &bool, window, cx| {
                        this.engine = value;
                        this.schedule_validate(window, cx);
                    }))
            }),
        );

        // 模式按钮组
        let mode_row = div()
            .flex()
            .flex_row()
            .gap_2()
            .child(
                Toggle::new("rt-mode-test")
                    .label("测试匹配")
                    .outline()
                    .checked(self.mode == Mode::Test)
                    .on_click(cx.listener(|this, _checked: &bool, _window, cx| {
                        this.mode = Mode::Test;
                        cx.notify();
                    })),
            )
            .child(
                Toggle::new("rt-mode-replace")
                    .label("替换文本")
                    .outline()
                    .checked(self.mode == Mode::Replace)
                    .on_click(cx.listener(|this, _checked: &bool, _window, cx| {
                        this.mode = Mode::Replace;
                        cx.notify();
                    })),
            );

        // 正则表达式 label 行：✓/✗ 状态 + （test 模式有效时的）匹配计数
        let mut label_row = div().flex().items_center().justify_between().child(
            div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("正则表达式"),
        );
        label_row = match self.pattern_valid {
            Some(true) => label_row.child(
                div().text_size(px(11.)).text_color(theme.colors.success).child("✓ 语法正确"),
            ),
            Some(false) => label_row.child(
                div().text_size(px(11.)).text_color(theme.colors.danger).child("✗ 语法错误"),
            ),
            None => label_row,
        };
        let mut count_label: Option<Div> = None;
        if self.mode == Mode::Test {
            if let Some(r) = &self.test_result {
                if r.is_valid {
                    count_label = Some(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.info)
                            .child(format!("找到 {} 个匹配", r.match_count)),
                    );
                }
            }
        }
        let label_row = label_row.children(count_label);

        // 正则输入（mono；语法错误红边框）
        let pattern_input = Input::new(&self.pattern_input)
            .font_family("Menlo")
            .when(self.pattern_valid == Some(false), |input| {
                input.border_color(theme.colors.danger)
            });

        // 正则标志 checkbox（2 列；字段为 core RegexFlags 的 snake_case 命名）
        let flag_i = Checkbox::new("rt-flag-i")
            .label("忽略大小写 (i)")
            .checked(self.flags.case_insensitive)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.flags.case_insensitive = *checked;
                cx.notify();
            }));
        let flag_m = Checkbox::new("rt-flag-m")
            .label("多行模式 (m)")
            .checked(self.flags.multiline)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.flags.multiline = *checked;
                cx.notify();
            }));
        let flag_s = Checkbox::new("rt-flag-s")
            .label(". 匹配换行 (s)")
            .checked(self.flags.dot_matches_new_line)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.flags.dot_matches_new_line = *checked;
                cx.notify();
            }));
        let flag_u = Checkbox::new("rt-flag-u")
            .label("非贪婪模式 (U)")
            .checked(self.flags.swap_greed)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.flags.swap_greed = *checked;
                cx.notify();
            }));
        let flag_unicode = Checkbox::new("rt-flag-unicode")
            .label("Unicode 支持")
            .checked(self.flags.unicode)
            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                this.flags.unicode = *checked;
                cx.notify();
            }));

        // 测试文本高度随模式收缩，保证页面在 750 高窗口内不滚动
        let text_height = match self.mode {
            Mode::Test => 280.,
            Mode::Replace => 170.,
        };

        let mut left = div().flex().flex_col().flex_1().min_h(px(0.)).gap_2().child(
            ui::card(cx)
                .child(
                    div().flex().flex_col().gap_2()
                        .child(div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("正则引擎"))
                        .child(engine_row),
                )
                .child(
                    div().flex().flex_col().gap_2()
                        .child(div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("操作模式"))
                        .child(mode_row),
                )
                .child(
                    div().flex().flex_col().gap_1()
                        .child(label_row)
                        .child(pattern_input),
                )
                .when(self.mode == Mode::Replace, |card| {
                    card.child(
                        div().flex().flex_col().gap_1()
                            .child(div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("替换文本"))
                            .child(Input::new(&self.replacement_input).font_family("Menlo"))
                            .child(
                                Checkbox::new("rt-replace-all")
                                    .label("替换所有匹配")
                                    .checked(self.replace_all)
                                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                        this.replace_all = *checked;
                                        cx.notify();
                                    })),
                            ),
                    )
                })
                .child(
                    div().flex().flex_col().gap_2()
                        .child(div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("正则标志"))
                        .child(
                            div().grid().grid_cols(2).gap_2()
                                .child(flag_i)
                                .child(flag_m)
                                .child(flag_s)
                                .child(flag_u)
                                .child(flag_unicode),
                        ),
                )
                .child(
                    ui::primary_button("rt-run", run_label, on_run)
                        .when(self.loading, |b| b.label(running_label).loading(true))
                        .disabled(disabled),
                )
                .children(self.error.clone().map(|e| ui::error_banner(&e, cx))),
        );

        left = left.child(
            div().flex().flex_col().gap_1()
                .child(div().text_size(px(12.)).text_color(theme.colors.muted_foreground).child("测试文本"))
                .child(ui::edit_area(&self.text_area, text_height)),
        );

        left
    }

    fn render_right(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let title = match self.mode {
            Mode::Test => "匹配结果",
            Mode::Replace => "替换结果",
        };
        let content = match self.mode {
            Mode::Test => self.render_test_result(cx).into_any_element(),
            Mode::Replace => self.render_replace_result(cx).into_any_element(),
        };

        div().flex().flex_col().flex_1().min_h(px(0.)).gap_2()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.foreground)
                    .child(title),
            )
            .child(
                div().id("rt-results")
                    .flex().flex_col().flex_1().min_h(px(0.))
                    .overflow_y_scroll()
                    .p_2()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.popover)
                    .child(content),
            )
    }

    fn render_test_result(&self, cx: &App) -> Div {
        let theme = cx.theme().clone();
        match &self.test_result {
            None => ui::empty_hint("输入正则表达式与测试文本后点击「测试匹配」查看结果", cx),
            Some(r) if !r.is_valid => invalid_card(r.error_message.as_deref(), cx),
            Some(r) if r.matches.is_empty() => div()
                .flex()
                .w_full()
                .items_center()
                .justify_center()
                .py_6()
                .text_size(px(13.))
                .text_color(theme.colors.muted_foreground)
                .child("没有找到匹配项"),
            Some(r) => {
                let mut list = div().flex().flex_col();
                for (idx, m) in r.matches.iter().enumerate() {
                    // 命名组按名称排序，保证渲染稳定
                    let mut named: Vec<(&String, &Option<String>)> = m.named_groups.iter().collect();
                    named.sort_by(|a, b| a.0.cmp(b.0));

                    let mut item = div().flex().flex_col().gap_1().px_3().py_2();
                    if idx > 0 {
                        item = item.border_t_1().border_color(theme.colors.border.opacity(0.5));
                    }
                    item = item
                        // 头行：匹配 N + [黄色 mark 语义片段] + 位置
                        .child(
                            div().flex().flex_wrap().items_center().gap_2()
                                .child(
                                    div().text_size(px(12.)).text_color(theme.colors.muted_foreground)
                                        .child(format!("匹配 {}:", idx + 1)),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(4.))
                                        .bg(theme.colors.warning.opacity(0.25))
                                        .font_family("Menlo")
                                        .text_size(px(13.))
                                        .text_color(theme.colors.foreground)
                                        .child(m.full_match.clone()),
                                )
                                .child(
                                    div().text_size(px(12.)).text_color(theme.colors.muted_foreground)
                                        .child(format!("位置: {}-{}", m.start, m.end)),
                                ),
                        )
                        // 编号捕获组
                        .children(m.groups.iter().enumerate().map(|(gi, g)| {
                            div().flex().items_start().gap_2()
                                .child(
                                    div().text_size(px(12.)).text_color(theme.colors.muted_foreground)
                                        .child(format!("组 {}:", gi + 1)),
                                )
                                .child(
                                    div().text_size(px(13.)).font_family("Menlo").text_color(theme.colors.foreground)
                                        .child(g.clone().unwrap_or_else(|| "(未匹配)".to_string())),
                                )
                        }))
                        // 命名捕获组
                        .children(named.into_iter().map(|(name, value)| {
                            div().flex().items_start().gap_2()
                                .child(
                                    div().text_size(px(12.)).text_color(theme.colors.muted_foreground)
                                        .child(format!("{}:", name)),
                                )
                                .child(
                                    div().text_size(px(13.)).font_family("Menlo").text_color(theme.colors.foreground)
                                        .child(value.clone().unwrap_or_else(|| "(未匹配)".to_string())),
                                )
                        }));
                    list = list.child(item);
                }
                div().flex().flex_col().gap_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.foreground)
                            .child("匹配详情："),
                    )
                    .child(list)
            }
        }
    }

    fn render_replace_result(&self, cx: &App) -> Div {
        let theme = cx.theme().clone();
        match &self.replace_result {
            None => ui::empty_hint("输入正则表达式与替换文本后点击「执行替换」查看结果", cx),
            Some(r) if !r.is_valid => invalid_card(r.error_message.as_deref(), cx),
            Some(r) => div().flex().flex_col().gap_2()
                .child(ui::success_banner(&format!("完成 {} 个替换", r.replacement_count), cx))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child("替换结果:"),
                )
                .child(ui::mono_block("mono-regex-result", r.result.clone().unwrap_or_default(), cx).max_h(px(300.))),
        }
    }
}

/// 「正则表达式无效」红卡（test / replace 共用）。
fn invalid_card(error_message: Option<&str>, cx: &App) -> Div {
    let theme = cx.theme().clone();
    div().flex().flex_col().gap_1()
        .p_3()
        .rounded(theme.radius)
        .border_1()
        .border_color(theme.colors.danger.opacity(0.4))
        .bg(theme.colors.danger.opacity(0.1))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.colors.danger)
                .child("正则表达式无效"),
        )
        .children(error_message.map(|m| {
            div().text_size(px(12.)).text_color(theme.colors.danger).child(m.to_string())
        }))
}

impl Render for RegexTester {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::tool_page("regextester")
            .child(ui::themed_header(
                "正则表达式测试器",
                "测试正则表达式匹配和替换，支持 Rust、RE2、PCRE、Golang、JavaScript 语法",
                cx,
            ))
            .child(
                div().flex().flex_row().flex_1().min_h(px(0.)).gap_4()
                    .child(self.render_left(cx))
                    .child(self.render_right(cx)),
            )
    }
}
