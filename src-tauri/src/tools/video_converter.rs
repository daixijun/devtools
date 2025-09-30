use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConversionRequest {
    pub input_path: String,
    pub output_path: String,
    pub delete_source_file: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConversionResponse {
    pub success: bool,
    pub output_path: String,
    pub message: String,
}

/// 验证输入文件是否存在且是有效的视频文件
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
            "mov", "mp4", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mpeg", "mpg",
        ]
        .contains(&ext.as_str())
        {
            return Err(format!("不支持的文件格式: {}", ext));
        }
    } else {
        return Err("文件没有扩展名，无法确定格式".to_string());
    }

    Ok(path.to_path_buf())
}

/// 生成输出文件路径
fn generate_output_path(input_path: &Path, custom_output_path: &str) -> Result<PathBuf, String> {
    let output_path = if custom_output_path.is_empty() {
        // 如果没有指定输出路径，使用输入文件所在目录，并转换为 MP4
        let mut output = input_path.to_path_buf();
        if let Some(stem) = input_path.file_stem() {
            output.set_file_name(format!("{}_converted.mp4", stem.to_string_lossy()));
        } else {
            output.set_file_name("converted_video.mp4");
        }
        output
    } else {
        // 使用前端提供的输出路径
        PathBuf::from(custom_output_path)
    };

    Ok(output_path)
}

/// 使用 FFmpeg 转换视频
fn convert_video_with_ffmpeg(input_path: &Path, output_path: &Path) -> Result<(), String> {
    // 检查输出目录是否存在，如果不存在则创建
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    // 构建 FFmpeg 命令
    let mut command = Command::new("ffmpeg");

    command
        .arg("-i")
        .arg(input_path)
        .arg("-c:v")
        .arg("libx264") // 视频编码器
        .arg("-c:a")
        .arg("aac") // 音频编码器
        .arg("-preset")
        .arg("medium") // 编码速度与质量平衡
        .arg("-crf")
        .arg("23") // 质量参数（0-51，越小质量越好）
        .arg("-movflags")
        .arg("+faststart") // 优化网络播放
        .arg("-y") // 覆盖输出文件
        .arg(output_path);

    // 执行转换
    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 FFmpeg 进程失败: {}", e))?
        .wait_with_output()
        .map_err(|e| format!("等待 FFmpeg 进程完成失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("视频转换失败: {}", error_msg))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub name: String,
    pub size: String,
    pub format: String,
    pub duration: String,
    pub resolution: String,
    pub path: String,
}

/// 获取视频文件信息（内部函数）
fn extract_video_info(input_path: &Path) -> Result<VideoInfo, String> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(input_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("获取视频信息失败: {}", e))?;

    if !output.status.success() {
        return Err("无法读取视频文件信息".to_string());
    }

    let probe_output = String::from_utf8_lossy(&output.stdout);

    // 解析 JSON 输出
    let probe_data: serde_json::Value =
        serde_json::from_str(&probe_output).map_err(|e| format!("解析视频信息失败: {}", e))?;

    // 获取时长
    let duration = probe_data
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|seconds| format_duration(seconds))
        .unwrap_or_else(|| "未知".to_string());

    // 获取分辨率
    let resolution = probe_data
        .get("streams")
        .and_then(|s| s.as_array())
        .and_then(|streams| {
            streams
                .iter()
                .find(|stream| stream.get("codec_type").and_then(|t| t.as_str()) == Some("video"))
        })
        .and_then(|video_stream| {
            let width = video_stream
                .get("width")
                .and_then(|w| w.as_u64())
                .unwrap_or(0);
            let height = video_stream
                .get("height")
                .and_then(|h| h.as_u64())
                .unwrap_or(0);
            if width > 0 && height > 0 {
                Some(format!("{}x{}", width, height))
            } else {
                None
            }
        })
        .unwrap_or_else(|| "未知".to_string());

    Ok(VideoInfo {
        name: input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: std::fs::metadata(input_path)
            .map(|m| format!("{:.2} MB", m.len() as f64 / 1024.0 / 1024.0))
            .unwrap_or_else(|_| "未知".to_string()),
        format: input_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_uppercase(),
        duration,
        resolution,
        path: input_path.to_string_lossy().to_string(),
    })
}

/// 将秒数格式化为 HH:MM:SS 格式
fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds as i64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Tauri 命令：转换视频文件
#[tauri::command]
pub async fn convert_video(
    request: VideoConversionRequest,
) -> Result<VideoConversionResponse, String> {
    // 检查 FFmpeg 是否安装
    let ffmpeg_check = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match ffmpeg_check {
        Ok(status) if status.success() => {},
        _ => return Err("FFmpeg 未安装。请先安装 FFmpeg：\n\nmacOS: brew install ffmpeg\nWindows: 下载 FFmpeg 并添加到 PATH\nLinux: sudo apt install ffmpeg (Ubuntu/Debian) 或 sudo dnf install ffmpeg (Fedora)".to_string()),
    }

    // 验证输入文件
    let input_path = validate_input_file(&request.input_path)?;

    // 生成输出路径
    let output_path = generate_output_path(&input_path, &request.output_path)?;

    // 执行转换
    convert_video_with_ffmpeg(&input_path, &output_path)?;

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

    let response = VideoConversionResponse {
        success: true,
        output_path: output_path.to_string_lossy().to_string(),
        message: format!("视频转换成功完成！{}", deletion_message),
    };

    Ok(response)
}

/// Tauri 命令：获取视频文件信息（通用命令）
#[tauri::command]
pub async fn get_video_info(input_path: String) -> Result<VideoInfo, String> {
    let path = validate_input_file(&input_path)?;
    let info = extract_video_info(&path)?;
    Ok(info)
}

/// Tauri 命令：检查 FFmpeg 是否可用
#[tauri::command]
pub async fn check_ffmpeg_available() -> Result<bool, String> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match output {
        Ok(status) if status.success() => Ok(true),
        _ => Ok(false),
    }
}
