//! 开发者工具箱纯 Rust 业务逻辑库。
//!
//! 目标：承载与 UI 框架无关的全部业务能力（错误模型、配置持久化、工具注册表、
//! 各工具的核心逻辑），供同包 GPUI 壳（bin target，src/main.rs）复用。

pub mod config;
pub mod crypto;
pub mod error;
pub mod i18n;
pub mod registry;
pub mod text_tools;
pub mod tools;
pub mod utils;
