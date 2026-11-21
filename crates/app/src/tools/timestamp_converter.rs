//! 时间戳与日期时间转换。
//!
//! 交互：当前时间卡每秒刷新；输入 + 转换类型（时间戳→日期时间 / 日期时间→时间戳）
//! + 时区（全量 tz 数据库，默认 Asia/Shanghai）+ 毫秒 checkbox（仅 ts→dt 显示）；
//!   「转换」手动触发，「当前时间」按钮填充输入并立即转换。
//!   错误以红色横条内联展示（不写入输出框）。

use std::sync::OnceLock;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{px, App, Div, Entity, IntoElement, Render, Window};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::ActiveTheme as _;

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::ui;
use devtools_core::text_tools::timestamp;

/// 转换类型选项（value 为历史契约值）。
const TYPE_OPTIONS: [(&str, &str); 2] = [
    ("timestampToDatetime", "时间戳 → 日期时间"),
    ("datetimeToTimestamp", "日期时间 → 时间戳"),
];

/// 默认时区。
const DEFAULT_TIMEZONE: &str = "Asia/Shanghai";

/// 时区下拉选项：默认时区置顶，其余按名称排序（进程内只构建一次）。
/// value 保留 `Area/City` 原名（chrono-tz 解析用），label 将 `_` 替换为空格。
fn timezone_options() -> &'static [(&'static str, &'static str)] {
    static OPTS: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    OPTS.get_or_init(|| {
        let mut names = timestamp::all_timezones();
        names.sort_unstable_by_key(|name| {
            if name == DEFAULT_TIMEZONE {
                (0u8, name.clone())
            } else {
                (1u8, name.clone())
            }
        });
        names
            .into_iter()
            .map(|name| {
                let value: &'static str = Box::leak(name.clone().into_boxed_str());
                let label: &'static str = Box::leak(name.replace('_', " ").into_boxed_str());
                (value, label)
            })
            .collect()
    })
}

pub struct TimestampConverter {
    input: Entity<InputState>,
    output: Entity<InputState>,
    type_select: Entity<SelectState<SimpleSelectDelegate>>,
    tz_select: Entity<SelectState<SimpleSelectDelegate>>,
    /// true = 时间戳 → 日期时间（默认），false = 日期时间 → 时间戳。
    to_datetime: bool,
    /// 毫秒单位（默认开启）。
    is_millis: bool,
    /// 当前时区（chrono-tz 名称）。
    time_zone: String,
    /// 当前时间 (秒, 毫秒)，每秒刷新。
    now: (i64, i64),
    error: Option<String>,
}

impl TimestampConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("输入时间戳或日期时间"));
        let output = cx.new(|cx| InputState::new(window, cx).placeholder("转换结果"));

        let type_select = select_state(window, cx, &TYPE_OPTIONS, 0);
        cx.subscribe(
            &type_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let value: &str = value;
                    this.to_datetime = value == "timestampToDatetime";
                    cx.notify();
                }
            },
        )
        .detach();

        let tz_select = select_state(window, cx, timezone_options(), 0);
        cx.subscribe(
            &tz_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.time_zone = value.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let this = Self {
            input,
            output,
            type_select,
            tz_select,
            to_datetime: true,
            is_millis: true,
            time_zone: DEFAULT_TIMEZONE.to_string(),
            now: timestamp::now_ts(),
            error: None,
        };

        // 当前时间卡每秒刷新；窗口/实体销毁后 update_in 返回 Err，循环退出。
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let alive = this
                .update_in(cx, |this, _, cx| {
                    this.now = timestamp::now_ts();
                    cx.notify();
                })
                .is_ok();
            if !alive {
                break;
            }
        })
        .detach();

        this
    }

    fn convert(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let value = self.input.read(cx).value().to_string();
        match timestamp::convert(&value, self.to_datetime, self.is_millis, &self.time_zone) {
            Ok(result) => {
                self.error = None;
                self.set_output(&result, window, cx);
            }
            Err(message) => {
                self.error = Some(message);
                self.set_output("", window, cx);
            }
        }
        cx.notify();
    }

    /// 「当前时间」按钮：按方向填入当前时间戳（秒/毫秒）或 ISO8601，并立即转换。
    fn use_current_time(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let fill = if self.to_datetime {
            let (sec, ms) = timestamp::now_ts();
            if self.is_millis {
                ms.to_string()
            } else {
                sec.to_string()
            }
        } else {
            timestamp::now_iso8601()
        };
        self.input.update(cx, |input, cx| {
            input.set_value(fill, window, cx);
        });
        self.convert(window, cx);
    }

    fn set_output(&mut self, text: &str, window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.output
            .update(cx, |out, cx| out.set_value(text, window, cx));
    }

    /// 「当前时间」卡片的一行展示。
    fn now_line(label: &str, value: String, cx: &App) -> Div {
        let theme = cx.theme().clone();
        gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(13.))
            .text_color(theme.colors.foreground)
            .child(label.to_string())
            .child(
                gpui::div()
                    .font_family("Menlo")
                    .text_color(theme.colors.foreground)
                    .child(value),
            )
    }
}

impl Render for TimestampConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let (now_sec, now_ms) = self.now;
        let localized_now = timestamp::ts_to_localized(now_ms.max(0) as u64, &self.time_zone)
            .unwrap_or_else(|_| "-".to_string());

        let input_row = gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .child(gpui::div().flex_1().child(Input::new(&self.input)))
            .child(ui::secondary_button(
                "ts-use-now",
                "当前时间",
                cx.listener(|this, _, window, cx| {
                    this.use_current_time(window, cx);
                }),
            ));

        let millis_row = gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Checkbox::new("ts-millis")
                    .checked(self.is_millis)
                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                        this.is_millis = *checked;
                        cx.notify();
                    })),
            )
            .child(if self.is_millis { "开启" } else { "关闭" });

        ui::tool_page("timestamp")
            .child(ui::themed_header(
                "时间戳与日期时间转换",
                "时间戳与日期时间格式之间的双向转换，支持多时区和毫秒级精度",
                cx,
            ))
            .child(
                ui::card(cx)
                    .child(ui::card_header("当前时间", cx))
                    .child(Self::now_line("时间戳（秒）: ", now_sec.to_string(), cx))
                    .child(Self::now_line("时间戳（毫秒）: ", now_ms.to_string(), cx))
                    .child(Self::now_line("当前时间: ", localized_now, cx)),
            )
            .child(
                ui::card(cx)
                    .child(ui::field("输入", input_row, cx))
                    .child(ui::field("转换类型", Select::new(&self.type_select), cx))
                    .child(ui::field("时区选择", Select::new(&self.tz_select), cx))
                    .when(self.to_datetime, |card| {
                        card.child(ui::hfield("毫秒单位", millis_row, cx))
                    })
                    .child(ui::primary_button(
                        "ts-convert",
                        "转换",
                        cx.listener(|this, _, window, cx| {
                            this.convert(window, cx);
                        }),
                    ))
                    .when_some(self.error.clone(), |card, message| {
                        card.child(ui::error_banner(&message, cx))
                    })
                    .child(ui::field(
                        "转换结果",
                        Input::new(&self.output).readonly(true),
                        cx,
                    )),
            )
    }
}
