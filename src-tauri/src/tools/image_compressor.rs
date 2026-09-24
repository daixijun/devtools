use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// 图片压缩请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressRequest {
    pub input_path: String,
    pub output_path: String,
    /// 压缩模式: "lossless" | "quality"
    pub mode: String,
    /// 图片质量 1-100, 仅 quality 模式下作用于 JPEG/WebP
    pub quality: Option<u8>,
    /// 等比缩放最大宽度约束, None 表示不约束
    pub max_width: Option<u32>,
    /// 等比缩放最大高度约束, None 表示不约束
    pub max_height: Option<u32>,
    /// 输出格式: "keep" | "png" | "jpeg" | "webp" | "gif" | "bmp" | "tiff"
    pub output_format: String,
    /// 是否移除 EXIF 等元数据 (重编码天然丢弃, 此项用于显式控制 PNG 元数据剥离)
    pub remove_exif: bool,
    /// PNG 无损优化级别 1-6
    pub png_optimize_level: Option<u8>,
}

/// 图片压缩响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResponse {
    pub success: bool,
    pub output_path: String,
    pub message: String,
    pub original_size: u64,
    pub compressed_size: u64,
    /// 压缩后/压缩前 比例
    pub compression_ratio: f64,
    pub output_width: u32,
    pub output_height: u32,
}

/// 读取图片为 data URL 的响应 (用于前端缩略图与对比预览)
#[derive(Debug, Clone, Serialize)]
pub struct ImageDataUrl {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

/// 支持压缩的输入扩展名
const SUPPORTED_INPUT_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
];

/// 验证输入文件
fn validate_input_file(input_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(input_path);
    if !path.exists() {
        return Err(format!("输入文件不存在: {}", input_path));
    }
    if !path.is_file() {
        return Err(format!("输入路径不是文件: {}", input_path));
    }
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        if !SUPPORTED_INPUT_EXTS.contains(&ext.as_str()) {
            return Err(format!("不支持的图片格式: {}", ext));
        }
    } else {
        return Err("文件没有扩展名，无法确定图片格式".to_string());
    }
    Ok(path.to_path_buf())
}

/// 根据源扩展名推断 ImageFormat
fn format_from_ext(ext: &str) -> Option<ImageFormat> {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "png" => Some(ImageFormat::Png),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        "tiff" | "tif" => Some(ImageFormat::Tiff),
        "webp" => Some(ImageFormat::WebP),
        _ => None,
    }
}

/// 构建 oxipng 优化选项
fn build_oxipng_options(level: u8, strip_metadata: bool) -> oxipng::Options {
    // oxipng 预设级别范围为 0-6, 超出会 panic, 这里做钳制
    let preset = level.clamp(1, 6);
    let mut opts = oxipng::Options::from_preset(preset);
    if strip_metadata {
        // Safe: 移除可安全剥离的元数据 (保留 ICC 颜色配置等关键信息)
        opts.strip = oxipng::StripChunks::Safe;
    }
    opts
}

/// 对 PNG 字节执行 oxipng 无损优化
fn optimize_png_in_memory(png_bytes: &[u8], level: u8, strip_metadata: bool) -> Result<Vec<u8>, String> {
    let opts = build_oxipng_options(level, strip_metadata);
    oxipng::optimize_from_memory(png_bytes, &opts)
        .map_err(|e| format!("PNG 无损优化失败: {}", e))
}

