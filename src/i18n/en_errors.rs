//! EN 词条：错误消息模板（error.rs 十个变体的固定文案段）。
//!
//! 中文输出保持历史契约原句（拼接段不含额外空格）；en 译法在语序需要处
//! 带空格。业务层传入的 detail 参数（如具体超时描述）暂随中文，后续批次
//! 再覆盖。

/// 错误消息词条表。
pub const EN_ERRORS: &[(&str, &str)] = &[
    ("不能为空", " cannot be empty"),
    ("解析失败", " failed to parse"),
    ("网络请求失败", "Network request failed"),
    ("未找到", "Not found"),
    ("证书处理错误", "Certificate error"),
    ("验证失败", "Validation failed"),
    ("文件操作错误", "File error"),
    ("系统错误", "System error"),
    ("格式转换失败", "Conversion failed"),
    ("异步执行失败", "Async execution failed"),
];
