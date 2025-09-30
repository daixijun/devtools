use image::{self, DynamicImage, ImageFormat};
use libheif_sys::*;
use nom_exif::{EntryValue, ExifIter, MediaParser, MediaSource};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConversionRequest {
    pub input_path: String,
    pub output_path: String,
    pub target_format: String,
    pub quality: Option<u8>,              // 1-100, 图片质量
    pub width: Option<u32>,               // 目标宽度，None表示保持原比例
    pub height: Option<u32>,              // 目标高度，None表示保持原比例
    pub remove_exif: bool,                // 是否移除EXIF信息
    pub delete_source_file: Option<bool>, // 是否删除源文件
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConversionResponse {
    pub success: bool,
    pub output_path: String,
    pub message: String,
    pub original_size: Option<u64>,     // 原始文件大小
    pub converted_size: Option<u64>,    // 转换后文件大小
    pub compression_ratio: Option<f64>, // 压缩比例
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    pub size: String,
    pub format: String,
    pub dimensions: String, // 宽x高，如 "1920x1080"
    pub has_exif: bool,
    pub path: String,
    pub exif_data: Option<ExifData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub make: Option<String>,
    pub model: Option<String>,
    pub datetime: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub software: Option<String>,
}

/// 检查系统支持的图片格式（image 库）
fn check_image_library_support() -> Result<(), String> {
    // 尝试加载一个测试图片来验证 image 库
    // 这里我们不需要做太多检查，因为 image 库会在编译时确定支持
    Ok(())
}

/// 验证输入文件是否存在且是有效的图片文件
fn validate_input_file(input_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(input_path);

    if !path.exists() {
        return Err(format!("输入文件不存在: {}", input_path));
    }

    if !path.is_file() {
        return Err(format!("输入路径不是文件: {}", input_path));
    }

    // 检查文件扩展名
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        if ![
            "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "ico", "heic", "heif",
        ]
        .contains(&ext.as_str())
        {
            return Err(format!("不支持的图片格式: {}", ext));
        }
    } else {
        return Err("文件没有扩展名，无法确定图片格式".to_string());
    }

    Ok(path.to_path_buf())
}

/// 使用 libheif 读取 HEIC 图片
fn read_heic_image(input_path: &Path) -> Result<DynamicImage, String> {
    // 读取文件数据
    let file_data = std::fs::read(input_path).map_err(|e| format!("读取 HEIC 文件失败: {}", e))?;

    // 初始化 libheif 上下文
    let ctx = unsafe { heif_context_alloc() };
    if ctx.is_null() {
        return Err("无法分配 HEIF 上下文".to_string());
    }

    // 确保上下文在函数结束时被释放
    let _ctx_guard = ContextGuard(ctx);

    // 读取 HEIC 数据到上下文
    let result = unsafe {
        heif_context_read_from_memory_without_copy(
            ctx,
            file_data.as_ptr() as *const _,
            file_data.len(),
            std::ptr::null_mut(),
        )
    };

    if result.code != heif_error_code_heif_error_Ok {
        return Err(format!("解析 HEIC 数据失败: {}", error_message(&result)));
    }

    // 获取主图像
    let mut handle = std::ptr::null_mut();
    let result = unsafe { heif_context_get_primary_image_handle(ctx, &mut handle) };

    if result.code != heif_error_code_heif_error_Ok {
        return Err(format!("获取主图像失败: {}", error_message(&result)));
    }

    // 确保句柄在函数结束时被释放
    let _handle_guard = ImageHandleGuard(handle);

    // 获取图像信息
    let width = unsafe { heif_image_handle_get_width(handle) };
    let height = unsafe { heif_image_handle_get_height(handle) };

    // 解码图像
    let mut img = std::ptr::null_mut();
    let result = unsafe {
        heif_decode_image(
            handle,
            &mut img,
            heif_colorspace_heif_colorspace_RGB,
            heif_chroma_heif_chroma_interleaved_RGBA,
            std::ptr::null_mut(),
        )
    };

    if result.code != heif_error_code_heif_error_Ok {
        return Err(format!("解码图像失败: {}", error_message(&result)));
    }

    // 确保图像在函数结束时被释放
    let _img_guard = ImageGuard(img);

    // 获取图像数据
    let mut stride = 0;
    let data_ptr =
        unsafe { heif_image_get_plane(img, heif_channel_heif_channel_interleaved, &mut stride) };

    if data_ptr.is_null() {
        return Err("无法获取图像数据".to_string());
    }

    // 创建 image buffer
    let img_buffer = unsafe {
        image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
            width as u32,
            height as u32,
            std::slice::from_raw_parts(data_ptr as *const u8, (stride * height) as usize).to_vec(),
        )
    };

    let img_buffer = img_buffer.ok_or_else(|| "无法创建图像缓冲区".to_string())?;

    // 转换为 DynamicImage
    Ok(DynamicImage::ImageRgba8(img_buffer))
}

