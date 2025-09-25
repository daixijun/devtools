# Developer Toolbox

A desktop developer toolbox application built with Tauri 2 + React

## ✨ Features

### Encoding/Decoding Tools

- **Base64 Encoding/Decoding** - Text encoding utilities
- **JWT Encoding/Decoding** - JWT token handling and validation

### Certificate Tools

- **Certificate Viewer** - PEM certificate analysis and information display
- **PEM to PFX Converter** - Convert PEM certificates to PFX format
- **PFX to PEM Converter** - Convert PFX certificates to PEM format
- **SSL Certificate Checker** - Check SSL certificate information from URLs

### Network Tools

- **IP/CIDR Calculator** - Subnet calculations and IP address manipulation
- **IP Address Information Lookup** - IP address details and geolocation

### Data Format Tools

- **JSON Formatter** - JSON formatting, validation, and pretty printing
- **Format Converter** - Multi-format conversion between JSON, YAML, and TOML
- **JSON to Go Struct** - Generate Go struct definitions from JSON

### Database Tools

- **SQL to Go Struct** - Generate Go struct definitions from SQL schema with multi-table support
- **SQL to Go Ent ORM** - Generate Go Ent ORM schema from SQL schema with multi-table support

### Other Tools

- **Timestamp Converter** - Unix timestamp conversion to human-readable dates
- **Regular Expression Tester** - Test and debug regular expressions with real-time matching
- **Settings** - Application configuration and preferences

## 🛠️ Tech Stack

### Frontend

- **React 18.3.1** - Modern frontend framework
- **TypeScript** - Type-safe JavaScript
- **Monaco Editor 4.7.0** - Code editor (VS Code engine)
- **Tailwind CSS 4.1.11** - Utility-first CSS framework
- **React Split 2.0.14** - Resizable split panes
- **Tauri API v2** - Frontend-backend communication

### Backend

- **Tauri 2.x** - Cross-platform desktop application framework
- **Rust** - Systems programming language
- **sqlparser 0.58** - Professional SQL parser
- **reqwest 0.12** - HTTP client
- **openssl 0.10.73** - Cryptographic library
- **chrono 0.4** - Time handling

## 🚀 Quick Start

### Prerequisites

- Node.js 18+
- Rust 1.70+
- pnpm package manager

### Install Dependencies

```bash
pnpm install
```

### Development Mode

```bash
pnpm tauri dev
```

### Build Application

```bash
# Build frontend
pnpm build

# Build desktop application
pnpm tauri build
```

## 📁 Project Structure

```bash
devtools/                    # Project root directory
├── .gitignore              # Git ignore rules
├── LICENSE                 # MIT License file
├── README.md               # English documentation
├── README_ZH.md           # Chinese documentation
├── index.html             # HTML entry point
├── package.json           # Node.js dependencies and scripts
├── pnpm-lock.yaml         # pnpm lock file
├── public/                # Static public assets
│   ├── tauri.svg          # Tauri logo
│   └── vite.svg           # Vite logo
├── src-tauri/             # Tauri backend Rust code
│   ├── .gitignore         # Git ignore for Rust
│   ├── Cargo.lock         # Rust dependencies lock
│   ├── Cargo.toml         # Rust project configuration
│   ├── build.rs           # Rust build script
│   ├── capabilities/      # Tauri capability definitions
│   │   └── default.json   # Default capabilities
│   ├── icons/             # Application icons (multiple sizes)
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
│   ├── src/               # Rust source code
│   │   ├── lib.rs         # Main library entry point
│   │   ├── main.rs        # Application entry point
│   │   ├── tools/         # Backend tool implementations
│   │   └── utils/         # Utility modules
│   └── tauri.conf.json    # Tauri application configuration
├── src/                   # Frontend React/TypeScript code
│   ├── App.css            # Main application styles
│   ├── App.tsx            # Main application component
│   ├── Toolbox.tsx        # Main navigation and tool switching
│   ├── assets/            # Static assets
│   │   └── react.svg      # React logo
│   ├── components/        # Shared UI components
│   │   ├── common/        # Common reusable components
│   │   └── layouts/       # Layout components
│   ├── hooks/             # Custom React hooks
│   │   ├── index.ts       # Hooks barrel export
│   │   ├── useAsyncState.ts
│   │   ├── useCopyToClipboard.ts
│   │   ├── useDebounce.ts
│   │   ├── useTheme.ts
│   │   ├── useToast.ts
│   ├── main.tsx           # React application entry point
│   ├── tools/             # Individual tool components (19 tools)
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
│   ├── utils/             # Utility functions
│   │   ├── api.ts         # Tauri API wrapper
│   │   ├── globalShortcut.ts # Global shortcut utilities
│   │   └── index.ts       # Utility exports
│   └── vite-env.d.ts      # Vite environment types
├── tailwind.config.js     # Tailwind CSS configuration
├── tsconfig.json         # TypeScript configuration
├── tsconfig.node.json    # TypeScript node configuration
└── vite.config.ts        # Vite build configuration
```

