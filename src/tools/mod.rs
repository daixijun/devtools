//! 工具业务逻辑。
//!
//! 每个工具的核心计算在此以纯函数形式提供，GPUI 壳（src/pages/）直接调用。

pub mod cert_chain_utils;
pub mod certificate_converter;
pub mod certificate_viewer;
pub mod csr_generator;
pub mod csr_parser;
pub mod dns_resolver;
pub mod image_compressor;
pub mod image_converter;
pub mod ip_info;
pub mod json_to_go;
pub mod regex_tester;
pub mod rsa_key_generator;
pub mod sql_to_ent;
pub mod sql_to_go;
pub mod ssl_checker;
pub mod video_converter;
pub mod whois;

use std::path::{Path, PathBuf};

/// 媒体类工具（图片转换/压缩、视频转换）的输出路径守卫，防止转换破坏既有数据：
/// - 输出与输入为同一文件 → 拒绝（同格式默认输出即此场景；批量「删除源文件」
///   会把唯一副本连同刚写出的结果一起删掉）；
/// - 输出已存在 → 拒绝（避免静默覆盖同名产物或用户的其他文件）。
///
/// 大小写不敏感文件系统（macOS APFS / Windows NTFS）按不敏感比较，
/// 避免 `A.JPG` → `A.jpg` 绕过同文件判定。
pub fn ensure_output_safe(input_path: &str, output_path: &str) -> Result<(), String> {
    let input = Path::new(input_path);
    let output = Path::new(output_path);

    // 归一化：存在的路径直接 canonicalize；尚不存在的输出用「已存在的父目录 + 文件名」拼出
    let normalize = |p: &Path| -> Option<PathBuf> {
        if let Ok(c) = p.canonicalize() {
            return Some(c);
        }
        let parent = p.parent()?.canonicalize().ok()?;
        Some(parent.join(p.file_name()?))
    };
    if let (Some(a), Some(b)) = (normalize(input), normalize(output)) {
        let case_insensitive = cfg!(target_os = "macos") || cfg!(target_os = "windows");
        let equal = if case_insensitive {
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        } else {
            a == b
        };
        if equal {
            return Err(format!(
                "输出路径与输入文件相同（{}），已取消以保护源文件；请更换目标格式或输出目录",
                input_path
            ));
        }
    }
    if output.exists() {
        return Err(format!(
            "输出文件已存在（{output_path}），已取消以避免覆盖；请更换目标格式或输出目录"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_guard_rejects_same_file_via_symlink_like_normalization() {
        let dir = std::env::temp_dir().join("devtools-output-guard-test");
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("src.jpg");
        std::fs::write(&input, b"x").unwrap();
        // 相对路径 / 带点号路径归一化后与输入相同
        let messy = dir.join("./sub/../src.jpg");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let err = ensure_output_safe(input.to_str().unwrap(), messy.to_str().unwrap()).unwrap_err();
        assert!(err.contains("输出路径与输入文件相同"));

        // 已存在的其他输出文件拒绝
        let existing = dir.join("other.jpg");
        std::fs::write(&existing, b"y").unwrap();
        let err =
            ensure_output_safe(input.to_str().unwrap(), existing.to_str().unwrap()).unwrap_err();
        assert!(err.contains("输出文件已存在"));

        // 不存在的输出路径放行
        let fresh = dir.join("fresh.png");
        ensure_output_safe(input.to_str().unwrap(), fresh.to_str().unwrap()).unwrap();

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn output_guard_treats_case_insensitive_same_name_as_same_file_on_applicable_platforms() {
        let dir = std::env::temp_dir().join("devtools-output-guard-case-test");
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("Photo.JPG");
        std::fs::write(&input, b"x").unwrap();
        let output = dir.join("photo.jpg");
        let result = ensure_output_safe(input.to_str().unwrap(), output.to_str().unwrap());
        if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
            assert!(result.is_err());
        }
        // 大小写敏感平台（Linux）上两条路径不同，仅受 exists 约束：不存在则放行
        #[cfg(target_os = "linux")]
        result.unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}
