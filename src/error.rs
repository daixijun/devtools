//! 统一错误模型。
//!
//! 变体与中文文案保持历史版本一致；固定文案段经 i18n（英文模式查 EN_ERRORS
//! 表拼接），业务层传入的 detail 参数随原文。

use crate::i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevToolError {
    EmptyInput(String),
    ParseError(String, String), // (context, error)
    NetworkError(String),
    NotFound(String), // 资源不存在（如 GitHub Releases 尚无发布）
    CertificateError(String),
    ValidationError(String),
    FileError(String),
    SystemError(String),
    ConversionError(String), // 格式转换错误
    AsyncExecutionError,     // 异步执行错误
}

impl DevToolError {
    pub fn to_localized_string(&self) -> String {
        match self {
            DevToolError::EmptyInput(field) => format!("{}{}", field, t("不能为空")),
            DevToolError::ParseError(context, error) => {
                format!("{}{}: {}", context, t("解析失败"), error)
            }
            DevToolError::NetworkError(error) => format!("{}: {}", t("网络请求失败"), error),
            DevToolError::NotFound(detail) => format!("{}: {}", t("未找到"), detail),
            DevToolError::CertificateError(error) => format!("{}: {}", t("证书处理错误"), error),
            DevToolError::ValidationError(error) => format!("{}: {}", t("验证失败"), error),
            DevToolError::FileError(error) => format!("{}: {}", t("文件操作错误"), error),
            DevToolError::SystemError(error) => format!("{}: {}", t("系统错误"), error),
            DevToolError::ConversionError(error) => format!("{}: {}", t("格式转换失败"), error),
            DevToolError::AsyncExecutionError => t("异步执行失败").to_string(),
        }
    }
}

impl From<DevToolError> for String {
    fn from(error: DevToolError) -> Self {
        error.to_localized_string()
    }
}

impl std::fmt::Display for DevToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_localized_string())
    }
}

impl std::error::Error for DevToolError {}

// Convenience type alias
pub type DevToolResult<T> = Result<T, DevToolError>;
