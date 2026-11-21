//! 视频格式转换器。
//!
//! 功能：挂载检查 FFmpeg（不可用时红横幅 + 全部控件禁用）、源/目标格式选择、
//! 指定输出目录 / 删除源文件（红字警告）、添加文件/目录（递归）、逐个
//! get_video_info 回填时长/分辨率、文件表格、逐个 convert_video 批量转换 + 进度条。
//!
//! 实现说明：
//! - 转换进度：core 的 convert_video 不回报 ffmpeg 单文件进度，进度条按
//!   「第 i 个 / 共 n 个」推进（与历史版本行为一致，单文件 progress 恒为 0）。
//! - 行内「删除」：无确认对话框，
//!   直接删除列表项（不会删除磁盘文件，仅移出待转换列表）。
//! - 「错误信息」列截断 30 字符（与历史版本一致），完整错误仍保留在行数据中。
//! - 转换期间禁用行删除（历史版本只禁用正在转换的行；避免批量循环中索引漂移）。
//! - Select 组件无 disabled 支持：FFmpeg 未安装时仍可查看格式下拉，
//!   但所有动作按钮/复选框均禁用，动作入口全部收敛在按钮上。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use futures::StreamExt as _;
use gpui::{
    div, px, relative, prelude::*, App, Entity, FontWeight, IntoElement, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, Disableable as _,
};

use devtools_core::tools::video_converter::{
    check_ffmpeg_available, convert_video, get_video_info, VideoConversionRequest,
};

use crate::components::{notify, ui};
use crate::components::select::{select_state, SimpleSelectDelegate};

/// 支持的视频格式（value / 显示 label）。
const VIDEO_FORMATS: [(&str, &str); 10] = [
    ("mp4", "MP4 (mp4)"),
    ("mov", "MOV (mov)"),
    ("avi", "AVI (avi)"),
    ("mkv", "MKV (mkv)"),
    ("wmv", "WMV (wmv)"),
    ("flv", "FLV (flv)"),
    ("webm", "WebM (webm)"),
    ("m4v", "M4V (m4v)"),
    ("3gp", "3GP (3gp)"),
    ("mpeg", "MPEG (mpeg)"),
];

const ALL_EXTS: [&str; 10] = [
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mpeg",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConvertStatus {
    Pending,
    Converting,
    Completed,
    Error,
}

/// 待转换文件行（名称可直接从 path 推得，不再单存）。
struct VideoItem {
    path: String,
    size: u64,
    duration: Option<String>,
    resolution: Option<String>,
    output_path: Option<String>,
    status: ConvertStatus,
    error: Option<String>,
}

pub struct VideoConverter {
    /// FFmpeg 可用性：None=检查中 / Some(true|false)
    ffmpeg: Option<bool>,

    source_select: Entity<SelectState<SimpleSelectDelegate>>,
    target_select: Entity<SelectState<SimpleSelectDelegate>>,
    source_format: String,
    target_format: String,

    use_custom_dir: bool,
    custom_dir: Option<String>,
    delete_source: bool,

    files: Vec<VideoItem>,
    converting: bool,
    /// 当前正在转换的行下标（进度条）
    current: Option<usize>,
    /// 批量添加（选文件/目录）进行中
    batch_loading: bool,

    /// 批量转换完成横幅：(是否成功, 文案)
    banner: Option<(bool, String)>,
    error: Option<String>,
}