/// 上下文守卫，确保正确释放
struct ContextGuard(*mut heif_context);
impl Drop for ContextGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                heif_context_free(self.0);
            }
        }
    }
}

/// 图像句柄守卫，确保正确释放
struct ImageHandleGuard(*mut heif_image_handle);
impl Drop for ImageHandleGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                heif_image_handle_release(self.0);
            }
        }
    }
}

/// 图像守卫，确保正确释放
struct ImageGuard(*mut heif_image);
impl Drop for ImageGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                heif_image_release(self.0);
            }
        }
    }
}

/// 获取错误消息
fn error_message(error: &heif_error) -> String {
    unsafe {
        if !error.message.is_null() {
            std::ffi::CStr::from_ptr(error.message)
                .to_string_lossy()
                .into_owned()
        } else {
            format!("Unknown error (code: {})", error.code)
        }
    }
}

/// 使用 nom-exif 提取详细的 EXIF 信息
fn extract_exif_data(input_path: &Path) -> Result<Option<ExifData>, String> {
    match MediaSource::file_path(input_path) {
        Ok(ms) => {
            if ms.has_exif() {
                let mut parser = MediaParser::new();
                match parser.parse::<File, _, ExifIter>(ms) {
                    Ok(exif_iter) => {
                        let mut exif_data = ExifData {
                            make: None,
                            model: None,
                            datetime: None,
                            exposure_time: None,
                            f_number: None,
                            iso: None,
                            focal_length: None,
                            software: None,
                        };

                        // 提取常见的 EXIF 字段
                        for mut field in exif_iter {
                            let tag_code = field.tag_code();
                            match tag_code {
                                // 相机制造商
                                0x010F => {
                                    if let Ok(make) = field.take_result() {
                                        if let Some(make_str) = make.as_str() {
                                            exif_data.make = Some(make_str.to_string());
                                        }
                                    }
                                }
                                // 相机型号
                                0x0110 => {
                                    if let Ok(model) = field.take_result() {
                                        if let Some(model_str) = model.as_str() {
                                            exif_data.model = Some(model_str.to_string());
                                        }
                                    }
                                }
                                // 拍摄时间
                                0x9003 => {
                                    if let Ok(datetime) = field.take_result() {
                                        if let Some(datetime_str) = datetime.as_str() {
                                            exif_data.datetime = Some(datetime_str.to_string());
                                        }
                                    }
                                }
                                // 曝光时间
                                0x829A => {
                                    if let Ok(value) = field.take_result() {
                                        exif_data.exposure_time = format_exif_value(&value);
                                    }
                                }
                                // 光圈值
                                0x829D => {
                                    if let Ok(value) = field.take_result() {
                                        exif_data.f_number = format_exif_value(&value);
                                    }
                                }
                                // ISO 感光度
                                0x8827 => {
                                    if let Ok(value) = field.take_result() {
                                        exif_data.iso = format_exif_value(&value);
                                    }
                                }
                                // 焦距
                                0x920A => {
                                    if let Ok(value) = field.take_result() {
                                        exif_data.focal_length = format_exif_value(&value);
                                    }
                                }
                                // 处理软件
                                0x0131 => {
                                    if let Ok(software) = field.take_result() {
                                        if let Some(software_str) = software.as_str() {
                                            exif_data.software = Some(software_str.to_string());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        Ok(Some(exif_data))
                    }
                    Err(_) => Ok(None), // 解析失败
                }
            } else {
                Ok(None) // 没有 EXIF 数据
            }
        }
        Err(_) => Ok(None), // 无法打开文件
    }
}

/// 格式化 EXIF 值为字符串
fn format_exif_value(value: &EntryValue) -> Option<String> {
    match value {
        EntryValue::URational(val) => Some(format!("{}/{}", val.0, val.1)),
        EntryValue::U16(val) => Some(val.to_string()),
        EntryValue::U32(val) => Some(val.to_string()),
        EntryValue::Text(s) => Some(s.to_string()),
        EntryValue::Undefined(data) => Some(format!("{:?}", data)),
        EntryValue::URationalArray(rats) => {
            let formatted: Vec<String> = rats.iter().map(|r| format!("{}/{}", r.0, r.1)).collect();
            Some(formatted.join(", "))
        }
        EntryValue::U16Array(shorts) => {
            let formatted: Vec<String> = shorts.iter().map(|s| s.to_string()).collect();
            Some(formatted.join(", "))
        }
        EntryValue::U32Array(longs) => {
            let formatted: Vec<String> = longs.iter().map(|l| l.to_string()).collect();
            Some(formatted.join(", "))
        }
        _ => None,
    }
}

/// 获取图片信息
fn get_image_info(input_path: &Path) -> Result<ImageInfo, String> {
    // 检查是否为 HEIC 格式
    if let Some(extension) = input_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        if ext == "heic" || ext == "heif" {
            return get_heic_image_info(input_path);
        }
    }

    // 使用 image 库获取图片信息
    let img = match image::open(input_path) {
        Ok(img) => img,
        Err(e) => return Err(format!("无法读取图片文件: {}", e)),
    };

    let dimensions = format!("{}x{}", img.width(), img.height());

    // 提取 EXIF 信息
    let exif_data = extract_exif_data(input_path)?;
    let has_exif = exif_data.is_some();

    // 确定格式
    let format = match img.color() {
        image::ColorType::L8 => "GRAY",
        image::ColorType::La8 => "GRAYA",
        image::ColorType::Rgb8 => "RGB",
        image::ColorType::Rgba8 => "RGBA",
        image::ColorType::L16 => "GRAY16",
        image::ColorType::La16 => "GRAYA16",
        image::ColorType::Rgb16 => "RGB16",
        image::ColorType::Rgba16 => "RGBA16",
        image::ColorType::Rgb32F => "RGB32F",
        image::ColorType::Rgba32F => "RGBA32F",
        _ => "UNKNOWN",
    };

    Ok(ImageInfo {
        name: input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: std::fs::metadata(input_path)
            .map(|m| format!("{:.2} KB", m.len() as f64 / 1024.0))
            .unwrap_or_else(|_| "未知".to_string()),
        format: format.to_uppercase(),
        dimensions,
        has_exif,
        path: input_path.to_string_lossy().to_string(),
        exif_data,
    })
}

/// 获取 HEIC 图片信息
fn get_heic_image_info(input_path: &Path) -> Result<ImageInfo, String> {
    // 读取文件数据
    let file_data = std::fs::read(input_path).map_err(|e| format!("读取 HEIC 文件失败: {}", e))?;

    // 初始化 libheif 上下文
    let ctx = unsafe { heif_context_alloc() };
    if ctx.is_null() {
        return Err("无法分配 HEIF 上下文".to_string());
    }

    let _ctx_guard = ContextGuard(ctx);

    // 读取 HEIC 数据到上下文
    let result = unsafe {
        heif_context_read_from_memory_without_copy(
            ctx,
            file_data.as_ptr() as *const _,
            file_data.len(),
            std::ptr::null_mut(),
        )
    };

    if result.code != heif_error_code_heif_error_Ok {
        return Err(format!("解析 HEIC 数据失败: {}", error_message(&result)));
    }

    // 获取主图像
    let mut handle = std::ptr::null_mut();
    let result = unsafe { heif_context_get_primary_image_handle(ctx, &mut handle) };

    if result.code != heif_error_code_heif_error_Ok {
        return Err(format!("获取主图像失败: {}", error_message(&result)));
    }

    let _handle_guard = ImageHandleGuard(handle);

    // 获取图像信息
    let width = unsafe { heif_image_handle_get_width(handle) };
    let height = unsafe { heif_image_handle_get_height(handle) };

    // 提取 EXIF 信息
    let exif_data = extract_exif_data(input_path)?;
    let has_exif = exif_data.is_some();

    Ok(ImageInfo {
        name: input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: std::fs::metadata(input_path)
            .map(|m| format!("{:.2} KB", m.len() as f64 / 1024.0))
            .unwrap_or_else(|_| "未知".to_string()),
        format: "HEIC".to_string(),
        dimensions: format!("{}x{}", width, height),
        has_exif,
        path: input_path.to_string_lossy().to_string(),
        exif_data,
    })
}

/// 使用 Rust image 库转换图片
fn convert_image_with_image_library(
    input_path: &Path,
    output_path: &Path,
    request: &ImageConversionRequest,
) -> Result<(u64, u64), String> {
    // 检查输出目录是否存在，如果不存在则创建
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    // 获取原始文件大小
    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);

    // 加载图片
    let mut img = if let Some(extension) = input_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        if ext == "heic" || ext == "heif" {
            read_heic_image(input_path)?
        } else {
            image::open(input_path).map_err(|e| format!("无法加载图片: {}", e))?
        }
    } else {
        return Err("无法确定文件格式".to_string());
    };

    // 调整尺寸
    if let Some(width) = request.width {
        if let Some(height) = request.height {
            img = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        } else {
            // 保持宽高比
            let ratio = width as f32 / img.width() as f32;
            let new_height = (img.height() as f32 * ratio) as u32;
            img = img.resize(width, new_height, image::imageops::FilterType::Lanczos3);
        }
    } else if let Some(height) = request.height {
        // 保持宽高比
        let ratio = height as f32 / img.height() as f32;
        let new_width = (img.width() as f32 * ratio) as u32;
        img = img.resize(new_width, height, image::imageops::FilterType::Lanczos3);
    }

    // 确定输出格式
    let output_format = match request.target_format.to_lowercase().as_str() {
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "png" => ImageFormat::Png,
        "gif" => ImageFormat::Gif,
        "bmp" => ImageFormat::Bmp,
        "tiff" => ImageFormat::Tiff,
        "webp" => ImageFormat::WebP,
        "ico" => ImageFormat::Ico,
        _ => return Err(format!("不支持的目标格式: {}", request.target_format)),
    };

    // 创建输出文件
    let output_file = File::create(output_path).map_err(|e| format!("创建输出文件失败: {}", e))?;

    // 根据格式保存图片
    match output_format {
        ImageFormat::Jpeg => {
            let quality = request.quality.unwrap_or(85);
            img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut output_file.try_clone().unwrap(),
                quality,
            ))
            .map_err(|e| format!("保存JPEG失败: {}", e))?;
        }
        ImageFormat::Png => {
            img.write_with_encoder(image::codecs::png::PngEncoder::new(
                &mut output_file.try_clone().unwrap(),
            ))
            .map_err(|e| format!("保存PNG失败: {}", e))?;
        }
        ImageFormat::Gif => {
            img.write_with_encoder(image::codecs::gif::GifEncoder::new(
                &mut output_file.try_clone().unwrap(),
            ))
            .map_err(|e| format!("保存GIF失败: {}", e))?;
        }
        ImageFormat::Bmp => {
            img.write_with_encoder(image::codecs::bmp::BmpEncoder::new(
                &mut output_file.try_clone().unwrap(),
            ))
            .map_err(|e| format!("保存BMP失败: {}", e))?;
        }
        ImageFormat::Tiff => {
            img.write_with_encoder(image::codecs::tiff::TiffEncoder::new(
                &mut output_file.try_clone().unwrap(),
            ))
            .map_err(|e| format!("保存TIFF失败: {}", e))?;
        }
        ImageFormat::WebP => {
            // 使用 WebPEncoder 进行编码
            let mut output_clone = output_file.try_clone().unwrap();
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut output_clone);
            img.write_with_encoder(encoder)
                .map_err(|e| format!("保存WebP失败: {}", e))?;
        }
        ImageFormat::Ico => {
            img.write_with_encoder(image::codecs::ico::IcoEncoder::new(
                &mut output_file.try_clone().unwrap(),
            ))
            .map_err(|e| format!("保存ICO失败: {}", e))?;
        }
        _ => {
            return Err(format!("不支持的输出格式: {:?}", output_format));
        }
    }

    // 获取转换后的文件大小
    let converted_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    Ok((original_size, converted_size))
}

