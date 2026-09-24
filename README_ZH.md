# 开发者工具箱

基于 **Tauri 2 + React 18 + TypeScript** 的 Spotlight 风格桌面开发者工具箱。7 大分类共 31 个开发工具，通过系统级全局快捷键随时呼出，支持 macOS、Windows 和 Linux。

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

- **设置** - 主题（浅色/深色/跟随系统）、系统托盘、开机自启、启动最小化、关闭到托盘、自定义全局快捷键

## 🧭 使用方式

1. 按全局快捷键（macOS 默认 **Option+Space**，其他平台 **Alt+Space**）呼出/隐藏 Spotlight 搜索条
2. 输入关键词按名称、描述或关键字过滤工具，↑/↓ + Enter 打开 —— 每个工具在独立的可缩放窗口中运行
3. 按 Esc 隐藏窗口；随时可通过快捷键或系统托盘图标重新呼出

## 🛠️ 技术栈

### 前端

- **React 18.3** - 现代化前端框架
- **TypeScript 5.6** - 类型安全的 JavaScript
- **Monaco Editor 4.7** - 代码编辑器（VS Code 同款）
- **Tailwind CSS 4** - 实用优先的 CSS 框架
- **React Split 2.0** - 可调整大小的分割面板
- **Tauri API v2** - 前后端通信，含 dialog/fs/global-shortcut/opener/autostart 插件

### 后端

- **Tauri 2.x** - 跨平台桌面应用框架，支持系统托盘
- **Rust** - 系统级编程语言
- **sqlparser 0.58** - 专业 SQL 解析器（基于 AST）
- **reqwest 0.12 + rustls** - 在线工具的 HTTP 客户端
- **openssl (vendored) + x509-parser + rustls** - 证书解析与加密
- **image 0.25 + oxipng + imagequant + nom-exif + libheif** - 图片转换、压缩与 EXIF 解析
- **dns-lookup + pcre2 + regex + chrono/chrono-tz** - 网络查询、正则引擎与时间处理
- **FFmpeg**（外部可选依赖）- 仅视频转换需要

## 🚀 快速开始

### 环境要求

- Node.js 18+
- Rust 1.77+（Tauri 2 最低要求）
- pnpm 包管理器
- FFmpeg（可选，仅视频转换工具需要）

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
pnpm tauri dev
# 或: make dev
```

### 构建应用

```bash
# 构建前端（含类型检查）
pnpm build

# 构建桌面应用
pnpm tauri build
```

### 检查

```bash
make check          # tsc + 样式检查 + cargo check
make check-tsc      # 仅 TypeScript
make check-styles   # 设计系统检查（slate 色系、rounded-lg、duration-200）
make cargo-clippy   # Rust 静态检查
```

## 📁 项目结构

```bash
devtools/                        # 项目根目录
├── Makefile                     # 开发/构建/检查工作流快捷入口
├── docs/                        # 文档（工具使用指南、UI 说明、规划）
├── scripts/
│   └── check-styles.cjs         # Tailwind/设计系统一致性检查
├── public/                      # 静态资源（应用 Logo）
├── src-tauri/                   # Tauri (Rust) 后端
│   ├── capabilities/            # Tauri 能力定义
│   ├── icons/                   # 应用图标
│   ├── src/
│   │   ├── lib.rs               # 插件初始化 + invoke_handler 命令注册
│   │   ├── main.rs              # 应用入口点
│   │   ├── tools/               # 每个后端工具一个 Rust 模块
│   │   └── utils/               # crypto、error、validation、格式化等辅助模块
│   └── tauri.conf.json          # 窗口/托盘/构建配置
├── src/                         # 前端 React/TypeScript 代码
│   ├── App.tsx                  # 哈希路由：Spotlight 搜索条 ↔ #/tool/<id> 工具窗口
│   ├── components/
│   │   ├── SpotlightSearch.tsx  # 主启动器（搜索 + 键盘导航）
│   │   ├── ToolWindow.tsx       # 工具窗口容器（懒加载 + 主题）
│   │   ├── common/              # 通用组件（文件上传、复制按钮等）
│   │   ├── icons/               # 工具图标
│   │   ├── layouts/             # 布局组件
│   │   └── templates/           # 工具页面模板
│   ├── hooks/                   # useTheme、useToast、useCopyToClipboard、useToolWindow 等
│   ├── tools/
│   │   ├── registry.ts          # 唯一事实来源：工具元数据 + 窗口尺寸
│   │   └── *.tsx                # 32 个工具组件（懒加载）
│   ├── utils/
│   │   ├── api.ts               # Tauri invoke() 命令的类型化封装
│   │   └── globalShortcut.ts    # 全局快捷键注册辅助
│   └── main.tsx                 # React 入口点
├── tailwind.config.js
└── vite.config.ts               # 开发服务器固定端口 1453
```

## 🎯 核心设计

### 前端架构

- **Spotlight 启动器** - 无边框搜索条即主窗口；各工具按 `registry.ts` 中定义的尺寸在独立窗口打开
- **工具注册表** - 每个工具在 `src/tools/registry.ts` 中声明一次（id、分类、图标、关键字、窗口尺寸）并按需懒加载
- **哈希路由** - `#/tool/<id>` 渲染工具窗口；根路由渲染 Spotlight 搜索条
- **Monaco Editor 集成** - 文本类工具使用 Monaco 编辑器，支持语法高亮和分栏布局
- **主题支持** - 深色/浅色/跟随系统主题，跨窗口实时同步

### 后端架构

- **Tauri 命令系统** - 前端通过 `src/utils/api.ts` 中类型化的 `invoke()` 封装调用 Rust
- **模块化工具结构** - 每个后端工具独立成模块，统一使用 `DevToolError` 错误处理与本地化消息
- **专业 SQL 解析** - sqlparser-rs 构建抽象语法树而非正则匹配；支持 Generic/MySQL/PostgreSQL/SQLite 方言
- **原生图片处理管线** - 图片转换、EXIF 解析和压缩均在 Rust 中完成（image/oxipng/imagequant），不依赖浏览器

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

- [Tauri 官方文档](https://tauri.app/)
- [React 官方文档](https://react.dev/)
- [TypeScript 官方文档](https://www.typescriptlang.org/)
