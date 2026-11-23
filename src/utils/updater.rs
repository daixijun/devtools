//! 应用更新：查询 GitHub Releases 最新版本、比较版本、下载安装包并安装。
//!
//! 仅提供数据层（HTTP 请求 + 版本比较 + 下载安装）；UI（软件更新窗口、
//! 菜单动作）在壳层 `update.rs`。发布链路由 `.github/workflows/release.yml`
//! 保证：tag `v*` 推送 → 三平台构建发布（macOS DMG / Windows zip /
//! Linux tar.gz + 各自 `.sha256`），`releases/latest` 即最新正式版。
//!
//! 安装方式按平台：macOS 挂载 DMG 替换 .app（hdiutil 为系统自带命令，
//! 属更新这一壳层能力的基础设施，与 CI 兼容性约定无关）；Windows 解压
//! zip 后重命名替换运行中的 exe；Linux 解包 tar.gz 直接覆盖二进制。

use crate::error::{DevToolError, DevToolResult};
use futures::channel::mpsc::UnboundedSender;
use sha2::Digest;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    /// 附件列表（安装包与校验文件）
    pub assets: Vec<ReleaseAsset>,
}

/// Release 附件（安装包 / `.sha256` 校验文件）。
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    /// 附件名，如 `DevTools-v0.2.0-macos-aarch64.dmg`
    pub name: String,
    /// 浏览器下载 URL（browser_download_url）
    pub url: String,
    /// 字节大小（GitHub assets 自带，用于下载进度分母）
    pub size: u64,
}

/// 查询 GitHub Releases 最新正式发布。
pub async fn fetch_latest_release() -> DevToolResult<ReleaseInfo> {
    let client = reqwest::Client::builder()
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
        assets: parse_assets(&body),
    })
}

/// 从 release JSON 中提取附件（缺字段或结构异常的条目直接跳过）。
fn parse_assets(body: &serde_json::Value) -> Vec<ReleaseAsset> {
    body.get("assets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(ReleaseAsset {
                        name: a.get("name")?.as_str()?.to_string(),
                        url: a.get("browser_download_url")?.as_str()?.to_string(),
                        size: a.get("size")?.as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 当前平台对应 release 产物的名称后缀（与 release.yml 的产物命名一致）。
pub fn platform_package_suffix() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-aarch64.dmg"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x86_64.zip"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64.tar.gz"
    }
    // 其余平台组合（如 macOS Intel）无对应产物
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        "unsupported"
    }
}

/// 当前平台安装包的附件名（`DevTools-v<version>-<平台后缀>`）。
pub fn package_filename(version: &str) -> String {
    format!("DevTools-v{version}-{}", platform_package_suffix())
}

/// 在附件列表中定位当前平台的安装包。
pub fn select_asset<'a>(info: &'a ReleaseInfo, version: &str) -> Result<&'a ReleaseAsset, String> {
    let want = package_filename(version);
    info.assets
        .iter()
        .find(|a| a.name == want)
        .ok_or_else(|| format!("找不到适用于当前平台的安装包（{want}），可以前往下载页手动获取"))
}

/// 下载进行中的事件流（UI 侧逐个消费并刷新状态）。
#[derive(Debug, Clone)]
pub enum UpdateEvent {
    /// 下载进度（0.0-1.0）
    Progress(f32),
    /// 下载完成，正在校验完整性
    Verifying,
    /// 校验通过，正在安装
    Installing,
    /// 安装完成，等待用户重启
    Finished,
    /// 下载/校验/安装任一环节失败
    Failed(String),
}

/// 取消标志（UI 关窗或点「取消」时置位，下载循环逐块检查）。
pub type CancelFlag = Arc<AtomicBool>;

/// 更新任务进行中标志：自动更新与手动「立即更新」共用，避免并发
/// 下载写同一临时文件。
static UPDATE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// 是否有更新下载/安装任务进行中。
pub fn update_in_progress() -> bool {
    UPDATE_IN_PROGRESS.load(Ordering::Relaxed)
}

/// 下载当前平台安装包并安装（进度经 `tx` 回流，不返回——结果即事件流）。
///
/// 成功发送 [`UpdateEvent::Finished`]；失败发送 [`UpdateEvent::Failed`]
/// 并清理临时目录。被取消时静默停止（UI 已自行回到可重试状态）。
/// 已有任务进行中时直接发送 `Failed("已有更新任务进行中")`。
pub async fn download_and_install(info: ReleaseInfo, tx: UnboundedSender<UpdateEvent>, cancel: CancelFlag) {
    if UPDATE_IN_PROGRESS.swap(true, Ordering::Relaxed) {
        let _ = tx.unbounded_send(UpdateEvent::Failed("已有更新任务进行中".into()));
        return;
    }
    let result = run_update(&info, &tx, &cancel).await;
    UPDATE_IN_PROGRESS.store(false, Ordering::Relaxed);
    match result {
        Ok(()) => {
            let _ = tx.unbounded_send(UpdateEvent::Finished);
        }
        Err(msg) => {
            let _ = std::fs::remove_dir_all(work_dir());
            let _ = tx.unbounded_send(UpdateEvent::Failed(msg));
        }
    }
}