impl VideoConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        // 挂载即检查 FFmpeg 可用性
        let mut rx = crate::core_rt::submit(cx, async move { check_ffmpeg_available().await });
        cx.spawn(async move |this, cx| {
            let Some(result) = rx.next().await else { return };
            let _ = this.update(cx, |this, cx| {
                this.ffmpeg = Some(result.unwrap_or(false));
                cx.notify();
            });
        })
        .detach();

        let mut source_options: Vec<(&str, &str)> = vec![("auto", "自动检测")];
        source_options.extend(VIDEO_FORMATS);
        let source_select = select_state(window, cx, &source_options, 0);
        cx.subscribe(
            &source_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.source_format = value.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let target_select = select_state(window, cx, &VIDEO_FORMATS, 0);
        cx.subscribe(
            &target_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.target_format = value.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            ffmpeg: None,
            source_select,
            target_select,
            source_format: "auto".into(),
            target_format: "mp4".into(),
            use_custom_dir: false,
            custom_dir: None,
            delete_source: false,
            files: Vec::new(),
            converting: false,
            current: None,
            batch_loading: false,
            banner: None,
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // 文件添加
    // ------------------------------------------------------------------

    /// 当前源格式对应的扩展名白名单。
    fn current_exts(&self) -> Vec<String> {
        if self.source_format == "auto" {
            ALL_EXTS.iter().map(|e| e.to_string()).collect()
        } else {
            vec![self.source_format.clone()]
        }
    }

    /// 「添加文件」：多选。
    fn add_files(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts = self.current_exts();
        let filter_name = if self.source_format == "auto" {
            "视频文件".to_string()
        } else {
            format!("{} 视频文件", self.source_format.to_uppercase())
        };
        self.batch_loading = true;
        cx.notify();

        // None = 用户取消了对话框（区别于“选到 0 个文件”）
        let mut rx = crate::core_rt::submit(cx, async move {
            // rfd 必须在后台线程调用，避免阻塞 UI
            rfd::FileDialog::new()
                .add_filter(filter_name, &exts)
                .pick_files()
        });
        cx.spawn_in(window, async move |this, cx| {
            let picked = rx.next().await.flatten();
            let _ = this.update_in(cx, |this, window, cx| match picked {
                Some(paths) => this.collect_paths(paths, window, cx),
                None => {
                    this.batch_loading = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// 「添加目录」：递归收集视频文件。
    fn add_directory(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts = self.current_exts();
        self.batch_loading = true;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            let dir = rfd::FileDialog::new().pick_folder()?;
            Some(walk_video_dir(&dir, &exts))
        });
        cx.spawn_in(window, async move |this, cx| {
            let picked = rx.next().await.flatten();
            let _ = this.update_in(cx, |this, window, cx| match picked {
                Some(paths) => this.collect_paths(paths, window, cx),
                None => {
                    this.batch_loading = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// 过滤、去重并入列表，随后逐个后台获取视频信息（时长/分辨率）。
    fn collect_paths(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.batch_loading = false;
        let exts = self.current_exts();
        let valid: Vec<PathBuf> = paths
            .into_iter()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| exts.iter().any(|x| x.eq_ignore_ascii_case(ext)))
                    .unwrap_or(false)
            })
            .collect();

        if valid.is_empty() {
            let list = if self.source_format == "auto" {
                "支持的视频格式".to_string()
            } else {
                format!("{} 格式", self.source_format.to_uppercase())
            };
            self.error = Some(format!("没有找到 {list} 的视频文件"));
            cx.notify();
            return;
        }

        let mut seen: HashSet<String> = self.files.iter().map(|f| f.path.clone()).collect();
        let mut jobs: Vec<(usize, String)> = Vec::new();
        for path in valid {
            let path_str = path.display().to_string();
            if !seen.insert(path_str.clone()) {
                continue; // 按 path 去重
            }
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            self.files.push(VideoItem {
                path: path_str.clone(),
                size,
                duration: None,
                resolution: None,
                output_path: None,
                status: ConvertStatus::Pending,
                error: None,
            });
            jobs.push((self.files.len() - 1, path_str));
        }
        cx.notify();
        if jobs.is_empty() {
            return;
        }

        // 逐个 get_video_info 回填时长/分辨率（ffprobe 串行调用，放后台）
        let mut rx = crate::core_rt::submit(cx, async move {
            let mut results = Vec::new();
            for (index, path) in jobs {
                let info = get_video_info(path).await;
                results.push((index, info));
            }
            results
        });
        cx.spawn_in(window, async move |this, cx| {
            let results = rx.next().await.unwrap_or_default();
            let _ = this.update_in(cx, |this, _window, cx| {
                for (index, info) in results {
                    if let Some(item) = this.files.get_mut(index) {
                        if let Ok(info) = info {
                            item.duration = Some(info.duration);
                            item.resolution = Some(info.resolution);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 列表操作
    // ------------------------------------------------------------------

    fn remove_file(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
        if self.converting {
            return;
        }
        self.files.remove(index);
        cx.notify();
    }

    fn clear_list(&mut self, cx: &mut gpui::Context<Self>) {
        self.files.clear();
        self.current = None;
        self.converting = false;
        self.banner = None;
        self.error = None;
        cx.notify();
    }

    fn copy_text(
        &mut self,
        text: Option<String>,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(text) = text {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            notify::toast_success(window, cx, "已复制到剪贴板");
        }
    }

    // ------------------------------------------------------------------
    // 批量转换
    // ------------------------------------------------------------------

    /// 计算输出路径：勾选自定义目录时写入该目录，否则写入源文件同目录
    /// （始终传绝对路径；相对文件名依赖进程 CWD，不可靠）。
    fn output_path_for(&self, item: &VideoItem) -> String {
        let stem = Path::new(&item.path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "output".into());
        let file_name = format!("{stem}.{}", self.target_format);
        if self.use_custom_dir {
            if let Some(dir) = &self.custom_dir {
                return Path::new(dir).join(file_name).display().to_string();
            }
        }
        Path::new(&item.path)
            .with_file_name(file_name)
            .display()
            .to_string()
    }

    fn start_conversion(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.files.is_empty() {
            self.error = Some("请先选择视频文件".into());
            cx.notify();
            return;
        }
        if self.ffmpeg != Some(true) {
            self.error = Some("FFmpeg 未安装，无法开始转换。请先安装 FFmpeg。".into());
            cx.notify();
            return;
        }
        let jobs: Vec<(String, String)> = self
            .files
            .iter()
            .map(|item| (item.path.clone(), self.output_path_for(item)))
            .collect();
        let delete_source = self.delete_source;

        for item in self.files.iter_mut() {
            item.status = ConvertStatus::Pending;
            item.error = None;
            item.output_path = None;
        }
        self.converting = true;
        self.current = Some(0);
        self.banner = None;
        self.error = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            for (index, (input, output)) in jobs.into_iter().enumerate() {
                let alive = this
                    .update_in(cx, |this, _window, cx| {
                        this.current = Some(index);
                        if let Some(item) = this.files.get_mut(index) {
                            item.status = ConvertStatus::Converting;
                            item.error = None;
                        }
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    return;
                }

                let request = VideoConversionRequest {
                    input_path: input,
                    output_path: output,
                    delete_source_file: Some(delete_source),
                };
                let mut rx = match cx.update(|_window, cx| {
                    crate::core_rt::submit(cx, async move { convert_video(request).await })
                }) {
                    Ok(rx) => rx,
                    Err(_) => return, // 窗口已关闭
                };
                let result = rx.next().await;

                let alive = this
                    .update_in(cx, |this, _window, cx| {
                        if let Some(item) = this.files.get_mut(index) {
                            match result {
                                Some(Ok(response)) => {
                                    item.status = ConvertStatus::Completed;
                                    item.output_path = Some(response.output_path);
                                    item.error = None;
                                }
                                Some(Err(message)) => {
                                    item.status = ConvertStatus::Error;
                                    item.error = Some(message);
                                }
                                None => {
                                    item.status = ConvertStatus::Error;
                                    item.error = Some("转换任务已中断".into());
                                }
                            }
                        }
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    return;
                }
            }

            // 全部完成：汇总横幅
            let _ = this.update_in(cx, |this, _window, cx| {
                this.converting = false;
                this.current = None;
                let success = this
                    .files
                    .iter()
                    .filter(|f| f.status == ConvertStatus::Completed)
                    .count();
                let failed = this
                    .files
                    .iter()
                    .filter(|f| f.status == ConvertStatus::Error)
                    .count();
                this.banner = Some(if failed == 0 {
                    (true, format!("✅ 批量转换完成：成功转换 {success} 个文件"))
                } else {
                    (
                        false,
                        format!("批量转换完成：成功转换 {success} 个文件，{failed} 个文件转换失败"),
                    )
                });
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    fn ffmpeg_unavailable(&self) -> bool {
        self.ffmpeg == Some(false)
    }

    fn settings_panel(&self, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let locked = self.converting || self.ffmpeg_unavailable();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_1()
                            .child(ui::field("源格式", Select::new(&self.source_select), cx))
                            .child(ui::hint_text(
                                "选择源视频文件格式，自动检测可支持所有格式",
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_1()
                            .child(ui::field("目标格式", Select::new(&self.target_select), cx))
                            .child(ui::hint_text("转换后的视频格式，MP4为通用格式", cx)),
                    ),
            )
            .child(div().flex().flex_col().gap_2().child(
                Checkbox::new("vc-use-dir")
                    .label("指定输出目录")
                    .checked(self.use_custom_dir)
                    .disabled(locked)
                    .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                        this.use_custom_dir = *checked;
                        cx.notify();
                    })),
            ))
            .when(self.use_custom_dir, |panel| {
                panel.child(self.output_dir_row(cx)).when_some(
                    self.custom_dir.clone(),
                    |panel, dir| {
                        panel.child(ui::hint_text(
                            &format!("转换后的文件将保存到: {dir}"),
                            cx,
                        ))
                    },
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().colors.border.opacity(0.5))
                    .child(
                        Checkbox::new("vc-del-src")
                            .label("删除源文件")
                            .checked(self.delete_source)
                            .disabled(locked)
                            .on_click(cx.listener(|this, checked: &bool, _window, cx| {
                                this.delete_source = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(cx.theme().colors.danger)
                            .child("⚠️ 转换成功后自动删除源文件，此操作不可恢复，请谨慎使用"),
                    ),
            )
    }

    /// 自定义输出目录行（只读展示 + 选择/清除）。
    fn output_dir_row(&self, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let locked = self.converting || self.ffmpeg_unavailable();
        let theme = cx.theme().clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px_2()
                    .py_1()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.muted.opacity(0.35))
                    .text_size(px(13.))
                    .text_color(match &self.custom_dir {
                        Some(_) => theme.colors.foreground,
                        None => theme.colors.muted_foreground,
                    })
                    .truncate()
                    .child(
                        self.custom_dir
                            .clone()
                            .unwrap_or_else(|| "选择输出目录...".into()),
                    ),
            )
            .child(
                Button::new("vc-pick-dir")
                    .label("选择目录")
                    .secondary()
                    .disabled(locked)
                    .on_click(cx.listener(|this, _ev, window, cx| {
                        this.pick_output_dir(window, cx);
                    })),
            )
            .when(self.custom_dir.is_some(), |row| {
                row.child(
                    Button::new("vc-clear-dir")
                        .label("清除")
                        .secondary()
                        .disabled(locked)
                        .on_click(cx.listener(|this, _ev, _window, cx| {
                            this.custom_dir = None;
                            cx.notify();
                        })),
                )
            })
    }

    fn pick_output_dir(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let mut rx = crate::core_rt::submit(cx, async move {
            rfd::FileDialog::new().pick_folder().map(|p| p.display().to_string())
        });
        cx.spawn_in(window, async move |this, cx| {
            let dir = rx.next().await.flatten();
            let _ = this.update_in(cx, |this, _window, cx| {
                this.custom_dir = dir;
                cx.notify();
            });
        })
        .detach();
    }

    fn table_header(&self, cx: &App) -> gpui::Div {
        let theme = cx.theme().clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(theme.colors.muted.opacity(0.5))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.colors.muted_foreground)
            .child(div().flex_1().min_w_0().child("路径信息"))
            .child(header_cell("大小", 76.))
            .child(header_cell("时长", 76.))
            .child(header_cell("分辨率", 96.))
            .child(header_cell("状态", 60.))
            .child(header_cell("错误信息", 150.))
            .child(header_cell("操作", 52.))
    }

    fn table_row(
        &self,
        index: usize,
        item: &VideoItem,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Div {
        let theme = cx.theme().clone();
        let (status_text, status_color) = match item.status {
            ConvertStatus::Pending => ("等待中", theme.colors.warning),
            ConvertStatus::Converting => ("转换中", theme.colors.info),
            ConvertStatus::Completed => ("已完成", theme.colors.success),
            ConvertStatus::Error => ("错误", theme.colors.danger),
        };

        // 错误列截断 30 字符（完整错误保留在数据中）
        let error_text = item.error.as_ref().map(|e| {
            if e.chars().count() > 30 {
                format!("{}...", e.chars().take(30).collect::<String>())
            } else {
                e.clone()
            }
        });

        let src_path = item.path.clone();
        let out_path = if item.status == ConvertStatus::Completed {
            item.output_path.clone()
        } else {
            None
        };

        // 路径信息单元格：源路径（点击复制）+ 转换后路径（两行）
        let mut path_cell = div().flex_1().min_w_0().flex_col().gap_0p5().child(
            div()
                .id(("vc-src-path", index))
                .flex()
                .items_center()
                .min_w_0()
                .gap_1()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _ev, window, cx| {
                    let text = this.files.get(index).map(|f| f.path.clone());
                    this.copy_text(text, window, cx);
                }))
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(theme.colors.muted_foreground)
                        .child("源:"),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_family("Menlo")
                        .text_color(theme.colors.foreground)
                        .child(src_path),
                ),
        );
        if let Some(path) = out_path {
            path_cell = path_cell.child(
                div()
                    .id(("vc-out-path", index))
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap_1()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        let text = this.files.get(index).and_then(|f| f.output_path.clone());
                        this.copy_text(text, window, cx);
                    }))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(theme.colors.muted_foreground)
                            .child("转换后:"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_family("Menlo")
                            .text_color(theme.colors.success)
                            .child(path),
                    ),
            );
        } else {
            path_cell = path_cell.child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(theme.colors.muted_foreground)
                            .child("转换后:"),
                    )
                    .child(
                        div()
                            .text_color(theme.colors.muted_foreground.opacity(0.8))
                            .child("未转换"),
                    ),
            );
        }

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .text_size(px(12.))
            .when(self.current == Some(index), |row| {
                row.bg(theme.colors.info.opacity(0.12))
            })
            .hover(|style| style.bg(theme.colors.muted.opacity(0.5)))
            .child(path_cell)
            // 大小 / 时长 / 分辨率
            .child(list_cell(
                format_file_size(item.size),
                theme.colors.foreground,
                76.,
            ))
            .child(list_cell(
                item.duration.clone().unwrap_or_else(|| "未知".into()),
                theme.colors.foreground,
                76.,
            ))
            .child(list_cell(
                item.resolution.clone().unwrap_or_else(|| "未知".into()),
                theme.colors.foreground,
                96.,
            ))
            // 状态
            .child(div().w(px(60.)).flex_shrink_0().child(ui::badge(status_text, status_color)))
            // 错误信息
            .child(
                div()
                    .w(px(150.))
                    .flex_shrink_0()
                    .min_w_0()
                    .truncate()
                    .text_color(match &error_text {
                        Some(_) => theme.colors.danger,
                        None => theme.colors.muted_foreground.opacity(0.8),
                    })
                    .child(error_text.unwrap_or_else(|| "-".into())),
            )
            // 操作：删除（仅从列表移除，不删磁盘文件；转换中禁用）
            .child(
                div()
                    .w(px(52.))
                    .flex_shrink_0()
                    .flex()
                    .justify_center()
                    .child(
                        Button::new(("vc-remove", index))
                            .label("删除")
                            .danger()
                            .disabled(self.converting || self.ffmpeg_unavailable())
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.remove_file(index, cx);
                            })),
                    ),
            )
    }
}

fn header_cell(label: &'static str, width: f32) -> gpui::Div {
    div().w(px(width)).flex_shrink_0().child(label)
}

/// 固定宽度单元格（超长文本省略号截断）。
fn list_cell(text: impl Into<SharedString>, color: gpui::Hsla, width: f32) -> gpui::Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .min_w_0()
        .truncate()
        .text_color(color)
        .child(text.into())
}

impl gpui::Render for VideoConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let unavailable = self.ffmpeg_unavailable();

        ui::tool_page("videoconverter")
            .child(ui::themed_header(
                "视频格式转换器",
                "支持多种主流视频格式的相互转换",
                cx,
            ))
            // FFmpeg 状态：检查中 / 未安装红横幅
            .when(self.ffmpeg.is_none(), |page| {
                page.child(ui::loading_row("正在检查 FFmpeg 安装状态...", cx))
            })
            .when(unavailable, |page| {
                page.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
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
                                .child("❌ FFmpeg 未安装，无法使用视频转换功能"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .text_size(px(12.))
                                .text_color(theme.colors.danger)
                                .child(div().child("安装方法："))
                                .child(
                                    div().font_family("Menlo").child("• macOS: brew install ffmpeg"),
                                )
                                .child(div().child("• Windows: 下载 FFmpeg 并添加到 PATH"))
                                .child(
                                    div()
                                        .font_family("Menlo")
                                        .child("• Linux: sudo apt install ffmpeg (Ubuntu/Debian)"),
                                )
                                .child(
                                    div()
                                        .font_family("Menlo")
                                        .child("• Linux: sudo dnf install ffmpeg (Fedora)"),
                                )
                                .child(div().child("安装完成后请重新打开本工具窗口。")),
                        ),
                )
            })
            // 转换设置卡
            .child(
                ui::card(cx)
                    .child(ui::card_header("转换设置", cx))
                    .child(self.settings_panel(cx)),
            )
            // 文件列表区
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.colors.foreground)
                                    .child(format!(
                                        "待转换文件列表 ({}个文件)",
                                        self.files.len()
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        ui::secondary_button(
                                            "vc-add-files",
                                            "添加文件",
                                            cx.listener(|this, _ev, window, cx| {
                                                this.add_files(window, cx);
                                            }),
                                        )
                                        .disabled(self.batch_loading || unavailable),
                                    )
                                    .child(
                                        ui::secondary_button(
                                            "vc-add-dir",
                                            "添加目录",
                                            cx.listener(|this, _ev, window, cx| {
                                                this.add_directory(window, cx);
                                            }),
                                        )
                                        .disabled(self.batch_loading || unavailable),
                                    ),
                            ),
                    )
                    .when(self.batch_loading, |list| {
                        list.child(ui::loading_row("正在读取文件信息...", cx))
                    })
                    .when(!self.files.is_empty(), |list| {
                        list.child(
                            div()
                                .id("vc-table-scroll")
                                .max_h(px(300.))
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(self.table_header(cx))
                                .children(
                                    (0..self.files.len())
                                        .map(|i| self.table_row(i, &self.files[i], cx))
                                        .collect::<Vec<gpui::Div>>(),
                                ),
                        )
                    })
                    .when(self.files.is_empty() && !self.batch_loading, |list| {
                        list.child(ui::empty_hint(
                            "暂无待转换文件，请点击右上角按钮添加文件或目录",
                            cx,
                        ))
                    }),
            )
            // 转换控制（列表非空时显示）
            .when(!self.files.is_empty(), |page| {
                let total = self.files.len();
                let current = self.current.unwrap_or(0);
                let pct = if self.converting && total > 0 {
                    ((current + 1) as f32 / total as f32).min(1.)
                } else {
                    0.
                };
                page.child(
                    ui::card(cx)
                        .child(ui::card_header("转换控制", cx))
                        .when(self.converting, |card| {
                            card.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(12.))
                                            .text_color(theme.colors.muted_foreground)
                                            .child(format!(
                                                "总体进度: {} / {} 个文件",
                                                current + 1,
                                                total
                                            ))
                                            .child(format!("{:.0}%", pct * 100.)),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(6.))
                                            .rounded(px(3.))
                                            .bg(theme.colors.muted)
                                            .child(
                                                div()
                                                    .h_full()
                                                    .rounded(px(3.))
                                                    .bg(theme.colors.accent)
                                                    .w(relative(pct)),
                                            ),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    ui::primary_button(
                                        "vc-start",
                                        if self.converting { "转换中..." } else { "开始转换" },
                                        cx.listener(|this, _ev, window, cx| {
                                            this.start_conversion(window, cx);
                                        }),
                                    )
                                    .flex_1()
                                    .loading(self.converting)
                                    .disabled(
                                        self.converting
                                            || unavailable
                                            || (self.use_custom_dir && self.custom_dir.is_none()),
                                    ),
                                )
                                .child(
                                    ui::secondary_button(
                                        "vc-clear",
                                        "清空列表",
                                        cx.listener(|this, _ev, _window, cx| {
                                            this.clear_list(cx);
                                        }),
                                    )
                                    .flex_1()
                                    .disabled(self.converting || unavailable),
                                ),
                        ),
                )
            })
            // 错误横条
            .when_some(self.error.clone(), |page, error| {
                page.child(ui::error_banner(&error, cx))
            })
            // 完成横幅
            .when_some(self.banner.clone(), |page, banner| {
                let (success, text) = banner;
                page.child(if success {
                    ui::success_banner(&text, cx)
                } else {
                    ui::error_banner(&text, cx)
                })
            })
    }
}

/// 递归收集目录下的视频文件。
fn walk_video_dir(dir: &Path, exts: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_video_dir(&path, exts));
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.iter().any(|x| x.eq_ignore_ascii_case(ext)) {
                out.push(path);
            }
        }
    }
    out
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["Bytes", "KB", "MB", "GB"];
    if bytes == 0 {
        return "0 Bytes".into();
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024. && unit < UNITS.len() - 1 {
        size /= 1024.;
        unit += 1;
    }
    format!("{:.2} {}", size, UNITS[unit])
}
