//! 应用更新检查：查询 GitHub Releases 最新版本并与当前版本比较。
//!
//! 仅提供数据层（HTTP 请求 + 版本比较）；UI（软件更新窗口、菜单动作）在
//! 壳层 `update.rs`。发布链路由 `.github/workflows/release.yml` 保证：tag
//! `v*` 推送 → 构建 DMG → GitHub Release（`releases/latest` 即最新正式版，
//! 不含 prerelease/draft）。

use crate::error::{DevToolError, DevToolResult};

/// GitHub 仓库（owner/name）；Releases API 与下载页均由此派生。
pub const GITHUB_REPO: &str = "daixijun/devtools";

/// 当前构建版本（`Cargo.toml` package version）。
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 最新发布页 URL（GitHub 302 跳转到最新 release）。
pub fn releases_page_url() -> String {
    format!("https://github.com/{GITHUB_REPO}/releases/latest")
}

/// 最新 release 信息。
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// 版本号（tag_name 去掉 `v` 前缀，如 `0.2.0`）
    pub version: String,
    /// 该 release 的页面 URL
    pub url: String,
    /// Release Notes 原文（markdown）
    pub notes: String,
}

/// 查询 GitHub Releases 最新正式发布。
pub async fn fetch_latest_release() -> DevToolResult<ReleaseInfo> {    let client = reqwest::Client::builder()
        .user_agent(format!("DevTools/{}", current_version()))
        .build()
        .map_err(|e| DevToolError::NetworkError(format!("构建 HTTP 客户端失败: {e}")))?;

    let resp = client
        .get(format!(
            "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
        ))
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| DevToolError::NetworkError(format!("请求 GitHub Releases 失败: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // /releases/latest 在没有任何正式发布时返回 404——这是「尚无版本」而非网络故障
        return Err(DevToolError::NotFound(
            "GitHub Releases（仓库还没有已发布的正式版本）".into(),
        ));
    }
    if !resp.status().is_success() {
        return Err(DevToolError::NetworkError(format!(
            "GitHub API 返回异常状态: {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| DevToolError::ParseError("GitHub Releases 响应".into(), e.to_string()))?;

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            DevToolError::ParseError("GitHub Releases 响应".into(), "缺少 tag_name 字段".into())
        })?
        .to_string();

    Ok(ReleaseInfo {
        version: normalize_version(&tag).to_string(),
        url: body
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(releases_page_url),
        notes: body
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// 归一化版本号：去首尾空白与 `v`/`V` 前缀（`v0.2.0` → `0.2.0`）。
pub fn normalize_version(tag: &str) -> &str {
    tag.trim().trim_start_matches(['v', 'V'])
}

/// 解析版本号核心段为数值列表：去掉 prerelease/build 后缀后按 `.` 切分，
/// 非数字段记 0（`v1.0.0-beta.1` → `[1, 0, 0]`）。
fn version_segments(version: &str) -> Vec<u64> {
    let core = normalize_version(version)
        .split(['-', '+'])
        .next()
        .unwrap_or("");
    core.split('.')
        .map(|seg| seg.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

/// 判断 `latest` 是否比 `current` 新（仅比较数值段，缺段补 0；相等或更低、
/// 或任一侧无法解析出有效数值时均视为无更新，不做降级/预发布提示）。
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_segments = version_segments(latest);
    let current_segments = version_segments(current);
    for i in 0..latest_segments.len().max(current_segments.len()) {
        let l = latest_segments.get(i).copied().unwrap_or(0);
        let c = current_segments.get(i).copied().unwrap_or(0);
        if l != c {
            return l > c;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("v0.2.0"), "0.2.0");
        assert_eq!(normalize_version("V1.0.0"), "1.0.0");
        assert_eq!(normalize_version(" 0.1.0 "), "0.1.0");
        assert_eq!(normalize_version("0.1.0"), "0.1.0");
    }

    #[test]
    fn test_is_newer_version() {
        // 常规升级
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("v0.1.1", "0.1.0"));
        // 相等 / 更低 → 无更新
        assert!(!is_newer_version("v0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
        // 数值比较而非字典序（0.10 > 0.9）
        assert!(is_newer_version("0.10.0", "0.9.9"));
        // 缺段补 0（1.0 == 1.0.0）
        assert!(!is_newer_version("1.0", "1.0.0"));
        assert!(is_newer_version("1.1", "1.0.0"));
        // 预发布后缀忽略：1.0.0-beta 与 1.0.0 相等
        assert!(!is_newer_version("1.0.0-beta.1", "1.0.0"));
        // 无法解析 → 不提示
        assert!(!is_newer_version("garbage", "0.1.0"));
    }

    #[test]
    fn test_current_version_matches_cargo_toml() {
        // 与包版本联动：升级版本时此处无需改动（env! 编译期取值）
        assert!(!current_version().is_empty());
    }
}
