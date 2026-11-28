//! 共享工具函数。
//!
//! 统一错误模型 `error.rs` 保持在 crate 顶层（`crate::error`），此处为
//! 工具实现所需的辅助模块。

pub mod code_formatter;
pub mod crypto;
pub mod fs;
pub mod image;
pub mod string_utils;
pub mod updater;
