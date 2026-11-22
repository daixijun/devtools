//! 图片预览器。
//!
//! 功能：http(s)/data: URL/本地文件加载预览、滚轮缩放（0.5–5）、拖拽平移、
//! 双击 1×/2×、图片信息面板、格式转换保存（rfd save_file + image crate 重编码）。
//!
//! 降级说明：
//! - Cmd+V 直接粘贴图片：GPUI 没有可直接复用的窗口级图片剪贴板事件，
//!   v1 降级为仅 URL / 本地文件加载。
//! - 全屏遮罩：GPUI 无独立遮罩层窗口能力，降级为窗口内自适应预览（不提供全屏）。
//! - 远程 http(s) URL：交由 gpui img 内置 loader（`ImageSource::Resource(
//!   Resource::Uri)`，gpui `ImageAssetLoader` 用自带 http_client 拉取并解码），
//!   UI 侧不持有字节，因此尺寸/格式/大小显示「未知」。
//! - data: URL：gpui 内置 http client 不支持 `data:` scheme，改为在后台
//!   base64 解码后以 `ImageSource::Image(Arc<gpui::Image>)` 渲染。
//! - 本地文件：以 `Resource::Path` 交给 gpui 内置 loader 渲染；元信息在后台
//!   用 image crate 解码一次（读取尺寸/格式/字节数，供信息面板与保存）。
//!   image 特性为 `default-formats`，全格式解码/编码可用。
//! - webp/gif 保存重编码：image crate 的 webp 编码器仅支持无损、gif 无质量
//!   参数，质量选项实际仅对 jpeg 生效。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use devtools::i18n::t;
use futures::StreamExt as _;
use gpui::{
    div, img, prelude::*, px, App, ClickEvent, Entity, ImageFormat, ImageSource, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Resource, ScrollEvent, Window,
};
use gpui_component::{
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    slider::{Slider, SliderEvent, SliderState},
    ActiveTheme as _, Disableable as _,
};
use image::ImageEncoder as _;

use crate::components::select::{select_state, SimpleSelectDelegate};
use crate::components::{notify, ui};

/// 当前图片来源。
#[derive(Clone)]
enum LoadedSource {
    /// 本地文件（gpui 内置 loader 按路径读取）
    File(PathBuf),
    /// 远程 http(s) URL（gpui 内置 http loader 拉取）
    Uri(String),
    /// 内存图片（data: URL 后台解码所得）
    Data(Arc<gpui::Image>),
}

/// 图片元信息（信息面板展示 + 缩放计算）。
struct ImageMeta {
    width: u32,
    height: u32,
    format: String,
    size: u64,
}

/// 后台加载结果。
enum LoadOutcome {
    /// 用户取消了文件选择对话框
    Cancelled,
    Local {
        path: PathBuf,
        bytes: Arc<Vec<u8>>,
        meta: Option<ImageMeta>,
    },
    Remote {
        url: String,
    },
    Data {
        image: Arc<gpui::Image>,
        bytes: Arc<Vec<u8>>,
        meta: Option<ImageMeta>,
    },
    Failed(String),
}

/// 保存任务结果。
enum SaveOutcome {
    Saved(String),
    Cancelled,
    Failed(String),
}

pub struct ImagePreview {
    // 输入与保存控件
    url_input: Entity<InputState>,
    filename_input: Entity<InputState>,
    save_format_select: Entity<SelectState<SimpleSelectDelegate>>,
    quality_slider: Entity<SliderState>,

    // 图片状态
    source: Option<LoadedSource>,
    /// 复制 URL 用的展示文本（URL / 本地路径 / data: URL）
    source_label: Option<String>,
    /// 原始字节（本地文件与 data: URL 有；远程 URL 为 None）
    source_bytes: Option<Arc<Vec<u8>>>,
    meta: Option<ImageMeta>,

    // 视图状态（缩放 / 平移）
    scale: f32,
    offset: (f32, f32),
    dragging: bool,
    drag_last: (f32, f32),

    // 保存设置
    save_format: String,
    quality: u32,

    // 任务状态
    loading: bool,
    saving: bool,
    error: Option<String>,
}

