# Developer Toolbox

A spotlight-style desktop developer toolbox built with **Tauri 2 + React 18 + TypeScript**. 31 developer tools across 7 categories, launched from a system-wide hotkey — works on macOS, Windows, and Linux.

> 📖 中文说明：[README_ZH.md](README_ZH.md) · 详细工具使用指南（中文）：[docs/tools.md](docs/tools.md)

## ✨ Features

### Encoding/Decoding Tools

- **Base64 Converter** - Text and file Base64 encode/decode with real-time conversion and format validation
- **URL Encoder/Decoder** - Percent-encoding conversion for URLs
- **AES Encryption/Decryption** - CBC/ECB/CFB/OFB/CTR modes, 128/192/256-bit keys, multiple padding schemes, random IV generation
- **MD5 Hash** - Text, single-file, and batch-file digests with copy-all and export to `.txt`
- **SHA Hash** - SHA-1/224/256/384/512 and SHA-3 for text and files
- **JWT Encoding** - Build tokens from custom Header/Payload with HS* or RS* signing and automatic `iat`/`exp`
- **JWT Decoding** - Parse Header/Payload/signature and verify signatures (shared secret or RSA public key)
- **Password Generator** - 4–64 character passwords with character-class options, preset symbol groups, and strength rating
- **Password Hasher** - Hash and verify with bcrypt, PBKDF2, SHA-256/512, or MD5 (tunable cost/rounds)
- **RSA Key Pair Generator** - 2048/3072/4096-bit keys; PKCS#8/PKCS#1 private keys and PEM (SPKI)/PKCS#1/OpenSSH public keys

### Certificate Tools

- **Certificate Viewer** - Parse PEM/PFX certificates and full chains (leaf/intermediate/root), validity checks, chain completeness and **order detection** with one-click corrected-order copy
- **CSR Viewer** - Parse PKCS#10 requests: subject, public key, signature algorithm, requested extensions; optional private-key matching
- **CSR Generator** - Generate PKCS#10 CSRs with RSA (2048–4096) or ECC (P-256/384/521) keys and SAN extensions; downloads for CSR, private key, and public key
- **PEM to PFX Converter** - Convert PEM certificates (with optional encrypted private keys) to PFX/PKCS#12
- **PFX to PEM Converter** - Extract certificates and private keys from PFX/PKCS#12 files
- **SSL Certificate Checker** - Online detection with security score, SSL Labs grade, certificate chain, cipher suites, CVE list, and hardening recommendations

### Network Tools

- **Subnet Calculator** - IPv4/IPv6 CIDR computations: network/broadcast addresses, usable IP ranges, prefix details
- **IP Address Information Lookup** - Geolocation and ASN/org details for your local IP or any IPv4/IPv6, aggregated from multiple data sources
- **DNS Resolver** - Query A/AAAA/CNAME/MX/TXT/NS/SOA records against multiple DNS servers concurrently; single and batch reverse (PTR) lookups
- **Domain Whois Lookup** - Batch queries with RDAP-first multi-source fallback, 24-hour result cache, query history, and CSV/JSON export

### Data Format Tools

- **JSON Formatter** - Pretty-print, minify, and unescape JSON with syntax highlighting
- **Format Converter** - Convert between JSON, YAML, and TOML
- **JSON to Go Struct** - Generate Go structs with selectable tags (json/yaml/gorm/db/sql/toml/env/ini), Go 1.18 `any`, and Go 1.24 `omitzero` support
- **SQL to Go Struct** - Multi-table `CREATE TABLE` parsing with unsigned type mapping, pluralization, and backtick handling
- **SQL to Go Ent ORM** - Generate Ent schemas with edges, mixins, hooks, policy, soft delete, and UUID primary key options

### Media Format Conversion

- **Image Format Converter** - Convert between PNG/JPEG/WebP/BMP/GIF/TIFF/ICO/HEIC with resizing, EXIF removal, and batch mode
- **Image Preview** - Zoom/pan viewer accepting URLs, pasted images, and local files; re-encode and save-as other formats
- **Video Format Converter** - Convert between MP4/WebM/AVI/MKV and more via system FFmpeg, with availability check, batch mode, and per-file progress
- **Image Compressor** - Lossless optimization (oxipng/zopfli) or quality compression (TinyPNG-style palette quantization), proportional scaling, EXIF stripping, batch processing, and a before/after comparison slider

### Development Tools

- **Regular Expression Tester** - Live matching with 5 engines (rust/re2/pcre/golang/javascript), common flags, capture/named group details, and replace mode

### Time Tools

- **Timestamp Converter** - Bidirectional Unix timestamp conversion (seconds/milliseconds) across all IANA timezones with a live clock

### Other Tools

- **Settings** - Theme (light/dark/system), tray icon, autostart, start-minimized, close-to-tray, and customizable global hotkey

## 🧭 How It Works

1. Press the global hotkey (default **Option+Space** on macOS, **Alt+Space** elsewhere) to toggle the spotlight search bar
2. Type to filter tools by name, description, or keyword; use ↑/↓ + Enter to launch — each tool opens in its own resizable window
3. Press Esc to hide the window; reopen it anytime with the hotkey or from the tray icon

## 🛠️ Tech Stack