/// Tauri 命令：转换图片文件
#[tauri::command]
pub async fn convert_image(
    request: ImageConversionRequest,
) -> Result<ImageConversionResponse, String> {
    // 检查 image 库支持
    check_image_library_support()?;

    // 验证输入文件
    let input_path = validate_input_file(&request.input_path)?;

    // 验证输出格式
    let supported_formats = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "ico"];
    if !supported_formats.contains(&request.target_format.to_lowercase().as_str()) {
        return Err(format!("不支持的目标格式: {}", request.target_format));
    }

    // 执行转换
    let (original_size, converted_size) = convert_image_with_image_library(
        &input_path,
        &PathBuf::from(&request.output_path),
        &request,
    )?;

    // 计算压缩比例
    let compression_ratio = if original_size > 0 {
        Some(converted_size as f64 / original_size as f64)
    } else {
        None
    };

    // 删除源文件（如果用户选择删除）
    let mut deletion_message = String::new();
    if request.delete_source_file.unwrap_or(false) {
        match std::fs::remove_file(&input_path) {
            Ok(_) => {
                deletion_message = "，源文件已删除".to_string();
            }
            Err(e) => {
                eprintln!("删除源文件失败: {}", e);
                deletion_message = format!("，但删除源文件失败: {}", e);
            }
        }
    }

    let response = ImageConversionResponse {
        success: true,
        output_path: request.output_path.clone(),
        message: format!("图片转换成功完成！{}", deletion_message),
        original_size: Some(original_size),
        converted_size: Some(converted_size),
        compression_ratio,
    };

    Ok(response)
}

/// Tauri 命令：获取图片文件信息
#[tauri::command]
pub async fn get_image_info_command(input_path: String) -> Result<ImageInfo, String> {
    let path = validate_input_file(&input_path)?;
    let info = get_image_info(&path)?;
    Ok(info)
}

/// Tauri 命令：获取图片详细EXIF信息
#[tauri::command]
pub async fn get_image_exif_data(input_path: String) -> Result<Option<ExifData>, String> {
    let path = validate_input_file(&input_path)?;
    let exif_data = extract_exif_data(&path)?;
    Ok(exif_data)
}
