//! MD5 加密工具。
//!
//! 三 Tab：文本加密（输入即算，无防抖）/ 文件加密（rfd 选文件 + 后台读取）/ 批量加密
//! （多选文件逐个后台处理、行内状态徽章、复制/导出结果）。
//! 文本哈希为同步纯函数（devtools::crypto::md5_hex）；文件哈希走 core 的
//! 流式 md5_file_hex（64KB 逐块读取，GB 级文件内存占用恒定）。

use std::path::PathBuf;

use devtools::crypto::{md5_file_hex, md5_hex};
use devtools::i18n::t;
use futures::StreamExt as _;
use gpui::{div, prelude::*, px, App, Context, Div, Entity, IntoElement, Render, Window};
use gpui_component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_component::{tab::TabBar, ActiveTheme as _, Disableable as _};

use crate::components::{notify, ui};

/// 示例文本。
const EXAMPLE_TEXT: &str = "Hello, World! 你好，世界！";

/// 输出占位文案。
const HASH_PLACEHOLDER: &str = "MD5 哈希值将在这里显示...";

/// 批量空态文案。
const BATCH_EMPTY_HINT: &str = "点击\"选择文件\"按钮选择多个文件进行批量MD5加密";

pub struct Md5Crypto {
    tab: Tab,
    // 文本加密
    text_input: Entity<TextareaState>,
    text_len: usize,
    text_hash: String,
    text_error: Option<String>,
    // 文件加密
    file_path_input: Entity<InputState>,
    file_path: Option<PathBuf>,
    file_hash: String,
    file_error: Option<String>,
    file_loading: bool,
    /// 文件计算任务序号（丢弃过期任务的结果）。
    file_task: u64,
    // 批量加密
    batch_files: Vec<BatchFile>,
    batch_processing: bool,
    /// 批量选择代数（丢弃过期批量循环的状态回写）。
    batch_generation: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Text,
    File,
    Batch,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Error,
}

struct BatchFile {
    path: PathBuf,
    name: String,
    hash: String,
    status: BatchStatus,
}

