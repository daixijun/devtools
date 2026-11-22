//! EN 词条：registry 元数据（8 分类名 + 31 工具名与描述）。
//!
//! 与 `src/registry.rs` 一一对应；`i18n/mod.rs` 的测试强制全量覆盖
//! （新增工具漏词条会 fail test_en_registry_covers_all_tools_and_categories）。

/// registry 词条表。
pub const EN_REGISTRY: &[(&str, &str)] = &[
    // ---- 分类 ----
    ("编码/解码", "Encoding / Decoding"),
    ("证书工具", "Certificates"),
    ("网络工具", "Network"),
    ("数据格式转换", "Data Format"),
    ("媒体格式转换", "Media"),
    ("开发工具", "Developer"),
    ("时间工具", "Time"),
    ("设置", "Settings"),
    // ---- encoding 编码/解码 ----
    (
        "Base64 编解码",
        "Base64 Encode/Decode",
    ),
    (
        "Base64 编码和解码工具，支持文本的 Base64 转换与格式校验",
        "Encode and decode text with Base64, plus format validation",
    ),
    ("URL 编解码", "URL Encode/Decode"),
    (
        "URL 编码和解码工具，对 URL 中的特殊字符进行百分号编码转换",
        "Percent-encode or decode special characters in URLs",
    ),
    ("AES 加密/解密", "AES Encrypt/Decrypt"),
    (
        "AES 对称加密和解密工具，支持 CBC/ECB/CFB/OFB/CTR 模式与多种填充，结果为 Base64",
        "AES symmetric encryption with CBC/ECB/CFB/OFB/CTR modes and multiple paddings; output as Base64",
    ),
    ("MD5 加密", "MD5 Hash"),
    (
        "MD5 哈希生成工具，计算文本或文件的 MD5 摘要值",
        "Compute MD5 digests for text or files",
    ),
    ("SHA 哈希加密", "SHA Hash"),
    (
        "SHA 系列哈希生成工具，支持 SHA-1/SHA-256/SHA-512 等算法",
        "SHA family hashing with SHA-1/SHA-256/SHA-512 and more",
    ),
    ("JWT 工具", "JWT Tool"),
    (
        "JWT Token 生成与解码工具，创建、解析和验证 JSON Web Token",
        "Create, decode and verify JSON Web Tokens",
    ),
    ("密码生成器", "Password Generator"),
    (
        "安全密码生成工具，可自定义长度、字符类型等选项生成强密码",
        "Generate strong passwords with custom length and character sets",
    ),
    ("密码加密验证", "Password Hash & Verify"),
    (
        "密码哈希和验证工具，支持 bcrypt 等算法对密码进行安全加密和比对",
        "Hash and verify passwords with bcrypt and more",
    ),
    ("RSA 密钥对生成", "RSA Key Pair Generator"),
    (
        "RSA 公私钥对生成工具，私钥与公钥统一格式选择：PKCS#8（通用）/ PKCS#1（传统）/ OpenSSH",
        "Generate RSA key pairs in PKCS#8, PKCS#1 or OpenSSH format",
    ),
    // ---- certificate 证书工具 ----
    ("证书查看器", "Certificate Viewer"),
    (
        "PEM 证书解析工具，查看证书的详细信息、有效期和公钥等",
        "Parse PEM certificates to inspect details, validity and public keys",
    ),
    ("CSR 查看", "CSR Viewer"),
    (
        "PKCS#10 证书签名请求（CSR）解析工具，查看主题、公钥、签名算法与请求的扩展信息",
        "Parse PKCS#10 CSRs: subject, public key, signature algorithm and extensions",
    ),
    ("CSR 生成", "CSR Generator"),
    (
        "生成 PKCS#10 证书签名请求（CSR）与配套私钥，支持 RSA/ECC 密钥和 SAN 扩展",
        "Generate PKCS#10 CSRs with a private key; RSA/ECC keys and SAN supported",
    ),
    ("PEM 转 PFX", "PEM to PFX"),
    (
        "将 PEM 格式证书转换为 PFX/PKCS#12 格式，方便在 Windows 等环境中使用",
        "Convert PEM certificates to PFX/PKCS#12 for Windows and beyond",
    ),
    ("PFX 转 PEM", "PFX to PEM"),
    (
        "将 PFX/PKCS#12 格式证书转换为 PEM 格式，提取证书和私钥",
        "Convert PFX/PKCS#12 bundles to PEM, extracting certificates and keys",
    ),
    ("在线 SSL 检测", "SSL Checker"),
    (
        "在线 SSL 证书检测工具，检查网站的 SSL 证书信息和到期时间",
        "Check a website's SSL certificate details and expiry online",
    ),
    // ---- network 网络工具 ----
    ("子网掩码计算器", "Subnet Calculator"),
    (
        "子网掩码计算工具，支持 IP 地址与子网掩码的网络规划计算",
        "Subnet planning math for IP addresses and netmasks",
    ),
    ("IP 地址信息查询", "IP Address Lookup"),
    (
        "IP 地址信息查询工具，获取 IP 地址的地理位置、运营商等详细信息",
        "Look up geolocation, ISP and more for an IP address",
    ),
    ("DNS 解析工具", "DNS Lookup"),
    (
        "DNS 记录查询工具，解析域名对应的 A/AAAA/CNAME/MX/TXT 等记录",
        "Resolve A/AAAA/CNAME/MX/TXT and other DNS records for a domain",
    ),
    ("域名 Whois 查询", "Whois Lookup"),
    (
        "域名 Whois 信息查询工具，查看域名的注册信息、到期时间和注册商",
        "Query Whois registration info, expiry and registrar for a domain",
    ),
    // ---- dataformat 数据格式转换 ----
    ("JSON 格式化", "JSON Formatter"),
    (
        "JSON 格式化工具，支持美化、压缩、验证 JSON 数据并高亮显示",
        "Pretty-print, minify and validate JSON with syntax highlighting",
    ),
    ("格式转换器", "Format Converter"),
    (
        "多格式转换工具，支持 JSON、YAML、TOML 等数据格式之间的互相转换",
        "Convert between JSON, YAML, TOML and other data formats",
    ),
    ("JSON 转 Go 结构体", "JSON to Go Struct"),
    (
        "将 JSON 数据自动生成 Go 语言结构体定义，支持嵌套类型",
        "Generate Go struct definitions from JSON, nested types included",
    ),
    ("SQL 转 Go 结构体", "SQL to Go Struct"),
    (
        "将 SQL CREATE TABLE 语句转换为 Go 语言结构体定义，支持多表解析",
        "Convert SQL CREATE TABLE statements into Go structs, multiple tables supported",
    ),
    ("SQL 转 Go Ent ORM", "SQL to Go Ent ORM"),
    (
        "将 SQL DDL 语句转换为 Go Ent ORM Schema 代码，支持多表和复杂类型映射",
        "Convert SQL DDL into Go Ent ORM schema code, multi-table and complex mappings supported",
    ),
    // ---- media 媒体格式转换 ----
    ("图片格式转换", "Image Converter"),
    (
        "图片格式转换工具，支持 PNG、JPEG、WebP、BMP 等常见图片格式互转",
        "Convert between PNG, JPEG, WebP, BMP and other common image formats",
    ),
    ("图片预览器", "Image Preview"),
    (
        "图片预览和信息查看工具，支持拖拽上传、缩放浏览和图片元数据查看",
        "Preview images with drag-and-drop, zoom and metadata inspection",
    ),
    ("视频格式转换", "Video Converter"),
    (
        "视频格式转换工具，支持 MP4、WebM、AVI 等常见视频格式互转",
        "Convert between MP4, WebM, AVI and other common video formats",
    ),
    ("图片压缩工具", "Image Compressor"),
    (
        "PNG/JPG 无损优化与质量压缩，支持等比缩放、批量处理、压缩前后滑块对比预览",
        "Lossless PNG/JPG optimization and quality compression with scaling, batching and a before/after slider",
    ),
    // ---- developer 开发工具 ----
    ("正则表达式测试器", "Regex Tester"),
    (
        "正则表达式在线测试工具，实时匹配高亮并显示捕获组结果",
        "Test regular expressions with live match highlighting and capture groups",
    ),
    // ---- time 时间工具 ----
    ("时间戳转换", "Timestamp Converter"),
    (
        "Unix 时间戳转换工具，支持时间戳与可读日期的双向转换，支持秒级和毫秒级",
        "Convert Unix timestamps to dates and back, seconds or milliseconds",
    ),
    // ---- settings 设置 ----
    (
        "应用配置和偏好设置，全局快捷键、主题等",
        "App configuration and preferences: language, theme and more",
    ),
];
