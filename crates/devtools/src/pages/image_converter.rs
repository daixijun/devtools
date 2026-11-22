//! 图片格式转换器。
//!
//! 「设置卡 + 文件列表表格 + 开始按钮 + 进度条」模式：rfd 选文件/目录（后台线程），
//! 逐个 get_image_info_command 回填尺寸/EXIF，批量转换逐文件经 core_rt 桥执行并
//! 回流更新行状态。EXIF 详情为「点击展开行内 info_row 区」。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::div;
use gpui::{prelude::*, px, AnyElement, App, ClipboardItem, Div, Entity, IntoElement, Window};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::progress::Progress;
use gpui_component::radio::Radio;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::slider::{Slider, SliderEvent, SliderState};
use gpui_component::{ActiveTheme as _, Disableable as _};

use devtools::tools::image_converter::{
    convert_image, get_image_exif_data, get_image_info_command, ExifData, ImageConversionRequest,
};

use crate::components::notify;
use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::ui;

/// 源格式选项（含 auto 自动检测）。
const SOURCE_FORMATS: &[(&str, &str)] = &[
    ("auto", "自动检测 (auto)"),
    ("jpg", "JPG (jpg)"),
    ("jpeg", "JPEG (jpeg)"),
    ("png", "PNG (png)"),
    ("gif", "GIF (gif)"),
    ("bmp", "BMP (bmp)"),
    ("tiff", "TIFF (tiff)"),
    ("webp", "WebP (webp)"),
    ("ico", "ICO (ico)"),
    ("heic", "HEIC (heic)"),
];

/// 目标格式选项（9 项，不含 auto，默认 jpg）。
const TARGET_FORMATS: &[(&str, &str)] = &[
    ("jpg", "JPG (jpg)"),
    ("jpeg", "JPEG (jpeg)"),
    ("png", "PNG (png)"),
    ("gif", "GIF (gif)"),
    ("bmp", "BMP (bmp)"),
    ("tiff", "TIFF (tiff)"),
    ("webp", "WebP (webp)"),
    ("ico", "ICO (ico)"),
    ("heic", "HEIC (heic)"),
];

/// 自动检测模式下的源扩展名白名单。
const CONVERTER_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "ico", "heic",
];

/// 尺寸调整模式。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeMode {
    None,
    Width,
    Height,
    Both,
}

/// 文件行状态。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowStatus {
    Pending,
    Converting,
    Completed,
    Error,
}

/// 表格中的一行。
struct FileRow {
    path: String,
    name: String,
    size: u64,
    dims: Option<String>,
    has_exif: bool,
    exif: Option<ExifData>,
    output_path: Option<String>,
    status: RowStatus,
    error: Option<String>,
}

pub struct ImageConverter {
    // 转换设置
    source_sel: Entity<SelectState<SimpleSelectDelegate>>,
    source_format: String,
    target_sel: Entity<SelectState<SimpleSelectDelegate>>,
    target_format: String,
    quality_slider: Entity<SliderState>,
    quality: u8,
    resize_mode: ResizeMode,
    width_input: Entity<InputState>,
    height_input: Entity<InputState>,
    remove_exif: bool,
    delete_source: bool,
    use_custom_dir: bool,
    custom_dir: String,

    // 文件列表与执行状态
    files: Vec<FileRow>,
    converting: bool,
    /// 已完成/正在处理的文件数（转换中 = 当前下标，用于进度显示）。
    progress_index: usize,
    /// 批量结果横幅（是否成功, 文案）。
    message: Option<(bool, String)>,
    /// 点击展开 EXIF 详情的文件路径。
    expanded_exif: Option<String>,
}

