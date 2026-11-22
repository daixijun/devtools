# 开发者工具箱

基于 **纯 Rust + GPUI**（原生 GPU 渲染）的桌面开发者工具箱：主窗口形态（左侧分类导航 + 右侧工具内容区），配系统托盘与原生应用菜单。8 大分类共 32 个开发工具，仅支持 macOS。

> 📖 English: [README.md](README.md) · 详细工具使用指南：[docs/tools.md](docs/tools.md)

## ✨ 功能特性

### 编码/解码工具

- **Base64 编解码** - 支持文本和文件的 Base64 编解码，实时转换并校验格式
- **URL 编解码** - URL 百分号编码转换
- **AES 加密/解密** - 支持 CBC/ECB/CFB/OFB/CTR 模式、128/192/256 位密钥、多种填充方式，可随机生成 IV
- **MD5 加密** - 文本、单文件、批量文件哈希计算，支持一键复制和导出 `.txt`
- **SHA 哈希加密** - 支持 SHA-1/224/256/384/512 和 SHA-3，覆盖文本和文件
- **JWT 生成** - 自定义 Header/Payload，支持 HS*/RS* 签名，自动写入 `iat`/`exp` 有效期
- **JWT 解码** - 解析 Header/Payload/签名并支持签名验证（共享密钥或 RSA 公钥）
- **密码生成器** - 4–64 位密码，可选字符类型、特殊符号预设分组，带强度评级
- **密码加密验证** - 支持 bcrypt、PBKDF2、SHA-256/512、MD5 的加密与比对（可调轮数/迭代次数）
- **RSA 密钥对生成** - 2048/3072/4096 位密钥；私钥支持 PKCS#8/PKCS#1，公钥支持 PEM (SPKI)/PKCS#1/OpenSSH 格式

### 证书工具

- **证书查看器** - 解析 PEM/PFX 证书及完整证书链（终端/中间 CA/根 CA），有效期检查、链完整性与**证书顺序检测**，支持一键复制正确顺序的证书
- **CSR 查看** - 解析 PKCS#10 证书签名请求：主题、公钥、签名算法、请求扩展；可选私钥匹配校验
- **CSR 生成** - 生成 PKCS#10 CSR，支持 RSA（2048–4096）/ECC（P-256/384/521）密钥与 SAN 扩展，CSR/私钥/公钥均可下载
- **PEM 转 PFX** - 将 PEM 证书（支持加密私钥）转换为 PFX/PKCS#12 格式
- **PFX 转 PEM** - 从 PFX/PKCS#12 文件提取证书和私钥
- **在线 SSL 检测** - 在线检测：安全评分、SSL Labs 等级、证书链、加密套件、CVE 漏洞列表与加固建议

### 网络工具

- **子网掩码计算器** - IPv4/IPv6 CIDR 计算：网络地址、广播地址、可用 IP 范围、前缀详情
- **IP 地址信息查询** - 查询本机或任意 IPv4/IPv6 的地理位置与运营商信息，多数据源结果汇总
- **DNS 解析工具** - 并发查询多台 DNS 服务器的 A/AAAA/CNAME/MX/TXT/NS/SOA 记录；支持单个和批量反向解析（PTR）
- **域名 Whois 查询** - 批量查询，RDAP 优先多数据源自动切换，24 小时结果缓存、查询历史，支持导出 CSV/JSON

### 数据格式转换

- **JSON 格式化** - JSON 美化、压缩、去除转义，带语法高亮
- **格式转换器** - JSON、YAML、TOML 三者互转
- **JSON 转 Go 结构体** - 可选标签（json/yaml/gorm/db/sql/toml/env/ini），支持 Go 1.18 `any` 与 Go 1.24 `omitzero`
- **SQL 转 Go 结构体** - 多表 `CREATE TABLE` 解析，无符号类型映射、表名单数化、反引号处理
- **SQL 转 Go Ent ORM** - 生成 Ent Schema，支持边缘关系、Mixin、Hooks、Policy、软删除、UUID 主键等选项