## 🎯 Core Features

### Frontend Architecture

- **Sidebar Navigation Pattern** - Each tool is a separate React component with consistent layout
- **Monaco Editor Integration** - Most tools use Monaco Editor for text input/output with syntax highlighting
- **Multi-tab Support** - SQL tools support multi-table generation with tabbed interface
- **Theme Support** - Dark/light mode with system theme detection
- **Split Pane Layout** - Most tools use react-split for resizable input/output panels

### Backend Architecture

- **Tauri Command System** - Frontend communicates with Rust backend via Tauri commands
- **Modular Tool Structure** - Each tool has its own Rust module with standardized error handling
- **Professional SQL Parsing** - Uses sqlparser-rs (v0.58) for robust SQL parsing instead of regex
- **Multi-dialect SQL Support** - Supports Generic, MySQL, PostgreSQL, and SQLite dialects
- **Centralized Error Handling** - All tools use unified DevToolError with localized messages
- **Standardized Response Format** - All commands return DevToolResponse<T> for consistency

### SQL Tools Enhancements

- **Multi-table Support** - Both SQL to Go and SQL to Ent tools can parse multiple CREATE TABLE statements
- **Advanced Type Mapping** - Correctly handles unsigned integers (uint8, uint16, uint32, uint64)
- **Pluralization Logic** - Intelligent table name singularization with support for complex plurals
- **Backtick Handling** - Properly processes MySQL-style backticked table/column names
- **AST-based Parsing** - Uses Abstract Syntax Tree parsing for accurate SQL analysis

## 📋 Supported SQL Formats

Tools support various SQL formats including:

- MySQL with backticks: `CREATE TABLE \`users\` (\`id\` int unsigned...)`
- Multi-table statements with different dialects
- Complex plural table names like "pipelines" -> "Pipeline"
- Mixed case unsigned types: "int unsigned", "INT UNSIGNED", "bigint unsigned"

## 🤝 Contributing

Issues and Pull Requests are welcome!

## 📄 License

[MIT License](LICENSE)

## ⚠️ Disclaimer

This software is provided "as is" without warranty of any kind, express or implied. The developers and contributors shall not be held liable for any damages arising from the use of this software, including but not limited to:

- Data loss or corruption
- Security vulnerabilities
- Incorrect output or functionality
- Compatibility issues with other software

Users are advised to:

- Test the software thoroughly before using it in production environments
- Verify the accuracy of generated code and conversions
- Implement appropriate security measures when handling sensitive data
- Keep backups of important data

This tool is intended for development and testing purposes only. Always review and validate generated code before deployment.

## 🔗 Related Links

- [Tauri Official Documentation](https://tauri.app/)
- [React Official Documentation](https://react.dev/)
- [TypeScript Official Documentation](https://www.typescriptlang.org/)
