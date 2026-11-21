//! 工具界面组件层：封装 gpui-component 语义，供 32 个工具页面复用。
//!
//! 工具实现只应依赖本模块的辅助函数与 gpui-component 的现成组件，
//! 保证各工具页面的视觉与交互一致。

pub mod notify;
pub mod select;
pub mod ui;

// 供工具页直接引用的便捷路径
pub use select::select_state;