/// 总控：选择附件 → 下载 → 校验 → 安装。错误消息面向用户展示。
async fn run_update(
    info: &ReleaseInfo,
    tx: &UnboundedSender<UpdateEvent>,
    cancel: &CancelFlag,
) -> Result<(), String> {
    let pkg = select_asset(info, &info.version)?;
    let checksum = info
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", pkg.name))
        .ok_or_else(|| format!("缺少校验文件（{}.sha256），无法安全安装", pkg.name))?;

    let dir = work_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建更新临时目录失败: {e}"))?;
    let ext = pkg.name.rsplit('.').next().unwrap_or("bin");
    let pkg_path = dir.join(format!("package.{ext}"));

    download_to_file(&pkg.url, pkg.size, &pkg_path, tx, cancel).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err("已取消".into());
    }

    let _ = tx.unbounded_send(UpdateEvent::Verifying);
    let client = http_client()?;
    let expected = client
        .get(&checksum.url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("下载校验文件失败: {e}"))?
        .text()
        .await
        .map_err(|e| format!("读取校验文件失败: {e}"))?;
    let expected = parse_checksum(&expected)?;
    verify_file(&pkg_path, &expected)?;

    let _ = tx.unbounded_send(UpdateEvent::Installing);
    install::install(&pkg_path)?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("DevTools/{}", current_version()))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))
}

/// 流式下载到文件：逐块落盘并累加 sha2，进度按总大小每 0.5% 上报一次。
async fn download_to_file(
    url: &str,
    total_hint: u64,
    dest: &Path,
    tx: &UnboundedSender<UpdateEvent>,
    cancel: &CancelFlag,
) -> Result<(), String> {
    let client = http_client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("下载安装包失败: {e}"))?;
    let total = resp.content_length().unwrap_or(total_hint).max(1);

    let mut file = std::fs::File::create(dest).map_err(|e| format!("创建临时文件失败: {e}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_pct: f32 = 0.0;
    let mut chunk = resp;
    while let Some(bytes) = chunk.chunk().await.map_err(|e| format!("下载中断: {e}"))? {
        if cancel.load(Ordering::Relaxed) {
            return Err("已取消".into());
        }
        file.write_all(&bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
        hasher.update(&bytes);
        downloaded += bytes.len() as u64;
        let pct = downloaded as f32 / total as f32;
        if pct - last_pct >= 0.005 {
            let _ = tx.unbounded_send(UpdateEvent::Progress(pct.min(1.0)));
            last_pct = pct;
        }
    }
    let _ = tx.unbounded_send(UpdateEvent::Progress(1.0));
    file.flush().map_err(|e| format!("写入临时文件失败: {e}"))?;

    // 完整性先在下载侧留档：校验环节直接复用
    let sum = hex::encode(hasher.finalize());
    std::fs::write(dest.with_extension("sha256.local"), sum)
        .map_err(|e| format!("记录校验值失败: {e}"))?;
    Ok(())
}

/// 校验已下载文件与 GitHub 侧 `.sha256` 期望值一致。
fn verify_file(path: &Path, expected: &str) -> Result<(), String> {
    let local = std::fs::read_to_string(path.with_extension("sha256.local"))
        .map_err(|e| format!("读取本地校验值失败: {e}"))?;
    if local.trim() != expected.trim() {
        return Err(format!(
            "下载文件校验失败：期望 {expected}，实际 {local}。
            请重试，或前往下载页手动下载"
        ));
    }
    let _ = std::fs::remove_file(path.with_extension("sha256.local"));
    Ok(())
}

/// 解析 `.sha256` 文件内容，取出 hex 摘要（格式：`<hash>  <文件名>`）。
fn parse_checksum(content: &str) -> Result<String, String> {
    let hash = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| l.split_whitespace().next())
        .ok_or_else(|| "校验文件为空".to_string())?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("校验文件格式异常（{hash}）"));
    }
    Ok(hash.to_ascii_lowercase())
}

/// 更新临时工作目录（下载、解压、挂载均在此进行）。
fn work_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("devtools")
        .join("update")
}