impl ImagePreview {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        let url_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("粘贴图片 URL 或 data: URL...")));
        cx.subscribe(&url_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.load_from_input(cx);
            }
        })
        .detach();

        let filename_input = cx.new(|cx| InputState::new(window, cx));
        filename_input.update(cx, |input, cx| input.set_value("image", window, cx));

        let save_format_select = select_state(
            window,
            cx,
            &[
                ("png", t("PNG (无损)")),
                ("jpeg", t("JPEG (有损)")),
                ("gif", t("GIF (动图)")),
                ("webp", t("WebP (现代格式)")),
            ],
            0,
        );
        cx.subscribe(
            &save_format_select,
            |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.save_format = value.to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        let quality_slider = cx.new(|_| {
            SliderState::new()
                .min(10.)
                .max(100.)
                .step(1.)
                .default_value(85.)
        });
        cx.subscribe(&quality_slider, |this, _, event: &SliderEvent, cx| {
            let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
            let q = value.end().round().clamp(10., 100.) as u32;
            if this.quality != q {
                this.quality = q;
                cx.notify();
            }
        })
        .detach();

        Self {
            url_input,
            filename_input,
            save_format_select,
            quality_slider,
            source: None,
            source_label: None,
            source_bytes: None,
            meta: None,
            scale: 1.,
            offset: (0., 0.),
            dragging: false,
            drag_last: (0., 0.),
            save_format: "png".into(),
            quality: 85,
            loading: false,
            saving: false,
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // 加载流程
    // ------------------------------------------------------------------

    /// 从 URL 输入框加载（「加载」按钮 / Enter 触发）。
    fn load_from_input(&mut self, cx: &mut gpui::Context<Self>) {
        let input = self.url_input.read(cx).value().trim().to_string();
        if input.is_empty() {
            self.error = Some(t("请输入图片 URL 或 data: URL").into());
            self.loading = false;
            cx.notify();
            return;
        }
        self.loading = true;
        self.error = None;
        self.source_label = Some(input.clone());
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move { decode_source_job(input) });
        cx.spawn(async move |this, cx| {
            let outcome = rx
                .next()
                .await
                .unwrap_or(LoadOutcome::Failed(t("加载任务已中断").into()));
            let _ = this.update(cx, |this, cx| this.on_loaded(outcome, cx));
        })
        .detach();
    }

    /// 「上传文件」：rfd pick_file（后台线程）+ 后台解码。
    fn pick_file(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.loading = true;
        self.error = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    t("图片文件"),
                    &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tiff", "ico"],
                )
                .pick_file()
            else {
                return LoadOutcome::Cancelled;
            };
            let path_str = path.display().to_string();
            decode_source_job(path_str)
        });
        cx.spawn_in(window, async move |this, cx| {
            let outcome = rx
                .next()
                .await
                .unwrap_or(LoadOutcome::Failed(t("加载任务已中断").into()));
            let _ = this.update_in(cx, |this, _window, cx| this.on_loaded(outcome, cx));
        })
        .detach();
    }

    fn on_loaded(&mut self, outcome: LoadOutcome, cx: &mut gpui::Context<Self>) {
        self.loading = false;
        match outcome {
            LoadOutcome::Cancelled => {
                if self.source.is_none() {
                    self.source_label = None;
                }
            }
            LoadOutcome::Local { path, bytes, meta } => {
                self.source_label = Some(path.display().to_string());
                self.source = Some(LoadedSource::File(path));
                self.source_bytes = Some(bytes);
                self.meta = meta;
                self.reset_view(cx);
            }
            LoadOutcome::Remote { url } => {
                self.source_label = Some(url.clone());
                self.source = Some(LoadedSource::Uri(url));
                // gpui 内置 loader 拉取，UI 侧无字节与尺寸信息（降级）
                self.source_bytes = None;
                self.meta = None;
                self.reset_view(cx);
            }
            LoadOutcome::Data { image, bytes, meta } => {
                self.source = Some(LoadedSource::Data(image));
                self.source_bytes = Some(bytes);
                self.meta = meta;
                self.reset_view(cx);
            }
            LoadOutcome::Failed(message) => {
                self.error = Some(message);
                self.source = None;
                self.source_label = None;
                self.source_bytes = None;
                self.meta = None;
            }
        }
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 缩放 / 平移
    // ------------------------------------------------------------------

    fn on_wheel(&mut self, ev: &ScrollEvent, cx: &mut gpui::Context<Self>) {
        // 远程 URL 无尺寸信息，无法按比例缩放（降级为自适应显示）
        if self.meta.is_none() {
            return;
        }
        let dy: f32 = ev.delta.pixel_delta(px(20.)).y.into();
        if dy == 0. {
            return;
        }
        self.apply_zoom(if dy > 0. { 0.1 } else { -0.1 }, cx);
    }

    fn apply_zoom(&mut self, delta: f32, cx: &mut gpui::Context<Self>) {
        let next = (self.scale + delta).clamp(0.5, 5.0);
        if (next - self.scale).abs() > f32::EPSILON {
            self.scale = next;
            cx.notify();
        }
    }

    fn on_drag_start(&mut self, ev: &MouseDownEvent, cx: &mut gpui::Context<Self>) {
        if self.source.is_none() {
            return;
        }
        self.dragging = true;
        self.drag_last = (ev.position.x.into(), ev.position.y.into());
        cx.notify();
    }

    fn on_drag_move(&mut self, ev: &MouseMoveEvent, cx: &mut gpui::Context<Self>) {
        if !self.dragging {
            return;
        }
        let x: f32 = ev.position.x.into();
        let y: f32 = ev.position.y.into();
        self.offset.0 += x - self.drag_last.0;
        self.offset.1 += y - self.drag_last.1;
        self.drag_last = (x, y);
        cx.notify();
    }

    fn on_drag_end(&mut self, cx: &mut gpui::Context<Self>) {
        if self.dragging {
            self.dragging = false;
            cx.notify();
        }
    }

    fn on_double_click(&mut self, cx: &mut gpui::Context<Self>) {
        if self.meta.is_none() {
            return;
        }
        self.scale = if self.scale > 1.5 { 1. } else { 2. };
        self.offset = (0., 0.);
        cx.notify();
    }

    fn reset_view(&mut self, cx: &mut gpui::Context<Self>) {
        self.scale = 1.;
        self.offset = (0., 0.);
        self.dragging = false;
        cx.notify();
    }

    fn copy_source_url(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if let Some(label) = self.source_label.clone() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(label));
            notify::toast_success(window, cx, t("已复制"));
        }
    }

    // ------------------------------------------------------------------
    // 保存
    // ------------------------------------------------------------------

    fn save_image(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let Some(bytes) = self.source_bytes.clone() else {
            self.error = Some(
                t("远程 URL 图片没有本地数据，暂不支持保存；请通过「上传文件」加载本地图片后再保存")
                    .into(),
            );
            cx.notify();
            return;
        };
        let format = self.save_format.clone();
        let quality = self.quality;
        let filename = self.filename_input.read(cx).value().trim().to_string();
        let filename = if filename.is_empty() {
            "image".into()
        } else {
            filename
        };
        self.saving = true;
        self.error = None;
        cx.notify();

        let mut rx = crate::core_rt::submit(cx, async move {
            save_image_job(bytes, &format, quality, &filename)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = rx
                .next()
                .await
                .unwrap_or(SaveOutcome::Failed(t("保存任务已中断").into()));
            let _ = this.update_in(cx, |this, window, cx| this.on_saved(result, window, cx));
        })
        .detach();
    }

    fn on_saved(&mut self, result: SaveOutcome, window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.saving = false;
        match result {
            SaveOutcome::Saved(path) => {
                self.error = None;
                notify::toast_success(window, cx, format!("{} {}", t("图片已保存到"), path));
            }
            SaveOutcome::Cancelled => {}
            SaveOutcome::Failed(message) => {
                self.error = Some(format!("{}: {}", t("保存失败"), message));
                notify::toast_error(window, cx, t("保存图片失败"));
            }
        }
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 渲染子面板
    // ------------------------------------------------------------------

    fn info_panel(&self, cx: &App) -> gpui::Div {
        match &self.meta {
            Some(meta) => ui::info_rows(
                [
                    ui::info_row(t("尺寸"), format!("{} × {}", meta.width, meta.height), cx),
                    ui::info_row(t("格式"), meta.format.clone(), cx),
                    ui::info_row(t("大小"), format_file_size(meta.size), cx),
                ],
                cx,
            ),
            None => div().child(ui::hint_text(t("暂无图片信息"), cx)),
        }
    }

    fn save_panel(&self, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let has_bytes = self.source_bytes.is_some();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(ui::field(
                t("格式"),
                Select::new(&self.save_format_select),
                cx,
            ))
            .when(
                matches!(self.save_format.as_str(), "jpeg" | "webp"),
                |panel| {
                    panel.child(ui::field(
                        t("质量"),
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Slider::new(&self.quality_slider).flex_1())
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(cx.theme().colors.muted_foreground)
                                    .child(format!("{}%", self.quality)),
                            ),
                        cx,
                    ))
                },
            )
            .child(ui::field(t("文件名"), Input::new(&self.filename_input), cx))
            .child(
                ui::primary_button(
                    "ip-save",
                    t("保存图片到指定目录"),
                    cx.listener(|this, _ev, window, cx| this.save_image(window, cx)),
                )
                .disabled(!has_bytes || self.saving)
                .loading(self.saving)
                .w_full(),
            )
            .child(ui::hint_text(
                t("点击后将弹出保存对话框，您可以自由选择保存目录和文件名"),
                cx,
            ))
    }
}