impl ImageConverter {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let source_sel = select_state(window, cx, SOURCE_FORMATS, 0);
        cx.subscribe(
            &source_sel,
            |this, _, ev: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.source_format = v.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let target_sel = select_state(window, cx, TARGET_FORMATS, 0);
        cx.subscribe(
            &target_sel,
            |this, _, ev: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.target_format = v.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let quality_slider = cx.new(|_| SliderState::new().min(1.).max(100.).default_value(85.));
        cx.subscribe(&quality_slider, |this, _, ev: &SliderEvent, cx| {
            let (SliderEvent::Change(v) | SliderEvent::Release(v)) = ev;
            this.quality = v.start().round().clamp(1., 100.) as u8;
            cx.notify();
        })
        .detach();

        let width_input = ui::new_input(window, cx);
        width_input.update(cx, |s, cx| s.set_value("1920", window, cx));
        let height_input = ui::new_input(window, cx);
        height_input.update(cx, |s, cx| s.set_value("1080", window, cx));

        Self {
            source_sel,
            source_format: "auto".to_string(),
            target_sel,
            target_format: "jpg".to_string(),
            quality_slider,
            quality: 85,
            resize_mode: ResizeMode::None,
            width_input,
            height_input,
            remove_exif: false,
            delete_source: false,
            use_custom_dir: false,
            custom_dir: String::new(),
            files: Vec::new(),
            converting: false,
            progress_index: 0,
            message: None,
            expanded_exif: None,
        }
    }

    // -----------------------------------------------------------------------
    // 设置辅助
    // -----------------------------------------------------------------------

    /// 当前源格式对应的扩展名白名单（auto = 全部支持格式）。
    fn source_exts(&self) -> Vec<String> {
        if self.source_format == "auto" {
            CONVERTER_EXTS.iter().map(|s| s.to_string()).collect()
        } else {
            vec![self.source_format.clone()]
        }
    }

    /// 添加失败时的提示文案。
    fn exts_label(&self) -> String {
        if self.source_format == "auto" {
            "所有支持的图片格式".to_string()
        } else {
            format!("{} 格式", self.source_format.to_uppercase())
        }
    }

    /// 宽/高输入实时取值（1-10000，非法回退默认）。
    fn width_value(&self, cx: &App) -> u32 {
        self.width_input
            .read(cx)
            .value()
            .trim()
            .parse::<u32>()
            .unwrap_or(1920)
            .clamp(1, 10000)
    }

    fn height_value(&self, cx: &App) -> u32 {
        self.height_input
            .read(cx)
            .value()
            .trim()
            .parse::<u32>()
            .unwrap_or(1080)
            .clamp(1, 10000)
    }

    // -----------------------------------------------------------------------
    // 文件添加
    // -----------------------------------------------------------------------

    fn add_files(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts = self.source_exts();
        // rfd 对话框在后台线程运行（内部已 dispatch 到主线程弹面板）
        let rx = crate::core_rt::submit(cx, async move {
            let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            rfd::FileDialog::new()
                .add_filter("图片文件", &refs)
                .pick_files()
        });
        cx.spawn_in(window, async move |this, cx| {
            let mut rx = rx;
            if let Some(paths) = futures::StreamExt::next(&mut rx).await.flatten() {
                let strs = paths
                    .into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect::<Vec<_>>();
                this.update_in(cx, |this, window, cx| this.add_paths(strs, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    fn add_dir(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts = self.source_exts();
        let rx = crate::core_rt::submit(cx, async move {
            let dir = rfd::FileDialog::new().pick_folder()?;
            // walkdir 不在依赖中：手写 std::fs::read_dir 递归
            let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            let mut out = Vec::new();
            collect_image_files(&dir, &refs, &mut out, 0);
            Some(out)
        });
        cx.spawn_in(window, async move |this, cx| {
            let mut rx = rx;
            if let Some(paths) = futures::StreamExt::next(&mut rx).await.flatten() {
                let strs = paths
                    .into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect::<Vec<_>>();
                this.update_in(cx, |this, window, cx| this.add_paths(strs, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    /// 过滤、按 path 去重后加入列表，并逐个异步回填尺寸/EXIF。
    fn add_paths(&mut self, paths: Vec<String>, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts = self.source_exts();
        let existing: HashSet<String> = self.files.iter().map(|f| f.path.clone()).collect();
        let fresh: Vec<String> = paths
            .into_iter()
            .filter(|p| !existing.contains(p))
            .filter(|p| has_supported_ext(p, &exts))
            .collect();
        if fresh.is_empty() {
            self.message = Some((false, format!("没有找到 {} 的图片文件", self.exts_label())));
            cx.notify();
            return;
        }

        for p in &fresh {
            self.files.push(FileRow {
                path: p.clone(),
                name: file_name_of(p),
                size: 0,
                dims: None,
                has_exif: false,
                exif: None,
                output_path: None,
                status: RowStatus::Pending,
                error: None,
            });
        }
        self.message = None;
        cx.notify();

        // 逐个 get_image_info_command 回填尺寸/EXIF（后台顺序执行）
        cx.spawn_in(window, async move |this, cx| {
            for path in fresh {
                let rx = this
                    .update_in(cx, |_this, _w, cx| {
                        let p = path.clone();
                        crate::core_rt::submit(cx, async move {
                            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                            let info = get_image_info_command(p.clone()).await;
                            (p, size, info)
                        })
                    })
                    .ok();
                if let Some(mut rx) = rx {
                    if let Some((path, size, info)) = futures::StreamExt::next(&mut rx).await {
                        this.update_in(cx, |this, _w, cx| {
                            if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                                row.size = size;
                                if let Ok(info) = info {
                                    row.dims = Some(info.dimensions);
                                    row.has_exif = info.has_exif;
                                    row.exif = info.exif_data;
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                    }
                }
            }
        })
        .detach();
    }

    // -----------------------------------------------------------------------
    // 批量转换
    // -----------------------------------------------------------------------

    fn start_convert(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.converting || self.files.is_empty() {
            return;
        }
        if self.use_custom_dir && self.custom_dir.is_empty() {
            self.message = Some((false, "已启用自定义输出目录，但尚未选择目录".to_string()));
            cx.notify();
            return;
        }

        // 快照全部设置（转换过程中设置项已禁用）
        let target = self.target_format.clone();
        let quality = self.quality;
        let remove_exif = self.remove_exif;
        let delete_source = self.delete_source;
        let use_custom = self.use_custom_dir;
        let custom_dir = self.custom_dir.clone();
        let width = self.width_value(cx);
        let height = self.height_value(cx);
        let resize_mode = self.resize_mode;

        let mut jobs: Vec<(String, ImageConversionRequest)> = Vec::new();
        for row in &self.files {
            let output_path = build_output_path(&row.path, &target, use_custom, &custom_dir);
            jobs.push((
                row.path.clone(),
                ImageConversionRequest {
                    input_path: row.path.clone(),
                    output_path,
                    target_format: target.clone(),
                    quality: if matches!(target.as_str(), "jpg" | "jpeg" | "webp") {
                        Some(quality)
                    } else {
                        None
                    },
                    width: if matches!(resize_mode, ResizeMode::Width | ResizeMode::Both) {
                        Some(width)
                    } else {
                        None
                    },
                    height: if matches!(resize_mode, ResizeMode::Height | ResizeMode::Both) {
                        Some(height)
                    } else {
                        None
                    },
                    remove_exif,
                    delete_source_file: Some(delete_source),
                },
            ));
        }

        self.converting = true;
        self.progress_index = 0;
        self.message = None;
        self.expanded_exif = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let mut success = 0usize;
            let mut failed = 0usize;

            for (i, (path, request)) in jobs.into_iter().enumerate() {
                // 标记转换中 + 提交该文件转换任务
                let rx = this
                    .update_in(cx, |this, _w, cx| {
                        if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                            row.status = RowStatus::Converting;
                            row.error = None;
                        }
                        this.progress_index = i;
                        cx.notify();
                        crate::core_rt::submit(cx, async move { convert_image(request).await })
                    })
                    .ok();

                if let Some(mut rx) = rx {
                    if let Some(result) = futures::StreamExt::next(&mut rx).await {
                        let outcome: (RowStatus, Option<String>, Option<String>) = match result {
                            Ok(resp) if resp.success => {
                                success += 1;
                                (RowStatus::Completed, None, Some(resp.output_path))
                            }
                            Ok(resp) => {
                                failed += 1;
                                (RowStatus::Error, Some(resp.message), None)
                            }
                            Err(e) => {
                                failed += 1;
                                (RowStatus::Error, Some(e), None)
                            }
                        };
                        this.update_in(cx, |this, _w, cx| {
                            if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                                row.status = outcome.0;
                                row.error = outcome.1;
                                row.output_path = outcome.2;
                            }
                            this.progress_index = i + 1;
                            cx.notify();
                        })
                        .ok();
                    } else {
                        failed += 1;
                    }
                } else {
                    failed += 1;
                }
            }

            this.update_in(cx, |this, _w, cx| {
                this.converting = false;
                this.message = Some(if failed == 0 {
                    (true, format!("✅ 批量转换完成：成功 {} 个文件", success))
                } else {
                    (
                        false,
                        format!("批量转换完成：成功 {} 个文件，{} 个失败", success, failed),
                    )
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // -----------------------------------------------------------------------
    // 列表操作
    // -----------------------------------------------------------------------

    fn remove_row(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
        // 转换中的行禁删
        if self.files.get(index).map(|f| f.status) == Some(RowStatus::Converting) {
            return;
        }
        if index < self.files.len() {
            let removed = self.files.remove(index);
            if self.expanded_exif.as_deref() == Some(removed.path.as_str()) {
                self.expanded_exif = None;
            }
            cx.notify();
        }
    }

    fn clear_list(&mut self, cx: &mut gpui::Context<Self>) {
        // 转换中禁止清空，避免打断写盘
        if self.converting {
            return;
        }
        self.files.clear();
        self.progress_index = 0;
        self.expanded_exif = None;
        self.message = None;
        cx.notify();
    }

    fn copy_text(&mut self, text: String, window: &mut Window, cx: &mut gpui::Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        notify::toast_success(window, cx, "已复制到剪贴板");
    }

    fn choose_dir(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let rx = crate::core_rt::submit(cx, async move { rfd::FileDialog::new().pick_folder() });
        cx.spawn_in(window, async move |this, cx| {
            let mut rx = rx;
            if let Some(dir) = futures::StreamExt::next(&mut rx).await.flatten() {
                this.update_in(cx, |this, _w, cx| {
                    this.custom_dir = dir.to_string_lossy().to_string();
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// 点击 EXIF「包含」：展开/收起行内详情；无缓存数据时后台调 get_image_exif_data 拉取。
    fn toggle_exif(&mut self, path: String, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.expanded_exif.as_deref() == Some(path.as_str()) {
            self.expanded_exif = None;
            cx.notify();
            return;
        }
        self.expanded_exif = Some(path.clone());
        cx.notify();

        let need_fetch = self
            .files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.has_exif && f.exif.is_none())
            .unwrap_or(false);
        if need_fetch {
            let rx = crate::core_rt::submit(cx, {
                let p = path.clone();
                async move {
                    let exif = get_image_exif_data(p.clone()).await;
                    (p, exif)
                }
            });
            cx.spawn_in(window, async move |this, cx| {
                let mut rx = rx;
                if let Some((path, exif)) = futures::StreamExt::next(&mut rx).await {
                    this.update_in(cx, |this, _w, cx| {
                        if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                            row.exif = exif.ok().flatten();
                        }
                        cx.notify();
                    })
                    .ok();
                }
            })
            .detach();
        }
    }
}

// ---------------------------------------------------------------------------
// 纯函数辅助
// ---------------------------------------------------------------------------

/// 输出路径 = 自定义目录（若启用且非空），否则源文件同名换扩展名。
fn build_output_path(
    source: &str,
    target_format: &str,
    use_custom: bool,
    custom_dir: &str,
) -> String {
    let p = Path::new(source);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    let name = format!("{}.{}", stem, target_format.to_lowercase());
    if use_custom && !custom_dir.is_empty() {
        let mut buf = PathBuf::from(custom_dir);
        buf.push(name);
        buf.to_string_lossy().to_string()
    } else {
        match p.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let mut buf = parent.to_path_buf();
                buf.push(name);
                buf.to_string_lossy().to_string()
            }
            _ => name,
        }
    }
}

/// 手写递归目录遍历（walkdir 不在依赖中）。
fn collect_image_files(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 12 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_image_files(&path, exts, out, depth + 1);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.contains(&ext.to_lowercase().as_str()) {
                out.push(path);
            }
        }
    }
}

fn has_supported_ext(path: &str, exts: &[String]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// 按字符数截断（中文按 1 字计），超长补省略号。
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{}...", cut)
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{:.2} {}", value, UNITS[unit])
}

// ---------------------------------------------------------------------------
// 渲染
// ---------------------------------------------------------------------------

impl gpui::Render for ImageConverter {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let mut page = ui::tool_page("imageconverter").child(self.render_settings(cx));

        page = page.child(self.render_file_list(cx));

        if !self.files.is_empty() {
            page = page.child(self.render_control(cx));
        }

        if let Some((ok, text)) = self.message.clone() {
            page = page.child(if ok {
                ui::success_banner(&text, cx)
            } else {
                ui::error_banner(&text, cx)
            });
        }

        page
    }
}

impl ImageConverter {
    fn render_settings(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let converting = self.converting;

        let card = ui::card(cx)
            .child(ui::card_header("转换设置", cx))
            // 源格式 / 目标格式
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .child(
                        div().flex_1().min_w(px(220.)).child(ui::field(
                            "源格式",
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(Select::new(&self.source_sel).disabled(converting))
                                .child(ui::hint_text(
                                    "选择源图片文件格式，自动检测可支持所有格式",
                                    cx,
                                )),
                            cx,
                        )),
                    )
                    .child(
                        div().flex_1().min_w(px(220.)).child(ui::field(
                            "目标格式",
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(Select::new(&self.target_sel).disabled(converting))
                                .child(ui::hint_text("转换后的图片格式", cx)),
                            cx,
                        )),
                    ),
            );

        let card = if matches!(self.target_format.as_str(), "jpg" | "jpeg" | "webp") {
            // 质量滑块（仅 jpg/jpeg/webp 显示）
            card.child(
                ui::field(
                    &format!("图片质量: {}%", self.quality),
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Slider::new(&self.quality_slider)
                                .disabled(converting)
                                .flex_1(),
                        )
                        .child(
                            div()
                                .w(px(44.))
                                .text_size(px(12.))
                                .text_color(theme.colors.muted_foreground)
                                .child(format!("{}%", self.quality)),
                        ),
                    cx,
                )
                .child(ui::hint_text("质量越高文件越大，建议设置在 70-90 之间", cx)),
            )
        } else {
            card
        };

        // 尺寸调整
        let card = card.child(ui::field(
            "尺寸调整",
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(self.resize_radio(
                    "ic-resize-none",
                    "保持原尺寸",
                    ResizeMode::None,
                    converting,
                    cx,
                ))
                .child(self.resize_radio(
                    "ic-resize-width",
                    "按宽度调整",
                    ResizeMode::Width,
                    converting,
                    cx,
                ))
                .child(self.resize_radio(
                    "ic-resize-height",
                    "按高度调整",
                    ResizeMode::Height,
                    converting,
                    cx,
                ))
                .child(self.resize_radio(
                    "ic-resize-both",
                    "指定宽高",
                    ResizeMode::Both,
                    converting,
                    cx,
                )),
            cx,
        ));

        let card = if matches!(self.resize_mode, ResizeMode::Width | ResizeMode::Both) {
            card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_4()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.colors.muted_foreground)
                            .child("宽度:"),
                    )
                    .child(
                        Input::new(&self.width_input)
                            .w(px(96.))
                            .disabled(converting),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child("px（1-10000）"),
                    ),
            )
        } else {
            card
        };

        let card = if matches!(self.resize_mode, ResizeMode::Height | ResizeMode::Both) {
            card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_4()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.colors.muted_foreground)
                            .child("高度:"),
                    )
                    .child(
                        Input::new(&self.height_input)
                            .w(px(96.))
                            .disabled(converting),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child("px（1-10000）"),
                    ),
            )
        } else {
            card
        };

        card.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Checkbox::new("ic-opt-exif")
                        .label("移除EXIF信息")
                        .checked(self.remove_exif)
                        .disabled(converting)
                        .on_click(cx.listener(|this, checked: &bool, _w, cx| {
                            this.remove_exif = *checked;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.danger)
                        .child("（保护隐私）"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Checkbox::new("ic-opt-delete")
                        .label("转换成功后删除源文件")
                        .checked(self.delete_source)
                        .disabled(converting)
                        .on_click(cx.listener(|this, checked: &bool, _w, cx| {
                            this.delete_source = *checked;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.danger)
                        .child("（谨慎操作）"),
                ),
        )
        .child(
            Checkbox::new("ic-opt-custom-dir")
                .label("指定输出目录")
                .checked(self.use_custom_dir)
                .disabled(converting)
                .on_click(cx.listener(|this, checked: &bool, _w, cx| {
                    this.use_custom_dir = *checked;
                    cx.notify();
                })),
        )
        .children(self.use_custom_dir.then(|| self.render_custom_dir(cx)))
    }

    fn render_custom_dir(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let converting = self.converting;
        // 只读目录显示框（muted 底路径块）+ 右侧操作按钮
        let dir_text: &str = if self.custom_dir.is_empty() {
            "选择输出目录..."
        } else {
            &self.custom_dir
        };
        let mut dir_row = ui::file_row(dir_text, cx).child(
            ui::secondary_button(
                "ic-choose-dir",
                "选择目录",
                cx.listener(|this, _ev, window, cx| this.choose_dir(window, cx)),
            )
            .disabled(converting),
        );
        if !self.custom_dir.is_empty() {
            dir_row = dir_row.child(
                ui::secondary_button(
                    "ic-clear-dir",
                    "清除",
                    cx.listener(|this, _ev, _window, cx| {
                        this.custom_dir.clear();
                        cx.notify();
                    }),
                )
                .disabled(converting),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pl_4()
            .child(dir_row)
            .children(
                (!self.custom_dir.is_empty()).then(|| {
                    ui::hint_text(&format!("转换后的文件将保存到: {}", self.custom_dir), cx)
                }),
            )
    }

    fn resize_radio(
        &self,
        id: &'static str,
        label: &'static str,
        mode: ResizeMode,
        disabled: bool,
        cx: &gpui::Context<Self>,
    ) -> Radio {
        Radio::new(id)
            .label(label)
            .checked(self.resize_mode == mode)
            .disabled(disabled)
            .on_click(cx.listener(move |this, _checked: &bool, _w, cx| {
                this.resize_mode = mode;
                cx.notify();
            }))
    }

    fn render_file_list(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();

        let card = ui::card(cx).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(format!("待转换文件列表 ({}个文件)", self.files.len())),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(ui::secondary_button(
                            "ic-add-files",
                            "添加文件",
                            cx.listener(|this, _ev, window, cx| this.add_files(window, cx)),
                        ))
                        .child(ui::secondary_button(
                            "ic-add-dir",
                            "添加目录",
                            cx.listener(|this, _ev, window, cx| this.add_dir(window, cx)),
                        )),
                ),
        );

        if self.files.is_empty() {
            return card.child(ui::empty_hint(
                "暂无待转换文件，请点击右上角按钮添加文件或目录",
                cx,
            ));
        }

        // 表头
        let header = ui::table_head(cx)
            .px_2()
            .pb_1()
            .child(ui::table_head_cell("路径信息", cx).flex_1().min_w_0())
            .child(ui::table_head_cell("大小", cx).w(px(76.)))
            .child(ui::table_head_cell("尺寸", cx).w(px(88.)))
            .child(ui::table_head_cell("EXIF", cx).w(px(56.)))
            .child(ui::table_head_cell("状态", cx).w(px(64.)))
            .child(ui::table_head_cell("错误信息", cx).w(px(150.)))
            .child(ui::table_head_cell("操作", cx).w(px(52.)));

        // 数据行（行间 hairline 由 table_rows 统一处理；EXIF 展开区跟随所属行）
        let mut rows = Vec::with_capacity(self.files.len());
        for (index, row) in self.files.iter().enumerate() {
            let mut group = div()
                .flex()
                .flex_col()
                .child(self.render_row(index, row, cx));
            if self.expanded_exif.as_deref() == Some(row.path.as_str()) {
                group = group.child(self.render_exif_detail(row, cx));
            }
            rows.push(group);
        }

        card.child(
            div()
                .id("ic-table")
                .max_h(px(340.))
                .overflow_y_scroll()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(header)
                        .child(ui::table_rows(rows, cx)),
                ),
        )
    }

    fn render_row(
        &self,
        index: usize,
        row: &FileRow,
        cx: &gpui::Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = cx.theme().clone();

        let src_path = row.path.clone();
        let out_path = row.output_path.clone();
        let exif_path = row.path.clone();
        let status = row.status;

        // 路径信息列
        let out_line: AnyElement = match out_path {
            Some(out) => {
                let out_for_copy = out.clone();
                div()
                    .id(("ic-out", index))
                    .min_w_0()
                    .truncate()
                    .cursor_pointer()
                    .text_size(px(12.))
                    .text_color(theme.colors.success)
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        this.copy_text(out_for_copy.clone(), window, cx);
                    }))
                    .child(out)
                    .into_any_element()
            }
            None => div()
                .min_w_0()
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child("未转换")
                .into_any_element(),
        };

        div()
            .id(("ic-row", index))
            .flex()
            .items_start()
            .gap_2()
            .px_2()
            .py_1()
            .hover(|s| s.bg(theme.colors.muted))
            // 路径信息
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child("源文件:"),
                            )
                            .child(
                                div()
                                    .id(("ic-src", index))
                                    .min_w_0()
                                    .truncate()
                                    .cursor_pointer()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.foreground)
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        this.copy_text(src_path.clone(), window, cx);
                                    }))
                                    .child(row.path.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child("转换后:"),
                            )
                            .child(out_line),
                    ),
            )
            // 大小
            .child(size_cell(row.size, cx).w(px(76.)))
            // 尺寸
            .child(
                div()
                    .w(px(88.))
                    .text_size(px(12.))
                    .text_color(theme.colors.foreground)
                    .child(row.dims.clone().unwrap_or_else(|| "未知".to_string())),
            )
            // EXIF
            .child(div().w(px(56.)).child(if row.has_exif {
                div()
                    .id(("ic-exif", index))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        this.toggle_exif(exif_path.clone(), window, cx);
                    }))
                    .child(ui::badge("包含", theme.colors.warning))
                    .into_any_element()
            } else {
                div()
                    .text_size(px(12.))
                    .text_color(theme.colors.muted_foreground)
                    .child("无")
                    .into_any_element()
            }))
            // 状态
            .child(div().w(px(64.)).child(status_badge(status, cx)))
            // 错误信息（截断 30 字）
            .child(
                div().w(px(150.)).child(match &row.error {
                    Some(err) => div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(theme.colors.danger)
                        .child(truncate_chars(err, 30))
                        .into_any_element(),
                    None => div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child("-")
                        .into_any_element(),
                }),
            )
            // 操作：删除（转换中禁删）
            .child(
                div().w(px(52.)).flex().justify_center().child(
                    Button::new(("ic-del", index))
                        .label("删除")
                        .ghost()
                        .on_click(cx.listener(move |this, _ev, _window, cx| {
                            this.remove_row(index, cx);
                        }))
                        .disabled(status == RowStatus::Converting)
                        .text_color(theme.colors.danger),
                ),
            )
    }

