# 开发者工具箱

一个基于 Tauri 2 + React 的桌面开发者工具箱应用

## ✨ 功能特性

### 编码/解码工具

- **Base64 编码/解码** - 文本编码工具
- **JWT 编码/解码** - JWT 令牌处理和验证

### 证书工具

- **证书查看器** - PEM 证书分析和信息显示
- **PEM 转 PFX 转换器** - 将 PEM 证书转换为 PFX 格式
- **PFX 转 PEM 转换器** - 将 PFX 证书转换为 PEM 格式
- **SSL 证书检查器** - 从 URL 检查 SSL 证书信息

### 网络工具

- **IP/CIDR 计算器** - 子网计算和 IP 地址操作
- **IP 地址信息查询** - IP 地址详细信息和地理位置查询

### 数据格式工具

- **JSON 格式化** - JSON 格式化、验证和美化打印
- **格式转换器** - JSON、YAML、TOML 之间的多格式转换
- **JSON 转 Go 结构体** - 从 JSON 生成 Go 结构体定义

### 数据库工具

- **SQL 转 Go 结构体** - 从 SQL 架构生成 Go 结构体定义，支持多表
- **SQL 转 Go Ent ORM** - 从 SQL 架构生成 Go Ent ORM 架构，支持多表

### 其他工具

- **时间戳转换器** - Unix 时间戳转换为人类可读日期
- **正则表达式测试器** - 测试和调试正则表达式，支持实时匹配
- **设置** - 应用配置和偏好设置

## 🛠️ 技术栈

### 前端

- **React 18.3.1** - 现代化前端框架
- **TypeScript** - 类型安全的 JavaScript
- **Monaco Editor 4.7.0** - 代码编辑器（VS Code 同款）
- **Tailwind CSS 4.1.11** - 实用优先的 CSS 框架
- **React Split 2.0.14** - 可调整大小的分割面板
- **Tauri API v2** - 前后端通信

### 后端

- **Tauri 2.x** - 跨平台桌面应用框架
- **Rust** - 系统级编程语言
- **sqlparser 0.58** - 专业 SQL 解析器
- **reqwest 0.12** - HTTP 客户端
- **openssl 0.10.73** - 加密库
- **chrono 0.4** - 时间处理

## 🚀 快速开始

### 环境要求

- Node.js 18+
- Rust 1.70+
- pnpm 包管理器

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
pnpm tauri dev
```

### 构建应用

```bash
# 构建前端
pnpm build

