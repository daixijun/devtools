//! 图片压缩工具。
//!
//! 「设置卡 + 文件列表表格 + 开始按钮 + 进度条 + 完成统计」模式。
//! 降级点（v1）：
//! - 缩略图：app 的 image 依赖仅启用 png feature，无法解码 jpeg/webp 生成真实缩略图，
//!   改为显示「格式徽章 + 尺寸文字」（read_image_data_url 的 data_url 无法直接喂给 GPUI）。
//! - 对比滑块弹窗：降级为行内对比信息（压缩后列展示 原 size → 新 size 与 -N%）。

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

use devtools::i18n::t;
use devtools::tools::image_compressor::{compress_image, CompressRequest};
use devtools::tools::image_converter::get_image_info_command;

use crate::components::notify;
use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::ui;

/// 支持压缩的输入扩展名（与 core 白名单一致）。
const SUPPORTED_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp"];

/// 输出格式选项。
const OUTPUT_FORMATS: &[(&str, &str)] = &[
    ("keep", "保持原格式"),
    ("png", "PNG"),
    ("jpeg", "JPEG"),
    ("webp", "WebP"),
];

/// PNG 无损优化级别选项。
const PNG_LEVELS: &[(&str, &str)] = &[
    ("1", "1 - 最快"),
    ("2", "2 - 快速"),
    ("3", "3 - 标准 (推荐)"),
    ("4", "4 - 较强"),
    ("5", "5 - 强力"),
    ("6", "6 - 最大压缩"),
];

/// 文件行状态。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowStatus {
    Pending,
    Compressing,
    Completed,
    Error,
}

/// 表格中的一行。
struct FileRow {
    path: String,
    name: String,
    format: String,
    size: u64,
    dims: Option<String>,
    output_path: Option<String>,
    compressed_size: Option<u64>,
    ratio: Option<f64>,
    out_dims: Option<String>,
    status: RowStatus,
    error: Option<String>,
}

pub struct ImageCompressor {
    // 压缩设置
    mode: String, // "lossless" | "quality"
    output_sel: Entity<SelectState<SimpleSelectDelegate>>,
    output_format: String, // "keep" | "png" | "jpeg" | "webp"
    quality_slider: Entity<SliderState>,
    quality: u8,
    png_sel: Entity<SelectState<SimpleSelectDelegate>>,
    png_level: u8,
    max_width_input: Entity<InputState>,
    max_height_input: Entity<InputState>,
    remove_exif: bool,
    custom_output: bool, // false = 源同级 compressed/ 子目录
    custom_dir: String,

    // 文件列表与执行状态
    files: Vec<FileRow>,
    converting: bool,
    /// 已完成/正在处理的文件数（用于进度显示）。
    progress_index: usize,
    /// 批量结果横幅（是否成功, 文案）。
    message: Option<(bool, String)>,
}

