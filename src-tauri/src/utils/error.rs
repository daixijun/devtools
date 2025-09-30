use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevToolError {
    EmptyInput(String),
    ParseError(String, String), // (context, error)
    NetworkError(String),
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
            DevToolError::EmptyInput(field) => format!("{}不能为空", field),
            DevToolError::ParseError(context, error) => format!("{}解析失败: {}", context, error),
            DevToolError::NetworkError(error) => format!("网络请求失败: {}", error),
            DevToolError::CertificateError(error) => format!("证书处理错误: {}", error),
            DevToolError::ValidationError(error) => format!("验证失败: {}", error),
            DevToolError::FileError(error) => format!("文件操作错误: {}", error),
            DevToolError::SystemError(error) => format!("系统错误: {}", error),
            DevToolError::ConversionError(error) => format!("格式转换失败: {}", error),
            DevToolError::AsyncExecutionError => "异步执行失败".to_string(),
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