impl Md5Crypto {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_input =
            cx.new(|cx| TextareaState::new(window, cx).placeholder(t("请输入要加密的文本...")));
        // 输入即算（无防抖）：Change 直接调 core 同步哈希
        cx.subscribe(&text_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.compute_text_hash(cx);
            }
        })
        .detach();

        let file_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("点击右边按钮选择文件...")));

        Self {
            tab: Tab::Text,
            text_input,
            text_len: 0,
            text_hash: String::new(),
            text_error: None,
            file_path_input,
            file_path: None,
            file_hash: String::new(),
            file_error: None,
            file_loading: false,
            file_task: 0,
            batch_files: Vec::new(),
            batch_processing: false,
            batch_generation: 0,
        }
    }

    // ------------------------------------------------------------------
    // 文本加密
    // ------------------------------------------------------------------

    fn compute_text_hash(&mut self, cx: &mut Context<Self>) {
        let text = self.text_input.read(cx).value().to_string();
        self.text_len = text.chars().count();
        if text.is_empty() {
            self.text_hash.clear();
            self.text_error = None;
            cx.notify();
            return;
        }
        match hash_text(&text) {
            Ok(hash) => {
                self.text_hash = hash;
                self.text_error = None;
            }
            Err(msg) => {
                self.text_hash.clear();
                self.text_error = Some(format!("{}: {msg}", t("哈希过程中发生错误")));
            }
        }
        cx.notify();
    }

    /// 示例 / 清空共用：set_value 不触发 Change 事件，需手动重算。
    fn set_text_input(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.text_input
            .update(cx, |input, cx| input.set_value(value, window, cx));
        self.compute_text_hash(cx);
    }

    // ------------------------------------------------------------------
    // 文件加密
    // ------------------------------------------------------------------

    /// 选择文件：rfd 对话框放 core_rt 后台（阻塞主线程会卡 UI）。
    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rx = crate::core_rt::submit(cx, async move { rfd::FileDialog::new().pick_file() });
        cx.spawn_in(window, async move |this, cx| {
            let Some(Some(path)) = rx.next().await else {
                return; // 用户取消选择
            };
            let _ = this.update_in(cx, |this, window, cx| this.on_file_picked(path, window, cx));
        })
        .detach();
    }

    fn on_file_picked(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.file_path = Some(path.clone());
        self.file_hash.clear();
        self.file_error = None;
        self.file_path_input.update(cx, |input, cx| {
            input.set_value(path.display().to_string(), window, cx);
        });
        // 路径变化自动计算
        self.start_file_hash(cx);
    }

    /// 后台读取文件并计算哈希（完成回调无需 window，普通 spawn 即可）。
    fn start_file_hash(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.file_path.clone() else {
            self.file_error = Some(t("请先选择一个文件").to_string());
            cx.notify();
            return;
        };
        self.file_task += 1;
        let task = self.file_task;
        self.file_loading = true;
        self.file_error = None;
        self.file_hash.clear();
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move { md5_file_hex(&path) });
        cx.spawn(async move |this, cx| {
            let result = rx
                .next()
                .await
                .unwrap_or_else(|| Err(std::io::Error::other(t("任务已取消"))));
            let _ = this.update(cx, |this, cx| this.on_file_hashed(task, result, cx));
        })
        .detach();
    }

    fn on_file_hashed(
        &mut self,
        task: u64,
        result: std::io::Result<String>,
        cx: &mut Context<Self>,
    ) {
        if task != self.file_task {
            return; // 已有更新的选择/清空，丢弃过期结果
        }
        self.file_loading = false;
        match result {
            Ok(hash) => self.file_hash = hash,
            Err(err) => {
                self.file_hash.clear();
                self.file_error = Some(format!("{}: {err}", t("处理文件时发生错误")));
            }
        }
        cx.notify();
    }

    fn clear_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.file_path = None;
        self.file_hash.clear();
        self.file_error = None;
        self.file_loading = false;
        self.file_task += 1; // 使仍在运行的任务结果失效
        self.file_path_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 批量加密
    // ------------------------------------------------------------------

    fn pick_batch_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rx = crate::core_rt::submit(cx, async move { rfd::FileDialog::new().pick_files() });
        cx.spawn_in(window, async move |this, cx| {
            let Some(Some(files)) = rx.next().await else {
                return;
            };
            if files.is_empty() {
                return;
            }
            let _ = this.update_in(cx, |this, _window, cx| this.on_batch_picked(files, cx));
        })
        .detach();
    }

    /// 顺序逐个处理：每个文件一个后台任务，回流后处理下一个。
    fn on_batch_picked(&mut self, files: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.batch_generation += 1;
        let generation = self.batch_generation;
        self.batch_files = files
            .into_iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                BatchFile {
                    path,
                    name,
                    hash: String::new(),
                    status: BatchStatus::Pending,
                }
            })
            .collect();
        self.batch_processing = !self.batch_files.is_empty();
        cx.notify();

        let paths: Vec<PathBuf> = self.batch_files.iter().map(|f| f.path.clone()).collect();
        cx.spawn(async move |this, cx| {
            for path in paths {
                let _ = this.update(cx, |this, cx| {
                    if let Some(row) = this.batch_files.iter_mut().find(|f| f.path == path) {
                        row.status = BatchStatus::Processing;
                        cx.notify();
                    }
                });
                // AsyncApp::update 直接返回值（区别于 AsyncWindowContext::update 的 Result）
                let mut rx = cx.update(|cx| {
                    crate::core_rt::submit(cx, {
                        let path = path.clone();
                        async move { md5_file_hex(&path) }
                    })
                });
                let result = match rx.next().await {
                    Some(result) => result,
                    None => Err(std::io::Error::other(t("任务已取消"))),
                };
                let _ = this.update(cx, |this, cx| {
                    if generation != this.batch_generation {
                        return;
                    }
                    if let Some(row) = this.batch_files.iter_mut().find(|f| f.path == path) {
                        match result {
                            Ok(hash) => {
                                row.hash = hash;
                                row.status = BatchStatus::Completed;
                            }
                            Err(_) => {
                                // 单文件失败仅置 error，不弹 UI
                                row.hash.clear();
                                row.status = BatchStatus::Error;
                            }
                        }
                        cx.notify();
                    }
                });
            }
            let _ = this.update(cx, |this, cx| {
                if generation == this.batch_generation {
                    this.batch_processing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn clear_batch(&mut self, cx: &mut Context<Self>) {
        self.batch_generation += 1;
        self.batch_files.clear();
        self.batch_processing = false;
        cx.notify();
    }

    fn completed_files(&self) -> Vec<&BatchFile> {
        self.batch_files
            .iter()
            .filter(|f| f.status == BatchStatus::Completed)
            .collect()
    }

    /// 已完成行 `名: 哈希` 换行拼接。
    fn completed_results_text(&self) -> String {
        self.completed_files()
            .iter()
            .map(|f| format!("{}: {}", f.name, f.hash))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 导出结果：rfd save_file（txt）+ std::fs::write 全在后台，完成后 toast。
    fn export_results(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let results = self.completed_results_text();
        if results.is_empty() {
            return;
        }
        let mut rx = crate::core_rt::submit(cx, async move {
            let Some(path) = rfd::FileDialog::new()
                .add_filter(t("文本文件"), &["txt"])
                .save_file()
            else {
                return Ok(None); // 用户取消
            };
            std::fs::write(&path, results.as_bytes())
                .map(|()| Some(path))
                .map_err(|err| err.to_string())
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = rx.next().await else {
                return;
            };
            let _ = this.update_in(cx, |_this, window, cx| match result {
                Ok(Some(path)) => notify::toast_success(
                    window,
                    cx,
                    format!("{} {}", t("结果已导出到"), path.display()),
                ),
                Ok(None) => {}
                Err(err) => {
                    notify::toast_error(window, cx, format!("{}: {err}", t("导出结果失败")))
                }
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    /// Tab 分段控件（文本 / 文件 / 批量）。
    fn render_tab_bar(&self, cx: &Context<Self>) -> TabBar {
        let selected = match self.tab {
            Tab::Text => 0,
            Tab::File => 1,
            Tab::Batch => 2,
        };
        ui::tab_bar(
            "md5-tabs",
            &[t("文本加密"), t("文件加密"), t("批量加密")],
            selected,
            cx.listener(move |this, ix: &usize, _window, cx| {
                this.tab = match ix {
                    1 => Tab::File,
                    2 => Tab::Batch,
                    _ => Tab::Text,
                };
                cx.notify();
            }),
        )
    }

    fn render_text_tab(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme().colors;
        let hash_display = if self.text_hash.is_empty() {
            t(HASH_PLACEHOLDER).to_string()
        } else {
            self.text_hash.clone()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            // 输入卡：长度 + 示例/清空 + textarea
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{}: {}", t("输入长度"), self.text_len)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(ui::secondary_button(
                                        "md5-example",
                                        t("示例"),
                                        cx.listener(|this, _, window, cx| {
                                            this.set_text_input(EXAMPLE_TEXT, window, cx);
                                        }),
                                    ))
                                    .child(
                                        ui::secondary_button(
                                            "md5-clear-text",
                                            t("清空"),
                                            cx.listener(|this, _, window, cx| {
                                                this.set_text_input("", window, cx);
                                            }),
                                        )
                                        .disabled(self.text_len == 0),
                                    ),
                            ),
                    )
                    .child(ui::edit_area(&self.text_input, 160.)),
            )
            .children(self.text_error.clone().map(|e| ui::error_banner(&e, cx)))
            // 输出卡：浅底等宽块 + 复制
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(ui::card_header(t("MD5 哈希值"), cx))
                            .children((!self.text_hash.is_empty()).then(|| {
                                ui::copy_result_button(
                                    "md5-copy-text",
                                    t("复制结果"),
                                    self.text_hash.clone(),
                                )
                            })),
                    )
                    .child(ui::mono_block("mono-md5-text", hash_display, cx).min_h(px(96.))),
            )
    }

    fn render_file_tab(&self, cx: &Context<Self>) -> Div {
        let hash_display = if self.file_hash.is_empty() {
            t(HASH_PLACEHOLDER).to_string()
        } else {
            self.file_hash.clone()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            // 选择文件卡
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(ui::card_header(t("选择文件"), cx))
                            .child(
                                ui::secondary_button(
                                    "md5-file-clear",
                                    t("清空"),
                                    cx.listener(|this, _, window, cx| {
                                        this.clear_file(window, cx);
                                    }),
                                )
                                .disabled(self.file_path.is_none()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    // readonly：路径可选中复制（disabled 会禁掉选择）
                                    .child(Input::new(&self.file_path_input).readonly(true)),
                            )
                            .child(ui::secondary_button(
                                "md5-file-pick",
                                t("选择文件"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_file(window, cx);
                                }),
                            )),
                    ),
            )
            .children(self.file_error.clone().map(|e| ui::error_banner(&e, cx)))
            // 输出卡：处理中 loading / 哈希块
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(ui::card_header(t("MD5 哈希值"), cx))
                            .children(
                                (!self.file_loading
                                    && self.file_error.is_none()
                                    && !self.file_hash.is_empty())
                                .then(|| {
                                    ui::copy_result_button(
                                        "md5-copy-file",
                                        t("复制结果"),
                                        self.file_hash.clone(),
                                    )
                                }),
                            ),
                    )
                    .child(if self.file_loading {
                        ui::loading_row(t("正在处理文件..."), cx).into_any_element()
                    } else {
                        ui::mono_block("mono-md5-file", hash_display, cx)
                            .min_h(px(96.))
                            .into_any_element()
                    }),
            )
    }

    fn render_batch_tab(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme().colors;
        let has_completed = self
            .batch_files
            .iter()
            .any(|f| f.status == BatchStatus::Completed);

        let mut container = div().flex().flex_col().gap_3().child(
            ui::card(cx).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(ui::card_header(
                        &format!("{} ({})", t("选择多个文件"), self.batch_files.len()),
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::secondary_button(
                                    "md5-batch-clear",
                                    t("清空"),
                                    cx.listener(|this, _, _, cx| {
                                        this.clear_batch(cx);
                                    }),
                                )
                                .disabled(self.batch_files.is_empty()),
                            )
                            .child(ui::secondary_button(
                                "md5-batch-pick",
                                t("选择文件"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_batch_files(window, cx);
                                }),
                            )),
                    ),
            ),
        );

        if self.batch_files.is_empty() {
            container = container.child(ui::empty_hint(t(BATCH_EMPTY_HINT), cx));
        } else {
            let results = self.completed_results_text();
            let list_card = ui::card(cx)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(ui::card_header(t("文件列表"), cx))
                                .children(
                                    self.batch_processing
                                        .then(|| ui::badge(t("处理中..."), theme.info)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .children(has_completed.then(|| {
                                    ui::copy_result_button("md5-copy-batch", t("复制结果"), results)
                                }))
                                .child(
                                    ui::secondary_button(
                                        "md5-batch-export",
                                        t("导出结果"),
                                        cx.listener(|this, _, window, cx| {
                                            this.export_results(window, cx);
                                        }),
                                    )
                                    .disabled(!has_completed),
                                ),
                        ),
                )
                .child(self.render_batch_table(cx));
            container = container.child(list_card);
        }
        container
    }

    /// 批量表格：文件名 / 状态 / 哈希值。
    fn render_batch_table(&self, cx: &Context<Self>) -> Div {
        let header = ui::table_head(cx)
            .child(ui::table_head_cell(t("文件名"), cx).w(px(220.)))
            .child(ui::table_head_cell(t("状态"), cx).w(px(72.)))
            .child(ui::table_head_cell(t("MD5 哈希值"), cx).flex_1());

        let rows = self
            .batch_files
            .iter()
            .map(|file| {
                let hash_text = if file.hash.is_empty() {
                    "-".to_string()
                } else {
                    file.hash.clone()
                };
                ui::table_row()
                    .px_3()
                    .text_size(px(12.))
                    .child(
                        div()
                            .w(px(220.))
                            .pr_2()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(file.name.clone()),
                    )
                    .child(div().w(px(72.)).child(status_badge(file.status, cx)))
                    .child(
                        div()
                            .flex_1()
                            .pr_2()
                            .overflow_hidden()
                            .font_family("Menlo")
                            .text_ellipsis_middle()
                            .child(hash_text),
                    )
            })
            .collect::<Vec<_>>();

        ui::panel(cx).child(header).child(ui::table_rows(rows, cx))
    }
}

/// 计算文本哈希（同步纯函数）。
fn hash_text(text: &str) -> Result<String, String> {
    Ok(md5_hex(text))
}

/// 批量行状态徽章。
fn status_badge(status: BatchStatus, cx: &App) -> Div {
    let colors = cx.theme().colors;
    match status {
        BatchStatus::Pending => ui::badge(t("待处理"), colors.muted_foreground),
        BatchStatus::Processing => ui::badge(t("处理中"), colors.info),
        BatchStatus::Completed => ui::badge(t("已完成"), colors.success),
        BatchStatus::Error => ui::badge(t("错误"), colors.danger),
    }
}

impl Render for Md5Crypto {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut page = ui::tool_page("md5crypto").child(self.render_tab_bar(cx));

        page = match self.tab {
            Tab::Text => page.child(self.render_text_tab(cx)),
            Tab::File => page.child(self.render_file_tab(cx)),
            Tab::Batch => page.child(self.render_batch_tab(cx)),
        };
        page
    }
}
