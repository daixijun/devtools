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
- **JSON/YAML Converter** - Bidirectional format conversion between JSON and YAML
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
src/                    # Frontend React/TypeScript code
├── App.tsx            # Main application component
├── Toolbox.tsx        # Main navigation and tool switching component
├── tools/             # Individual tool components (19 tools)
│   ├── Base64Decode.tsx          # Base64 decoding tool
│   ├── Base64Encode.tsx          # Base64 encoding tool
│   ├── CertificateViewer.tsx     # Certificate analysis tool
│   ├── IpInfo.tsx                # IP address information lookup
│   ├── JsonFormatter.tsx         # JSON formatting and validation
│   ├── JsonToGo.tsx              # JSON to Go struct conversion
│   ├── JsonToYaml.tsx            # JSON to YAML conversion
│   ├── JwtDecode.tsx             # JWT token decoding
│   ├── JwtEncode.tsx             # JWT token encoding
│   ├── PemToPfxConverter.tsx     # PEM to PFX certificate conversion
│   ├── PfxToPemConverter.tsx     # PFX to PEM certificate conversion
│   ├── RegexTester.tsx           # Regular expression testing tool
│   ├── Settings.tsx              # Application settings and preferences
│   ├── SqlToEnt.tsx              # SQL to Ent ORM schema generation
│   ├── SqlToGo.tsx               # SQL to Go struct conversion
│   ├── SslChecker.tsx            # SSL certificate checking
│   ├── SubnetCalculator.tsx      # IP/CIDR calculation
│   ├── TimestampConverter.tsx     # Unix timestamp conversion
│   └── YamlToJson.tsx            # YAML to JSON conversion
├── components/        # Shared components
│   ├── common/        # Common components
│   │   ├── Button.tsx
│   │   ├── Card/
│   │   ├── CodeEditor/
│   │   ├── ErrorMessage.tsx
│   │   ├── FileUpload.tsx
│   │   ├── InputField.tsx
│   │   ├── PageHeader.tsx
│   │   └── Toast/
│   └── layouts/       # Layout components
│       ├── SplitEditorLayout.tsx
│       └── ToolLayout.tsx
├── hooks/             # Custom React hooks
│   ├── useAsyncState.ts
│   ├── useCopyToClipboard.ts
│   ├── useDebounce.ts
│   ├── useTheme.ts
│   └── useToast.ts
└── utils/             # Utility functions

src-tauri/             # Tauri backend Rust code
├── src/               # Rust source files
│   ├── main.rs        # Application entry point
│   ├── lib.rs         # Main application setup with Tauri command handlers
│   ├── tray.rs        # System tray implementation
│   ├── tools/         # Backend tool implementations
│   │   ├── autostart.rs               # Auto-start functionality
│   │   ├── certificate_converter.rs   # Certificate format conversion
│   │   ├── certificate_viewer.rs      # Certificate parsing and analysis
│   │   ├── global_shortcut.rs         # Global shortcut handling
│   │   ├── ip_info.rs                 # IP address information lookup
│   │   ├── json_to_go.rs              # JSON to Go struct conversion
│   │   ├── regex_tester.rs            # Regular expression testing
│   │   ├── sql_to_ent.rs              # SQL to Ent ORM schema generation
│   │   ├── sql_to_go.rs               # SQL to Go struct conversion
│   │   ├── ssl_checker.rs             # SSL certificate checking
│   │   └── system_settings.rs         # System settings management
│   └── utils/         # Shared utility modules
│       ├── code_formatter.rs
│       ├── command_handler.rs
│       ├── crypto.rs
│       ├── error.rs
│       ├── response.rs
│       ├── string_utils.rs
│       └── validation.rs
├── tauri.conf.json    # Tauri configuration file
└── icons/             # Application icons
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