impl gpui::Render for ImagePreview {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let preview_h = px(400.);

        let load_btn = ui::primary_button(
            "ip-load",
            t("加载"),
            cx.listener(|this, _ev, _window, cx| this.load_from_input(cx)),
        )
        .loading(self.loading)
        .disabled(self.loading);
        let upload_btn = ui::secondary_button(
            "ip-upload",
            t("上传文件"),
            cx.listener(|this, _ev, window, cx| this.pick_file(window, cx)),
        )
        .disabled(self.loading);

        // 预览区（页面根可滚动，这里用固定高度保证 img 有界；muted 实色画布，无框）
        let preview_area = div()
            .id("ip-preview-area")
            .relative()
            .h(preview_h)
            .rounded(theme.radius)
            .bg(theme.colors.muted)
            .overflow_hidden()
            .when(self.source.is_some(), |area| area.cursor_pointer())
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _window, cx| this.on_wheel(ev, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| this.on_drag_start(ev, cx)),
            )
            .on_mouse_move(
                cx.listener(|this, ev: &MouseMoveEvent, _window, cx| this.on_drag_move(ev, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _ev: &MouseUpEvent, _window, cx| this.on_drag_end(cx)),
            )
            .on_click(cx.listener(|this, ev: &ClickEvent, _window, cx| {
                if ev.click_count() >= 2 {
                    this.on_double_click(cx);
                }
            }));

        let preview_area = if self.loading {
            preview_area.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ui::loading_row(t("加载中..."), cx)),
            )
        } else if let Some(source) = self.source.clone() {
            // gpui img 渲染：本地文件用 Resource::Path（gpui 内置 loader 读盘），
            // 远程 URL 用 Resource::Uri（gpui 内置 http loader），data: URL 用
            // ImageSource::Image（后台 base64 解码后的内存图片）。
            let image_source: ImageSource = match &source {
                LoadedSource::File(path) => {
                    ImageSource::Resource(Resource::Path(path.clone().into()))
                }
                LoadedSource::Uri(url) => ImageSource::Resource(Resource::Uri(url.clone().into())),
                LoadedSource::Data(image) => ImageSource::Image(image.clone()),
            };

            let mut image_el = img(image_source);
            let has_meta = self.meta.is_some();
            if has_meta && self.scale != 1. {
                // 缩放态：按原始尺寸 × scale 显式宽高（配合容器 overflow_hidden + 平移）
                let meta = self.meta.as_ref().unwrap();
                image_el = image_el
                    .w(px(meta.width as f32 * self.scale))
                    .h(px(meta.height as f32 * self.scale));
            } else {
                // 原始尺寸态：max_w/max_h 容器自适应
                image_el = image_el.max_w_full().max_h(preview_h - px(32.));
            }

            // 浮动按钮（右上角）
            let floating = div()
                .absolute()
                .top(px(8.))
                .right(px(8.))
                .flex()
                .gap_1()
                .child(ui::secondary_button(
                    "ip-reset",
                    t("重置"),
                    cx.listener(|this, _ev, _window, cx| this.reset_view(cx)),
                ))
                .child(ui::secondary_button(
                    "ip-copy-url",
                    t("复制 URL"),
                    cx.listener(|this, _ev, window, cx| this.copy_source_url(window, cx)),
                ));

            // 缩放指示器（左下角，「− N% +」；浮于图片上方，popover 实色底）
            let percent = format!("{}%", (self.scale * 100.).round() as i64);
            let zoom_bar = div()
                .absolute()
                .bottom(px(8.))
                .left(px(8.))
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .py_0p5()
                .rounded(theme.radius)
                .bg(theme.colors.popover)
                .child(ui::secondary_button(
                    "ip-zoom-out",
                    "−",
                    cx.listener(|this, _ev, _window, cx| this.apply_zoom(-0.1, cx)),
                ))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.foreground)
                        .child(percent),
                )
                .child(ui::secondary_button(
                    "ip-zoom-in",
                    "+",
                    cx.listener(|this, _ev, _window, cx| this.apply_zoom(0.1, cx)),
                ));

            preview_area
                .child(
                    // 平移载体：绝对定位偏移整个居中容器（不改变 img 自身布局）
                    div()
                        .absolute()
                        .left(px(self.offset.0))
                        .top(px(self.offset.1))
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(image_el),
                )
                .child(floating)
                .when(has_meta, |area| area.child(zoom_bar))
        } else {
            preview_area.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ui::empty_hint(t("粘贴图片或输入 URL 开始预览"), cx)),
            )
        };

        ui::tool_page("imagepreview")
            .child(
                ui::card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&self.url_input)))
                            .child(load_btn)
                            .child(upload_btn),
                    )
                    .child(ui::hint_text(
                        t("💡 提示：支持 http(s) 链接、data: URL 与本地文件路径；预览区支持滚轮缩放、拖拽平移、双击切换 1×/2×"),
                        cx,
                    ))
                    .when_some(self.error.clone(), |card, error| {
                        card.child(ui::error_banner(&error, cx))
                    }),
            )
            .child(preview_area)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_3()
                    .child(
                        ui::card(cx)
                            .flex_1()
                            .min_w(px(280.))
                            .child(ui::card_header(t("图片信息"), cx))
                            .child(self.info_panel(cx)),
                    )
                    .child(
                        ui::card(cx)
                            .w(px(340.))
                            .child(ui::card_header(t("保存设置"), cx))
                            .child(self.save_panel(cx)),
                    ),
            )
    }
}

