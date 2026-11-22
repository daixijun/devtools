//! 域名 Whois 查询（多源）。
//!
//! 批量查询：textarea 按 `\s , ;` 切分 + 校验 + 去重 + 小写化；并发 6
//! （tokio Semaphore）+ 按源 700ms 限流，逐域名调用 core
//! `query_whois_unified(domain, source)`；每个结果经 mpsc 流式回流 UI 更新行状态。
//!
//! 注：查询历史实现为内存会话态（保留 200 条，
//! 展示最近 20 条）。批量任务函数写在本文件
//! （core_rt.rs 为共享桥），经 `crate::core_rt::submit` 提交
//! 到 tokio Runtime 执行。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use devtools::tools::whois::{query_whois_unified, WhoisParsed};
use futures::StreamExt as _;
use gpui::{
    div, px, App, AppContext as _, Context, Div, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::{
    button::ButtonVariants as _,
    checkbox::Checkbox,
    input::{InputEvent, TextareaState},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _, Sizable as _,
};
use serde::Serialize;

use crate::components::{notify, select::select_state, select::SimpleSelectDelegate, ui};

// ---------------------------------------------------------------------------
// 批量任务（提交到 core_rt 的 tokio Runtime 执行）
// ---------------------------------------------------------------------------

/// 并发上限。
const BATCH_CONCURRENCY: usize = 6;
/// 每个查询源的最小间隔（限流）。
const RATE_INTERVAL_MS: u64 = 700;
/// 历史容量（内存会话态）。
const HISTORY_CAP: usize = 200;
/// 历史展示条数。
const HISTORY_SHOW: usize = 20;

/// 单个域名的批量查询结果（经 mpsc 回流 UI）。
struct WhoisBatchItem {
    domain: String,
    parsed: Option<WhoisParsed>,
    error: Option<String>,
    duration_ms: u64,
}

/// 按源限流：预占该源下一个可用时间片，需要时 sleep 到位。
/// 并发任务依次排队（首个立即放行，后续逐个 +700ms），避免同时唤醒挤在同一时刻。
async fn whois_rate_wait(last_query: &Mutex<HashMap<String, Instant>>, key: &str) {
    let wait = {
        let mut map = last_query.lock().unwrap();
        let now = Instant::now();
        let earliest = map.get(key).copied().unwrap_or(now);
        let delay = earliest.saturating_duration_since(now);
        map.insert(
            key.to_string(),
            earliest.max(now) + Duration::from_millis(RATE_INTERVAL_MS),
        );
        delay
    };
    if wait > Duration::ZERO {
        tokio::time::sleep(wait).await;
    }
}

/// 批量查询任务：返回结果流（每个域名完成后立即回流一条）。
/// 并发 6：先取 Semaphore 许可再 spawn，任务结束（发送结果后）归还许可。
/// auto 模式下 core 内部会按 TLD 依次尝试多个服务器，此处按所选「源模式」
/// 整体限流 700ms。
async fn run_whois_batch(
    domains: Vec<String>,
    server: Option<String>,
    rate_key: &'static str,
) -> futures::channel::mpsc::UnboundedReceiver<WhoisBatchItem> {
    let (tx, rx) = futures::channel::mpsc::unbounded::<WhoisBatchItem>();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(BATCH_CONCURRENCY));
    let last_query: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    for domain in domains {
        // 占并发名额（最多 BATCH_CONCURRENCY 个在途查询）
        let Ok(permit) = semaphore.clone().acquire_owned().await else {
            break;
        };
        let tx = tx.clone();
        let last_query = last_query.clone();
        let server = server.clone();
        tokio::spawn(async move {
            let started = Instant::now();
            whois_rate_wait(&last_query, rate_key).await;
            let res = query_whois_unified(domain.clone(), server.clone()).await;
            let (parsed, error) = match res {
                Ok(p) => (Some(p), None),
                Err(e) => (None, Some(e)),
            };
            let _ = tx.unbounded_send(WhoisBatchItem {
                domain,
                parsed,
                error,
                duration_ms: started.elapsed().as_millis() as u64,
            });
            drop(permit); // 归还并发名额
        });
    }
    // 所有任务结束后 tx 全部 drop，rx 的 Stream 随即结束（= 批量完成）
    rx
}

// ---------------------------------------------------------------------------
// UI 状态
// ---------------------------------------------------------------------------

