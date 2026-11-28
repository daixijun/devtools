//! 哈希工具页共享实现（Md5Crypto / ShaCrypto 共用的三 Tab 编排）。
//!
//! 页面 = 算法集（[`HashAlgoSpec`] 列表）+ 三 Tab（文本加密 / 文件加密 /
//! 批量加密）的通用状态与交互：文本输入即算（无防抖）、文件选择后自动
//! 计算（file_task 作废过期任务）、批量逐文件顺序后台处理（batch 代数
//! 作废过期回写、以批次所用算法为准）。单算法工具传单项 spec（不渲染
//! 算法选择栏），多算法工具传完整列表（顶部 select，切换立即重算文本页
//! 与文件页）。具体页面文件只提供算法派发与静态装配参数。

use std::path::{Path, PathBuf};

use devtools::i18n::t;
use futures::StreamExt as _;
use gpui::{div, prelude::*, px, App, Context, Div, Entity, Window};
use gpui_component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{tab::TabBar, ActiveTheme as _, Disableable as _};

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::{notify, ui};

/// 示例文本。
pub const EXAMPLE_TEXT: &str = "Hello, World! 你好，世界！";

/// 单个哈希算法的派发与展示描述。
pub struct HashAlgoSpec {
    /// 算法内部 id（select value，如 "sha256"）
    pub id: &'static str,
    /// 算法展示名（输出标题/占位文案，如 "SHA-256"）
    pub name: &'static str,
    /// 文本哈希（同步纯函数）
    pub text_hash: fn(&str) -> Result<String, String>,
    /// 文件哈希（core 流式实现，IO 错误转字符串）
    pub file_hash: fn(&Path) -> Result<String, String>,
}

/// 页面元素 id 集（gpui 状态组件 id 须为 `&'static str`，按页面前缀区分）。
pub struct HashPageIds {
    pub tabs: &'static str,
    pub example: &'static str,
    pub clear_text: &'static str,
    pub copy_text: &'static str,
    pub mono_text: &'static str,
    pub file_clear: &'static str,
    pub file_pick: &'static str,
    pub copy_file: &'static str,
    pub mono_file: &'static str,
    pub batch_clear: &'static str,
    pub batch_pick: &'static str,
    pub copy_batch: &'static str,
    pub batch_export: &'static str,
}

/// 页面静态装配参数。
pub struct HashPageSpec {
    /// tool_page 标识（registry id）
    pub page_id: &'static str,
    pub algorithms: &'static [HashAlgoSpec],
    /// 默认算法下标
    pub default_selected: usize,
    pub ids: HashPageIds,
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

/// 三 Tab 哈希工具页的完整状态与编排。
pub struct HashPageCore {
    spec: &'static HashPageSpec,
    /// 当前算法下标（多算法工具经 select 切换）
    selected: usize,
    /// 批量任务所用算法下标（表头/结果以批次为准，切换全局算法不影响进行中的批次）
    batch_selected: usize,
    /// 顶部全局算法 select（单算法工具不创建/不渲染）
    algo_select: Option<Entity<SelectState<SimpleSelectDelegate>>>,
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

impl HashPageCore {
    pub fn new(spec: &'static HashPageSpec, window: &mut Window, cx: &mut Context<Self>) -> Self {
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

        // 多算法工具：全局算法 select，切换立即重算文本页 + 文件页
        let algo_select = if spec.algorithms.len() > 1 {
            let options: Vec<(&str, &str)> =
                spec.algorithms.iter().map(|a| (a.id, a.name)).collect();
            let select = select_state(window, cx, &options, spec.default_selected);
            cx.subscribe(
                &select,
                |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        if let Some(index) = this
                            .spec
                            .algorithms
                            .iter()
                            .position(|a| a.id == value.as_ref())
                        {
                            this.selected = index;
                            this.compute_text_hash(cx);
                            // 切换算法必须重算文件哈希：跳过会让旧算法的结果
                            // 冠以新算法标题（start_file_hash 已靠 file_task 自增作废旧任务）
                            if this.file_path.is_some() {
                                this.start_file_hash(cx);
                            }
                            cx.notify();
                        }
                    }
                },
            )
            .detach();
            Some(select)
        } else {
            None
        };