// ----------------------------------------------------------------------
// 后台任务
// ----------------------------------------------------------------------

/// 解析输入并解码（后台线程）：data: URL / 本地路径 → 字节 + 元信息；http(s) 交内置 loader。
fn decode_source_job(input: String) -> LoadOutcome {
    if input.starts_with("data:") {
        let Some((header, payload)) = input.split_once(',') else {
            return LoadOutcome::Failed(t("无效的 data: URL 格式：缺少数据段").into());
        };
        if !header.ends_with(";base64") {
            return LoadOutcome::Failed(t("仅支持 base64 编码的 data: URL").into());
        }
        let Some(bytes) = base64_decode(payload) else {
            return LoadOutcome::Failed(t("data: URL base64 解码失败").into());
        };
        let meta = decode_meta(&bytes, None);
        let gpui_format = gpui_format_of(&bytes);
        LoadOutcome::Data {
            image: Arc::new(gpui::Image::from_bytes(gpui_format, bytes.clone())),
            bytes: Arc::new(bytes),
            meta,
        }
    } else if input.starts_with("http://") || input.starts_with("https://") {
        LoadOutcome::Remote { url: input }
    } else {
        match std::fs::read(&input) {
            Ok(bytes) => {
                let meta = decode_meta(&bytes, Some(&input));
                LoadOutcome::Local {
                    path: PathBuf::from(input),
                    bytes: Arc::new(bytes),
                    meta,
                }
            }
            Err(e) => LoadOutcome::Failed(format!("{}: {}", t("读取本地文件失败"), e)),
        }
    }
}