### 媒体格式转换

- **图片格式转换** - PNG/JPEG/WebP/BMP/GIF/TIFF/ICO/HEIC 互转，支持尺寸调整、移除 EXIF、批量处理
- **图片预览器** - 支持缩放/平移的图片查看器，可输入 URL、粘贴图片或打开本地文件；支持重新编码另存为其他格式
- **视频格式转换** - 基于系统 FFmpeg 在 MP4/WebM/AVI/MKV 等格式间转换，带环境检测、批量模式和逐文件进度
- **图片压缩工具** - 无损优化（oxipng/zopfli）或质量压缩（TinyPNG 式调色板量化），支持等比缩放、移除 EXIF、批量处理和压缩前后滑块对比

### 开发工具

- **正则表达式测试器** - 实时匹配，5 种引擎（rust/re2/pcre/golang/javascript）可选，常用标志、捕获组/命名组详情、替换模式

### 时间工具

- **时间戳转换器** - Unix 时间戳双向转换（秒/毫秒），支持全部 IANA 时区，内置实时时钟

### 其他工具

- **设置** - 主题（浅色/深色/跟随系统）、系统托盘、开机自启、启动最小化、关闭到托盘

## 🧭 使用方式

1. 启动应用（或从托盘图标打开）
2. 输入关键词按名称、描述或关键字过滤工具，↑/↓ + Enter 打开 —— 每个工具在独立的可缩放窗口中运行
3. 关闭窗口即隐藏到托盘（启用托盘时）；随时可从托盘图标重新打开

## 🛠️ 技术栈

### UI（GPUI）

- **gpui（gpui-pre 快照）** - Zed 的原生 GPU 渲染 UI 框架（macOS Metal）
- **gpui-component 0.10** - longbridge 组件库（输入、编辑器、下拉、通知、分栏等）
- **tray-icon + muda** - 系统托盘；**rfd** - 原生文件对话框

### 业务层（devtools lib target）

- **Rust 2021** - 全部业务逻辑，零框架耦合
- **sqlparser 0.58 / reqwest + rustls / openssl + x509-parser** - SQL 解析、网络、证书
- **image + oxipng + imagequant + nom-exif + libheif** - 图像转换/压缩/EXIF
- **aes/sha/md-5/hmac/pbkdf2/bcrypt** - 加密（与历史 JS 实现逐字节兼容）
- **dns-lookup + pcre2 + regex + chrono/chrono-tz** - 网络、正则、时间
- **FFmpeg**（外部，可选）- 仅视频转换需要

## 🚀 快速开始

### 环境要求

- Rust 1.77+
- FFmpeg（可选，仅视频转换工具需要）

### 安装依赖

```bash
# 直接构建，无需安装额外依赖
```

### 开发模式

```bash
make run
# watch 模式：应用运行中保存源文件自动重编译并重启；
# 退出应用（Cmd+Q）后 watch 一并结束，回到 shell；
# make run-direct 始终普通运行
```

### 构建应用

```bash
make app-release    # 打包 target/DevTools.app (release)
make dmg            # 打包 target/DevTools.dmg 安装镜像（release；先构建 .app）
```

### 检查

```bash
make check          # cargo check + clippy（零警告标准）+ core 测试
make test           # 单元测试（含 crypto 黄金向量）
make fmt            # cargo fmt
```

## 📁 项目结构

