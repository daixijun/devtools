//! 文本文件保存流程：对话框选择 + 受限权限写盘 + 结果 toast。
//!
//! rfd 的 save_file 是阻塞调用，必须放进 core_rt 后台任务避免卡 UI；
//! 写盘统一走 [`devtools::utils::fs::write_text_file`]，敏感内容（私钥）
//! 在 Unix 下以 0600 权限落盘。证书/密钥类页面共用此流程。

use futures::StreamExt as _;
use gpui::{Context, Window};

use crate::components::notify;
use devtools::i18n::t;

/// 保存参数
pub struct SaveTextParams {
    pub content: String,
    pub default_name: String,
    /// 过滤器显示名（如 "PEM文件"）
    pub filter_name: &'static str,
    /// 扩展名（如 "pem"）
    pub extension: &'static str,
    /// 私钥等敏感内容：Unix 下以 0600 落盘
    pub private: bool,
}

/// 发起文本文件保存：对话框与写盘在后台任务中完成，结果以 toast 呈现。
pub fn save_text_file<V: 'static>(
    params: SaveTextParams,
    window: &mut Window,
    cx: &mut Context<V>,
) {
    enum SaveOutcome {
        Cancelled,
        Saved,
        Failed(String),
    }

    let mut rx = crate::core_rt::submit(cx, async move {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(params.filter_name, &[params.extension])
            .set_file_name(params.default_name)
            .save_file()
        else {
            return SaveOutcome::Cancelled;
        };
        match devtools::utils::fs::write_text_file(&path, &params.content, params.private) {
            Ok(_) => SaveOutcome::Saved,
            Err(e) => SaveOutcome::Failed(format!("{}: {}", t("保存文件失败"), e)),
        }
    });

    cx.spawn_in(window, async move |this, cx| {
        if let Some(outcome) = rx.next().await {
            let _ = this.update_in(cx, |_this, window, cx| match outcome {
                SaveOutcome::Cancelled => {}
                SaveOutcome::Saved => notify::toast_success(window, cx, t("文件保存成功")),
                SaveOutcome::Failed(message) => notify::toast_error(window, cx, message),
            });
        }
    })
    .detach();
}
