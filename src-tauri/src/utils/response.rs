use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DevToolResponse<T> {
    pub data: Option<T>,
    pub success: bool,
    pub error: Option<String>,
}

impl<T> DevToolResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            success: true,
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            data: None,
            success: false,
            error: Some(message.into()),
        }
    }
}

// Helper for converting Result types
impl<T, E> From<Result<T, E>> for DevToolResponse<T>
where
    E: Into<String>,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => DevToolResponse::success(data),
            Err(error) => DevToolResponse::error(error.into()),
        }
    }
}