```bash
devtools/                        # 项目根目录
├── Cargo.toml                   # workspace（pin gpui / gpui-platform / gpui-component）
├── Makefile                     # run/check/app 工作流快捷入口
├── scripts/
│   └── build-app.sh             # DevTools.app 打包（Info.plist + icns）
├── crates/
│   └── devtools/                # devtools：单包双 target（lib = 业务层，bin = GPUI 壳）
│       ├── src/lib.rs           # 业务层根（无 UI 依赖）
│       ├── src/registry.rs      # 32 工具元数据注册表（权威源）
│       ├── src/config.rs        # 共享配置 JSON（双向兼容）
│       ├── src/crypto.rs        # AES/MD5/SHA/JWT/bcrypt（字节级兼容历史行为）
│       ├── src/text_tools/      # base64 / url / password / timestamp / subnet / format
│       ├── src/tools/           # 16 个工具逻辑模块（ssl、证书、dns、whois、媒体、sql…）
│       ├── src/utils/           # 共享辅助模块 + DevToolError
│       ├── src/main.rs          # 壳层入口：启动流程（config → theme → core_rt → 应用菜单 → 主窗口 → 托盘）
│       ├── src/windows.rs       # WindowManager 全局（主窗口 + 关于窗口）
│       ├── src/main_window.rs   # 主窗口：左侧导航（搜索 + 分类分组）+ 右侧工具内容区
│       ├── src/app_menu.rs      # macOS 应用菜单（关于/设置/编辑/窗口/帮助）
│       ├── src/tool_window.rs   # 工具窗口宿主（页头 + 页面视图）
│       ├── src/tray.rs          # 托盘图标 + 菜单事件泵
│       ├── src/macos_window_ctl.rs  # unsafe objc：逐窗口显隐（唯一 unsafe 文件）
│       ├── src/core_rt.rs       # tokio 运行时桥（业务层异步逻辑）
│       ├── src/components/      # UI 组件层（ui/notify/select）
│       └── src/pages/           # 32 个工具页，每工具一个文件
└── docs/plans/                  # 工具 UI 契约规格
```

## 🎯 核心设计

### 应用架构

- **主窗口形态** - 左侧分类导航（搜索 + 分组工具列表），右侧工具内容区
- **工具注册表** - 每个工具在 `crates/devtools/src/registry.rs` 中声明一次（id、中文名、描述、分类、关键字），单一事实来源
- **代码编辑器** - gpui-component Editor（代码模式）+ syntect 语法高亮（SQL/Go/JSON 等，配色跟随主题）
- **主题支持** - 深色/浅色/跟随系统主题，窗口实时跟随系统外观
- **应用菜单栏** - 关于/设置/编辑（剪贴板）/窗口/帮助，菜单项走 gpui 全局 action 分发

### 业务逻辑（devtools lib target）

- **模块化工具结构** - 每个工具逻辑独立成模块，统一使用 `DevToolError` 错误处理与历史中文消息契约
- **专业 SQL 解析** - sqlparser-rs 构建抽象语法树而非正则匹配；支持 Generic/MySQL/PostgreSQL/SQLite 方言
- **原生图片处理管线** - 图片转换、EXIF 解析和压缩均在 Rust 中完成（image/oxipng/imagequant/libheif）
- **字节级兼容密码学** - AES 5 模式 × 6 填充、MD5/SHA/Keccak、PBKDF2、JWT、bcrypt，与历史版本逐字节对齐

## 📋 支持的 SQL 格式

SQL 工具支持以下格式：

- MySQL 反引号格式：`CREATE TABLE \`users\` (\`id\` int unsigned...)`
- 不同方言的多表语句
- 复数表名，如 "pipelines" -> "Pipeline"
- 混合大小写无符号类型："int unsigned"、"INT UNSIGNED"、"bigint unsigned"

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

[MIT License](LICENSE)

## ⚠️ 免责声明

本软件按"现状"提供，不附带任何明示或暗示的担保。开发者和贡献者不对因使用本软件而产生的任何损害承担责任，包括但不限于：

- 数据丢失或损坏
- 安全漏洞
- 输出或功能错误
- 与其他软件的兼容性问题

建议用户：

- 在生产环境使用前彻底测试软件
- 验证生成的代码和转换的准确性
- 处理敏感数据时实施适当的安全措施
- 对重要数据保持备份

本工具仅用于开发和测试目的。部署前请务必审查和验证生成的代码。

## 🔗 相关链接

- [gpui-component 文档](https://longbridge.github.io/gpui-component/)
- [gpui-component 文档](https://longbridge.github.io/gpui-component/)
- [TypeScript 官方文档](https://www.typescriptlang.org/)