/// 结果表一行（含查询中状态）。
#[derive(Clone)]
struct DomainRow {
    domain: String,
    parsed: Option<WhoisParsed>,
    error: Option<String>,
    duration_ms: u64,
    /// 是否已完成（成功或失败）。
    done: bool,
}

/// 历史记录（仅成功结果；内存会话态，上限 HISTORY_CAP）。
#[derive(Clone)]
struct HistoryRecord {
    domain: String,
    parsed: WhoisParsed,
}

/// JSON 导出行（camelCase 字段名）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WhoisExportRow {
    domain: String,
    source: Option<String>,
    registrar: Option<String>,
    registrant: Option<String>,
    created: Option<String>,
    expires: Option<String>,
    updated: Option<String>,
    /// status.join("|")
    status: Option<String>,
    /// nameServers.join("|")
    name_servers: Option<String>,
    error: Option<String>,
}

/// 域名合法性校验（正则
/// `^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,}$`）。
fn is_valid_domain(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let Some((tld, labels)) = parts.split_last() else {
        return false;
    };
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|label| {
        let n = label.len();
        if n == 0 || n > 63 {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        true
    })
}

/// 查询源模式 → (传给 core 的 server 参数, 限流 key)。
fn server_for_mode(mode: &str) -> (Option<String>, &'static str) {
    match mode {
        "whois_verisign" => (Some("whois.verisign-grs.com".to_string()), "whois_verisign"),
        "whois_cndns" => (Some("grs-whois.cndns.com".to_string()), "whois_cndns"),
        "whois_hichina" => (Some("grs-whois.hichina.com".to_string()), "whois_hichina"),
        _ => (Some("auto".to_string()), "auto"),
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// CSV：BOM + 表头 + 全字段双引号转义。
fn build_csv(rows: &[DomainRow]) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str("domain,source,registrar,registrant,created,expires,updated,status,nameServers\n");
    for row in rows {
        let p = row.parsed.as_ref();
        let cols = [
            Some(row.domain.clone()),
            p.map(|p| p.source.clone()),
            p.and_then(|p| p.registrar.clone()),
            p.and_then(|p| p.registrant.clone()),
            p.and_then(|p| p.created.clone()),
            p.and_then(|p| p.expires.clone()),
            p.and_then(|p| p.updated.clone()),
            p.and_then(|p| p.status.as_ref().map(|s| s.join("|"))),
            p.and_then(|p| p.name_servers.as_ref().map(|s| s.join("|"))),
        ];
        for (i, col) in cols.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&col.clone().unwrap_or_default().replace('"', "\"\""));
            out.push('"');
        }
        out.push('\n');
    }
    out
}

/// JSON 导出（拍平为行数组，error 一并带出）。
fn build_json(rows: &[DomainRow]) -> Option<String> {
    let export: Vec<WhoisExportRow> = rows
        .iter()
        .map(|row| {
            let p = row.parsed.as_ref();
            WhoisExportRow {
                domain: row.domain.clone(),
                source: p.map(|p| p.source.clone()),
                registrar: p.and_then(|p| p.registrar.clone()),
                registrant: p.and_then(|p| p.registrant.clone()),
                created: p.and_then(|p| p.created.clone()),
                expires: p.and_then(|p| p.expires.clone()),
                updated: p.and_then(|p| p.updated.clone()),
                status: p.and_then(|p| p.status.as_ref().map(|s| s.join("|"))),
                name_servers: p.and_then(|p| p.name_servers.as_ref().map(|s| s.join("|"))),
                error: row.error.clone(),
            }
        })
        .collect();
    serde_json::to_string_pretty(&export).ok()
}

// ---------------------------------------------------------------------------
// 结果表格（自制 div 表格）
// ---------------------------------------------------------------------------

/// 列宽：选择/域名/来源/注册商/注册人/注册日期/过期日期/状态/NS 数/NS 列表/错误
const COL_W: [f32; 11] = [
    46., 150., 150., 130., 120., 150., 150., 130., 50., 220., 170.,
];
const COL_LABELS: [&str; 11] = [
    "选择",
    "域名",
    "来源",
    "注册商",
    "注册人",
    "注册日期",
    "过期日期",
    "状态",
    "NS 数",
    "NS 列表",
    "错误",
];

/// 表头单元格。
fn head_cell(width: f32, label: &str, cx: &App) -> Div {
    ui::table_head_cell(label, cx)
        .flex_none()
        .w(px(width))
        .px_1()
}

