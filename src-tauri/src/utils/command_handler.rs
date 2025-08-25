use crate::utils::{error::DevToolResult, response::DevToolResponse};

/// 统一的命令处理器特征
/// 定义所有命令处理器应该实现的接口
#[allow(dead_code)]
pub trait CommandHandler<I, O> {
    fn handle(&self, input: I) -> DevToolResult<O>;
}

/// 命令执行器
/// 提供统一的错误处理和响应包装
#[allow(dead_code)]
pub struct CommandExecutor;

#[allow(dead_code)]
impl CommandExecutor {
    /// 执行命令并返回统一响应格式
    ///
    /// # Arguments
    /// * `handler` - 命令处理器
    /// * `input` - 输入参数
    ///
    /// # Returns
    /// 统一的响应结构
    pub fn execute<I, O, H>(handler: H, input: I) -> DevToolResponse<O>
    where
        H: CommandHandler<I, O>,
    {
        match handler.handle(input) {
            Ok(result) => DevToolResponse::success(result),
            Err(error) => DevToolResponse::error(error.to_string()),
        }
    }

    /// 带输入验证的命令执行
    ///
    /// # Arguments
    /// * `handler` - 命令处理器
    /// * `input` - 输入参数
    /// * `validator` - 验证函数
    ///
    /// # Returns
    /// 统一的响应结构
    pub fn execute_with_validation<I, O, H, V>(
        handler: H,
        input: I,
        validator: V,
    ) -> DevToolResponse<O>
    where
        H: CommandHandler<I, O>,
        V: Fn(&I) -> DevToolResult<()>,
    {
        match validator(&input) {
            Ok(_) => Self::execute(handler, input),
            Err(error) => DevToolResponse::error(error.to_string()),
        }
    }

    /// 异步命令执行（用于future扩展）
    pub async fn execute_async<I, O, H>(handler: H, input: I) -> DevToolResponse<O>
    where
        H: CommandHandler<I, O> + Send + 'static,
        I: Send + 'static,
        O: Send + 'static,
    {
        // 在tokio运行时中执行
        tokio::task::spawn_blocking(move || Self::execute(handler, input))
            .await
            .unwrap_or_else(|_| DevToolResponse::error("命令执行失败".to_string()))
    }
}

/// 为各种处理器实现统一接口的宏
///
/// # Usage
/// ```rust
/// impl_command_handler!(MyHandler, String, String, process);
/// ```
#[macro_export]
macro_rules! impl_command_handler {
    ($handler:ty, $input:ty, $output:ty, $method:ident) => {
        impl crate::utils::command_handler::CommandHandler<$input, $output> for $handler {
            fn handle(&self, input: $input) -> crate::utils::error::DevToolResult<$output> {
                self.$method(input)
            }
        }
    };
}

/// 简化命令创建的宏
///
/// # Usage
/// ```rust
/// create_command!(convert_base64, String, String, |input: String| {
///     // 处理逻辑
///     Ok(base64::encode(input))
/// });
/// ```
#[macro_export]
macro_rules! create_command {
    ($name:ident, $input:ty, $output:ty, $handler:expr) => {
        pub struct $name;

        impl $name {
            pub fn process(&self, input: $input) -> crate::utils::error::DevToolResult<$output> {
                $handler(input)
            }
        }

        impl crate::utils::command_handler::CommandHandler<$input, $output> for $name {
            fn handle(&self, input: $input) -> crate::utils::error::DevToolResult<$output> {
                self.process(input)
            }
        }
    };
}

/// 带验证的命令创建宏
#[macro_export]
macro_rules! create_validated_command {
    ($name:ident, $input:ty, $output:ty, $validator:expr, $handler:expr) => {
        pub struct $name;

        impl $name {
            pub fn process(&self, input: $input) -> crate::utils::error::DevToolResult<$output> {
                $validator(&input)?;
                $handler(input)
            }
        }

        impl crate::utils::command_handler::CommandHandler<$input, $output> for $name {
            fn handle(&self, input: $input) -> crate::utils::error::DevToolResult<$output> {
                self.process(input)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::error::DevToolError;

    // 测试用的简单处理器
    struct TestHandler;

    impl TestHandler {
        fn process(&self, input: String) -> DevToolResult<String> {
            if input.is_empty() {
                Err(DevToolError::EmptyInput("input".to_string()))
            } else {
                Ok(format!("processed: {}", input))
            }
        }
    }

    impl CommandHandler<String, String> for TestHandler {
        fn handle(&self, input: String) -> DevToolResult<String> {
            self.process(input)
        }
    }

    #[test]
    fn test_command_executor() {
        let handler = TestHandler;

        // 测试成功情况
        let result = CommandExecutor::execute(handler, "test".to_string());
        assert!(result.success);
        assert_eq!(result.data, Some("processed: test".to_string()));

        // 测试失败情况
        let handler = TestHandler;
        let result = CommandExecutor::execute(handler, "".to_string());
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_create_command_macro() {
        create_command!(TestCommand, String, String, |input: String| {
            Ok(format!("test: {}", input))
        });

        let command = TestCommand;
        let result = CommandExecutor::execute(command, "hello".to_string());
        assert!(result.success);
        assert_eq!(result.data, Some("test: hello".to_string()));
    }
}