impl ImageCompressor {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        // 常量 label 含中文：使用处经 t() 翻译（select_state 会拷贝 label）
        let output_options: Vec<(String, String)> = OUTPUT_FORMATS
            .iter()
            .map(|(value, label)| (value.to_string(), t(label).to_string()))
            .collect();
        let output_refs: Vec<(&str, &str)> = output_options
            .iter()
            .map(|(value, label)| (value.as_str(), label.as_str()))
            .collect();
        let output_sel = select_state(window, cx, &output_refs, 0);
        cx.subscribe(
            &output_sel,
            |this, _, ev: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.output_format = v.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let quality_slider = cx.new(|_| SliderState::new().min(1.).max(100.).default_value(80.));
        cx.subscribe(&quality_slider, |this, _, ev: &SliderEvent, cx| {
            let (SliderEvent::Change(v) | SliderEvent::Release(v)) = ev;
            this.quality = v.start().round().clamp(1., 100.) as u8;
            cx.notify();
        })
        .detach();

        let png_options: Vec<(String, String)> = PNG_LEVELS
            .iter()
            .map(|(value, label)| (value.to_string(), t(label).to_string()))
            .collect();
        let png_refs: Vec<(&str, &str)> = png_options
            .iter()
            .map(|(value, label)| (value.as_str(), label.as_str()))
            .collect();
        let png_sel = select_state(window, cx, &png_refs, 2); // 默认 3
        cx.subscribe(
            &png_sel,
            |this, _, ev: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.png_level = v.parse::<u8>().unwrap_or(3).clamp(1, 6);
                    cx.notify();
                }
            },
        )
        .detach();

        // placeholder 挂在 InputState 上（gpui-component 0.10 的 Input 元素无 placeholder 方法）
        let max_width_input = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 1920")));
        let max_height_input = cx.new(|cx| InputState::new(window, cx).placeholder(t("如 1080")));

        Self {
            mode: "quality".to_string(),
            output_sel,
            output_format: "keep".to_string(),
            quality_slider,
            quality: 80,
            png_sel,
            png_level: 3,
            max_width_input,
            max_height_input,
            remove_exif: true,
            custom_output: false,
            custom_dir: String::new(),
            files: Vec::new(),
            converting: false,
            progress_index: 0,
            message: None,
        }
    }

    // -----------------------------------------------------------------------
    // 设置辅助
    // -----------------------------------------------------------------------

    /// 质量滑块可见性（mode=quality 时显示）。
    fn quality_visible(&self) -> bool {
        self.mode == "quality"
    }

    /// PNG 无损级别是否生效（输出含 png 时显示）。
    fn png_level_applies(&self) -> bool {
        self.output_format == "png"
            || (self.output_format == "keep" && self.files.iter().any(|f| f.format == "png"))
    }

    fn parse_bound(&self, entity: &Entity<InputState>, cx: &App) -> Option<u32> {
        let text = entity.read(cx).value().trim().to_string();
        match text.parse::<u32>() {
            Ok(v) if v > 0 => Some(v.min(100000)),
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
    // 文件添加
    // -----------------------------------------------------------------------

    fn add_files(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let exts: Vec<String> = SUPPORTED_EXTS.iter().map(|s| s.to_string()).collect();
        let rx = crate::core_rt::submit(cx, async move {
            let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            rfd::FileDialog::new()
                .add_filter(t("图片文件"), &refs)
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
        let exts: Vec<String> = SUPPORTED_EXTS.iter().map(|s| s.to_string()).collect();
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

    /// 过滤、按 path 去重后加入列表，并逐个异步回填尺寸。
    fn add_paths(&mut self, paths: Vec<String>, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let existing: HashSet<String> = self.files.iter().map(|f| f.path.clone()).collect();
        let fresh: Vec<String> = paths
            .into_iter()
            .filter(|p| !existing.contains(p))
            .filter(|p| has_supported_ext(p))
            .collect();
        if fresh.is_empty() {
            self.message = Some((
                false,
                t("未找到支持的图片文件 (jpg/jpeg/png/gif/bmp/tiff/tif/webp)").to_string(),
            ));
            cx.notify();
            return;
        }

        for p in &fresh {
            self.files.push(FileRow {
                path: p.clone(),
                name: file_name_of(p),
                format: ext_of(p),
                size: 0,
                dims: None,
                output_path: None,
                compressed_size: None,
                ratio: None,
                out_dims: None,
                status: RowStatus::Pending,
                error: None,
            });
        }
        self.message = None;
        cx.notify();

        // 逐个 get_image_info_command 回填尺寸（后台顺序执行）
        cx.spawn_in(window, async move |this, cx| {
            for path in fresh {
                let rx = this
                    .update_in(cx, |_this, _w, cx| {
                        let p = path.clone();
                        crate::core_rt::submit(cx, async move {
                            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                            let info = get_image_info_command(p.clone()).await;
                            (p, size, info.map(|i| i.dimensions))
                        })
                    })
                    .ok();
                if let Some(mut rx) = rx {
                    if let Some((path, size, dims)) = futures::StreamExt::next(&mut rx).await {
                        this.update_in(cx, |this, _w, cx| {
                            if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                                row.size = size;
                                if let Ok(dims) = dims {
                                    row.dims = Some(dims);
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
    // 批量压缩
    // -----------------------------------------------------------------------

    fn start_compress(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.converting || self.files.is_empty() {
            return;
        }
        if self.custom_output && self.custom_dir.is_empty() {
            self.message = Some((false, t("已启用自定义输出目录, 但未选择目录").to_string()));
            cx.notify();
            return;
        }

        // 快照全部设置（压缩过程中设置项已禁用）
        let mode = self.mode.clone();
        let quality = self.quality;
        let output_format = self.output_format.clone();
        let png_level = self.png_level;
        let remove_exif = self.remove_exif;
        let custom_output = self.custom_output;
        let custom_dir = self.custom_dir.clone();
        let max_width = self.parse_bound(&self.max_width_input, cx);
        let max_height = self.parse_bound(&self.max_height_input, cx);

        // 批内输出冲突预检：不同源文件映射到同一输出时会相互覆盖
        let mut seen_outputs = std::collections::HashSet::new();
        for row in &self.files {
            let output_path =
                build_output_path(&row.path, &output_format, custom_output, &custom_dir);
            if !seen_outputs.insert(output_path.clone()) {
                self.message = Some((
                    false,
                    format!(
                        "{}: {}",
                        t("输出路径冲突：多个文件映射到同一输出"),
                        output_path
                    ),
                ));
                cx.notify();
                return;
            }
        }

        let mut jobs: Vec<(String, CompressRequest)> = Vec::new();
        for row in &self.files {
            let output_path =
                build_output_path(&row.path, &output_format, custom_output, &custom_dir);
            jobs.push((
                row.path.clone(),
                CompressRequest {
                    input_path: row.path.clone(),
                    output_path,
                    mode: mode.clone(),
                    quality: if mode == "quality" {
                        Some(quality)
                    } else {
                        None
                    },
                    max_width,
                    max_height,
                    output_format: output_format.clone(),
                    remove_exif,
                    png_optimize_level: Some(png_level),
                },
            ));
        }

        self.converting = true;
        self.progress_index = 0;
        self.message = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let mut success = 0usize;
            let mut failed = 0usize;

            for (i, (path, request)) in jobs.into_iter().enumerate() {
                // 标记压缩中 + 提交该文件压缩任务
                let rx = this
                    .update_in(cx, |this, _w, cx| {
                        if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                            row.status = RowStatus::Compressing;
                            row.error = None;
                        }
                        this.progress_index = i;
                        cx.notify();
                        crate::core_rt::submit(cx, async move { compress_image(request).await })
                    })
                    .ok();

                if let Some(mut rx) = rx {
                    if let Some(result) = futures::StreamExt::next(&mut rx).await {
                        let outcome: Result<(String, u64, f64, String), String> = match result {
                            Ok(resp) if resp.success => {
                                success += 1;
                                Ok((
                                    resp.output_path,
                                    resp.compressed_size,
                                    resp.compression_ratio,
                                    format!("{}x{}", resp.output_width, resp.output_height),
                                ))
                            }
                            Ok(resp) => {
                                failed += 1;
                                Err(resp.message)
                            }
                            Err(e) => {
                                failed += 1;
                                Err(e)
                            }
                        };
                        this.update_in(cx, |this, _w, cx| {
                            if let Some(row) = this.files.iter_mut().find(|f| f.path == path) {
                                match outcome {
                                    Ok((out, csize, ratio, out_dims)) => {
                                        row.status = RowStatus::Completed;
                                        row.output_path = Some(out);
                                        row.compressed_size = Some(csize);
                                        row.ratio = Some(ratio);
                                        row.out_dims = Some(out_dims);
                                    }
                                    Err(e) => {
                                        row.status = RowStatus::Error;
                                        row.error = Some(e);
                                    }
                                }
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
                    (
                        true,
                        format!("✅ {} {} {}", t("批量压缩完成: 成功"), success, t("个文件")),
                    )
                } else {
                    (
                        false,
                        format!(
                            "{} {} {} {} {}",
                            t("批量压缩完成: 成功"),
                            success,
                            t("个, 失败"),
                            failed,
                            t("个")
                        ),
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
        // 压缩中的行禁删
        if self.files.get(index).map(|f| f.status) == Some(RowStatus::Compressing) {
            return;
        }
        if index < self.files.len() {
            self.files.remove(index);
            cx.notify();
        }
    }

    fn clear_list(&mut self, cx: &mut gpui::Context<Self>) {
        // 压缩中禁止清空，避免打断写盘（历史版本允许，属修正偏差）
        if self.converting {
            return;
        }
        self.files.clear();
        self.progress_index = 0;
        self.message = None;
        cx.notify();
    }

    fn copy_text(&mut self, text: String, window: &mut Window, cx: &mut gpui::Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        notify::toast_success(window, cx, t("已复制到剪贴板"));
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

    /// 完成统计：已压缩数 / 原始合计 / 压缩后 / 节省空间。
    fn stats(&self) -> (usize, u64, u64) {
        let done: Vec<&FileRow> = self
            .files
            .iter()
            .filter(|f| f.status == RowStatus::Completed)
            .collect();
        let total_original: u64 = done.iter().map(|f| f.size).sum();
        let total_compressed: u64 = done.iter().map(|f| f.compressed_size.unwrap_or(0)).sum();
        (done.len(), total_original, total_compressed)
    }
}

// ---------------------------------------------------------------------------
// 纯函数辅助
// ---------------------------------------------------------------------------

/// 输出路径 = 自定义目录（若启用且非空），否则源同级 compressed/ 子目录。
/// keep 保持同名；png/jpeg/webp 换扩展名（jpeg → jpg）。
fn build_output_path(
    source: &str,
    output_format: &str,
    use_custom: bool,
    custom_dir: &str,
) -> String {
    let p = Path::new(source);
    let name = if output_format == "keep" {
        file_name_of(source)
    } else {
        let ext = if output_format == "jpeg" {
            "jpg"
        } else {
            output_format
        };
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".to_string());
        format!("{}.{}", stem, ext)
    };

    let dir = if use_custom && !custom_dir.is_empty() {
        PathBuf::from(custom_dir)
    } else {
        match p.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join("compressed"),
            _ => PathBuf::from("compressed"),
        }
    };
    let mut buf = dir;
    buf.push(name);
    buf.to_string_lossy().to_string()
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

fn has_supported_ext(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn ext_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
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

impl gpui::Render for ImageCompressor {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let mut page = ui::tool_page("imagecompressor").child(self.render_settings(cx));

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

impl ImageCompressor {
    fn render_settings(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        let converting = self.converting;

        let card = ui::card(cx)
            .child(ui::card_header(t("压缩设置"), cx))
            // 压缩模式 + 输出格式
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .child(
                        div().flex_1().min_w(px(300.)).child(
                            ui::field(
                                t("压缩模式"),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_4()
                                            .child(
                                                Radio::new("ic-mode-lossless")
                                                    .label(t("无损优化"))
                                                    .checked(self.mode == "lossless")
                                                    .disabled(converting)
                                                    .on_click(cx.listener(
                                                        |this, _c: &bool, _w, cx| {
                                                            this.mode = "lossless".to_string();
                                                            cx.notify();
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Radio::new("ic-mode-quality")
                                                    .label(t("质量压缩"))
                                                    .checked(self.mode == "quality")
                                                    .disabled(converting)
                                                    .on_click(cx.listener(
                                                        |this, _c: &bool, _w, cx| {
                                                            this.mode = "quality".to_string();
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(ui::hint_text(
                                        if self.mode == "lossless" {
                                            t("PNG 由 oxipng 真无损优化(对已优化的 PNG 效果有限); JPEG 以最高质量重编码(近似无损)")
                                        } else {
                                            t("PNG 走色彩量化(视觉无损, 类 TinyPNG); JPEG/WebP 通过降低质量换取更小体积")
                                        },
                                        cx,
                                    )),
                                cx,
                            ),
                        ),
                    )
                    .child(
                        div().flex_1().min_w(px(220.)).child(
                            ui::field(
                                t("输出格式"),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Select::new(&self.output_sel).disabled(converting))
                                    .child(ui::hint_text(t("WebP 仅支持无损编码"), cx)),
                                cx,
                            ),
                        ),
                    ),
            );

        // 质量滑块（mode=quality 显示）
        let card = if self.quality_visible() {
            card.child(
                ui::field(
                    &format!("{}: {}%", t("图片质量"), self.quality),
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
                .child(ui::hint_text(
                    t("质量越低文件越小(PNG 为色彩数, JPEG/WebP 为编码质量), 建议 70-90"),
                    cx,
                )),
            )
        } else {
            card
        };

        // PNG 无损级别（输出含 png 时显示）
        let card = if self.png_level_applies() {
            card.child(ui::field(
                t("PNG 无损优化级别"),
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Select::new(&self.png_sel).disabled(converting))
                    .child(ui::hint_text(t("级别越高压缩率越好, 但耗时增加"), cx)),
                cx,
            ))
        } else {
            card
        };

        // 等比缩放
        let card = card.child(
            ui::field(
                t("等比缩放 (可选, 留空则保持原尺寸)"),
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(t("最大宽度")),
                            )
                            .child(
                                Input::new(&self.max_width_input)
                                    .w(px(110.))
                                    .disabled(converting),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child("px"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(t("最大高度")),
                            )
                            .child(
                                Input::new(&self.max_height_input)
                                    .w(px(110.))
                                    .disabled(converting),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.colors.muted_foreground)
                                    .child("px"),
                            ),
                    ),
                cx,
            )
            .child(ui::hint_text(
                t("同时填写时按约束框等比缩放; 只填一项则仅约束该方向"),
                cx,
            )),
        );

        // 移除 EXIF（默认勾选）
        card.child(
            Checkbox::new("ic-opt-exif")
                .label(t("移除 EXIF 等元数据 (保护隐私且进一步减小体积)"))
                .checked(self.remove_exif)
                .disabled(converting)
                .on_click(cx.listener(|this, checked: &bool, _w, cx| {
                    this.remove_exif = *checked;
                    cx.notify();
                })),
        )
        // 输出位置
        .child(ui::field(
            t("输出位置"),
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(
                    Radio::new("ic-out-auto")
                        .label(t("自动创建 compressed 子目录"))
                        .checked(!self.custom_output)
                        .disabled(converting)
                        .on_click(cx.listener(|this, _c: &bool, _w, cx| {
                            this.custom_output = false;
                            cx.notify();
                        })),
                )
                .child(
                    Radio::new("ic-out-custom")
                        .label(t("自定义输出目录"))
                        .checked(self.custom_output)
                        .disabled(converting)
                        .on_click(cx.listener(|this, _c: &bool, _w, cx| {
                            this.custom_output = true;
                            cx.notify();
                        })),
                ),
            cx,
        ))
        .children(if self.custom_output {
            Some(self.render_custom_dir(cx))
        } else {
            Some(ui::hint_text(
                t("压缩结果将写入每个源文件同级的 compressed/ 子目录"),
                cx,
            ))
        })
    }

    fn render_custom_dir(&self, cx: &gpui::Context<Self>) -> gpui::Div {
        let converting = self.converting;
        // 只读目录显示框（muted 底路径块）+ 右侧操作按钮
        let dir_text: &str = if self.custom_dir.is_empty() {
            t("选择输出目录...")
        } else {
            &self.custom_dir
        };
        let mut dir_row = ui::file_row(dir_text, cx).child(
            ui::secondary_button(
                "ic-choose-dir",
                t("选择目录"),
                cx.listener(|this, _ev, window, cx| this.choose_dir(window, cx)),
            )
            .disabled(converting),
        );
        if !self.custom_dir.is_empty() {
            dir_row = dir_row.child(
                ui::secondary_button(
                    "ic-clear-dir",
                    t("清除"),
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
            .children((!self.custom_dir.is_empty()).then(|| {
                ui::hint_text(
                    &format!("{} {}", t("压缩后的文件将保存到:"), self.custom_dir),
                    cx,
                )
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
                        .child(format!("{} ({})", t("待压缩文件"), self.files.len())),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(ui::secondary_button(
                            "ic-add-files",
                            t("添加文件"),
                            cx.listener(|this, _ev, window, cx| this.add_files(window, cx)),
                        ))
                        .child(ui::secondary_button(
                            "ic-add-dir",
                            t("添加目录"),
                            cx.listener(|this, _ev, window, cx| this.add_dir(window, cx)),
                        )),
                ),
        );

        if self.files.is_empty() {
            return card.child(ui::empty_hint(
                t("暂无文件, 点击右上角「添加文件」或「添加目录」"),
                cx,
            ));
        }

        // 表头
        let header = ui::table_head(cx)
            .px_2()
            .pb_1()
            .child(ui::table_head_cell(t("预览"), cx).w(px(56.)))
            .child(ui::table_head_cell(t("文件名"), cx).flex_1().min_w_0())
            .child(ui::table_head_cell(t("原始大小"), cx).w(px(84.)))
            .child(ui::table_head_cell(t("压缩后"), cx).w(px(140.)))
            .child(ui::table_head_cell(t("尺寸"), cx).w(px(170.)))
            .child(ui::table_head_cell(t("状态"), cx).w(px(76.)))
            .child(ui::table_head_cell(t("操作"), cx).w(px(52.)));

        // 数据行（行间 hairline 由 table_rows 统一处理）
        let rows: Vec<gpui::Div> = self
            .files
            .iter()
            .enumerate()
            .map(|(index, row)| div().child(self.render_row(index, row, cx)))
            .collect();

        card.child(
            div()
                .id("ic-table")
                .max_h(px(360.))
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

        let path_for_copy = row.path.clone();
        let out_path = row.output_path.clone();
        let status = row.status;

        // 文件名列（名 + 格式 + 完成后的输出文件名，均可点击复制）
        let name_cell: AnyElement = {
            let mut cell = div()
                .flex()
                .flex_col()
                .gap_0p5()
                .min_w_0()
                .child(
                    div()
                        .id(("ic-name", index))
                        .min_w_0()
                        .truncate()
                        .cursor_pointer()
                        .text_size(px(12.))
                        .text_color(theme.colors.foreground)
                        .on_click(cx.listener(move |this, _ev, window, cx| {
                            this.copy_text(path_for_copy.clone(), window, cx);
                        }))
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.colors.muted_foreground)
                        .child(row.format.to_uppercase()),
                );
            if let Some(out) = &out_path {
                let out_name = file_name_of(out);
                let out = out.clone();
                cell = cell.child(
                    div()
                        .id(("ic-out", index))
                        .min_w_0()
                        .truncate()
                        .cursor_pointer()
                        .text_size(px(11.))
                        .text_color(theme.colors.success)
                        .on_click(cx.listener(move |this, _ev, window, cx| {
                            this.copy_text(out.clone(), window, cx);
                        }))
                        .child(format!("→ {}", out_name)),
                );
            }
            cell.into_any_element()
        };

        // 压缩后列（完成时展示行内对比信息：新 size + 节省百分比）
        let compressed_cell: AnyElement = match (status, row.compressed_size, row.ratio) {
            (RowStatus::Completed, Some(csize), Some(ratio)) => {
                let pct = ((1.0 - ratio) * 100.0).round() as i64;
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.foreground)
                            .child(format_file_size(csize)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(if ratio < 1.0 {
                                theme.colors.success
                            } else {
                                theme.colors.warning
                            })
                            .child(format!(
                                "{}{}%",
                                if pct >= 0 { "-" } else { "+" },
                                pct.abs()
                            )),
                    )
                    .into_any_element()
            }
            _ => div()
                .text_size(px(12.))
                .text_color(theme.colors.muted_foreground)
                .child("-")
                .into_any_element(),
        };

        // 尺寸列：原 → 输出
        let dims_text = match &row.out_dims {
            Some(out_dims) => format!(
                "{} → {}",
                row.dims.clone().unwrap_or_else(|| t("未知").to_string()),
                out_dims
            ),
            None => row.dims.clone().unwrap_or_else(|| t("未知").to_string()),
        };

        div()
            .id(("ic-row", index))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .hover(|s| s.bg(theme.colors.muted))
            // 预览（v1 降级：格式徽章 + 尺寸文字，见文件头注释）
            .child(self.render_preview_cell(row, cx))
            // 文件名
            .child(div().flex_1().min_w_0().child(name_cell))
            // 原始大小
            .child(
                div()
                    .w(px(84.))
                    .text_size(px(12.))
                    .text_color(theme.colors.foreground)
                    .child(if row.size > 0 {
                        format_file_size(row.size)
                    } else {
                        t("未知").to_string()
                    }),
            )
            // 压缩后（行内对比信息）
            .child(div().w(px(140.)).child(compressed_cell))
            // 尺寸
            .child(
                div()
                    .w(px(170.))
                    .truncate()
                    .text_size(px(12.))
                    .text_color(theme.colors.foreground)
                    .child(dims_text),
            )
            // 状态
            .child(div().w(px(76.)).child(status_badge(status, cx)))
            // 操作：删除（压缩中禁删）
            .child(
                div().w(px(52.)).flex().justify_center().child(
                    Button::new(("ic-del", index))
                        .label(t("删除"))
                        .ghost()
                        .on_click(cx.listener(move |this, _ev, _window, cx| {
                            this.remove_row(index, cx);
                        }))
                        .disabled(status == RowStatus::Compressing)
                        .text_color(theme.colors.danger),
                ),
            )
    }

    /// 预览单元格（降级实现：48×48 框内显示格式徽章 + 尺寸文字）。
    fn render_preview_cell(&self, row: &FileRow, cx: &gpui::Context<Self>) -> gpui::Div {
        let theme = cx.theme().clone();
        div()
            .w(px(48.))
            .h(px(48.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_0p5()
            .rounded(theme.radius)
            .bg(theme.colors.muted)
            .overflow_hidden()
            .child(ui::badge(&row.format.to_uppercase(), theme.colors.info))
            .child(
                div()
                    .text_size(px(11.))
                    .truncate()
                    .max_w(px(44.))
                    .text_color(theme.colors.muted_foreground)
                    .child(row.dims.clone().unwrap_or_else(|| "N/A".to_string())),
            )
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

        let mut card = ui::card(cx);

        // 完成统计卡（4 格）
        let (count, total_original, total_compressed) = self.stats();
        if count > 0 && !self.converting {
            let saved = total_original.saturating_sub(total_compressed);
            let saved_pct = if total_original > 0 {
                (1.0 - total_compressed as f64 / total_original as f64) * 100.0
            } else {
                0.0
            };
            card = card.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_4()
                    .pb_1()
                    .child(
                        ui::stat_tile(&count.to_string(), t("已压缩"), cx)
                            .flex_1()
                            .min_w(px(140.)),
                    )
                    .child(
                        ui::stat_tile(&format_file_size(total_original), t("原始合计"), cx)
                            .flex_1()
                            .min_w(px(140.)),
                    )
                    .child(
                        ui::stat_tile(&format_file_size(total_compressed), t("压缩后"), cx)
                            .flex_1()
                            .min_w(px(140.)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(140.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_0p5()
                            .py_1()
                            .rounded(theme.radius)
                            .bg(theme.colors.success.opacity(0.1))
                            .child(
                                div()
                                    .text_size(px(17.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.success)
                                    .child(format!(
                                        "{} ({:.1}%)",
                                        format_file_size(saved),
                                        saved_pct
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.colors.success)
                                    .child(t("节省空间")),
                            ),
                    ),
            );
        }

        if self.converting {
            card = card.child(
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
                                &format!(
                                    "{} {} / {} {}",
                                    t("总体进度:"),
                                    current,
                                    total,
                                    t("个文件")
                                ),
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
            );
        }

        let start_disabled = self.converting || self.files.is_empty();

        card.child(
            div()
                .flex()
                .gap_2()
                .child(
                    ui::primary_button(
                        "ic-start",
                        if self.converting {
                            t("压缩中...")
                        } else {
                            t("开始压缩")
                        },
                        cx.listener(|this, _ev, window, cx| this.start_compress(window, cx)),
                    )
                    .disabled(start_disabled)
                    .loading(self.converting)
                    .flex_1(),
                )
                .child(
                    ui::secondary_button(
                        "ic-clear",
                        t("清空列表"),
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

fn status_badge(status: RowStatus, cx: &gpui::Context<ImageCompressor>) -> gpui::Div {
    let theme = cx.theme().clone();
    match status {
        RowStatus::Pending => ui::badge(t("等待中"), theme.colors.warning),
        RowStatus::Compressing => ui::badge(t("压缩中..."), theme.colors.info),
        RowStatus::Completed => ui::badge(t("已完成"), theme.colors.success),
        RowStatus::Error => ui::badge(t("失败"), theme.colors.danger),
    }
}