/// 数据单元格（文本自动省略；空值显示 —）。
fn data_cell(width: f32, text: &str, cx: &App) -> Div {
    let theme = cx.theme().clone();
    let value = if text.is_empty() { "—" } else { text };
    div()
        .flex_none()
        .w(px(width))
        .px_1()
        .text_size(px(12.))
        .text_color(theme.colors.foreground)
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .child(value.to_string())
}

// ---------------------------------------------------------------------------
// 视图
// ---------------------------------------------------------------------------

pub struct WhoisLookup {
    domains_input: Entity<TextareaState>,
    source_state: Entity<SelectState<SimpleSelectDelegate>>,
    /// 当前查询源模式（auto / whois_verisign / whois_cndns / whois_hichina）。
    source_mode: SharedString,
    rows: Vec<DomainRow>,
    running: bool,
    /// 勾选行（对比用），存域名。
    selected: HashSet<String>,
    history: Vec<HistoryRecord>,
}

impl WhoisLookup {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let domains_input = ui::new_textarea(window, cx);
        domains_input.update(cx, |state, cx| {
            state.set_placeholder("例如:\nexample.com\nopenai.com\ncloudflare.net", window, cx);
        });
        // 输入变化 → 重算有效域名数（按钮 N 与禁用态）
        cx.subscribe_in(
            &domains_input,
            window,
            |_, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        )
        .detach();

        let source_state = select_state(
            window,
            cx,
            &[
                ("auto", "自动（verisign → cndns → hichina）"),
                ("whois_verisign", "whois.verisign-grs.com"),
                ("whois_cndns", "grs-whois.cndns.com"),
                ("whois_hichina", "grs-whois.hichina.com"),
            ],
            0,
        );
        cx.subscribe_in(
            &source_state,
            window,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, _window, cx| {
                if let SelectEvent::Confirm(Some(v)) = event {
                    this.source_mode = v.clone();
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            domains_input,
            source_state,
            source_mode: SharedString::from("auto"),
            rows: Vec::new(),
            running: false,
            selected: HashSet::new(),
            history: Vec::new(),
        }
    }

    /// 解析有效域名：按 \s , ; 切分 + 校验 + 去重 + 小写化（保持输入顺序）。
    fn parse_domains(&self, cx: &App) -> Vec<String> {
        let raw = self.domains_input.read(cx).value().to_string();
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for token in raw.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
            let d = token.trim().to_lowercase();
            if d.is_empty() || !is_valid_domain(&d) || !seen.insert(d.clone()) {
                continue;
            }
            out.push(d);
        }
        out
    }

    // -- 批量查询 -----------------------------------------------------------

