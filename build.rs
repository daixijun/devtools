//! 构建脚本：Windows 上构建时嵌入 exe 资源（应用图标）。
//!
//! 资源编译依赖 Windows 工具链（rc.exe/windres），只在 Windows 主机上执行；
//! 非 Windows 主机交叉检查/构建时整段跳过（embed-resource 依赖仅在
//! Windows 目标声明，本文件在 macOS/Linux 上编译时不引用它）。

#[cfg(target_os = "windows")]
fn main() {
    if std::path::Path::new("assets/devtools.rc").exists() {
        if let Err(e) = embed_resource::try_compile("assets/devtools.rc", embed_resource::NONE) {
            println!("cargo:warning=跳过 Windows 图标嵌入（资源编译失败: {e}）");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