/// 解码元信息（尺寸/格式/大小）。解码失败返回 None（面板显示「未知」，图片仍可由 gpui 尝试渲染）。
fn decode_meta(bytes: &[u8], path: Option<&str>) -> Option<ImageMeta> {
    let img = image::load_from_memory(bytes).ok()?;
    let format = image::guess_format(bytes)
        .ok()
        .map(format_name)
        .or_else(|| {
            path.and_then(|p| Path::new(p).extension())
                .map(|ext| ext.to_string_lossy().to_uppercase())
        })?;
    Some(ImageMeta {
        width: img.width(),
        height: img.height(),
        format,
        size: bytes.len() as u64,
    })
}

fn format_name(format: image::ImageFormat) -> String {
    match format {
        image::ImageFormat::Png => "PNG",
        image::ImageFormat::Jpeg => "JPEG",
        image::ImageFormat::Gif => "GIF",
        image::ImageFormat::WebP => "WEBP",
        image::ImageFormat::Bmp => "BMP",
        image::ImageFormat::Tiff => "TIFF",
        image::ImageFormat::Ico => "ICO",
        image::ImageFormat::Pnm => "PNM",
        _ => "OTHER",
    }
    .to_string()
}

/// image crate 格式 → gpui ImageFormat（gpui Image 按 format 字段选择解码路径）。
fn gpui_format_of(bytes: &[u8]) -> ImageFormat {
    match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => ImageFormat::Jpeg,
        Ok(image::ImageFormat::Gif) => ImageFormat::Gif,
        Ok(image::ImageFormat::WebP) => ImageFormat::Webp,
        Ok(image::ImageFormat::Bmp) => ImageFormat::Bmp,
        Ok(image::ImageFormat::Tiff) => ImageFormat::Tiff,
        Ok(image::ImageFormat::Ico) => ImageFormat::Ico,
        Ok(image::ImageFormat::Pnm) => ImageFormat::Pnm,
        _ => ImageFormat::Png,
    }
}