/// 色彩量化为 ≤256 色并写成 8-bit 索引 PNG (pngquant/TinyPNG 式, 视觉无损)
/// quality 越高保留的颜色越多 (画质越好/体积越大), 与滑块直觉一致
fn quantize_to_indexed_png(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    use imagequant::{Attributes, RGBA};

    let rgba_img = img.to_rgba8();
    let width = rgba_img.width() as usize;
    let height = rgba_img.height() as usize;
    if width == 0 || height == 0 {
        return Err("图片尺寸无效, 无法量化".to_string());
    }

    // 构造 imagequant 需要的 RGBA 像素数组
    let pixels: Vec<RGBA> = rgba_img
        .as_raw()
        .chunks_exact(4)
        .map(|c| RGBA {
            r: c[0],
            g: c[1],
            b: c[2],
            a: c[3],
        })
        .collect();

    let mut attr = Attributes::new();
    attr.set_max_colors(256)
        .map_err(|e| format!("量化参数设置失败: {}", e))?;
    // set_quality(min, target): min=0 表示始终允许压缩; target=quality 决定激进程度
    attr.set_quality(0, quality)
        .map_err(|e| format!("量化参数设置失败: {}", e))?;
    // speed: 1=最佳质量(慢), 10=最快; 取折中值兼顾批量速度
    attr.set_speed(4)
        .map_err(|e| format!("量化参数设置失败: {}", e))?;

    let mut image = attr
        .new_image(pixels, width, height, 0.0)
        .map_err(|e| format!("创建量化图像失败: {}", e))?;
    let mut result = attr
        .quantize(&mut image)
        .map_err(|e| format!("色彩量化失败: {}", e))?;
    result
        .set_dithering_level(1.0)
        .map_err(|e| format!("设置抖动级别失败: {}", e))?;
    let (palette, indices) = result
        .remapped(&mut image)
        .map_err(|e| format!("生成索引图失败: {}", e))?;

    if palette.is_empty() {
        return Err("量化结果为空".to_string());
    }

    // 用 png crate 写 8-bit 索引 PNG
    let mut out = Cursor::new(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);

        // 调色板: 每项 3 字节 RGB
        let mut palette_bytes = Vec::with_capacity(palette.len() * 3);
        for c in &palette {
            palette_bytes.extend_from_slice(&[c.r, c.g, c.b]);
        }
        encoder.set_palette(palette_bytes);

        // 若存在半透明/透明色, 写 tRNS chunk (每项 1 字节 alpha)
        let has_alpha = palette.iter().any(|c| c.a < 255);
        if has_alpha {
            let trns: Vec<u8> = palette.iter().map(|c| c.a).collect();
            encoder.set_trns(trns);
        }

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("写入 PNG 头失败: {}", e))?;
        writer
            .write_image_data(&indices)
            .map_err(|e| format!("写入 PNG 数据失败: {}", e))?;
    }

    Ok(out.into_inner())
}

/// 解析目标输出格式
fn resolve_output_format(request: &CompressRequest, input_path: &Path) -> Result<ImageFormat, String> {
    match request.output_format.to_lowercase().as_str() {
        "keep" => {
            let ext = input_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            format_from_ext(&ext).ok_or_else(|| format!("无法保留原格式: {}", ext))
        }
        "png" => Ok(ImageFormat::Png),
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        "webp" => Ok(ImageFormat::WebP),
        "gif" => Ok(ImageFormat::Gif),
        "bmp" => Ok(ImageFormat::Bmp),
        "tiff" | "tif" => Ok(ImageFormat::Tiff),
        other => Err(format!("不支持的输出格式: {}", other)),
    }
}

/// 执行单张图片压缩的核心逻辑
fn compress_image_inner(
    input_path: &Path,
    output_path: &Path,
    request: &CompressRequest,
) -> Result<(u64, u64, u32, u32), String> {
    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);

    // 解码图片
    let mut img = image::open(input_path).map_err(|e| format!("无法加载图片: {}", e))?;

    // 等比缩放: 未指定的维度使用原始尺寸作为约束 (即不约束该方向, 同时避免误放大)
    let bound_w = request.max_width.unwrap_or(img.width());
    let bound_h = request.max_height.unwrap_or(img.height());
    if bound_w != img.width() || bound_h != img.height() {
        img = img.resize(bound_w, bound_h, image::imageops::FilterType::Lanczos3);
    }

    let out_width = img.width();
    let out_height = img.height();

    let output_format = resolve_output_format(request, input_path)?;
    let strip_metadata = request.remove_exif;

    match output_format {
        ImageFormat::Png => {
            // quality 模式: 色彩量化 (类 TinyPNG) → 索引 PNG → oxipng 再优化 (大幅减小体积, 视觉无损)
            // lossless 模式: 仅 oxipng 优化 (真无损, 对已优化的 PNG 效果有限)
            let png_bytes = if request.mode == "quality" {
                let quality = request.quality.unwrap_or(80).clamp(1, 100);
                let quantized = quantize_to_indexed_png(&img, quality)?;
                optimize_png_in_memory(
                    &quantized,
                    request.png_optimize_level.unwrap_or(3),
                    strip_metadata,
                )?
            } else {
                let mut png_buf = Cursor::new(Vec::new());
                img.write_with_encoder(image::codecs::png::PngEncoder::new(&mut png_buf))
                    .map_err(|e| format!("PNG 编码失败: {}", e))?;
                optimize_png_in_memory(
                    &png_buf.into_inner(),
                    request.png_optimize_level.unwrap_or(3),
                    strip_metadata,
                )?
            };
            std::fs::write(output_path, &png_bytes)
                .map_err(|e| format!("写入输出文件失败: {}", e))?;
        }
        ImageFormat::Jpeg => {
            // lossless 模式: 近似无损 (最高质量重编码); quality 模式: 使用指定质量
            let quality = if request.mode == "lossless" {
                100u8
            } else {
                request.quality.unwrap_or(80).clamp(1, 100)
            };
            let mut out_file = std::fs::File::create(output_path)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut out_file,
                quality,
            ))
            .map_err(|e| format!("JPEG 编码失败: {}", e))?;
        }
        ImageFormat::WebP => {
            // image crate 的 WebP 编码器仅支持无损
            let mut out_file = std::fs::File::create(output_path)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out_file);
            img.write_with_encoder(encoder)
                .map_err(|e| format!("WebP 编码失败: {}", e))?;
        }
        ImageFormat::Gif => {
            let mut out_file = std::fs::File::create(output_path)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            img.write_with_encoder(image::codecs::gif::GifEncoder::new(&mut out_file))
                .map_err(|e| format!("GIF 编码失败: {}", e))?;
        }
        ImageFormat::Bmp => {
            let mut out_file = std::fs::File::create(output_path)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            img.write_with_encoder(image::codecs::bmp::BmpEncoder::new(&mut out_file))
                .map_err(|e| format!("BMP 编码失败: {}", e))?;
        }
        ImageFormat::Tiff => {
            let mut out_file = std::fs::File::create(output_path)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            img.write_with_encoder(image::codecs::tiff::TiffEncoder::new(&mut out_file))
                .map_err(|e| format!("TIFF 编码失败: {}", e))?;
        }
        _ => return Err(format!("不支持的输出格式: {:?}", output_format)),
    }

    let compressed_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    Ok((original_size, compressed_size, out_width, out_height))
}