    /// 行内 EXIF 详情展开区。
    fn render_exif_detail(&self, row: &FileRow, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let mut block = ui::panel(cx).mx_2().mb_1().child(
            div()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.colors.foreground)
                .child(format!("EXIF 信息 - {}", row.name)),
        );

        match &row.exif {
            Some(exif) => {
                let mut rows = Vec::new();
                for (label, value) in [
                    ("相机制造商", &exif.make),
                    ("相机型号", &exif.model),
                    ("拍摄时间", &exif.datetime),
                    ("曝光时间", &exif.exposure_time),
                    ("光圈值", &exif.f_number),
                    ("ISO 感光度", &exif.iso),
                    ("焦距", &exif.focal_length),
                    ("处理软件", &exif.software),
                ] {
                    if let Some(v) = value {
                        if !v.trim().is_empty() {
                            rows.push(ui::info_row(label, v.clone(), cx));
                        }
                    }
                }
                if rows.is_empty() {
                    block = block.child(ui::hint_text("无详细信息", cx));
                } else {
                    block = block.child(ui::info_rows(rows, cx));
                }
            }
            None => {
                block = block.child(ui::loading_row("正在读取 EXIF 数据...", cx));
            }
        }
        block
    }

    fn render_control(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let total = self.files.len();
        let current = self.progress_index.min(total);
        let pct = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let card = ui::card(cx).child(ui::card_header("转换控制", cx));

        let card = if self.converting {
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
                            .child(ui::hint_text(
                                &format!("总体进度: {} / {} 个文件", current, total),
                                cx,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(format!("{:.0}%", pct)),
                            ),
                    )
                    .child(Progress::new("ic-progress").value(pct)),
            )
        } else {
            card
        };

        let start_disabled = self.converting
            || self.files.is_empty()
            || (self.use_custom_dir && self.custom_dir.is_empty());

        card.child(
            div()
                .flex()
                .gap_2()
                .child(
                    ui::primary_button(
                        "ic-start",
                        if self.converting {
                            "转换中..."
                        } else {
                            "开始转换"
                        },
                        cx.listener(|this, _ev, window, cx| this.start_convert(window, cx)),
                    )
                    .disabled(start_disabled)
                    .loading(self.converting)
                    .flex_1(),
                )
                .child(
                    ui::secondary_button(
                        "ic-clear",
                        "清空列表",
                        cx.listener(|this, _ev, _window, cx| this.clear_list(cx)),
                    )
                    .disabled(self.converting)
                    .flex_1(),
                ),
        )
    }
}

// ---------------------------------------------------------------------------
// 小部件
// ---------------------------------------------------------------------------

fn size_cell(size: u64, cx: &gpui::Context<ImageConverter>) -> gpui::Div {
    let theme = cx.theme().clone();
    div()
        .text_size(px(12.))
        .text_color(theme.colors.foreground)
        .child(if size > 0 {
            format_file_size(size)
        } else {
            "未知".to_string()
        })
}

fn status_badge(status: RowStatus, cx: &gpui::Context<ImageConverter>) -> gpui::Div {
    let theme = cx.theme().clone();
    match status {
        RowStatus::Pending => ui::badge("等待中", theme.colors.warning),
        RowStatus::Converting => ui::badge("转换中", theme.colors.info),
        RowStatus::Completed => ui::badge("已完成", theme.colors.success),
        RowStatus::Error => ui::badge("错误", theme.colors.danger),
    }
}