        Self {
            spec,
            selected: spec.default_selected,
            batch_selected: spec.default_selected,
            algo_select,
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

    fn current_algo(&self) -> &'static HashAlgoSpec {
        &self.spec.algorithms[self.selected]
    }

    /// 输出标题（如 "SHA-256 哈希值"）。
    fn hash_label(algo: &HashAlgoSpec) -> String {
        format!("{} {}", algo.name, t("哈希值"))
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
        match (self.current_algo().text_hash)(&text) {
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

    /// 后台读取文件并按当前算法计算哈希（完成回调无需 window，普通 spawn 即可）。
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

        let file_hash = self.current_algo().file_hash;
        let mut rx = crate::core_rt::submit(cx, async move {
            // 文件流式哈希（core 统一实现；PDF/DOCX 等二进制文件直接按字节计算）
            file_hash(&path)
        });
        cx.spawn(async move |this, cx| {
            let result = rx
                .next()
                .await
                .unwrap_or_else(|| Err(t("任务已取消").to_string()));
            let _ = this.update(cx, |this, cx| this.on_file_hashed(task, result, cx));
        })
        .detach();
    }

    fn on_file_hashed(
        &mut self,
        task: u64,
        result: Result<String, String>,
        cx: &mut Context<Self>,
    ) {
        if task != self.file_task {
            return; // 已有更新的选择/清空，丢弃过期结果
        }
        self.file_loading = false;
        match result {
            Ok(hash) => self.file_hash = hash,
            Err(msg) => {
                self.file_hash.clear();
                self.file_error = Some(format!("{}: {msg}", t("处理文件时发生错误")));
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

    /// 顺序逐个处理：每个文件一个后台任务，回流后处理下一个（算法取任务开始时的值）。
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
        // 表头与批量结果以批次所用算法为准（任务闭包里捕获的是同一值）
        self.batch_selected = self.selected;
        cx.notify();

        let file_hash = self.spec.algorithms[self.batch_selected].file_hash;
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
                        async move { file_hash(&path) }
                    })
                });
                let result = match rx.next().await {
                    Some(result) => result,
                    None => Err(t("任务已取消").to_string()),
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

    /// 顶部全局算法 select（多算法工具）。
    fn render_algorithm_bar(&self, cx: &Context<Self>) -> Div {
        let algo_select = self
            .algo_select
            .as_ref()
            .expect("多算法工具必有算法 select");
        ui::card(cx).child(ui::hfield(t("选择哈希算法:"), Select::new(algo_select), cx))
    }

    /// Tab 分段控件（文本 / 文件 / 批量）。
    fn render_tab_bar(&self, cx: &Context<Self>) -> TabBar {
        let ids = &self.spec.ids;
        let selected = match self.tab {
            Tab::Text => 0,
            Tab::File => 1,
            Tab::Batch => 2,
        };
        ui::tab_bar(
            ids.tabs,
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
        let ids = &self.spec.ids;
        let label = Self::hash_label(self.current_algo());
        let hash_display = if self.text_hash.is_empty() {
            format!(
                "{} {}",
                self.current_algo().name,
                t("哈希值将在这里显示...")
            )
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
                                        ids.example,
                                        t("示例"),
                                        cx.listener(|this, _, window, cx| {
                                            this.set_text_input(EXAMPLE_TEXT, window, cx);
                                        }),
                                    ))
                                    .child(
                                        ui::secondary_button(
                                            ids.clear_text,
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
            // 输出卡：浅底等宽块 + 复制（标题随算法）
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(ui::card_header(&label, cx))
                            .children((!self.text_hash.is_empty()).then(|| {
                                ui::copy_result_button(
                                    ids.copy_text,
                                    t("复制结果"),
                                    self.text_hash.clone(),
                                )
                            })),
                    )
                    .child(ui::mono_block(ids.mono_text, hash_display, cx).min_h(px(96.))),
            )
    }

    fn render_file_tab(&self, cx: &Context<Self>) -> Div {
        let ids = &self.spec.ids;
        let label = Self::hash_label(self.current_algo());
        let hash_display = if self.file_hash.is_empty() {
            format!(
                "{} {}",
                self.current_algo().name,
                t("哈希值将在这里显示...")
            )
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
                                    ids.file_clear,
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
                                ids.file_pick,
                                t("选择文件"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_file(window, cx);
                                }),
                            )),
                    ),
            )
            .children(self.file_error.clone().map(|e| ui::error_banner(&e, cx)))
            // 输出卡：处理中 loading / 哈希块（标题随算法）
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(ui::card_header(&label, cx))
                            .children(
                                (!self.file_loading
                                    && self.file_error.is_none()
                                    && !self.file_hash.is_empty())
                                .then(|| {
                                    ui::copy_result_button(
                                        ids.copy_file,
                                        t("复制结果"),
                                        self.file_hash.clone(),
                                    )
                                }),
                            ),
                    )
                    .child(if self.file_loading {
                        ui::loading_row(t("正在处理文件..."), cx).into_any_element()
                    } else {
                        ui::mono_block(ids.mono_file, hash_display, cx)
                            .min_h(px(96.))
                            .into_any_element()
                    }),
            )
    }

    fn render_batch_tab(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme().colors;
        let ids = &self.spec.ids;
        // 批量表头取批次所用算法（进行中切换全局算法不改变批次归属）
        let label = Self::hash_label(&self.spec.algorithms[self.batch_selected]);
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
                                    ids.batch_clear,
                                    t("清空"),
                                    cx.listener(|this, _, _, cx| {
                                        this.clear_batch(cx);
                                    }),
                                )
                                .disabled(self.batch_files.is_empty()),
                            )
                            .child(ui::secondary_button(
                                ids.batch_pick,
                                t("选择文件"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_batch_files(window, cx);
                                }),
                            )),
                    ),
            ),
        );

        if self.batch_files.is_empty() {
            container = container.child(ui::empty_hint(
                &format!(
                    "{} {} {}",
                    t("点击\"选择文件\"按钮选择多个文件进行"),
                    self.spec.algorithms[self.batch_selected].name,
                    t("批量加密")
                ),
                cx,
            ));
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
                                    ui::copy_result_button(ids.copy_batch, t("复制结果"), results)
                                }))
                                .child(
                                    ui::secondary_button(
                                        ids.batch_export,
                                        t("导出结果"),
                                        cx.listener(|this, _, window, cx| {
                                            this.export_results(window, cx);
                                        }),
                                    )
                                    .disabled(!has_completed),
                                ),
                        ),
                )
                .child(self.render_batch_table(&label, cx));
            container = container.child(list_card);
        }
        container
    }

    /// 批量表格：文件名 / 状态 / 哈希值（列标题随算法）。
    fn render_batch_table(&self, hash_label: &str, cx: &Context<Self>) -> Div {
        let header = ui::table_head(cx)
            .child(ui::table_head_cell(t("文件名"), cx).w(px(220.)))
            .child(ui::table_head_cell(t("状态"), cx).w(px(72.)))
            .child(ui::table_head_cell(hash_label, cx).flex_1());

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

    /// 页面整体渲染（外层页面薄壳转调）。
    pub fn render(&self, cx: &Context<Self>) -> gpui::Stateful<Div> {
        let mut page = ui::tool_page(self.spec.page_id);
        if self.algo_select.is_some() {
            page = page.child(self.render_algorithm_bar(cx));
        }
        let mut page = page.child(self.render_tab_bar(cx));

        page = match self.tab {
            Tab::Text => page.child(self.render_text_tab(cx)),
            Tab::File => page.child(self.render_file_tab(cx)),
            Tab::Batch => page.child(self.render_batch_tab(cx)),
        };
        page
    }
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
