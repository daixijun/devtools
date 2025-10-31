# 项目上下文

## 项目目的

一个基于 Tauri 2 + React 构建的综合性桌面开发者工具箱应用程序。该应用程序为开发者提供了统一的界面来访问各种实用工具，包括编码/解码、加密、证书管理、网络工具、数据库转换和媒体格式转换器。

## 技术栈

- **前端**: TypeScript, React 18, Vite
- **桌面框架**: Tauri 2
- **后端**: Rust
- **样式**: Tailwind CSS 4，2 空格缩进
- **UI 组件**: 自定义组件 + Monaco Editor 集成
- **状态管理**: React hooks (useState, useEffect, 自定义 hooks)
- **构建工具**: Vite 6, TypeScript 5.6, pnpm 包管理器
- **开发**: 热模块替换，Tauri dev 服务器运行在 1420 端口

### 核心依赖

- `@tauri-apps/api` & `@tauri-apps/cli` - Tauri 桌面应用框架
- `@monaco-editor/react` - 代码编辑器集成
- `crypto-js`, `bcryptjs`, `jsrsasign` - 加密工具
- `js-yaml`, `jsonwebtoken`, `toml` - 数据格式处理
- `react-split` - 可调整大小的面板布局
- `@tailwindcss/vite` - Tailwind CSS 集成

## 项目约定

### 代码风格

- **文件命名**:
  - React 组件: 帕斯卡命名法 (例如，`JsonFormatter.tsx`)
  - 工具组件: 帕斯卡命名法在 `src/tools/` 目录中
  - Rust 文件: 蛇形命名法 (例如，`certificate_viewer.rs`)
- **缩进**: 所有文件使用 2 个空格
- **JSX 运行时**: 自动 (`react-jsx`)
- **TypeScript**: 严格模式启用，包含全面的代码检查
- **导入**: 排序和组织，移除未使用的导入
- **样式**: 仅使用 Tailwind 工具类，无内联样式
- **格式化**:
  - 前端: Prettier 格式化所有前端代码
  - Rust: Rustfmt 格式化所有 Rust 代码
- **Lint**:
  - 前端: ESLint 检查所有前端代码
  - Rust: Clippy 检查所有 Rust 代码

### 架构模式

- **模块化工具结构**: 每个工具都是 `src/tools/` 中独立的 React 组件
- **混合架构**: React 前端通过 Tauri 命令与 Rust 后端通信
- **命令注册**: Rust 命令通过 `#[tauri::command]` 暴露并在 `invoke_handler` 中注册
- **组件层次结构**: 工具组件 → 通用组件 → 布局组件
- **自定义 Hooks**: 可重用逻辑提取到自定义 hooks (例如，`useCopyToClipboard`, `useDebounce`)
- **工具层**: 共享的验证、错误处理和常用操作工具

### 测试策略

- **当前状态**: 尚未实现测试
- **计划测试**:
  - Web 测试: Vitest + React Testing Library (测试文件放在 `src/**/__tests__` 或 `*.test.tsx` 下)
  - Rust 测试: 模块文件中的单元测试，`src-tauri/tests/` 中的集成测试 (在 `src-tauri/` 目录中运行 `cargo test`)
  - Package.json: 引入 Web 测试时添加 `"test": "vitest"`

### Git 工作流

- **提交约定**: 带作用域前缀的传统提交
  - 示例: `feat:`, `fix:`, `refactor:`, `style:`, `docs:`, `tools:`
  - 作用域前缀: `tools`, `tauri`, `ui`, `utils` 等
  - 格式: `scope(description)` 使用祈使语气
- **分支**: 标准功能分支工作流
- **PR 要求**: 包含目的、关键更改、UI 更改的截图/GIF
- **构建验证**: 确保 PR 前 `pnpm build` 和 `pnpm tauri build` 成功

## 领域上下文

### 工具类别

1. **编码/解码**: Base64、URL 编码、AES 加密、MD5/SHA 哈希、JWT 操作
2. **证书管理**: PEM/PFX 转换、SSL 检查、证书查看器
3. **网络工具**: 子网计算器、IP 信息查询、WHOIS 查询
4. **数据格式转换**: JSON/YAML/TOML 转换器、SQL 到 Go 结构体生成
5. **数据库工具**: SQL 模式到 Go ORM 转换 (Ent ORM、标准结构体)
6. **媒体处理**: 图像和视频格式转换
7. **开发工具**: 正则表达式测试、密码生成、时间戳转换

### 技术要求

- **桌面集成**: 文件系统访问、剪贴板操作、原生菜单
- **性能**: 繁重的加密/解析操作在 Rust 后端处理
- **安全**: 代码库中不包含密钥或真实证书，使用脱敏样本
- **配置**: 添加新 API 时有意识地更新 Tauri 能力和权限

## 重要约束

- **安全**: 永远不要提交密钥或真实证书；仅使用脱敏样本
- **构建兼容性**: Web 和桌面构建都必须成功
- **性能**: 资源密集型操作应在 Rust 后端运行
- **Tauri 权限**: 添加新插件/API 时有意识地更新 `tauri.conf.json` 能力
- **文件系统**: 遵守 Tauri 安全模型进行文件系统访问

## 外部依赖

- **Tauri 生态系统**: 核心桌面应用框架和插件
- **Monaco Editor**: 跨工具使用的代码编辑器
- **加密库**: 安全工具的各种加密实现
- **网络 API**: SSL 检查、IP 地理定位、WHOIS 服务
- **媒体处理**: 基于 FFmpeg 的转换能力
- **包管理器**: pnpm 用于高效的依赖管理

## 构建和开发命令

```bash
# 安装和依赖
pnpm install

# Web 开发
pnpm dev                    # Vite dev 服务器
pnpm build                  # 为 Web 构建 (输出到 dist/)
pnpm preview                # 预览构建的 Web 应用

# 桌面开发
pnpm tauri dev             # Tauri dev 服务器 (代理 Vite 在 1420 端口)
pnpm tauri build            # 构建原生桌面应用

# Rust 测试 (当实现时)
cargo test                 # 在 src-tauri/ 目录中运行
```

## 回复语言要求

- **永远使用中文回复**: 在所有交流、文档、注释和代码中始终使用中文