    fn start_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        let domains = self.parse_domains(cx);
        if domains.is_empty() {
            return;
        }
        let (server, rate_key) = server_for_mode(&self.source_mode);
        // 初始化行状态（全部「查询中」），并清空旧的勾选
        self.rows = domains
            .iter()
            .map(|d| DomainRow {
                domain: d.clone(),
                parsed: None,
                error: None,
                duration_ms: 0,
                done: false,
            })
            .collect();
        self.selected.clear();
        self.running = true;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            run_whois_batch(domains, server, rate_key).await
        });
        cx.spawn_in(window, async move |this, cx| {
            // core_rt 返回的「结果流本身」经第一个 mpsc 送达
            let Some(mut stream) = rx.next().await else {
                return;
            };
            // 逐个回流：每条结果立即更新对应行
            while let Some(item) = stream.next().await {
                let alive = this
                    .update_in(cx, |this, _window, cx| this.apply_item(item, cx))
                    .is_ok();
                if !alive {
                    return; // 窗口已关闭，批量任务在后台自行结束
                }
            }
            let _ = this.update_in(cx, |this, _window, cx| this.finish_batch(cx));
        })
        .detach();
    }

    fn apply_item(&mut self, item: WhoisBatchItem, cx: &mut Context<Self>) {
        let success = item.parsed.clone();
        if let Some(row) = self.rows.iter_mut().find(|r| r.domain == item.domain) {
            row.parsed = item.parsed;
            row.error = item.error;
            row.duration_ms = item.duration_ms;
            row.done = true;
        }
        // 仅成功记录历史（失败不记录）
        if let Some(parsed) = success {
            self.history.push(HistoryRecord {
                domain: item.domain,
                parsed,
            });
            if self.history.len() > HISTORY_CAP {
                let overflow = self.history.len() - HISTORY_CAP;
                self.history.drain(0..overflow);
            }
        }
        cx.notify();
    }

    fn finish_batch(&mut self, cx: &mut Context<Self>) {
        self.running = false;
        cx.notify();
    }

    // -- 导出 ---------------------------------------------------------------

    fn export(&mut self, kind: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        let rows = self.rows.clone();
        let Some(content) = (match kind {
            "csv" => Some(build_csv(&rows)),
            _ => build_json(&rows),
        }) else {
            notify::toast_error(window, cx, "序列化导出数据失败");
            return;
        };
        let name = format!("whois_results_{}.{kind}", now_millis());
        // rfd 对话框 + 文件写入放后台线程（core_rt），避免阻塞 UI
        let mut rx = crate::core_rt::submit(cx, async move {
            let dialog = rfd::FileDialog::new().set_file_name(name.as_str());
            let dialog = if kind == "csv" {
                dialog.add_filter("CSV 文件", &["csv"])
            } else {
                dialog.add_filter("JSON 文件", &["json"])
            };
            let Some(path) = dialog.save_file() else {
                return None; // 用户取消
            };
            Some(
                std::fs::write(&path, content.as_bytes())
                    .map(|_| path)
                    .map_err(|e| e.to_string()),
            )
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(outcome) = rx.next().await else {
                return;
            };
            let _ = this.update_in(cx, |_this, window, cx| match outcome {
                Some(Ok(path)) => {
                    let msg = format!("已导出：{}", path.display());
                    notify::toast_success(window, cx, msg);
                }
                Some(Err(e)) => {
                    let msg = format!("导出失败：{e}");
                    notify::toast_error(window, cx, msg);
                }
                None => {}
            });
        })
        .detach();
    }

    fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.history.clear();
        cx.notify();
    }

    // -- 统计 ---------------------------------------------------------------

    fn stats(&self) -> (usize, usize, usize, u64) {
        let total = self.rows.len();
        let done = self.rows.iter().filter(|r| r.done).count();
        let success = self.rows.iter().filter(|r| r.parsed.is_some()).count();
        let avg = {
            let done_rows: Vec<u64> = self
                .rows
                .iter()
                .filter(|r| r.done)
                .map(|r| r.duration_ms)
                .collect();
            if done_rows.is_empty() {
                0
            } else {
                let sum: u64 = done_rows.iter().sum();
                sum / done_rows.len() as u64
            }
        };
        (total, done, success, avg)
    }

    // -- 渲染 ---------------------------------------------------------------

    fn render_input_area(&mut self, domains: &[String], cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let n = domains.len();
        let run_label = if self.running {
            "查询中...".to_string()
        } else {
            format!("开始查询 ({n})")
        };
        let running = self.running;

        // 左侧输入区 + 右侧统计卡（两栏布局）
        let mut stats_card = ui::card(cx).w(px(240.));
        let (total, done, success, avg) = self.stats();
        let rate = (success * 100).checked_div(total).unwrap_or(0) as u64;
        for (label, value) in [
            ("总数", total.to_string()),
            ("已完成", done.to_string()),
            ("成功", format!("{success}（成功率 {rate}%）")),
            ("平均响应时间", format!("{avg} ms")),
        ] {
            stats_card = stats_card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child(label.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.colors.foreground)
                            .child(value),
                    ),
            );
        }

        div()
            .flex()
            .flex_row()
            .gap_3()
            .items_start()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_2()
                    .child(ui::field(
                        "批量域名（每行一个或用逗号/空格分隔）",
                        ui::edit_area(&self.domains_input, 160.),
                        cx,
                    ))
                    .child(ui::hfield(
                        "查询源",
                        Select::new(&self.source_state).w(px(280.)),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::primary_button(
                                    "whois-run",
                                    &run_label,
                                    cx.listener(|this, _ev: &gpui::ClickEvent, window, cx| {
                                        this.start_query(window, cx);
                                    }),
                                )
                                .loading(running)
                                .disabled(running || n == 0),
                            )
                            .child(
                                ui::secondary_button(
                                    "whois-export-csv",
                                    "导出 CSV",
                                    cx.listener(|this, _ev: &gpui::ClickEvent, window, cx| {
                                        this.export("csv", window, cx);
                                    }),
                                )
                                .success()
                                .disabled(self.rows.is_empty()),
                            )
                            .child(
                                ui::secondary_button(
                                    "whois-export-json",
                                    "导出 JSON",
                                    cx.listener(|this, _ev: &gpui::ClickEvent, window, cx| {
                                        this.export("json", window, cx);
                                    }),
                                )
                                .disabled(self.rows.is_empty()),
                            )
                            .child(ui::danger_button(
                                "whois-clear-history",
                                "清空历史",
                                cx.listener(|this, _ev: &gpui::ClickEvent, _window, cx| {
                                    this.clear_history(cx);
                                }),
                            )),
                    ),
            )
            .child(stats_card)
    }

    fn render_table(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let mut header = ui::table_head(cx);
        for (i, label) in COL_LABELS.iter().enumerate() {
            header = header.child(head_cell(COL_W[i], label, cx));
        }

        let mut rows = Vec::new();
        for (idx, row) in self.rows.iter().enumerate() {
            let p = row.parsed.as_ref();
            let checked = self.selected.contains(&row.domain);
            let status = p
                .and_then(|p| p.status.as_ref().map(|s| s.join("|")))
                .unwrap_or_default();
            let ns_list = p
                .and_then(|p| p.name_servers.as_ref())
                .map(|ns| {
                    let mut s = ns.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                    if ns.len() > 3 {
                        s.push_str(" …");
                    }
                    s
                })
                .unwrap_or_default();
            let ns_count = p
                .and_then(|p| p.name_servers.as_ref().map(|ns| ns.len().to_string()))
                .unwrap_or_default();

            let mut line = ui::table_row();

            // 选择列：checkbox（小号）
            line = line.child(
                div().flex_none().w(px(COL_W[0])).px_1().child(
                    Checkbox::new(("whois-select", idx))
                        .checked(checked)
                        .small()
                        .on_click({
                            let domain = row.domain.clone();
                            cx.listener(move |this, checked: &bool, _window, cx| {
                                if *checked {
                                    this.selected.insert(domain.clone());
                                } else {
                                    this.selected.remove(&domain);
                                }
                                cx.notify();
                            })
                        }),
                ),
            );
            // 域名（等宽）
            line = line.child(
                div()
                    .flex_none()
                    .w(px(COL_W[1]))
                    .px_1()
                    .text_size(px(12.))
                    .font_family("Menlo")
                    .text_color(theme.colors.foreground)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(row.domain.clone()),
            );
            // 来源（查询中显示占位）
            let source_text = if !row.done {
                "查询中…".to_string()
            } else {
                p.map(|p| p.source.clone()).unwrap_or_default()
            };
            line = line.child(data_cell(COL_W[2], &source_text, cx));
            line = line.child(data_cell(
                COL_W[3],
                &p.and_then(|p| p.registrar.clone()).unwrap_or_default(),
                cx,
            ));
            line = line.child(data_cell(
                COL_W[4],
                &p.and_then(|p| p.registrant.clone()).unwrap_or_default(),
                cx,
            ));
            line = line.child(data_cell(
                COL_W[5],
                &p.and_then(|p| p.created.clone()).unwrap_or_default(),
                cx,
            ));
            line = line.child(data_cell(
                COL_W[6],
                &p.and_then(|p| p.expires.clone()).unwrap_or_default(),
                cx,
            ));
            line = line.child(data_cell(COL_W[7], &status, cx));
            line = line.child(data_cell(COL_W[8], &ns_count, cx));
            line = line.child(data_cell(COL_W[9], &ns_list, cx));
            // 错误（红色）
            line = line.child(
                div()
                    .flex_none()
                    .w(px(COL_W[10]))
                    .px_1()
                    .text_size(px(12.))
                    .text_color(theme.colors.danger)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(row.error.clone().unwrap_or_default()),
            );
            rows.push(line);
        }

        let mut body = div().flex().flex_col();
        if self.rows.is_empty() {
            body = body.child(ui::empty_hint(
                "暂无查询结果，输入域名后点击「开始查询」",
                cx,
            ));
        } else {
            body = body.child(ui::table_rows(rows, cx));
        }

        ui::card(cx).child(ui::card_header("查询结果", cx)).child(
            div().id("whois-table").overflow_x_scroll().child(
                div()
                    .flex()
                    .flex_col()
                    .min_w(px(1470.))
                    .child(header)
                    .child(body),
            ),
        )
    }

    /// 结果对比（勾选 ≥2 时渲染「字段 × 所选域名」矩阵）。
    fn render_compare(&self, cx: &App) -> Div {
        let theme = cx.theme().clone();
        let sel: Vec<&DomainRow> = self
            .rows
            .iter()
            .filter(|r| self.selected.contains(&r.domain))
            .collect();
        if sel.len() < 2 {
            return div();
        }

        let field_of = |p: Option<&WhoisParsed>,
                        pick: fn(&WhoisParsed) -> Option<String>|
         -> String { p.and_then(pick).unwrap_or_else(|| "—".to_string()) };
        type WhoisRowDef = (&'static str, fn(&WhoisParsed) -> Option<String>);
        let rows_def: Vec<WhoisRowDef> = vec![
            ("来源", |p: &WhoisParsed| Some(p.source.clone())),
            ("注册商", |p: &WhoisParsed| p.registrar.clone()),
            ("注册人", |p: &WhoisParsed| p.registrant.clone()),
            ("注册日期", |p: &WhoisParsed| p.created.clone()),
            ("过期日期", |p: &WhoisParsed| p.expires.clone()),
            ("更新日期", |p: &WhoisParsed| p.updated.clone()),
            ("状态", |p: &WhoisParsed| {
                p.status.as_ref().map(|s| s.join("|"))
            }),
            ("NS 列表", |p: &WhoisParsed| {
                p.name_servers.as_ref().map(|s| s.join(", "))
            }),
        ];

        // 表头：字段 + 所选域名
        let mut header = ui::table_head(cx).child(head_cell(90., "字段", cx));
        for row in &sel {
            header = header.child(
                div()
                    .flex_1()
                    .min_w(px(140.))
                    .px_1()
                    .text_size(px(12.))
                    .font_family("Menlo")
                    .text_color(theme.colors.foreground)
                    .child(row.domain.clone()),
            );
        }

        let mut rows = Vec::new();
        for (label, pick) in &rows_def {
            let mut line = ui::table_row().child(
                div()
                    .flex_none()
                    .w(px(90.))
                    .px_1()
                    .text_size(px(12.))
                    .text_color(theme.colors.muted_foreground)
                    .child((*label).to_string()),
            );
            for row in &sel {
                let text = field_of(row.parsed.as_ref(), *pick);
                line = line.child(
                    div()
                        .flex_1()
                        .min_w(px(140.))
                        .px_1()
                        .text_size(px(12.))
                        .text_color(theme.colors.foreground)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(text),
                );
            }
            rows.push(line);
        }

        ui::card(cx)
            .child(ui::card_header(&format!("结果对比（{}）", sel.len()), cx))
            .child(
                div().id("whois-compare").overflow_x_scroll().child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w(px(370. + 140. * sel.len() as f32))
                        .child(header)
                        .child(ui::table_rows(rows, cx)),
                ),
            )
    }

    fn render_history(&self, cx: &App) -> Div {
        let theme = cx.theme().clone();
        let mut card = ui::card(cx)
            .child(ui::card_header("查询历史", cx))
            .child(ui::hint_text(
                "保留最近 200 条，以下展示最近 20 条（仅本会话有效）",
                cx,
            ));
        if self.history.is_empty() {
            card = card.child(ui::empty_hint("暂无查询历史", cx));
            return card;
        }
        let mut grid = div().flex().flex_wrap().gap_2();
        for record in self.history.iter().rev().take(HISTORY_SHOW) {
            let p = &record.parsed;
            grid = grid.child(
                ui::panel(cx)
                    .min_w(px(240.))
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_family("Menlo")
                            .text_color(theme.colors.foreground)
                            .child(record.domain.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!(
                                "来源：{}；注册商：{}",
                                p.source,
                                p.registrar.as_deref().unwrap_or("—")
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!(
                                "注册：{}；过期：{}",
                                p.created.as_deref().unwrap_or("—"),
                                p.expires.as_deref().unwrap_or("—")
                            )),
                    ),
            );
        }
        card.child(grid)
    }
}

impl gpui::Render for WhoisLookup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let domains = self.parse_domains(cx);
        ui::tool_page("whois")
            .child(self.render_input_area(&domains, cx))
            .child(self.render_table(cx))
            .child(self.render_compare(cx))
            .child(self.render_history(cx))
            .child(ui::hint_text(
                "频率限制：WHOIS 每 700ms（按服务器区分）。查询历史保存在本次会话中。失败不记录历史。",
                cx,
            ))
    }
}
