//! 图片解码安全限制。

use image::DynamicImage;
use std::path::Path;

/// 像素总量上限（RGBA 约 400MB）：几 MB 的特制图片可声明
/// 100000×100000 尺寸，解码即占满内存（解压炸弹）
const MAX_PIXELS: u64 = 100_000_000;
/// 单边尺寸上限
const MAX_DIMENSION: u64 = 30_000;

/// 校验图像尺寸是否在支持范围内。
/// image crate 自带的 512MB 分配上限是解码阶段的兜底，这里把拒绝
/// 提前到解码前，并给出可操作的错误说明
pub fn validate_dimensions(width: u32, height: u32) -> Result<(), String> {
    let (w, h) = (u64::from(width), u64::from(height));
    if w == 0 || h == 0 {
        return Err(format!("图像尺寸无效: {}x{}", width, height));
    }
    if w > MAX_DIMENSION || h > MAX_DIMENSION {
        return Err(format!(
            "图像尺寸超出支持范围（{}x{}），单边最大 {} 像素",
            width, height, MAX_DIMENSION
        ));
    }
    if w * h > MAX_PIXELS {
        return Err(format!(
            "图像像素总量超出支持范围（{}x{}），最大 {} 百万像素",
            width,
            height,
            MAX_PIXELS / 1_000_000
        ));
    }
    Ok(())
}

/// 打开并解码图片，解码前按文件头声明的尺寸做上限预检。
/// 替代直接 `image::open`：后者要到解码阶段才可能因分配上限失败，
/// 且错误信息对用户不可读
pub fn open_image_limited(path: &Path) -> Result<DynamicImage, String> {
    let (width, height) = image::ImageReader::open(path)
        .map_err(|e| format!("无法打开图片文件: {}", e))?
        .with_guessed_format()
        .map_err(|e| format!("无法读取图片文件: {}", e))?
        .into_dimensions()
        .map_err(|e| format!("无法读取图片文件: {}", e))?;
    validate_dimensions(width, height)?;
    // into_dimensions 消耗 reader，预检通过后重新打开解码
    image::ImageReader::open(path)
        .map_err(|e| format!("无法打开图片文件: {}", e))?
        .with_guessed_format()
        .map_err(|e| format!("无法读取图片文件: {}", e))?
        .decode()
        .map_err(|e| format!("无法解码图片文件: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_oversized_dimensions() {
        assert!(validate_dimensions(1, 1).is_ok());
        assert!(validate_dimensions(10000, 10000).is_ok()); // 100MP 边界内
        assert!(validate_dimensions(0, 100).is_err());
        assert!(validate_dimensions(30001, 1).is_err(), "单边超限");
        assert!(validate_dimensions(1, 30001).is_err(), "单边超限");
        assert!(validate_dimensions(20000, 6000).is_err(), "120MP 超总量");
        assert!(
            validate_dimensions(10001, 10000).is_err(),
            "100.01MP 超总量"
        );
    }

    #[test]
    fn open_image_limited_rejects_oversized_but_tiny_file() {
        // 真实编码一张 30001x1 的 PNG：文件极小但单边超限，
        // 必须在解码前被尺寸预检拒绝
        let dir = std::env::temp_dir().join("devtools-image-limit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let huge = dir.join("huge.png");
        image::DynamicImage::new_rgb8(30001, 1).save(&huge).unwrap();
        assert!(
            open_image_limited(&huge).is_err(),
            "单边超限的图片应被拒绝解码"
        );

        let small = dir.join("ok.png");
        image::DynamicImage::new_rgb8(8, 8).save(&small).unwrap();
        assert!(open_image_limited(&small).is_ok(), "正常小图应可解码");

        std::fs::remove_dir_all(&dir).ok();
    }
}
