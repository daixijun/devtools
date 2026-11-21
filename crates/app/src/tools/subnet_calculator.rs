//! 子网计算器 IPv4 & IPv6。
//!
//! 交互：IP 版本 radio 三选（自动检测默认 / IPv4 / IPv6，placeholder 随版本切换）、
//! 输入 + 「计算」按钮；输入经 500ms 防抖自动计算（有值才算，空值清空结果），
//! 「计算」按钮立即计算（空值报错）。结果/错误均为内联展示。

use std::time::Duration;

use devtools_core::text_tools::subnet::{self, IpVersion, SubnetResult};
use gpui::prelude::*;
use gpui::{px, App, Div, Entity, IntoElement, Render, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::{Radio, RadioGroup};
use gpui_component::ActiveTheme as _;

use crate::components::ui;

/// 输入框占位文案随所选版本变化（与历史版本一致）。
fn placeholder_for(version: IpVersion) -> &'static str {
    match version {
        IpVersion::V4 => "输入IPv4和CIDR或子网掩码 (例如: 192.168.1.0/24)",
        IpVersion::V6 => "输入IPv6和前缀 (例如: 2001:db8::/64)",
        IpVersion::Auto => "输入IP和CIDR (例如: 192.168.1.0/24 或 2001:db8::/64)",
    }
}

fn version_index(version: IpVersion) -> usize {
    match version {
        IpVersion::V4 => 1,
        IpVersion::V6 => 2,
        IpVersion::Auto => 0,
    }
}

pub struct SubnetCalculator {
    input: Entity<InputState>,
    version: IpVersion,
    /// 防抖代际计数：仅让最后一次输入变化的定时器生效。
    debounce_gen: u64,
    error: Option<String>,
    result: Option<SubnetResult>,
}

impl SubnetCalculator {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder_for(IpVersion::Auto)));

        // 输入变化 → 500ms 防抖自动计算
        cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.schedule_debounce(cx);
            }
        })
        .detach();

        Self {
            input,
            version: IpVersion::Auto,
            debounce_gen: 0,
            error: None,
            result: None,
        }
    }

    /// 500ms 防抖（同窗口关闭时任务随实体释放而失效）。
    fn schedule_debounce(&mut self, cx: &mut gpui::Context<Self>) {
        self.debounce_gen += 1;
        let gen = self.debounce_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.debounce_gen == gen {
                    let text = this.input.read(cx).value().to_string();
                    if text.is_empty() {
                        this.error = None;
                        this.result = None;
                    } else {
                        this.compute(&text);
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn compute(&mut self, text: &str) {
        match subnet::calculate(text, self.version) {
            Ok(result) => {
                self.result = Some(result);
                self.error = None;
            }
            Err(message) => {
                self.error = Some(message);
                self.result = None;
            }
        }
    }

    /// 「计算」按钮：立即计算（空输入也报错）。
    fn calculate(&mut self, cx: &mut gpui::Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        self.compute(&text);
        cx.notify();
    }

    /// 示例格式卡片的一行。
    fn example_row(label: &str, value: &str, cx: &App) -> Div {
        let theme = cx.theme().clone();
        gpui::div()
            .flex()
            .gap_2()
            .text_size(px(13.))
            .child(
                gpui::div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.info)
                    .child(label.to_string()),
            )
            .child(
                gpui::div()
                    .text_color(theme.colors.foreground)
                    .child(value.to_string()),
            )
    }

    /// 结果卡片的键值行（主机部分为空时显示 N/A）。
    fn result_row(label: &str, value: &str, cx: &App) -> Div {
        ui::info_row(label, value.to_string(), cx)
    }
}

impl Render for SubnetCalculator {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let version_group = RadioGroup::horizontal("subnet-version")
            .selected_index(Some(version_index(self.version)))
            .on_click(cx.listener(|this, index: &usize, window, cx| {
                this.version = match index {
                    1 => IpVersion::V4,
                    2 => IpVersion::V6,
                    _ => IpVersion::Auto,
                };
                this.input.update(cx, |input, cx| {
                    input.set_placeholder(placeholder_for(this.version), window, cx);
                });
                cx.notify();
            }))
            .children([
                Radio::new("subnet-version-auto").label("自动检测"),
                Radio::new("subnet-version-v4").label("IPv4"),
                Radio::new("subnet-version-v6").label("IPv6"),
            ]);

        let input_row = gpui::div()
            .flex()
            .items_center()
            .gap_3()
            .child(gpui::div().flex_1().child(Input::new(&self.input)))
            .child(ui::primary_button(
                "subnet-calculate",
                "计算",
                cx.listener(|this, _, _, cx| {
                    this.calculate(cx);
                }),
            ));

        let example_card = gpui::div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.colors.info.opacity(0.35))
            .bg(theme.colors.info.opacity(0.08))
            .child(
                gpui::div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.info)
                    .child("示例格式:"),
            )
            .child(Self::example_row(
                "IPv4:",
                "192.168.1.0/24 或 10.0.0.0/255.0.0.0",
                cx,
            ))
            .child(Self::example_row("IPv6:", "2001:db8::/64 或 fe80::/10", cx))
            .child(Self::example_row(
                "简写IPv6:",
                "::1/128 或 2001:db8::1/64",
                cx,
            ));

        let mut page = ui::tool_page("subnetcalculator")
            .child(ui::themed_header(
                "子网计算器 (IPv4 & IPv6)",
                "支持IPv4和IPv6网络地址的子网计算，包括CIDR转换、子网划分和网络信息查询",
                cx,
            ))
            .child(
                ui::card(cx)
                    .child(ui::field("IP版本", version_group, cx))
                    .child(ui::field("地址", input_row, cx))
                    .child(example_card),
            )
            .when_some(self.error.clone(), |page, message| {
                page.child(ui::error_banner(&message, cx))
            });

        if let Some(result) = &self.result {
            let ipv4 = result.version == "IPv4";
            let mut result_card = ui::card(cx)
                .child(ui::card_header(
                    &format!("计算结果 ({})", result.version),
                    cx,
                ))
                .child(Self::result_row("版本", &result.version, cx))
                .child(Self::result_row("CIDR", &result.cidr, cx));
            if ipv4 {
                result_card = result_card
                    .child(Self::result_row("子网掩码", &result.mask, cx))
                    .child(Self::result_row("广播地址", &result.broadcast, cx));
            }
            result_card = result_card
                .child(Self::result_row("网络地址", &result.network, cx))
                .child(Self::result_row(
                    if ipv4 {
                        "第一个可用IP"
                    } else {
                        "网络前缀"
                    },
                    &result.first_ip,
                    cx,
                ));
            if ipv4 {
                result_card =
                    result_card.child(Self::result_row("最后一个可用IP", &result.last_ip, cx));
            }
            result_card = result_card
                .child(Self::result_row(
                    if ipv4 { "总IP数" } else { "总地址数" },
                    &result.total_ips,
                    cx,
                ))
                .child(Self::result_row(
                    if ipv4 {
                        "可用IP数"
                    } else {
                        "可用地址数"
                    },
                    &result.usable_ips,
                    cx,
                ));
            if !ipv4 {
                result_card = result_card
                    .child(Self::result_row("网络部分", &result.network_portion, cx))
                    .child(Self::result_row(
                        "主机部分",
                        if result.host_portion.is_empty() {
                            "N/A"
                        } else {
                            &result.host_portion
                        },
                        cx,
                    ));
            }
            page = page.child(result_card);
        }

        page
    }
}