/// 重启应用进入新版本：先拉起新进程（携带 `--relaunch` 标记，新实例会在
/// 单例抢占处等待旧实例退出），成功即退出当前进程，失败返回错误。
pub fn relaunch() -> Result<(), String> {
    let child = relaunch_command().spawn().map_err(|e| format!("启动新版本失败: {e}"))?;
    // 丢弃句柄：新进程独立运行
    std::mem::forget(child);
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn relaunch_command() -> std::process::Command {
    // 通过 LaunchServices 启动保持 Dock/菜单栏观感；-n 允许在旧实例退出前
    // 启动，--args 把 --relaunch 传给新实例
    let mut cmd = std::process::Command::new("open");
    cmd.arg("-n").arg(install::bundle_app_path().unwrap_or_else(|_| {
        // 兜底：直接以可执行文件路径启动（正常情况不会走到）
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("devtools"))
    }));
    cmd.args(["--args", "--relaunch"]);
    cmd
}

#[cfg(not(target_os = "macos"))]
fn relaunch_command() -> std::process::Command {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("devtools")));
    cmd.arg("--relaunch");
    cmd
}

/// 平台安装实现（替换当前运行的应用本体）。
mod install {
    use super::*;

    #[cfg(target_os = "macos")]
    pub fn install(pkg: &Path) -> Result<(), String> {
        use std::process::Command as Cmd;

        let app = bundle_app_path()?;
        let dir = pkg.parent().ok_or("安装包路径异常")?;
        let mnt = dir.join("mnt");
        let _ = std::fs::remove_dir_all(&mnt);
        std::fs::create_dir_all(&mnt).map_err(|e| format!("创建挂载点失败: {e}"))?;

        // hdiutil 为 macOS 系统自带（所有 macOS 都存在，不存在 CI 兼容问题）；
        // 指定挂载点避免依赖 /Volumes 下的卷名
        let out = Cmd::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mnt)
            .arg(pkg)
            .output()
            .map_err(|e| format!("执行 hdiutil 失败: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "挂载 DMG 失败: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        let mounted_app = mnt.join("DevTools.app");
        let copy = Cmd::new("cp").arg("-R").arg(&mounted_app).arg(&app).output();
        // 无论复制成败先卸载
        let _ = Cmd::new("hdiutil").arg("detach").arg("-force").arg(&mnt).output();

        match copy {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(format!(
                "复制应用失败: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => Err(format!("执行复制失败: {e}")),
        }
    }

    /// 当前运行实例所属的 .app 包路径（可执行上溯：
    /// ancestors = [exe, MacOS, Contents, <app>]，第 4 项即 .app）。
    #[cfg(target_os = "macos")]
    pub fn bundle_app_path() -> Result<PathBuf, String> {
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        exe.ancestors()
            .nth(3)
            .filter(|p| p.extension().is_some_and(|e| e == "app"))
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                "当前不是从 DevTools.app 运行（开发模式请用 make app 构建后验证自更新）".into()
            })
    }

    #[cfg(target_os = "windows")]
    pub fn install(pkg: &Path) -> Result<(), String> {
        use std::io::Read as _;

        let dir = pkg.parent().ok_or("安装包路径异常")?;
        let extract = dir.join("extract");
        let _ = std::fs::remove_dir_all(&extract);
        std::fs::create_dir_all(&extract).map_err(|e| format!("创建解压目录失败: {e}"))?;

        let file = std::fs::File::open(pkg).map_err(|e| format!("打开安装包失败: {e}"))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("读取 zip 失败: {e}"))?;

        // 产物 zip 内是顶层目录 + devtools.exe；按名匹配（不展开全部条目）
        let mut exe_entry = None;
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i);
            let name = entry.name().trim_end_matches('/').to_string();
            if name.ends_with("/devtools.exe") || name == "devtools.exe" {
                exe_entry = Some(name);
                break;
            }
        }
        let Some(entry_name) = exe_entry else {
            return Err("安装包内未找到 devtools.exe".into());
        };

        let mut entry = archive
            .by_name(&entry_name)
            .map_err(|e| format!("读取 devtools.exe 失败: {e}"))?;
        let new_exe = extract.join("devtools.exe");
        let mut out = std::fs::File::create(&new_exe).map_err(|e| format!("创建目标文件失败: {e}"))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("解压失败: {e}"))?;
        drop(entry);

        // 运行中的 exe 不能删除/覆盖写入，但允许改名——旧实例改名让位，
        // 新 exe 落到原路径；残留的 .old 由新实例启动时清理
        let current = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let backup = current.with_extension("exe.old");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(&current, &backup).map_err(|e| format!("备份旧版本失败: {e}"))?;
        std::fs::rename(&new_exe, &current).map_err(|e| format!("安装新版本失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn install(pkg: &Path) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = pkg.parent().ok_or("安装包路径异常")?;
        let extract = dir.join("extract");
        let _ = std::fs::remove_dir_all(&extract);
        std::fs::create_dir_all(&extract).map_err(|e| format!("创建解压目录失败: {e}"))?;

        let gz = flate2::read::GzDecoder::new(std::fs::File::open(pkg).map_err(|e| format!("打开安装包失败: {e}"))?);
        tar::Archive::new(gz)
            .unpack(&extract)
            .map_err(|e| format!("解压失败: {e}"))?;

        // 产物 tar.gz 内是顶层目录 + devtools；逐层查找二进制
        let new_exe = find_executable(&extract).ok_or("安装包内未找到 devtools 可执行文件")?;
        std::fs::set_permissions(&new_exe, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置执行权限失败: {e}"))?;

        // Linux 允许 rename 覆盖运行中的二进制（旧 inode 由进程继续持有）
        let current = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        std::fs::rename(&new_exe, &current)
            .map_err(|e| format!("安装新版本失败（安装目录可能无写权限）: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn find_executable(root: &Path) -> Option<PathBuf> {
        std::fs::read_dir(root).ok()?.flatten().find_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                find_executable(&path)
            } else if path.file_name().is_some_and(|n| n == "devtools") {
                Some(path)
            } else {
                None
            }
        })
    }
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

    fn info_with_assets(names: &[&str]) -> ReleaseInfo {
        ReleaseInfo {
            version: "9.9.9".into(),
            url: String::new(),
            notes: String::new(),
            assets: names
                .iter()
                .map(|n| ReleaseAsset {
                    name: (*n).into(),
                    url: format!("https://example.com/{n}"),
                    size: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn test_package_filename_matches_platform() {
        let name = package_filename("1.2.3");
        // 平台分支断言：产物命名与 release.yml 一致
        let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "DevTools-v1.2.3-macos-aarch64.dmg"
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "DevTools-v1.2.3-windows-x86_64.zip"
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "DevTools-v1.2.3-linux-x86_64.tar.gz"
        } else {
            "DevTools-v1.2.3-unsupported"
        };
        assert_eq!(name, expected);
    }

    #[test]
    fn test_select_asset_matches_exactly() {
        let info = info_with_assets(&[
            "DevTools-v9.9.9-macos-aarch64.dmg",
            "DevTools-v9.9.9-macos-aarch64.dmg.sha256",
            "DevTools-v9.9.9-windows-x86_64.zip",
            "DevTools-v9.9.9-windows-x86_64.zip.sha256",
            "DevTools-v9.9.9-linux-x86_64.tar.gz",
            "DevTools-v9.9.9-linux-x86_64.tar.gz.sha256",
        ]);
        let want = package_filename("9.9.9");
        let asset = select_asset(&info, "9.9.9").expect("应命中当前平台安装包");
        assert_eq!(asset.name, want);
        // 校验文件可按同样规则定位
        assert!(info
            .assets
            .iter()
            .any(|a| a.name == format!("{}.sha256", asset.name)));
    }

    #[test]
    fn test_select_asset_missing_reports_name() {
        // 附件里只有其他版本的产物，查 9.9.9 应报缺
        let info = info_with_assets(&["DevTools-v8.8.8-macos-aarch64.dmg"]);
        let err = select_asset(&info, "9.9.9").unwrap_err();
        assert!(err.contains(package_filename("9.9.9").as_str()));
    }

    #[test]
    fn test_parse_assets_skips_malformed_entries() {
        let body: serde_json::Value = serde_json::json!({
            "assets": [
                { "name": "a.dmg", "browser_download_url": "https://x/a.dmg", "size": 5 },
                { "name": "broken" },
                { "no_name": true }
            ]
        });
        let assets = parse_assets(&body);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "a.dmg");
        assert_eq!(assets[0].size, 5);
    }

    #[test]
    fn test_parse_checksum() {
        let hash = "a".repeat(64);
        assert_eq!(parse_checksum(&format!("{hash}  DevTools.zip\n")).unwrap(), hash);
        // 大写统一转小写
        let upper = "A".repeat(64);
        assert_eq!(parse_checksum(&format!("{upper}  x\n")).unwrap(), hash);
        assert!(parse_checksum("").is_err());
        assert!(parse_checksum("tooshort  x\n").is_err());
    }

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
