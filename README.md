# Developer Toolbox

A desktop developer toolbox built with **pure Rust + GPUI** (native GPU-rendered UI): a main window with a category sidebar + tool content area, plus a tray and native app menu. 32 developer tools across 8 categories — macOS only.

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

- **Settings** - Theme (light/dark/system), tray icon, autostart, start-minimized, close-to-tray

## 🧭 How It Works

1. Launch the app (or open it from the tray icon)
2. Type to filter tools by name, description, or keyword; use ↑/↓ + Enter to launch — each tool opens in its own resizable window
3. Closing the window hides it to the tray (when tray is enabled); reopen it anytime from the tray icon

## 🛠️ Tech Stack

### UI (GPUI)

- **gpui (gpui-pre snapshot)** - Zed's native GPU-rendered UI framework (Metal on macOS)
- **gpui-component 0.10** - longbridge component library (inputs, editor, select, buttons, notifications, resizable panes)
- **tray-icon + muda** - system tray menu; **rfd** - native file dialogs
- **rfd** - native file dialogs

### Core (devtools-core)

- **Rust 2021** - All business logic, zero framework coupling
- **sqlparser 0.58** - Professional SQL parsing (AST-based)
- **reqwest 0.12 + rustls** - HTTP client for online tools
- **openssl (vendored) + x509-parser + rustls** - Certificate parsing and crypto
- **image 0.25 + oxipng + imagequant + nom-exif + libheif** - Image conversion, compression, and EXIF
- **aes/cbc/ctr/cfb/ecb + sha1/sha2/sha3/md-5/hmac/pbkdf2/bcrypt** - Crypto (byte-compatible with the historical JS implementation)
- **dns-lookup + pcre2 + regex + chrono/chrono-tz** - Networking, regex engines, and time handling
- **FFmpeg** (external, optional) - Required only for video conversion
- **Rust** - Systems programming language
- **sqlparser 0.58** - Professional SQL parsing (AST-based)
- **reqwest 0.12 + rustls** - HTTP client for online tools
- **openssl (vendored) + x509-parser + rustls** - Certificate parsing and crypto
- **image 0.25 + oxipng + imagequant + nom-exif + libheif** - Image conversion, compression, and EXIF
- **dns-lookup + pcre2 + regex + chrono/chrono-tz** - Networking, regex engines, and time handling
- **FFmpeg** (external, optional) - Required only for video conversion

## 🚀 Quick Start

### Prerequisites

- Rust 1.85+ (workspace, edition 2021)
- Apple Metal Toolchain (`xcodebuild -downloadComponent MetalToolchain`) — required by gpui shaders
- FFmpeg (optional, only for the video converter)

### Development Mode

```bash
make run            # run (auto watch + restart if cargo-watch is installed)
```

### Build Application

```bash
make app            # package target/DevTools.app (debug)
make app-release    # package target/DevTools.app (release)
```

### Checks

```bash
make check          # cargo check + clippy (zero-warning) + core tests
make test           # unit tests incl. crypto golden vectors
make fmt            # cargo fmt
```

## 📁 Project Structure

```bash
devtools/                        # Project root
├── Cargo.toml                   # Workspace (pins gpui / gpui-platform / gpui-component)
├── Makefile                     # run/check/app workflow shortcuts
├── scripts/
│   └── build-app.sh             # DevTools.app packaging (Info.plist + icns)
├── crates/
│   ├── core/                    # devtools-core: pure business library (no UI)
│   │   ├── src/registry.rs      # 32-tool metadata registry (single source of truth)
│   │   ├── src/config.rs        # Shared config JSON (backward compatible)
│   │   ├── src/crypto.rs        # Byte-compatible port of the former cipher.ts/jwt.ts
│   │   ├── src/text_tools/      # base64 / url / password / timestamp / subnet / format
│   │   ├── src/tools/           # 16 tool logic modules (ssl, certs, dns, whois, media, sql…)
│   │   └── src/utils/           # shared helpers + DevToolError
│   └── app/                     # devtools-app: GPUI shell (bin: devtools-gpui)
│       ├── src/main.rs          # Startup flow (config → theme → core_rt → app menu → main window → tray)
│       ├── src/windows.rs       # WindowManager global (main window + about window)
│       ├── src/main_window.rs   # Main window: sidebar nav (search + categories) + tool content area
│       ├── src/app_menu.rs      # macOS app menu (About/Settings/Edit/Window/Help)
│       ├── src/tool_window.rs   # Tool window host (header + page view)
│       ├── src/tray.rs          # Tray icon + menu pump
│       ├── src/macos_window_ctl.rs  # unsafe objc: per-window show/hide (only unsafe file)
│       ├── src/core_rt.rs       # tokio runtime bridge for async core logic
│       ├── src/components/      # UI component layer (ui/notify/select)
│       └── src/tools/           # 32 tool pages, one file per tool
└── docs/plans/                  # Migration design + tool UI contracts
```

## 🎯 Core Design

- **Main Window** - Category sidebar (search + grouped tools) on the left, tool content on the right
- **Tool Registry** - Every tool is declared once in `crates/core/src/registry.rs` (id, category, keywords, window size)
- **Layered Architecture** - `devtools-core` holds all logic (pure functions + async), the GPUI shell only renders and forwards state; async core calls run on a background tokio Runtime bridged into the UI executor
- **Component Layer** - Shared widgets (cards, fields, banners, toasts, selects) wrap gpui-component so all 32 tool pages stay consistent
- **Theme Support** - Dark/light/system themes with live system-appearance following; theme is persisted to the shared config JSON
- **Config Compatibility** - `devtools-config.json` fields are add-only with serde defaults for backward compatibility
- **Crypto Compatibility** - `devtools-core/src/crypto.rs` reproduces the historical semantics byte-for-byte (AES 5 modes × 6 paddings, Keccak-512 as "SHA3", PBKDF2-HMAC-SHA256, JWT HS/RS, bcrypt), verified against golden vectors

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

- [gpui-component Documentation](https://longbridge.github.io/gpui-component/)
- [Rust Official Documentation](https://www.rust-lang.org/)
- [TypeScript Official Documentation](https://www.typescriptlang.org/)
