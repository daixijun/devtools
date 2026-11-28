//! EN 词条：业务层（tools/）产生的展示文案。
//!
//! 首批覆盖 ssl_checker：安全发现项（名称/描述/证据/修复建议）、证书链
//! 校验错误、握手验证问题、评分说明与连接错误。动态句子 key 带 `{}` 槽位，
//! 经 `tf()` 按顺序填充（槽位数须与实参数一致）。

/// 业务层文案词条表。
pub const EN_TOOLS: &[(&str, &str)] = &[
    // ---- 连接错误 ----
    ("域名不能为空", "Domain name cannot be empty"),
    ("无效的域名", "Invalid domain name"),
    ("TCP 连接失败: {}", "TCP connection failed: {}"),
    ("TCP 连接超时（10 秒）", "TCP connection timed out (10s)"),
    ("TLS 握手超时（10 秒）", "TLS handshake timed out (10s)"),
    ("TLS 连接失败: {}", "TLS connection failed: {}"),
    ("未找到证书", "No certificate found"),
    // ---- 握手验证问题（DiagnosticsCertVerifier）----
    ("证书已过期", "Certificate has expired"),
    ("证书尚未生效", "Certificate is not yet valid"),
    (
        "证书链无法锚定到受信任的根 CA（自签名或非公认 CA 签发）",
        "Certificate chain cannot be anchored to a trusted root CA (self-signed or issued by an untrusted CA)",
    ),
    (
        "证书不包含当前访问的域名（SAN 不匹配）",
        "Certificate does not cover the requested domain (SAN mismatch)",
    ),
    ("证书已被吊销", "Certificate has been revoked"),
    ("证书包含未被处理的 critical 扩展", "Certificate contains an unhandled critical extension"),
    (
        "证书签名无法通过其颁发者公钥验证",
        "Certificate signature cannot be verified with its issuer's public key",
    ),
    ("证书验证失败", "Certificate verification failed"),
    ("{}；标准验证错误：{}", "{}; standard verification error: {}"),
    (
        "TLS 1.2 握手签名验证失败；标准验证错误：{}",
        "TLS 1.2 handshake signature verification failed; standard verification error: {}",
    ),
    (
        "TLS 1.3 握手签名验证失败；标准验证错误：{}",
        "TLS 1.3 handshake signature verification failed; standard verification error: {}",
    ),
    // ---- 证书链校验 ----
    ("解析证书 {} 时出错: {}", "Failed to parse certificate {}: {}"),
    (
        "证书链中断: {} 的颁发者与 {} 的主体不匹配",
        "Broken certificate chain: issuer of {} does not match subject of {}",
    ),
    ("使用已弃用的SHA-1签名算法", "Uses deprecated SHA-1 signature algorithm"),
    ("RSA密钥长度过短: {} 位", "RSA key too short: {} bits"),
    ("EC密钥长度过短: {} 位", "EC key too short: {} bits"),
    ("未知受信任根 CA", "Unknown trusted root CA"),
    ("{}（服务器已发送）", "{} (sent by the server)"),
    (
        "{}（服务器未发送根证书，由客户端信任库提供，属标准配置）",
        "{} (root certificate not sent by the server; provided by the client trust store, standard configuration)",
    ),
    ("{}（未能匹配到受信任的根 CA）", "{} (no trusted root CA matched)"),
    // ---- 证书类发现 ----
    (
        "过期的证书会导致所有现代浏览器直接拒绝连接。",
        "An expired certificate is rejected outright by all modern browsers.",
    ),
    ("证书已于 {} 过期（{} 天前）", "Certificate expired on {} ({} days ago)"),
    ("有效期至 {}", "Valid until {}"),
    ("立即更换证书。建议使用 Let's Encrypt certbot 等工具实现自动化续期。",
     "Replace the certificate immediately. Consider automated renewal with Let's Encrypt certbot or similar tools."),
    ("证书即将过期", "Certificate is about to expire"),
    ("证书将在 30 天内过期，需安排续期以避免服务中断。",
     "The certificate expires within 30 days; schedule renewal to avoid service disruption."),
    ("证书将于 {} 天后过期（{}）", "Certificate expires in {} days ({})"),
    ("尽快续期证书，并配置到期监控或自动续期。",
     "Renew the certificate soon and configure expiry monitoring or automatic renewal."),
    ("使用已破解的 MD5 签名算法", "Uses the broken MD5 signature algorithm"),
    (
        "MD5 签名可被伪造（已有选择前缀碰撞的实践案例），攻击者可制作假证书。",
        "MD5 signatures can be forged (chosen-prefix collisions have been demonstrated in practice), allowing attackers to craft fake certificates.",
    ),
    ("证书签名为 {}", "Certificate signature is {}"),
    ("立即更换为 SHA-256 或更强的签名算法。",
     "Replace immediately with SHA-256 or a stronger signature algorithm."),
    ("使用已弃用的 SHA-1 签名算法", "Uses the deprecated SHA-1 signature algorithm"),
    (
        "SHA-1 已可被构造碰撞，所有主流浏览器自 2017 年起拒绝 SHA-1 证书。",
        "SHA-1 collisions can now be constructed; all major browsers have rejected SHA-1 certificates since 2017.",
    ),
    ("更换为 SHA-256 或更强的签名算法。", "Replace with SHA-256 or a stronger signature algorithm."),
    ("证书密钥长度不足", "Certificate key size is too small"),
    (
        "{} 位{}密钥不符合当前安全标准（RSA 至少 2048 位 / EC 至少 256 位）。",
        "A {}-bit {} key does not meet current security standards (RSA at least 2048 bits / EC at least 256 bits).",
    ),
    ("证书使用 {} 密钥，长度 {} 位", "Certificate uses an {} key of {} bits"),
    ("{} 密钥 {} 位", "{} key, {} bits"),
    ("更换为至少 2048 位的 RSA 密钥，推荐 3072 位以上。",
     "Replace with an RSA key of at least 2048 bits; 3072+ recommended."),
    ("更换为至少 256 位的 ECC 密钥（如 P-256）。",
     "Replace with an ECC key of at least 256 bits (e.g. P-256)."),
    // ---- 握手验证补充发现项 ----
    ("证书链不受信任", "Certificate chain is not trusted"),
    (
        "证书链无法锚定到客户端信任库中的根 CA（自签名或非公认 CA 签发），现代浏览器会直接拒绝连接。",
        "The certificate chain cannot be anchored to a root CA in the client trust store (self-signed or issued by an untrusted CA); modern browsers reject the connection outright.",
    ),
    (
        "使用受信任 CA 签发的证书；内网/自建服务可改用企业受信 CA 签发，或将根证书分发安装到客户端信任库。",
        "Use a certificate issued by a trusted CA; for internal/self-hosted services, issue via an enterprise trusted CA or distribute and install the root certificate into client trust stores.",
    ),
    ("证书域名不匹配", "Certificate domain mismatch"),
    ("证书的 SAN 不包含当前访问的域名，浏览器会直接拒绝连接。",
     "The certificate SAN does not include the requested domain; browsers reject the connection outright."),
    ("更换为覆盖该域名（含通配符）的证书，或在证书中补充对应 SAN 条目。",
     "Replace with a certificate covering the domain (including wildcards) or add the corresponding SAN entry."),
    // ---- 协议类发现 ----
    ("支持已废止的 SSL 2.0 协议", "Supports the deprecated SSL 2.0 protocol"),
    (
        "SSL 2.0 存在多个根本性设计缺陷，是 DROWN 攻击的基础，RFC 6176 已于 2011 年废止该协议。",
        "SSL 2.0 has multiple fundamental design flaws and is the basis of the DROWN attack; RFC 6176 deprecated it in 2011.",
    ),
    (
        "向服务器发送 SSL 2.0 握手探测，服务器以 SSLv2 ServerHello 应答",
        "Sent an SSL 2.0 handshake probe; the server responded with an SSLv2 ServerHello",
    ),
    (
        "完全禁用 SSL 2.0。现代服务器软件默认已移除该协议，请检查是否使用了过旧的服务器软件或代理。",
        "Disable SSL 2.0 entirely. Modern server software has removed it by default; check whether the server or proxy software is outdated.",
    ),
    ("支持已废止的 SSL 3.0 协议", "Supports the deprecated SSL 3.0 protocol"),
    (
        "SSL 3.0 受 POODLE 填充预言攻击影响，RFC 7568 已于 2015 年废止该协议。",
        "SSL 3.0 is vulnerable to the POODLE padding-oracle attack; RFC 7568 deprecated it in 2015.",
    ),
    (
        "向服务器发送 SSL 3.0 握手探测，服务器精确协商 SSL 3.0",
        "Sent an SSL 3.0 handshake probe; the server negotiated exactly SSL 3.0",
    ),
    ("完全禁用 SSL 3.0，仅保留 TLS 1.2 及以上版本。",
     "Disable SSL 3.0 entirely and keep only TLS 1.2 and above."),
    ("支持已废弃的 TLS 1.0 协议", "Supports the deprecated TLS 1.0 protocol"),
    (
        "TLS 1.0 已于 2021 年被 RFC 8996 正式废弃，主流合规标准（PCI DSS 等）均要求禁用；配合 CBC 套件时受 BEAST 影响。",
        "TLS 1.0 was formally deprecated by RFC 8996 in 2021 and major compliance standards (PCI DSS, etc.) require disabling it; with CBC suites it is vulnerable to BEAST.",
    ),
    (
        "向服务器发送 TLS 1.0 握手探测，服务器精确协商 TLS 1.0",
        "Sent a TLS 1.0 handshake probe; the server negotiated exactly TLS 1.0",
    ),
    (
        "向服务器发送 TLS 1.0 握手探测，服务器精确协商 TLS 1.0，且接受 CBC 模式套件（BEAST 利用前提）：{}",
        "Sent a TLS 1.0 handshake probe; the server negotiated exactly TLS 1.0 and accepts CBC suites (BEAST prerequisite): {}",
    ),
    ("禁用 TLS 1.0，仅保留 TLS 1.2 及以上版本。",
     "Disable TLS 1.0 and keep only TLS 1.2 and above."),
    ("支持已废弃的 TLS 1.1 协议", "Supports the deprecated TLS 1.1 protocol"),
    (
        "TLS 1.1 已于 2021 年被 RFC 8996 正式废弃，应迁移到 TLS 1.2 及以上版本。",
        "TLS 1.1 was formally deprecated by RFC 8996 in 2021; migrate to TLS 1.2 and above.",
    ),
    (
        "向服务器发送 TLS 1.1 握手探测，服务器精确协商 TLS 1.1",
        "Sent a TLS 1.1 handshake probe; the server negotiated exactly TLS 1.1",
    ),
    ("禁用 TLS 1.1，仅保留 TLS 1.2 及以上版本。",
     "Disable TLS 1.1 and keep only TLS 1.2 and above."),
    ("启用 TLS 级压缩", "TLS-level compression is enabled"),
    (
        "TLS 压缩是 CRIME 信息泄露攻击的前提，攻击者可据此恢复会话 Cookie 等明文内容。",
        "TLS compression is a prerequisite of the CRIME information-leakage attack, letting an attacker recover plaintext such as session cookies.",
    ),
    (
        "握手探测中服务器在 ServerHello 中选择了 zlib 压缩方法",
        "During handshake probing the server selected the zlib compression method in the ServerHello",
    ),
    ("禁用 TLS 压缩。Apache：SSLCompression off；Nginx：ssl_compression off。",
     "Disable TLS compression. Apache: SSLCompression off; Nginx: ssl_compression off."),
    // ---- 套件类发现 ----
    ("接受出口级（EXPORT）加密套件", "Accepts export-grade (EXPORT) cipher suites"),
    (
        "出口级套件使用 40/56 位密钥，可被现代算力实时破解，是 FREAK 和 Logjam 攻击的前提。",
        "Export-grade suites use 40/56-bit keys that can be broken in real time by modern hardware, enabling FREAK and Logjam attacks.",
    ),
    ("握手探测确认服务器接受了出口级套件：{}",
     "Handshake probe confirmed the server accepts export-grade suites: {}"),
    ("从服务器配置中移除所有 EXPORT 级加密套件，并更新 OpenSSL 到已修补版本。",
     "Remove all EXPORT-grade cipher suites from the server configuration and update OpenSSL to a patched version."),
    ("接受 RC4 加密套件", "Accepts RC4 cipher suites"),
    (
        "RC4 流密码存在多个偏差攻击（如 RC4 NOMORE），IETF 已于 RFC 7465 禁止其用于 TLS。",
        "The RC4 stream cipher has multiple bias attacks (e.g. RC4 NOMORE); RFC 7465 prohibits its use in TLS.",
    ),
    ("握手探测确认服务器接受了 RC4 套件：{}",
     "Handshake probe confirmed the server accepts RC4 suites: {}"),
    ("禁用所有 RC4 加密套件，改用 AES-GCM 或 ChaCha20-Poly1305。",
     "Disable all RC4 cipher suites and use AES-GCM or ChaCha20-Poly1305 instead."),
    ("接受 NULL 加密或匿名协商套件", "Accepts NULL-encryption or anonymous cipher suites"),
    (
        "NULL 套件不加密流量，匿名（aNULL）套件不验证服务器身份，均等同于明文通信。",
        "NULL suites do not encrypt traffic and anonymous (aNULL) suites do not authenticate the server; both are equivalent to plaintext communication.",
    ),
    ("握手探测确认服务器接受了无加密/匿名套件：{}",
     "Handshake probe confirmed the server accepts NULL/anonymous suites: {}"),
    ("禁用所有 NULL 与匿名（aNULL/anon）加密套件。",
     "Disable all NULL and anonymous (aNULL/anon) cipher suites."),
    ("接受 64 位分组密码（3DES/DES）", "Accepts 64-bit block ciphers (3DES/DES)"),
    (
        "64 位分组密码受 Sweet32 生日攻击影响，长期会话可在数小时内泄露明文内容。",
        "64-bit block ciphers are vulnerable to the Sweet32 birthday attack; long-lived sessions can leak plaintext within hours.",
    ),
    ("握手探测确认服务器接受了 64 位分组密码套件：{}",
     "Handshake probe confirmed the server accepts 64-bit block cipher suites: {}"),
    ("禁用 3DES 和 DES 套件，改用 AES-GCM 或 ChaCha20-Poly1305。",
     "Disable 3DES and DES suites and use AES-GCM or ChaCha20-Poly1305 instead."),
    // ---- 密钥交换类发现 ----
    ("接受无前向保密的静态 RSA 密钥交换", "Accepts static RSA key exchange without forward secrecy"),
    (
        "静态 RSA 密钥交换不具备前向保密性：服务器私钥一旦泄露，历史流量可被完整解密。",
        "Static RSA key exchange has no forward secrecy: once the server private key leaks, historical traffic can be fully decrypted.",
    ),
    ("握手探测确认服务器接受了静态 RSA 密钥交换套件：{}",
     "Handshake probe confirmed the server accepts static RSA key-exchange suites: {}"),
    ("仅保留 ECDHE/DHE 密钥交换套件以启用前向保密。",
     "Keep only ECDHE/DHE key-exchange suites to enable forward secrecy."),
    // ---- 评分与建议 ----
    ("未发现封顶级安全问题", "No grade-capping security issues found"),
    ("加权得分 {}，未触发等级封顶", "Weighted score {}, no grade cap triggered"),
    ("加权得分 {}，受 {} 项发现封顶至 {}", "Weighted score {}, {} findings cap the grade to {}"),
    ("：", ": "),
    ("SSL/TLS 配置优秀，建议定期复查。",
     "Excellent SSL/TLS configuration; recheck periodically."),
    ("SSL/TLS 配置良好，仍有优化空间。",
     "Good SSL/TLS configuration with room for improvement."),
    ("SSL/TLS 配置需要改进，存在安全风险。",
     "SSL/TLS configuration needs improvement; security risks present."),
    ("SSL/TLS 配置存在严重安全风险，需要立即修复。",
     "SSL/TLS configuration has serious security risks and needs immediate remediation."),
    (
        "启用 HSTS：Strict-Transport-Security: max-age=31536000; includeSubDomains; preload",
        "Enable HSTS: Strict-Transport-Security: max-age=31536000; includeSubDomains; preload",
    ),
    (
        "配置 OCSP Stapling 提高证书验证性能：ssl_stapling on; ssl_stapling_verify on;",
        "Configure OCSP Stapling to improve certificate validation performance: ssl_stapling on; ssl_stapling_verify on;",
    ),
    ("仅启用 TLS 1.2 和 TLS 1.3，禁用 SSL 2.0/3.0 与 TLS 1.0/1.1。",
     "Enable only TLS 1.2 and TLS 1.3; disable SSL 2.0/3.0 and TLS 1.0/1.1."),
];