### Frontend

- **React 18.3** - Modern frontend framework
- **TypeScript 5.6** - Type-safe JavaScript
- **Monaco Editor 4.7** - Code editor (VS Code engine)
- **Tailwind CSS 4** - Utility-first CSS framework
- **React Split 2.0** - Resizable split panes
- **Tauri API v2** - Frontend-backend communication, plus dialog/fs/global-shortcut/opener/autostart plugins

### Backend

- **Tauri 2.x** - Cross-platform desktop framework with tray icon support
- **Rust** - Systems programming language
- **sqlparser 0.58** - Professional SQL parsing (AST-based)
- **reqwest 0.12 + rustls** - HTTP client for online tools
- **openssl (vendored) + x509-parser + rustls** - Certificate parsing and crypto
- **image 0.25 + oxipng + imagequant + nom-exif + libheif** - Image conversion, compression, and EXIF
- **dns-lookup + pcre2 + regex + chrono/chrono-tz** - Networking, regex engines, and time handling
- **FFmpeg** (external, optional) - Required only for video conversion

## 🚀 Quick Start

### Prerequisites

- Node.js 18+
- Rust 1.77+ (Tauri 2 minimum)
- pnpm package manager
- FFmpeg (optional, only for the video converter)

### Install Dependencies

```bash
pnpm install
```

### Development Mode

```bash
pnpm tauri dev
# or: make dev
```

### Build Application

```bash
# Build frontend (includes typecheck)
pnpm build

# Build desktop application
pnpm tauri build
```

### Checks

```bash
make check          # tsc + style lint + cargo check
make check-tsc      # TypeScript only
make check-styles   # design-system lint (slate palette, rounded-lg, duration-200)
make cargo-clippy   # Rust lints
```

## 📁 Project Structure

```bash
devtools/                        # Project root
├── Makefile                     # Dev/build/check workflow shortcuts
├── docs/                        # Documentation (tool usage guide, UI notes, plans)
├── scripts/
│   └── check-styles.cjs         # Tailwind/design-system consistency checks
├── public/                      # Static assets (app logo)
├── src-tauri/                   # Tauri (Rust) backend
│   ├── capabilities/            # Tauri capability definitions
│   ├── icons/                   # Application icons
│   ├── src/
│   │   ├── lib.rs               # Plugin setup + invoke_handler command registry
│   │   ├── main.rs              # Application entry point
│   │   ├── tools/               # One Rust module per backend tool
│   │   └── utils/               # crypto, error, validation, formatting helpers
│   └── tauri.conf.json          # Window/tray/build configuration
├── src/                         # Frontend React/TypeScript code
│   ├── App.tsx                  # Hash router: spotlight bar ↔ #/tool/<id> windows
│   ├── components/
│   │   ├── SpotlightSearch.tsx  # Main launcher (search + keyboard navigation)
│   │   ├── ToolWindow.tsx       # Tool window container (lazy loading + theme)
│   │   ├── common/              # Shared widgets (file upload, copy button, …)
│   │   ├── icons/               # Tool icons
│   │   ├── layouts/             # Layout components
│   │   └── templates/           # Tool page templates
│   ├── hooks/                   # useTheme, useToast, useCopyToClipboard, useToolWindow, …
│   ├── tools/
│   │   ├── registry.ts          # Single source of truth: tool metadata + window sizes
│   │   └── *.tsx                # 32 tool components (lazy-loaded)
│   ├── utils/
│   │   ├── api.ts               # Typed wrappers around Tauri invoke() commands
│   │   └── globalShortcut.ts    # Global hotkey registration helpers
│   └── main.tsx                 # React entry point
├── tailwind.config.js
└── vite.config.ts               # Dev server on fixed port 1453
```

## 🎯 Core Design

### Frontend Architecture

- **Spotlight Launcher** - A frameless search bar is the main window; tools open in separate windows sized per `registry.ts`
- **Tool Registry** - Every tool is declared once in `src/tools/registry.ts` (id, category, icon, keywords, window size) and lazy-loaded
- **Hash Routing** - `#/tool/<id>` renders a tool window; the root route renders the spotlight bar
- **Monaco Editor Integration** - Text-heavy tools use Monaco with syntax highlighting and split panes
- **Theme Support** - Dark/light/system themes, synced across windows in real time

### Backend Architecture

- **Tauri Command System** - Frontend calls Rust via typed `invoke()` wrappers in `src/utils/api.ts`
- **Modular Tool Structure** - Each backend tool is its own module with standardized error handling (`DevToolError` with localized messages)
- **Professional SQL Parsing** - sqlparser-rs builds an AST instead of using regex; supports Generic/MySQL/PostgreSQL/SQLite dialects
- **Native Image Pipeline** - Conversion, EXIF parsing, and compression run in Rust (image/oxipng/imagequant), not the browser

## 📋 Supported SQL Formats

The SQL tools handle formats including:

- MySQL with backticks: `CREATE TABLE \`users\` (\`id\` int unsigned...)`
- Multi-table statements in different dialects
- Complex plural table names like "pipelines" -> "Pipeline"
- Mixed-case unsigned types: "int unsigned", "INT UNSIGNED", "bigint unsigned"

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