# 构建桌面应用
pnpm tauri build
```

## 📁 项目结构

```bash
devtools/                    # 项目根目录
├── .gitignore              # Git 忽略规则
├── LICENSE                 # MIT 许可证文件
├── README.md               # 英文文档
├── README_ZH.md           # 中文文档
├── index.html             # HTML 入口点
├── package.json           # Node.js 依赖和脚本
├── pnpm-lock.yaml         # pnpm 锁文件
├── public/                # 静态公共资源
│   ├── tauri.svg          # Tauri 徽标
│   └── vite.svg           # Vite 徽标
├── src-tauri/             # Tauri 后端 Rust 代码
│   ├── .gitignore         # Rust Git 忽略
│   ├── Cargo.lock         # Rust 依赖锁
│   ├── Cargo.toml         # Rust 项目配置
│   ├── build.rs           # Rust 构建脚本
│   ├── capabilities/      # Tauri 能力定义
│   │   └── default.json   # 默认能力
│   ├── icons/             # 应用图标（多种尺寸）
│   │   ├── 128x128.png
│   │   ├── 128x128@2x.png
│   │   ├── 32x32.png
│   │   ├── Square107x107Logo.png
│   │   ├── Square142x142Logo.png
│   │   ├── Square150x150Logo.png
│   │   ├── Square284x284Logo.png
│   │   ├── Square30x30Logo.png
│   │   ├── Square310x310Logo.png
│   │   ├── Square44x44Logo.png
│   │   ├── Square71x71Logo.png
│   │   ├── Square89x89Logo.png
│   │   ├── StoreLogo.png
│   │   ├── icon.icns
│   │   ├── icon.ico
│   │   └── icon.png
│   ├── src/               # Rust 源代码
│   │   ├── lib.rs         # 主库入口点
│   │   ├── main.rs        # 应用入口点
│   │   ├── tools/         # 后端工具实现
│   │   └── utils/         # 工具模块
│   └── tauri.conf.json    # Tauri 应用配置
├── src/                   # 前端 React/TypeScript 代码
│   ├── App.css            # 主应用样式
│   ├── App.tsx            # 主应用组件
│   ├── Toolbox.tsx        # 主导航和工具切换
│   ├── assets/            # 静态资源
│   │   └── react.svg      # React 徽标
│   ├── components/        # 共享 UI 组件
│   │   ├── common/        # 通用可重用组件
│   │   └── layouts/       # 布局组件
│   ├── hooks/             # 自定义 React hooks
│   │   ├── index.ts       # Hooks 统一导出
│   │   ├── useAsyncState.ts
│   │   ├── useCopyToClipboard.ts
│   │   ├── useDebounce.ts
│   │   ├── useTheme.ts
│   │   └── useToast.ts
│   ├── main.tsx           # React 应用入口点
│   ├── tools/             # 各个工具组件（19个工具）
│   │   ├── Base64Decode.tsx
│   │   ├── Base64Encode.tsx
│   │   ├── CertificateViewer.tsx
│   │   ├── IpInfo.tsx
│   │   ├── JsonFormatter.tsx
│   │   ├── JsonToGo.tsx
│   │   ├── JwtDecode.tsx
│   │   ├── JwtEncode.tsx
│   │   ├── PemToPfxConverter.tsx
│   │   ├── PfxToPemConverter.tsx
│   │   ├── RegexTester.tsx
│   │   ├── Settings.tsx
│   │   ├── SqlToEnt.tsx
│   │   ├── SqlToGo.tsx
│   │   ├── SslChecker.tsx
│   │   ├── SubnetCalculator.tsx
│   │   ├── TimestampConverter.tsx
│   ├── utils/             # 工具函数
│   │   ├── api.ts         # Tauri API 包装器
│   │   ├── globalShortcut.ts # 全局快捷键工具
│   │   └── index.ts       # 工具函数导出
│   └── vite-env.d.ts      # Vite 环境类型
├── tailwind.config.js     # Tailwind CSS 配置
├── tsconfig.json         # TypeScript 配置
├── tsconfig.node.json    # TypeScript node 配置
└── vite.config.ts        # Vite 构建配置
```

## 🎯 核心特性

### 前端架构

- **侧边栏导航模式** - 每个工具都是独立的 React 组件，布局一致
- **Monaco Editor 集成** - 大多数工具使用 Monaco Editor 进行文本输入/输出，支持语法高亮
- **多标签支持** - SQL 工具支持多表生成的标签界面
- **主题支持** - 深色/浅色模式，支持系统主题检测
- **分割面板布局** - 大多数工具使用 react-split 实现可调整大小的输入/输出面板

### 后端架构

- **Tauri 命令系统** - 前端通过 Tauri 命令与 Rust 后端通信
- **模块化工具结构** - 每个工具都有独立的 Rust 模块，具有标准化的错误处理
- **专业 SQL 解析** - 使用 sqlparser-rs (v0.58) 进行稳健的 SQL 解析，而非正则表达式
- **多方言 SQL 支持** - 支持 Generic、MySQL、PostgreSQL 和 SQLite 方言
- **集中式错误处理** - 所有工具使用统一的 DevToolError 和本地化消息

### SQL 工具增强

- **多表支持** - SQL 转 Go 和 SQL 转 Ent 工具都能解析多个 CREATE TABLE 语句
- **高级类型映射** - 正确处理无符号整数（uint8、uint16、uint32、uint64）
- **复数化逻辑** - 智能表名单数化，支持复杂复数形式
- **反引号处理** - 正确处理 MySQL 风格的反引号表名/列名
- **基于 AST 的解析** - 使用抽象语法树解析进行准确的 SQL 分析

## 📋 支持的 SQL 格式

工具支持各种 SQL 格式，包括：

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