/// Tauri 命令: 压缩单张图片 (前端循环调用以实现批量进度)
#[tauri::command]
pub async fn compress_image(request: CompressRequest) -> Result<CompressResponse, String> {
    let input_path = validate_input_file(&request.input_path)?;
    let output_path = PathBuf::from(&request.output_path);

    let (original_size, compressed_size, out_width, out_height) =
        compress_image_inner(&input_path, &output_path, &request)?;

    let compression_ratio = if original_size > 0 {
        compressed_size as f64 / original_size as f64
    } else {
        0.0
    };

    Ok(CompressResponse {
        success: true,
        output_path: request.output_path.clone(),
        message: "压缩完成".to_string(),
        original_size,
        compressed_size,
        compression_ratio,
        output_width: out_width,
        output_height: out_height,
    })
}

/// 将 (已缩放的) 图片编码为 data URL, 透明图用 PNG, 否则用 JPEG (体积更小)
fn encode_dynamic_image_to_data_url(img: &DynamicImage) -> Result<(String, u32, u32), String> {
    let (width, height) = (img.width(), img.height());
    let has_alpha = matches!(
        img.color(),
        image::ColorType::La8
            | image::ColorType::Rgba8
            | image::ColorType::La16
            | image::ColorType::Rgba16
            | image::ColorType::Rgba32F
    );

    let (mime, bytes) = if has_alpha {
        let mut buf = Cursor::new(Vec::new());
        img.write_with_encoder(image::codecs::png::PngEncoder::new(&mut buf))
            .map_err(|e| format!("PNG 编码失败: {}", e))?;
        ("image/png", buf.into_inner())
    } else {
        let mut buf = Cursor::new(Vec::new());
        img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut buf,
            85,
        ))
        .map_err(|e| format!("JPEG 编码失败: {}", e))?;
        ("image/jpeg", buf.into_inner())
    };

    let b64 = general_purpose::STANDARD.encode(&bytes);
    Ok((format!("data:{};base64,{}", mime, b64), width, height))
}

/// Tauri 命令: 读取任意路径图片并以 data URL 返回 (用于缩略图 / 对比预览)
/// 通过 std::fs 直接读取, 可访问用户经 dialog 选择的任意路径, 不受 JS fs scope 限制
#[tauri::command]
pub async fn read_image_data_url(
    path: String,
    max_dimension: Option<u32>,
) -> Result<ImageDataUrl, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {}", e))?;
    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("解码图片失败: {}", e))?;

    let img = if let Some(maxd) = max_dimension {
        if maxd > 0 && (img.width() > maxd || img.height() > maxd) {
            img.resize(maxd, maxd, image::imageops::FilterType::Lanczos3)
        } else {
            img
        }
    } else {
        img
    };

    let (data_url, width, height) = encode_dynamic_image_to_data_url(&img)?;
    Ok(ImageDataUrl {
        data_url,
        width,
        height,
    })
}