/// 保存任务：rfd save_file（后台线程）+ image crate 重编码 + 写盘。
fn save_image_job(bytes: Arc<Vec<u8>>, format: &str, quality: u32, filename: &str) -> SaveOutcome {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(t("图片文件"), &[format])
        .set_file_name(format!("{filename}.{format}"))
        .save_file()
    else {
        return SaveOutcome::Cancelled;
    };

    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => return SaveOutcome::Failed(format!("{}: {}", t("解码源图片失败"), e)),
    };

    let mut buf = Cursor::new(Vec::new());
    let encode_result = match format {
        "jpeg" => image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut buf,
            quality.clamp(1, 100) as u8,
        )
        .encode_image(&img)
        .map_err(|e| e.to_string()),
        // gif：无质量参数
        "gif" => image::codecs::gif::GifEncoder::new(&mut buf)
            .write_image(
                &img.to_rgba8(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string()),
        // webp：image crate 仅提供无损编码器，质量参数被忽略
        "webp" => image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
            .write_image(
                &img.to_rgba8(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string()),
        _ => img
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| e.to_string()),
    };
    if let Err(e) = encode_result {
        return SaveOutcome::Failed(format!("{}: {}", t("格式转换失败"), e));
    }

    match std::fs::write(&path, buf.into_inner()) {
        Ok(_) => SaveOutcome::Saved(path.display().to_string()),
        Err(e) => SaveOutcome::Failed(format!("{}: {}", t("写入文件失败"), e)),
    }
}

/// 极简 base64 解码器（标准 + URL-safe 字母表；app 壳未直接依赖 base64 crate）。
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(b: u8) -> Option<u32> {
        match b {
            b'A'..=b'Z' => Some((b - b'A') as u32),
            b'a'..=b'z' => Some((b - b'a') as u32 + 26),
            b'0'..=b'9' => Some((b - b'0') as u32 + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let filtered: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    let mut out = Vec::with_capacity(filtered.len() * 3 / 4 + 3);
    for chunk in filtered.chunks(4) {
        let n: u32 = match chunk {
            [a, b, c, d] => (val(*a)? << 18) | (val(*b)? << 12) | (val(*c)? << 6) | val(*d)?,
            [a, b, c] => (val(*a)? << 18) | (val(*b)? << 12) | (val(*c)? << 6),
            [a, b] => (val(*a)? << 18) | (val(*b)? << 12),
            _ => return None,
        };
        out.push((n >> 16) as u8);
        if chunk.len() >= 3 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() == 4 {
            out.push(n as u8);
        }
    }
    Some(out)
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
