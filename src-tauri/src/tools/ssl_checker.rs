use crate::tools::cert_chain_utils::{determine_chain_level, is_self_signed, issuer_in_chain};
use crate::tools::ssl_checker::oid_registry::Oid;
use chrono_tz::Asia::Shanghai;
use rustls::pki_types::ServerName;
use rustls::RootCertStore;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::{rustls, TlsConnector};
use x509_parser::prelude::*;
use x509_parser::public_key::PublicKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CipherSuite {
    pub name: String,
    /// 接受该套件的协议版本（真实探测结果，可能跨多个版本）
    pub versions: Vec<String>,
    pub strength: String,
    pub server_order: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolSupport {
    pub version: String,
    pub supported: bool,
    pub cipher_suites: Vec<CipherSuite>,
    pub alpn_protocols: Option<Vec<String>>,
    pub http2_support: Option<bool>,
    pub spdy_support: Option<bool>,
    pub http3_support: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    /// 发现项标识（如 "SSL-2.0"、"RC4-CIPHER"），非 CVE 发现也有稳定 id
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: String, // "CRITICAL", "HIGH", "MEDIUM", "LOW"
    /// 发现类别："protocol" / "cipher" / "certificate" / "key-exchange"
    pub category: String,
    /// 实际观测到的证据（探测结果），每条发现都有可复现的判定依据
    pub evidence: String,
    /// 关联的 CVE 编号列表
    pub cve_ids: Vec<String>,
    pub affected_components: Vec<String>,
    pub remediation: String,
    pub references: Vec<String>,
    /// 等级封顶（"F"/"C"/"B"/"A-"）；空字符串表示不封顶
    pub grade_cap: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SslCertificate {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub fingerprint: String,
    pub serial_number: String,
    pub signature_algorithm: String,
    pub public_key_algorithm: String,
    pub key_size: Option<u32>,
    pub san_domains: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateChainNode {
    pub certificate: SslCertificate,
    pub is_root: bool,
    pub is_leaf: bool,
    /// 链级别：0 = 终端证书，1 = 中间 CA，2 = 根 CA（与 CertificateViewer 共用同一套判定）
    pub chain_level: u32,
    pub trust_status: String, // "end-entity", "intermediate", "root-ca", "self-signed"
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateChain {
    pub certificates: Vec<CertificateChainNode>,
    pub chain_length: u32,
    pub is_complete: bool,
    /// 服务器是否发送了根（或交叉签名根）证书
    pub root_in_chain: bool,
    /// 链最终锚定到的受信任根 CA 名称（匹配自客户端信任库）
    pub trust_anchor_info: Option<String>,
    pub root_ca_info: Option<String>,
    pub chain_validation_status: String,
    pub chain_errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SslInfo {
    pub domain: String,
    pub server_ip: Option<String>,
    pub server_info: Option<String>,
    pub certificate: Option<SslCertificate>,
    pub certificate_chain: Option<CertificateChain>,
    pub ssl_versions: Option<Vec<String>>,
    pub cipher_suites: Option<Vec<CipherSuite>>,
    pub protocol_support: Option<Vec<ProtocolSupport>>,
    pub server_cipher_order: Option<bool>,
    pub security_score: Option<u32>,
    pub ssl_labs_rating: Option<SslLabsRating>,
    pub vulnerabilities: Option<Vec<String>>,
    pub recommendations: Option<Vec<String>>,
    pub cve_vulnerabilities: Option<Vec<SecurityVulnerability>>,
    pub http2_support: Option<bool>,
    pub spdy_support: Option<bool>,
    pub http3_support: Option<bool>,
    pub alpn_protocols: Option<Vec<String>>,
}


fn resolve_domain_ip(domain: &str) -> Result<IpAddr, String> {
    match dns_lookup::lookup_host(domain) {
        Ok(ips) => {
            let ip_vec: Vec<IpAddr> = ips.collect();
            if let Some(ip) = ip_vec.first() {
                Ok(*ip)
            } else {
                Err("No IP address found for domain".to_string())
            }
        }
        Err(e) => Err(format!("DNS resolution failed: {}", e)),
    }
}

fn get_server_info(domain: &str, port: u16) -> Option<String> {
    let addr = match format!("{}:{}", domain, port).parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(_) => return None,
    };

    match TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
        Ok(mut stream) => {
            // Try HTTPS request first for port 443, then HTTP for others
            let request = if port == 443 {
                // For HTTPS, we can't easily do a plain HTTP request over TLS here
                // We'll get this info from the TLS connection
                return None;
            } else {
                format!(
                    "HEAD / HTTP/1.1\r\nHost: {}\r\nUser-Agent: SSL-Checker/1.0\r\nConnection: close\r\n\r\n",
                    domain
                )
            };

            if let Err(_) = std::io::Write::write_all(&mut stream, request.as_bytes()) {
                return None;
            }

            let mut reader = BufReader::new(&stream);
            let mut server_header = String::new();
            let mut powered_by = String::new();

            // Read response headers
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.trim().is_empty() {
                            break; // End of headers
                        }

                        let line_lower = line.to_lowercase();
                        if line_lower.starts_with("server:") {
                            server_header =
                                line.trim().replace("Server: ", "").replace("server: ", "");
                        } else if line_lower.starts_with("x-powered-by:") {
                            powered_by = line
                                .trim()
                                .replace("X-Powered-By: ", "")
                                .replace("x-powered-by: ", "");
                        }
                    }
                    Err(_) => break,
                }
            }

            // Combine server info
            match (server_header.is_empty(), powered_by.is_empty()) {
                (false, false) => Some(format!("{} ({})", server_header, powered_by)),
                (false, true) => Some(server_header),
                (true, false) => Some(format!("Unknown ({})", powered_by)),
                (true, true) => None,
            }
        }
        Err(_) => None,
    }
}

async fn get_https_server_info(domain: &str) -> Option<String> {
    // Make an HTTPS request to get server headers
    let url = format!("https://{}/", domain);
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true) // We're just checking headers
        .build()
    {
        Ok(client) => match client.head(&url).send().await {
            Ok(response) => {
                let mut server_info = Vec::new();

                if let Some(server) = response.headers().get("server") {
                    if let Ok(server_str) = server.to_str() {
                        server_info.push(server_str.to_string());
                    }
                }

                if let Some(powered_by) = response.headers().get("x-powered-by") {
                    if let Ok(powered_str) = powered_by.to_str() {
                        server_info.push(format!("({})", powered_str));
                    }
                }

                if let Some(via) = response.headers().get("via") {
                    if let Ok(via_str) = via.to_str() {
                        server_info.push(format!("via {}", via_str));
                    }
                }

                if server_info.is_empty() {
                    None
                } else {
                    Some(server_info.join(" "))
                }
            }
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn get_signature_algorithm_name(oid: &Oid) -> String {
    let oid_str = oid.to_string();
    match oid_str.as_str() {
        "1.2.840.113549.1.1.5" => "SHA1withRSA".to_string(),
        "1.2.840.113549.1.1.11" => "SHA256withRSA".to_string(),
        "1.2.840.113549.1.1.12" => "SHA384withRSA".to_string(),
        "1.2.840.113549.1.1.13" => "SHA512withRSA".to_string(),
        "1.2.840.10045.4.1" => "ECDSA-with-SHA1".to_string(),
        "1.2.840.10045.4.3.2" => "ECDSA-with-SHA256".to_string(),
        "1.2.840.10045.4.3.3" => "ECDSA-with-SHA384".to_string(),
        "1.2.840.10045.4.3.4" => "ECDSA-with-SHA512".to_string(),
        "1.2.840.113549.1.1.1" => "RSA".to_string(),
        "1.2.840.10040.4.1" => "DSA".to_string(),
        _ => format!("Unknown ({})", oid_str),
    }
}

fn classify_cipher_suite_strength(cipher_name: &str) -> String {
    let cipher_lower = cipher_name.to_lowercase();

    // Weak ciphers
    if cipher_lower.contains("null")
        || cipher_lower.contains("anon")
        || cipher_lower.contains("export")
        || (cipher_lower.contains("des") && !cipher_lower.contains("3des"))
        || cipher_lower.contains("rc4")
        || cipher_lower.contains("md5")
    {
        return "WEAK".to_string();
    }

    // High strength - modern AEAD ciphers（需在 AES-CBC 判定之前检查，
    // 避免 AES_128_GCM 命中 "aes128" 被降级）
    if cipher_lower.contains("gcm")
        || cipher_lower.contains("chacha20")
        || cipher_lower.contains("poly1305")
    {
        return "HIGH".to_string();
    }

    // Medium strength
    if cipher_lower.contains("3des")
        || cipher_lower.contains("aes")
        || cipher_lower.contains("camellia")
    {
        return "MEDIUM".to_string();
    }

    "MEDIUM".to_string() // Default
}

/// 需要探测的协议版本（顺序即展示顺序）
const PROBED_VERSIONS: &[&str] = &["SSL 2.0", "SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2", "TLS 1.3"];

/// 原始握手探测用的协议版本参数
#[derive(Debug, Clone, Copy, PartialEq)]
struct ProbeVersion {
    name: &'static str,
    major: u8,
    minor: u8,
}

const PROBE_SSL30: ProbeVersion = ProbeVersion { name: "SSL 3.0", major: 3, minor: 0 };
const PROBE_TLS10: ProbeVersion = ProbeVersion { name: "TLS 1.0", major: 3, minor: 1 };
const PROBE_TLS11: ProbeVersion = ProbeVersion { name: "TLS 1.1", major: 3, minor: 2 };
const PROBE_TLS12: ProbeVersion = ProbeVersion { name: "TLS 1.2", major: 3, minor: 3 };
const PROBE_TLS13: ProbeVersion = ProbeVersion { name: "TLS 1.3", major: 3, minor: 4 };

fn probe_version_by_name(name: &str) -> Option<ProbeVersion> {
    match name {
        "SSL 3.0" => Some(PROBE_SSL30),
        "TLS 1.0" => Some(PROBE_TLS10),
        "TLS 1.1" => Some(PROBE_TLS11),
        "TLS 1.2" => Some(PROBE_TLS12),
        "TLS 1.3" => Some(PROBE_TLS13),
        _ => None,
    }
}

/// 检查各协议版本支持情况并真实枚举套件。
/// 返回 (各版本支持详情, 服务器是否在任一探测中选择了 zlib 压缩)。
async fn check_protocol_support(
    domain: &str,
    port: u16,
) -> Result<(Vec<ProtocolSupport>, bool), String> {
    // ALPN/HTTP2/SPDY/HTTP3 与具体协议版本无关，只检查一次
    let alpn_protocols = check_alpn_support(domain, port).await.ok();
    let http2_support = check_http2_support(domain, port).await.ok();
    let spdy_support = check_spdy_support(domain, port).await.ok();
    let http3_support = check_http3_support(domain, port).await.ok();

    let mut results = Vec::new();
    let mut compression_supported = false;

    for version in PROBED_VERSIONS {
        let supported = check_tls_version_support(domain, port, version).await;

        if supported {
            // 真实枚举服务器在该协议版本下接受的加密套件
            let (cipher_suites, zlib) = probe_cipher_suites_for_version(domain, port, version).await;
            compression_supported |= zlib;
            results.push(ProtocolSupport {
                version: version.to_string(),
                supported: true,
                cipher_suites,
                alpn_protocols: alpn_protocols.clone(),
                http2_support,
                spdy_support,
                http3_support,
            });
        } else {
            results.push(ProtocolSupport {
                version: version.to_string(),
                supported: false,
                cipher_suites: Vec::new(),
                alpn_protocols: None,
                http2_support: None,
                spdy_support: None,
                http3_support: None,
            });
        }
    }

    Ok((results, compression_supported))
}

async fn check_tls_version_support(domain: &str, port: u16, version: &str) -> bool {
    match version {
        // SSL 2.0 使用 V2 格式握手探测
        "SSL 2.0" => probe_ssl_v2(domain, port).await.unwrap_or(false),
        // SSL 3.0 / TLS 1.0 / TLS 1.1 rustls 不支持，用原始 ClientHello 探测。
        // 必须精确匹配响应版本：服务器响应更高版本属协议违规，不能视为支持。
        "SSL 3.0" | "TLS 1.0" | "TLS 1.1" => {
            let probe = probe_version_by_name(version).unwrap();
            let codes: Vec<u16> = CANDIDATE_SUITES
                .iter()
                .filter(|(_, _, vs)| vs.contains(&version))
                .map(|(code, _, _)| *code)
                .collect();
            let opts = ClientHelloOptions {
                probe,
                cipher_codes: codes,
                offer_zlib: false,
                include_sni: true,
                include_sig_algs: false,
                tls13_negotiation: false,
            };
            let hello = build_client_hello(domain, &opts);
            match probe_tls_handshake(domain, port, &hello).await {
                Ok(record) => parse_server_hello(&record)
                    .and_then(|info| info.legacy_version)
                    .map(|(major, minor)| major == probe.major && minor == probe.minor)
                    .unwrap_or(false),
                Err(_) => false,
            }
        }
        // TLS 1.2 / 1.3 用 rustls 建立受限版本的真实连接
        _ => {
            // Initialize crypto provider
            let _ = rustls::crypto::ring::default_provider().install_default();

            let server_name = match ServerName::try_from(domain.to_string()) {
                Ok(name) => name,
                Err(_) => return false,
            };

            let rustls_version: &'static rustls::SupportedProtocolVersion = if version == "TLS 1.3" {
                &rustls::version::TLS13
            } else {
                &rustls::version::TLS12
            };

            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            // Create a config that only supports the specific version
            let version_config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[rustls_version])
            .unwrap_or_else(|_| {
                rustls::ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .expect("safe default protocol versions")
            })
            .with_root_certificates(root_store.clone())
            .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(version_config));

            let stream = match tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await
            {
                Ok(stream) => stream,
                Err(_) => return false,
            };

            match tokio::time::timeout(
                Duration::from_secs(5),
                connector.connect(server_name, stream),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => false, // Timeout
            }
        }
    }
}

/// HelloRetryRequest 的固定随机值（RFC 8446 §4.1.3），用于区分 TLS 1.3 重试与真实协商
const HELLO_RETRY_REQUEST_RANDOM: [u8; 32] = [
    0xCF, 0x21, 0xAD, 0x74, 0xE5, 0x9A, 0x61, 0x11, 0xBE, 0x1D, 0x8C, 0x02, 0x1E, 0x65, 0xB8, 0x91,
    0xC2, 0xA2, 0x11, 0x16, 0x7A, 0xBB, 0x8C, 0x5E, 0x07, 0x9E, 0x09, 0xE2, 0xC8, 0xA8, 0x33, 0x9C,
];

/// 套件候选表：(IANA 代码, 名称, 适用探测版本)。
/// 覆盖 rustls 无法探测的 CBC/RC4/3DES/DES/EXPORT/NULL/aNULL 弱套件。
const CANDIDATE_SUITES: &[(u16, &str, &[&str])] = &[
    // TLS 1.3
    (0x1301, "TLS_AES_128_GCM_SHA256", &["TLS 1.3"]),
    (0x1302, "TLS_AES_256_GCM_SHA384", &["TLS 1.3"]),
    (0x1303, "TLS_CHACHA20_POLY1305_SHA256", &["TLS 1.3"]),
    // TLS 1.2 现代 AEAD
    (0xC02F, "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"]),
    (0xC030, "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384", &["TLS 1.2"]),
    (0xC02B, "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"]),
    (0xC02C, "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384", &["TLS 1.2"]),
    (0xCCA8, "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256", &["TLS 1.2"]),
    (0xCCA9, "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256", &["TLS 1.2"]),
    (0x009C, "TLS_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"]),
    (0x009D, "TLS_RSA_WITH_AES_256_GCM_SHA384", &["TLS 1.2"]),
    (0x0067, "TLS_DHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"]),
    (0x006D, "TLS_DHE_RSA_WITH_AES_256_GCM_SHA384", &["TLS 1.2"]),
    // TLS 1.2 CBC-SHA256
    (0xC027, "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA256", &["TLS 1.2"]),
    (0xC028, "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA384", &["TLS 1.2"]),
    (0xC023, "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA256", &["TLS 1.2"]),
    (0x003C, "TLS_RSA_WITH_AES_128_CBC_SHA256", &["TLS 1.2"]),
    (0x003D, "TLS_RSA_WITH_AES_256_CBC_SHA256", &["TLS 1.2"]),
    // CBC-SHA（TLS 1.0/1.1/1.2）
    (0xC013, "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0xC014, "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0xC009, "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0xC00A, "TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x002F, "TLS_RSA_WITH_AES_128_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x0035, "TLS_RSA_WITH_AES_256_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x0033, "TLS_DHE_RSA_WITH_AES_128_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x0039, "TLS_DHE_RSA_WITH_AES_256_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    // RC4（弱）
    (0x0005, "TLS_RSA_WITH_RC4_128_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x0004, "TLS_RSA_WITH_RC4_128_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0xC011, "TLS_ECDHE_RSA_WITH_RC4_128_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    // 3DES（弱）
    (0x000A, "TLS_RSA_WITH_3DES_EDE_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0xC012, "TLS_ECDHE_RSA_WITH_3DES_EDE_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x0016, "TLS_DHE_RSA_WITH_3DES_EDE_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    // 单重 DES（弱）
    (0x0009, "TLS_RSA_WITH_DES_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    // EXPORT 级（弱）
    (0x0003, "TLS_RSA_EXPORT_WITH_RC4_40_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0006, "TLS_RSA_EXPORT_WITH_RC2_CBC_40_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0008, "TLS_RSA_EXPORT_WITH_DES40_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0062, "TLS_RSA_EXPORT1024_WITH_DES_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0064, "TLS_RSA_EXPORT1024_WITH_RC4_56_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0063, "TLS_DHE_RSA_EXPORT1024_WITH_DES_CBC_SHA", &["TLS 1.0", "TLS 1.1"]),
    // NULL 加密（弱）
    (0x0001, "TLS_RSA_WITH_NULL_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0002, "TLS_RSA_WITH_NULL_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x003B, "TLS_RSA_WITH_NULL_SHA256", &["TLS 1.2"]),
    // 匿名 DH（弱）
    (0x0017, "TLS_DH_anon_EXPORT_WITH_RC4_40_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0018, "TLS_DH_anon_WITH_RC4_128_MD5", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x001B, "TLS_DH_anon_WITH_3DES_EDE_CBC_SHA", &["SSL 3.0", "TLS 1.0", "TLS 1.1"]),
    (0x0034, "TLS_DH_anon_WITH_AES_128_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
    (0x003A, "TLS_DH_anon_WITH_AES_256_CBC_SHA", &["TLS 1.0", "TLS 1.1", "TLS 1.2"]),
];

/// 探测用 ClientHello 参数
struct ClientHelloOptions {
    probe: ProbeVersion,
    cipher_codes: Vec<u16>,
    /// 同时提供 zlib 压缩方法，用于检测 CRIME 前提条件
    offer_zlib: bool,
    include_sni: bool,
    /// TLS 1.2 服务器通常要求 signature_algorithms 扩展
    include_sig_algs: bool,
    /// 附加 TLS 1.3 协商扩展（supported_versions / key_share / supported_groups）
    tls13_negotiation: bool,
}

/// 探测包随机数：无需密码学强度，避免每次发送相同内容即可
fn probe_random_bytes(len: usize) -> Vec<u8> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut state = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn push_extension(exts: &mut Vec<u8>, ext_type: u16, body: &[u8]) {
    exts.extend_from_slice(&ext_type.to_be_bytes());
    exts.extend_from_slice(&(body.len() as u16).to_be_bytes());
    exts.extend_from_slice(body);
}

/// 构造逐套件探测用的原始 ClientHello
fn build_client_hello(domain: &str, opts: &ClientHelloOptions) -> Vec<u8> {
    let mut body = Vec::new();

    // client_version：TLS 1.3 走 legacy_version 3.3，实际版本由 supported_versions 扩展指示
    let (client_major, client_minor) = if opts.tls13_negotiation {
        (3u8, 3u8)
    } else {
        (opts.probe.major, opts.probe.minor)
    };
    body.extend_from_slice(&[client_major, client_minor]);
    body.extend_from_slice(&probe_random_bytes(32));
    body.push(0x00); // session_id 长度 = 0

    // cipher_suites
    body.extend_from_slice(&((opts.cipher_codes.len() * 2) as u16).to_be_bytes());
    for code in &opts.cipher_codes {
        body.extend_from_slice(&code.to_be_bytes());
    }

    // compression_methods：null（可选 zlib）
    if opts.offer_zlib {
        body.extend_from_slice(&[0x02, 0x00, 0x01]);
    } else {
        body.extend_from_slice(&[0x01, 0x00]);
    }

    // SSL 3.0 不携带扩展
    if opts.probe != PROBE_SSL30 {
        let mut exts = Vec::new();

        if opts.include_sni && !domain.is_empty() {
            let mut list = vec![0x00u8];
            list.extend_from_slice(&(domain.len() as u16).to_be_bytes());
            list.extend_from_slice(domain.as_bytes());
            let mut sni_body = Vec::new();
            sni_body.extend_from_slice(&(list.len() as u16).to_be_bytes());
            sni_body.extend_from_slice(&list);
            push_extension(&mut exts, 0x0000, &sni_body);
        }

        if opts.include_sig_algs || opts.tls13_negotiation {
            // supported_groups：TLS 1.2 的 ECDHE 套件需要客户端提供曲线；
            // TLS 1.3 中该扩展与 signature_algorithms 均为必发（RFC 8446 §9.2）。
            // 注意不可重复添加，重复扩展会被服务器以 illegal_parameter 拒绝。
            push_extension(&mut exts, 0x000a, &[0x00, 0x04, 0x00, 0x1d, 0x00, 0x17]);
            let algs: [u16; 10] = [
                0x0403, 0x0503, 0x0603, // ECDSA SHA256/384/512
                0x0804, 0x0805, 0x0806, // RSA-PSS
                0x0401, 0x0501, 0x0601, // RSA PKCS1
                0x0402, // SHA1（旧服务器需要）
            ];
            let mut body = Vec::new();
            body.extend_from_slice(&((algs.len() * 2) as u16).to_be_bytes());
            for alg in algs {
                body.extend_from_slice(&alg.to_be_bytes());
            }
            push_extension(&mut exts, 0x000d, &body);
        }

        if opts.tls13_negotiation {
            // supported_versions: TLS 1.3
            push_extension(&mut exts, 0x002b, &[0x02, 0x03, 0x04]);
            // key_share：仅提供 x25519 占位公钥（任意 32 字节都是合法的 u 坐标，
            // 只需收到 ServerHello 即可完成套件探测，无需完成握手）。
            // 不提供随机构造的 P-256 点——非法点会被服务器以 illegal_parameter 拒绝；
            // 偏好 P-256 的服务器会对纯 x25519 的 ClientHello 回 HelloRetryRequest，
            // 探测层已将 HRR 视为“无法判定”而非“接受”。
            let mut body = Vec::new();
            let x25519_entry = {
                let mut e = 0x001du16.to_be_bytes().to_vec();
                e.extend_from_slice(&[0x00, 0x20]);
                e.extend_from_slice(&probe_random_bytes(32));
                e
            };
            body.extend_from_slice(&(x25519_entry.len() as u16).to_be_bytes());
            body.extend_from_slice(&x25519_entry);
            push_extension(&mut exts, 0x0033, &body);
        }

        body.extend_from_slice(&(exts.len() as u16).to_be_bytes());
        body.extend_from_slice(&exts);
    }

    // 记录层封装。记录版本按惯例：SSL 3.0 用 3.0，其余用 3.1
    let (rec_major, rec_minor) = if opts.probe == PROBE_SSL30 { (3u8, 0u8) } else { (3u8, 1u8) };
    let mut hello = Vec::new();
    hello.push(0x16);
    hello.extend_from_slice(&[rec_major, rec_minor]);
    hello.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
    hello.push(0x01); // ClientHello
    let hs_len = (body.len() as u32).to_be_bytes();
    hello.extend_from_slice(&hs_len[1..4]);
    hello.extend_from_slice(&body);
    hello
}

/// 解析 ServerHello 提取的信息
#[derive(Debug, Default)]
struct ServerHelloInfo {
    /// ServerHello 的 legacy_version 字段
    legacy_version: Option<(u8, u8)>,
    /// supported_versions 扩展选择的版本（TLS 1.3 时为 (3,4)）
    selected_version: Option<(u8, u8)>,
    cipher_suite: Option<u16>,
    /// legacy_compression_method（1 = zlib，即 CRIME 前提）
    compression_method: Option<u8>,
    is_hello_retry_request: bool,
}

fn parse_server_hello(record: &[u8]) -> Option<ServerHelloInfo> {
    // 记录头 5 字节 + 握手头 4 字节 + 版本 2 字节
    if record.len() < 11 || record[0] != 0x16 || record[5] != 0x02 {
        return None;
    }
    let mut info = ServerHelloInfo {
        legacy_version: Some((record[9], record[10])),
        ..Default::default()
    };
    if record.len() < 43 {
        return Some(info);
    }
    info.is_hello_retry_request = record[11..43] == HELLO_RETRY_REQUEST_RANDOM;

    let mut p = 43usize;
    // session_id
    let sid_len = *record.get(p)? as usize;
    p += 1 + sid_len;
    // cipher_suite
    let cipher = record.get(p..p + 2)?;
    info.cipher_suite = Some(u16::from_be_bytes([cipher[0], cipher[1]]));
    p += 2;
    // compression method
    info.compression_method = record.get(p).copied();
    p += 1;

    // TLS 1.3 ServerHello 带扩展，读取 supported_versions 确认实际版本
    if let Some(ext_bytes) = record.get(p..p + 2) {
        let ext_total = u16::from_be_bytes([ext_bytes[0], ext_bytes[1]]) as usize;
        let end = (p + 2 + ext_total).min(record.len());
        let mut q = p + 2;
        while q + 4 <= end {
            let ext_type = u16::from_be_bytes([record[q], record[q + 1]]);
            let ext_len = u16::from_be_bytes([record[q + 2], record[q + 3]]) as usize;
            q += 4;
            if ext_type == 0x002b && ext_len >= 2 && q + 2 <= end {
                info.selected_version = Some((record[q], record[q + 1]));
            }
            q += ext_len;
        }
    }

    Some(info)
}

/// 发送原始 ClientHello 并读取服务器返回的第一个记录
async fn probe_tls_handshake(domain: &str, port: u16, hello: &[u8]) -> Result<Vec<u8>, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let stream = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(format!("{}:{}", domain, port)),
    )
    .await
    .map_err(|_| "连接超时".to_string())?
    .map_err(|e| format!("TCP 连接失败: {}", e))?;

    let mut stream = stream;
    tokio::time::timeout(Duration::from_secs(3), stream.write_all(hello))
        .await
        .map_err(|_| "写入超时".to_string())?
        .map_err(|e| format!("写入失败: {}", e))?;

    let mut header = [0u8; 5];
    tokio::time::timeout(Duration::from_secs(3), stream.read_exact(&mut header))
        .await
        .map_err(|_| "读取超时".to_string())?
        .map_err(|e| format!("读取失败: {}", e))?;

    if header[0] == 0x15 {
        // 服务器返回 alert：拒绝本次握手（协议或套件不受支持）
        return Err("服务器返回 alert".to_string());
    }

    let len = u16::from_be_bytes([header[3], header[4]]) as usize;
    let mut body = vec![0u8; len];
    if len > 0 {
        tokio::time::timeout(Duration::from_secs(3), stream.read_exact(&mut body))
            .await
            .map_err(|_| "读取超时".to_string())?
            .map_err(|e| format!("读取失败: {}", e))?;
    }

    let mut record = header.to_vec();
    record.extend_from_slice(&body);
    Ok(record)
}

/// SSL 2.0 V2 格式握手探测
async fn probe_ssl_v2(domain: &str, port: u16) -> Result<bool, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // V2 cipher-spec：RC4-128-MD5 / RC4-128-EXPORT / RC2-CBC-MD5 / RC2-CBC-EXPORT /
    // DES-CBC-MD5 / DES-CBC3-MD5
    let ciphers: [[u8; 3]; 6] = [
        [0x01, 0x00, 0x80],
        [0x02, 0x00, 0x80],
        [0x03, 0x00, 0x80],
        [0x04, 0x00, 0x80],
        [0x06, 0x00, 0x40],
        [0x07, 0x00, 0xC0],
    ];
    let challenge = probe_random_bytes(16);

    let mut body = vec![0x01u8, 0x02, 0x00]; // ClientHello + 版本 SSL 2.0
    body.extend_from_slice(&((ciphers.len() * 3) as u16).to_be_bytes());
    body.extend_from_slice(&[0x00, 0x00]); // session_id 长度 = 0
    body.extend_from_slice(&[0x00, 0x10]); // challenge 长度 = 16
    for c in ciphers {
        body.extend_from_slice(&c);
    }
    body.extend_from_slice(&challenge);

    let mut hello = ((body.len() as u16) | 0x8000).to_be_bytes().to_vec();
    hello.extend_from_slice(&body);

    let stream = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(format!("{}:{}", domain, port)),
    )
    .await
    .map_err(|_| "连接超时".to_string())?
    .map_err(|e| format!("TCP 连接失败: {}", e))?;

    let mut stream = stream;
    tokio::time::timeout(Duration::from_secs(3), stream.write_all(&hello))
        .await
        .map_err(|_| "写入超时".to_string())?
        .map_err(|e| format!("写入失败: {}", e))?;

    let mut buffer = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buffer))
        .await
        .map_err(|_| "读取超时".to_string())?
        .map_err(|e| format!("读取失败: {}", e))?;

    if n < 8 {
        return Ok(false);
    }
    // TLS 格式响应（0x16/0x15）说明服务器不支持 SSLv2；
    // V2 ServerHello：消息类型 0x04，版本 0x02 0x00
    let resp = &buffer[..n];
    if resp[0] == 0x16 || resp[0] == 0x15 {
        return Ok(false);
    }
    Ok(resp[2] == 0x04 && resp[6] == 0x02 && resp[7] == 0x00)
}

/// 枚举服务器在指定协议版本下真实接受的加密套件。
/// 返回 (套件列表, 服务器是否选择了 zlib 压缩)。
async fn probe_cipher_suites_for_version(
    domain: &str,
    port: u16,
    version: &str,
) -> (Vec<CipherSuite>, bool) {
    let probe = match probe_version_by_name(version) {
        Some(p) => p,
        None => return (Vec::new(), false),
    };

    let jobs: Vec<(u16, &str)> = CANDIDATE_SUITES
        .iter()
        .filter(|(_, _, vs)| vs.contains(&version))
        .map(|(code, name, _)| (*code, *name))
        .collect();

    // 并发探测（并发度 16，单次连接 3s 超时）
    let semaphore = Arc::new(tokio::sync::Semaphore::new(16));
    let mut set = tokio::task::JoinSet::new();
    let domain_owned = domain.to_string();

    for (code, name) in jobs {
        let semaphore = semaphore.clone();
        let domain = domain_owned.clone();
        let version_owned = version.to_string();
        set.spawn(async move {
            let _permit = semaphore.acquire().await;
            let opts = ClientHelloOptions {
                probe,
                cipher_codes: vec![code],
                // 仅 ≤TLS 1.2 提供 zlib 以检测 CRIME 前提：
                // SSL 3.0 无 zlib 标准依据；TLS 1.3 要求压缩方法列表必须只含 null（RFC 8446 §9.2）
                offer_zlib: probe == PROBE_TLS10 || probe == PROBE_TLS11 || probe == PROBE_TLS12,
                include_sni: true,
                include_sig_algs: probe == PROBE_TLS12,
                tls13_negotiation: probe == PROBE_TLS13,
            };
            let hello = build_client_hello(&domain, &opts);
            let mut accepted = false;
            let mut zlib = false;
            if let Ok(record) = probe_tls_handshake(&domain, port, &hello).await {
                if let Some(info) = parse_server_hello(&record) {
                    let version_matches = if probe == PROBE_TLS13 {
                        info.selected_version == Some((3, 4))
                    } else {
                        info.legacy_version == Some((probe.major, probe.minor))
                    };
                    if version_matches && !info.is_hello_retry_request {
                        accepted = info.cipher_suite == Some(code);
                        zlib = info.compression_method == Some(1);
                    }
                }
            }
            (version_owned, name.to_string(), accepted, zlib)
        });
    }

    let mut suites = Vec::new();
    let mut zlib_accepted = false;
    while let Some(result) = set.join_next().await {
        if let Ok((_, name, accepted, zlib)) = result {
            if accepted {
                suites.push(CipherSuite {
                    name,
                    versions: vec![version.to_string()],
                    strength: String::new(), // 聚合时统一计算
                    server_order: false,
                });
            }
            zlib_accepted |= zlib;
        }
    }
    for suite in suites.iter_mut() {
        suite.strength = classify_cipher_suite_strength(&suite.name);
    }

    (suites, zlib_accepted)
}

/// 版本展示顺序，用于套件的 versions 排序
fn version_display_order(version: &str) -> usize {
    PROBED_VERSIONS.iter().position(|v| *v == version).unwrap_or(usize::MAX)
}

/// 把各协议版本的探测结果按套件名聚合，合并接受的版本列表
fn aggregate_cipher_suites(
    per_version: Vec<(String, Vec<CipherSuite>)>,
    negotiated: Option<CipherSuite>,
) -> Vec<CipherSuite> {
    let mut merged: Vec<CipherSuite> = Vec::new();

    let mut apply = |suite: &CipherSuite| {
        if let Some(existing) = merged.iter_mut().find(|e| e.name == suite.name) {
            for v in &suite.versions {
                if !existing.versions.contains(v) {
                    existing.versions.push(v.clone());
                }
            }
        } else {
            merged.push(CipherSuite {
                name: suite.name.clone(),
                versions: suite.versions.clone(),
                strength: suite.strength.clone(),
                server_order: suite.server_order,
            });
        }
    };

    for (_, list) in &per_version {
        for suite in list {
            apply(suite);
        }
    }
    if let Some(negotiated) = &negotiated {
        apply(negotiated);
    }

    for suite in merged.iter_mut() {
        suite.versions.sort_by_key(|v| version_display_order(v));
    }
    merged
}

async fn check_http2_support(domain: &str, port: u16) -> Result<bool, String> {
    // Initialize crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    let server_name = match ServerName::try_from(domain.to_string()) {
        Ok(name) => name,
        Err(_) => return Ok(false),
    };

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = match tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await {
        Ok(stream) => stream,
        Err(_) => return Ok(false),
    };

    let _tls_stream = match connector.connect(server_name, stream).await {
        Ok(stream) => stream,
        Err(_) => return Ok(false),
    };

    // Check if HTTP/2 is supported by trying to make an HTTP/2 request
    let url = format!("https://{}:{}", domain, port);
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .http2_prior_knowledge()
        .build()
    {
        Ok(client) => {
            match client.head(&url).send().await {
                Ok(response) => {
                    // Check if the response actually used HTTP/2
                    Ok(response.version() == reqwest::Version::HTTP_2)
                }
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

async fn check_spdy_support(domain: &str, port: u16) -> Result<bool, String> {
    // SPDY is deprecated and rarely used, but we can check for it
    // Most modern servers have disabled SPDY in favor of HTTP/2
    let url = format!("https://{}:{}", domain, port);

    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(client) => {
            match client.head(&url).send().await {
                Ok(response) => {
                    // Check response headers for SPDY indicators
                    if let Some(headers) = response.headers().get("x-spdy-used") {
                        return Ok(headers.to_str().unwrap_or("") == "true");
                    }

                    // Check if the server advertises SPDY via ALPN
                    if let Some(alpn) = response.headers().get("alt-svc") {
                        if let Ok(alpn_str) = alpn.to_str() {
                            return Ok(alpn_str.contains("spdy"));
                        }
                    }

                    Ok(false)
                }
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

async fn check_http3_support(domain: &str, _port: u16) -> Result<bool, String> {
    // HTTP/3 检测通过多种方法:
    // 1. 检查 Alt-Svc 头部是否支持 h3
    // 2. 尝试 UDP QUIC 连接
    // 3. 检查 DNS HTTPS 记录

    // Method 1: Check Alt-Svc header for HTTP/3 (h3) support
    let url = format!("https://{}/", domain);
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true) // We're checking protocol support
        .build()
    {
        Ok(client) => {
            match client.head(&url).send().await {
                Ok(response) => {
                    // Check Alt-Svc header for HTTP/3 support
                    if let Some(alt_svc) = response.headers().get("alt-svc") {
                        if let Ok(alt_svc_str) = alt_svc.to_str() {
                            // Look for h3, h3-29, h3-27, etc.
                            return Ok(alt_svc_str.to_lowercase().contains("h3"));
                        }
                    }

                    // Check for HTTP/3 indicators in other headers
                    if let Some(server) = response.headers().get("server") {
                        if let Ok(server_str) = server.to_str() {
                            // Some servers advertise HTTP/3 capabilities in server headers
                            if server_str.to_lowercase().contains("quic") || 
                               server_str.to_lowercase().contains("h3") {
                                return Ok(true);
                            }
                        }
                    }

                    // Check for QUIC indicators
                    if response.headers().contains_key("quic-status") ||
                       response.headers().contains_key("alt-used") {
                        return Ok(true);
                    }

                    Ok(false)
                }
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

async fn check_alpn_support(domain: &str, port: u16) -> Result<Vec<String>, String> {
    // Initialize crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    let server_name = match ServerName::try_from(domain.to_string()) {
        Ok(name) => name,
        Err(_) => return Ok(vec![]),
    };

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = match tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await {
        Ok(stream) => stream,
        Err(_) => return Ok(vec![]),
    };

    let tls_stream = match connector.connect(server_name, stream).await {
        Ok(stream) => stream,
        Err(_) => return Ok(vec![]),
    };

    // Get ALPN protocol from the connection
    let (_, connection_info) = tls_stream.get_ref();

    if let Some(alpn_protocol) = connection_info.alpn_protocol() {
        match std::str::from_utf8(alpn_protocol) {
            Ok(protocol) => Ok(vec![protocol.to_string()]),
            Err(_) => Ok(vec![]),
        }
    } else {
        Ok(vec![])
    }
}

async fn check_server_cipher_order(_domain: &str, _port: u16) -> Result<bool, String> {
    // This is a simplified check - in a real implementation, you would
    // need to make multiple connections with different cipher preferences
    // to determine if the server honors the client's cipher order

    // For now, we'll return a placeholder value
    // In practice, you would:
    // 1. Connect with one cipher order preference
    // 2. Connect with a different cipher order preference
    // 3. Compare which cipher was chosen in each case
    // 4. If the server chooses differently based on client preference, it honors client order

    Ok(false) // Default: server honors its own order
}

async fn check_tls_connection(
    domain: &str,
    port: u16,
) -> Result<(Vec<u8>, Vec<CipherSuite>, Vec<Vec<u8>>), String> {
    // Initialize crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    let server_name = match ServerName::try_from(domain.to_string()) {
        Ok(name) => name,
        Err(_) => return Err("Invalid domain name".to_string()),
    };

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = match tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await {
        Ok(stream) => stream,
        Err(e) => return Err(format!("TCP connection failed: {}", e)),
    };

    let tls_stream = match connector.connect(server_name, stream).await {
        Ok(stream) => stream,
        Err(e) => return Err(format!("TLS connection failed: {}", e)),
    };

    // Get connection info and cipher suites
    let (_, connection_info) = tls_stream.get_ref();

    let mut cipher_suites = Vec::new();
    let mut cert_chain = Vec::new();

    // Get peer certificates
    if let Some(certs) = connection_info.peer_certificates() {
        // Store all certificates in the chain
        for cert in certs {
            cert_chain.push(cert.to_vec());
        }

        if let Some(cert) = certs.first() {
            // Get the negotiated cipher suite（真实协商结果，不再补充演示数据）
            if let Some(cipher_suite) = connection_info.negotiated_cipher_suite() {
                let cipher_name = format!("{:?}", cipher_suite.suite());
                let strength = classify_cipher_suite_strength(&cipher_name);
                let version_str = connection_info
                    .protocol_version()
                    .map(protocol_version_name)
                    .unwrap_or_else(|| "TLS 1.2".to_string());

                cipher_suites.push(CipherSuite {
                    name: cipher_name,
                    versions: vec![version_str],
                    strength,
                    server_order: false,
                });
            }

            return Ok((cert.to_vec(), cipher_suites, cert_chain));
        }
    }

    Err("No certificate found".to_string())
}

fn protocol_version_name(version: rustls::ProtocolVersion) -> String {
    match version {
        rustls::ProtocolVersion::SSLv3 => "SSL 3.0".to_string(),
        rustls::ProtocolVersion::TLSv1_0 => "TLS 1.0".to_string(),
        rustls::ProtocolVersion::TLSv1_1 => "TLS 1.1".to_string(),
        rustls::ProtocolVersion::TLSv1_2 => "TLS 1.2".to_string(),
        rustls::ProtocolVersion::TLSv1_3 => "TLS 1.3".to_string(),
        other => format!("{:?}", other),
    }
}

fn build_certificate_chain(cert_chain_ders: &[Vec<u8>]) -> Result<CertificateChain, String> {
    if cert_chain_ders.is_empty() {
        return Err("No certificates in chain".to_string());
    }
    let mut chain_errors = Vec::new();

    // 一次性解析全部证书：info 用于展示，X509Certificate 用于链级别判定。
    // X509Certificate 借用 DER 缓冲，两者以相同下标平行保存。
    let mut infos: Vec<SslCertificate> = Vec::new();
    let mut x509s: Vec<X509Certificate> = Vec::new();
    for (index, cert_der) in cert_chain_ders.iter().enumerate() {
        match parse_certificate_with_x509(cert_der) {
            Ok((info, cert)) => {
                infos.push(info);
                x509s.push(cert);
            }
            Err(e) => chain_errors.push(format!("解析证书 {} 时出错: {}", index + 1, e)),
        }
    }
    if x509s.is_empty() {
        return Err(chain_errors.join("; "));
    }

    // 链级别判定与 CertificateViewer 共用同一套逻辑（BasicConstraints → keyCertSign →
    // 回退；自签名/发行者匹配均为完整 DN 的 DER 字节级比较；链顶无 pathlen 的 CA 按
    // 交叉签名根处理）
    let levels: Vec<usize> = x509s
        .iter()
        .map(|cert| determine_chain_level(cert, &x509s))
        .collect();

    // 按链级别排序：终端(0) → 中间CA(1) → 根(2)，纠正服务器发送顺序异常的情况
    let mut order: Vec<usize> = (0..x509s.len()).collect();
    order.sort_by_key(|&i| levels[i]);

    // 链连续性：相邻证书的 issuer 与 subject 做完整 DN 的 DER 字节级比较
    for pair in order.windows(2) {
        let (upper, lower) = (pair[0], pair[1]);
        if x509s[upper].issuer().as_raw() != x509s[lower].subject().as_raw() {
            chain_errors.push(format!(
                "证书链中断: {} 的颁发者与 {} 的主体不匹配",
                x509s[lower].subject(),
                x509s[upper].issuer()
            ));
        }
    }

    // 构建节点
    let mut certificates = Vec::new();
    for &i in &order {
        let cert = &x509s[i];
        let level = levels[i];
        let self_signed = is_self_signed(cert);
        let is_leaf = level == 0;
        let is_root = level == 2;

        let trust_status = if is_leaf {
            "end-entity".to_string()
        } else if is_root && self_signed {
            "self-signed".to_string()
        } else if is_root {
            // 链顶非自签名 CA 且无 pathlen：交叉签名形式的根证书
            "root-ca".to_string()
        } else {
            "intermediate".to_string()
        };

        let info = &infos[i];
        let mut validation_errors = Vec::new();

        if let Ok(valid_to) = chrono::DateTime::parse_from_rfc3339(&info.valid_to) {
            if valid_to.with_timezone(&chrono::Utc) < chrono::Utc::now() {
                validation_errors.push("证书已过期".to_string());
            }
        }
        if let Ok(valid_from) = chrono::DateTime::parse_from_rfc3339(&info.valid_from) {
            if valid_from.with_timezone(&chrono::Utc) > chrono::Utc::now() {
                validation_errors.push("证书尚未生效".to_string());
            }
        }
        if info.signature_algorithm.to_lowercase().contains("sha1") {
            validation_errors.push("使用已弃用的SHA-1签名算法".to_string());
        }
        match info.public_key_algorithm.as_str() {
            "RSA" => {
                if let Some(key_size) = info.key_size {
                    if key_size < 2048 {
                        validation_errors.push(format!("RSA密钥长度过短: {} 位", key_size));
                    }
                }
            }
            "EC" => {
                if let Some(key_size) = info.key_size {
                    if key_size < 256 {
                        validation_errors.push(format!("EC密钥长度过短: {} 位", key_size));
                    }
                }
            }
            _ => {}
        }

        certificates.push(CertificateChainNode {
            certificate: info.clone(),
            is_root,
            is_leaf,
            chain_level: level as u32,
            trust_status,
            validation_errors,
        });
    }

    // 信任锚与完整性判定：与 CertificateViewer 一致——
    // 链顶为自签名根、交叉签名形式的根（level 2），或 issuer 匹配客户端信任库中的
    // 受信任根时，链视为完整；否则提示缺少上级证书
    let root_in_chain = levels.contains(&2);
    let top = &x509s[*order.last().unwrap()];
    let trust_anchor_info = find_trust_anchor(top.issuer().as_raw())
        .or_else(|| find_trust_anchor(top.subject().as_raw()));

    let is_complete = x509s.iter().enumerate().all(|(i, cert)| {
        is_self_signed(cert)
            || issuer_in_chain(cert, &x509s)
            || find_trust_anchor(cert.issuer().as_raw()).is_some()
            || levels[i] == 2 // 交叉签名形式的根证书：链顶信任锚的等价形态
    });

    let chain_validation_status = if chain_errors.is_empty() && is_complete
        && certificates.iter().all(|c| c.validation_errors.is_empty())
    {
        "valid".to_string()
    } else if chain_errors.is_empty() && is_complete {
        "valid-with-warnings".to_string()
    } else {
        "invalid".to_string()
    };

    // 根 CA 信息：优先展示信任锚，说明根证书来源
    let root_ca_info = match &trust_anchor_info {
        Some(anchor) if root_in_chain => Some(format!("{}（服务器已发送）", anchor)),
        Some(anchor) => Some(format!(
            "{}（服务器未发送根证书，由客户端信任库提供，属标准配置）",
            anchor
        )),
        None => Some(format!(
            "{}（未能匹配到受信任的根 CA）",
            certificates.last().unwrap().certificate.subject
        )),
    };

    Ok(CertificateChain {
        certificates,
        chain_length: cert_chain_ders.len() as u32,
        is_complete,
        root_in_chain,
        trust_anchor_info,
        root_ca_info,
        chain_validation_status,
        chain_errors,
    })
}

/// 在客户端信任库中按完整 DN（DER 字节级）查找匹配的受信任根 CA 名称。
/// 注意：webpki 信任锚的 subject 是 Name SEQUENCE 的内容（0x31 开头的 RDN 序列），
/// 而 x509-parser 的 as_raw() 返回带 0x30 序列头的完整 DER，比较前需剥去外层头。
fn find_trust_anchor(name_raw: &[u8]) -> Option<String> {
    if name_raw.is_empty() {
        return None;
    }
    let contents = der_sequence_contents(name_raw)?;
    for anchor in webpki_roots::TLS_SERVER_ROOTS {
        if anchor.subject.as_ref() == contents {
            // 信任锚数据缺少外层 SEQUENCE 头，补全后再解析出可读名称
            let full_der = wrap_in_sequence(anchor.subject.as_ref());
            return match X509Name::from_der(&full_der) {
                Ok((_, name)) => Some(name.to_string()),
                Err(_) => Some("未知受信任根 CA".to_string()),
            };
        }
    }
    None
}

/// 取 DER SEQUENCE 的内容字节（剥去 tag + length 头）
fn der_sequence_contents(raw: &[u8]) -> Option<&[u8]> {
    if raw.len() < 2 || raw[0] != 0x30 {
        return None;
    }
    if raw[1] < 0x80 {
        raw.get(2..)
    } else {
        let len_bytes = (raw[1] & 0x7F) as usize;
        raw.get(2 + len_bytes..)
    }
}

/// 给内容字节补上 DER SEQUENCE 头（用于解析 webpki 信任锚的 subject）
fn wrap_in_sequence(contents: &[u8]) -> Vec<u8> {
    let len = contents.len();
    let mut out = if len < 0x80 {
        vec![0x30, len as u8]
    } else if len <= 0xFF {
        vec![0x30, 0x81, len as u8]
    } else {
        vec![0x30, 0x82, (len >> 8) as u8, (len & 0xFF) as u8]
    };
    out.extend_from_slice(contents);
    out
}

fn parse_certificate(cert_der: &[u8]) -> Result<SslCertificate, String> {
    parse_certificate_with_x509(cert_der).map(|(info, _)| info)
}

fn parse_certificate_with_x509(
    cert_der: &[u8],
) -> Result<(SslCertificate, X509Certificate<'_>), String> {
    let (_rem, cert) =
        parse_x509_certificate(cert_der).map_err(|e| format!("Certificate parse error: {}", e))?;

    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();

    let valid_from = {
        let dt = cert.validity().not_before.to_datetime();
        let utc_time = chrono::DateTime::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_default();
        let shanghai_time = utc_time.with_timezone(&Shanghai);
        shanghai_time.to_rfc3339()
    };
    let valid_to = {
        let dt = cert.validity().not_after.to_datetime();
        let utc_time = chrono::DateTime::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_default();
        let shanghai_time = utc_time.with_timezone(&Shanghai);
        shanghai_time.to_rfc3339()
    };

    let fingerprint = {
        // 使用CryptoUtils计算SHA256指纹
        crate::utils::crypto::CryptoUtils::calculate_sha256_fingerprint(cert_der)
    };

    let serial_number = {
        let serial = &cert.serial;
        hex::encode(serial.to_bytes_be()).to_uppercase()
    };

    let signature_algorithm = get_signature_algorithm_name(&cert.signature_algorithm.algorithm);

    let (public_key_algorithm, key_size) = match cert.public_key().parsed() {
        Ok(key) => match key {
            PublicKey::RSA(rsa_key) => {
                let size = rsa_key.key_size() as u32;
                ("RSA".to_string(), Some(size))
            }
            PublicKey::EC(ec_point) => {
                let size = ec_point.key_size() as u32;
                ("EC".to_string(), if size == 0 { None } else { Some(size) })
            }
            PublicKey::DSA(_) => ("DSA".to_string(), None),
            _ => ("Unknown".to_string(), None),
        },
        Err(_) => ("Unknown".to_string(), None),
    };

    let san_domains = cert.extensions().iter().find_map(|ext| {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            let domains: Vec<String> = san
                .general_names
                .iter()
                .filter_map(|name| {
                    if let GeneralName::DNSName(dns) = name {
                        Some(dns.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            if domains.is_empty() {
                None
            } else {
                Some(domains)
            }
        } else {
            None
        }
    });

    Ok((
        SslCertificate {
            subject,
            issuer,
            valid_from,
            valid_to,
            fingerprint,
            serial_number,
            signature_algorithm,
            public_key_algorithm,
            key_size,
            san_domains,
        },
        cert,
    ))
}

/// 从 EC 曲线 OID 推导密钥位长的辅助已不再需要：x509-parser 的
/// `ECPoint::key_size()` 可直接从公钥点长度得出位长。


/// 证据驱动的安全发现：每条发现都对应一个可通过远程握手观测到的前提条件。
/// 依赖服务器软件版本（如 Heartbleed）或本地配置（如 CRIME 需要开启压缩但未观测到）
/// 的已知漏洞不在此列出，前端会说明这一限制。
fn detect_cve_vulnerabilities(
    cert: &SslCertificate,
    supported_versions: &[String],
    cipher_suites: &[CipherSuite],
    compression_supported: bool,
) -> Vec<SecurityVulnerability> {
    fn finding(
        id: &str,
        name: &str,
        description: &str,
        severity: &str,
        category: &str,
        evidence: String,
        cve_ids: Vec<&str>,
        affected_components: Vec<String>,
        remediation: &str,
        references: Vec<&str>,
        grade_cap: &str,
    ) -> SecurityVulnerability {
        SecurityVulnerability {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            severity: severity.to_string(),
            category: category.to_string(),
            evidence,
            cve_ids: cve_ids.iter().map(|s| s.to_string()).collect(),
            affected_components,
            remediation: remediation.to_string(),
            references: references.iter().map(|s| s.to_string()).collect(),
            grade_cap: grade_cap.to_string(),
        }
    }

    // 收集符合条件的套件及接受的版本，用于生成证据文本
    let matched_suites =
        |pred: &dyn Fn(&CipherSuite) -> bool| -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            for suite in cipher_suites.iter().filter(|s| pred(s)) {
                out.push(format!(
                    "{}（{}）",
                    suite.name,
                    suite.versions.join("/")
                ));
            }
            out.sort();
            out
        };

    let has_version = |v: &str| supported_versions.iter().any(|x| x == v);
    let mut findings = Vec::new();

    // ---- 协议类发现 ----

    if has_version("SSL 2.0") {
        findings.push(finding(
            "SSL-2.0",
            "支持已废止的 SSL 2.0 协议",
            "SSL 2.0 存在多个根本性设计缺陷，是 DROWN 攻击的基础，RFC 6176 已于 2011 年废止该协议。",
            "CRITICAL",
            "protocol",
            "向服务器发送 SSL 2.0 握手探测，服务器以 SSLv2 ServerHello 应答".to_string(),
            vec!["CVE-2016-0800"],
            vec!["SSL 2.0".to_string()],
            "完全禁用 SSL 2.0。现代服务器软件默认已移除该协议，请检查是否使用了过旧的服务器软件或代理。",
            vec!["https://drownattack.com/", "https://nvd.nist.gov/vuln/detail/CVE-2016-0800", "https://datatracker.ietf.org/doc/html/rfc6176"],
            "F",
        ));
    }

    if has_version("SSL 3.0") {
        findings.push(finding(
            "SSL-3.0",
            "支持已废止的 SSL 3.0 协议",
            "SSL 3.0 受 POODLE 填充预言攻击影响，RFC 7568 已于 2015 年废止该协议。",
            "HIGH",
            "protocol",
            "向服务器发送 SSL 3.0 握手探测，服务器精确协商 SSL 3.0".to_string(),
            vec!["CVE-2014-3566"],
            vec!["SSL 3.0".to_string()],
            "完全禁用 SSL 3.0，仅保留 TLS 1.2 及以上版本。",
            vec!["https://nvd.nist.gov/vuln/detail/CVE-2014-3566", "https://datatracker.ietf.org/doc/html/rfc7568"],
            "F",
        ));
    }

    if has_version("TLS 1.0") {
        let cbc = matched_suites(&|s| {
            s.versions.iter().any(|v| v == "TLS 1.0") && s.name.contains("CBC")
        });
        let evidence = if cbc.is_empty() {
            "向服务器发送 TLS 1.0 握手探测，服务器精确协商 TLS 1.0".to_string()
        } else {
            format!(
                "向服务器发送 TLS 1.0 握手探测，服务器精确协商 TLS 1.0，且接受 CBC 模式套件（BEAST 利用前提）：{}",
                cbc.join("、")
            )
        };
        findings.push(finding(
            "TLS-1.0",
            "支持已废弃的 TLS 1.0 协议",
            "TLS 1.0 已于 2021 年被 RFC 8996 正式废弃，主流合规标准（PCI DSS 等）均要求禁用；配合 CBC 套件时受 BEAST 影响。",
            "MEDIUM",
            "protocol",
            evidence,
            vec!["CVE-2011-3389"],
            vec!["TLS 1.0".to_string()],
            "禁用 TLS 1.0，仅保留 TLS 1.2 及以上版本。",
            vec!["https://nvd.nist.gov/vuln/detail/CVE-2011-3389", "https://datatracker.ietf.org/doc/html/rfc8996"],
            "B",
        ));
    }

    if has_version("TLS 1.1") {
        findings.push(finding(
            "TLS-1.1",
            "支持已废弃的 TLS 1.1 协议",
            "TLS 1.1 已于 2021 年被 RFC 8996 正式废弃，应迁移到 TLS 1.2 及以上版本。",
            "LOW",
            "protocol",
            "向服务器发送 TLS 1.1 握手探测，服务器精确协商 TLS 1.1".to_string(),
            vec![],
            vec!["TLS 1.1".to_string()],
            "禁用 TLS 1.1，仅保留 TLS 1.2 及以上版本。",
            vec!["https://datatracker.ietf.org/doc/html/rfc8996"],
            "B",
        ));
    }

    // ---- 套件类发现 ----

    let export_suites = matched_suites(&|s| s.name.contains("EXPORT"));
    if !export_suites.is_empty() {
        findings.push(finding(
            "EXPORT-CIPHER",
            "接受出口级（EXPORT）加密套件",
            "出口级套件使用 40/56 位密钥，可被现代算力实时破解，是 FREAK 和 Logjam 攻击的前提。",
            "HIGH",
            "cipher",
            format!("握手探测确认服务器接受了出口级套件：{}", export_suites.join("、")),
            vec!["CVE-2015-0204", "CVE-2015-4000"],
            export_suites.clone(),
            "从服务器配置中移除所有 EXPORT 级加密套件，并更新 OpenSSL 到已修补版本。",
            vec!["https://freakattack.com/", "https://weakdh.org/", "https://nvd.nist.gov/vuln/detail/CVE-2015-0204"],
            "F",
        ));
    }

    let rc4_suites = matched_suites(&|s| s.name.contains("RC4"));
    if !rc4_suites.is_empty() {
        findings.push(finding(
            "RC4-CIPHER",
            "接受 RC4 加密套件",
            "RC4 流密码存在多个偏差攻击（如 RC4 NOMORE），IETF 已于 RFC 7465 禁止其用于 TLS。",
            "HIGH",
            "cipher",
            format!("握手探测确认服务器接受了 RC4 套件：{}", rc4_suites.join("、")),
            vec!["CVE-2013-2566", "CVE-2015-2808"],
            rc4_suites.clone(),
            "禁用所有 RC4 加密套件，改用 AES-GCM 或 ChaCha20-Poly1305。",
            vec!["https://www.rc4nomore.com/", "https://nvd.nist.gov/vuln/detail/CVE-2013-2566"],
            "F",
        ));
    }

    let null_suites = matched_suites(&|s| {
        s.name.contains("NULL") || s.name.contains("anon")
    });
    if !null_suites.is_empty() {
        findings.push(finding(
            "NULL-CIPHER",
            "接受 NULL 加密或匿名协商套件",
            "NULL 套件不加密流量，匿名（aNULL）套件不验证服务器身份，均等同于明文通信。",
            "CRITICAL",
            "cipher",
            format!("握手探测确认服务器接受了无加密/匿名套件：{}", null_suites.join("、")),
            vec![],
            null_suites.clone(),
            "禁用所有 NULL 与匿名（aNULL/anon）加密套件。",
            vec!["https://datatracker.ietf.org/doc/html/rfc7525"],
            "F",
        ));
    }

    let sweet32_suites = matched_suites(&|s| {
        s.name.contains("3DES") || s.name.contains("DES_CBC") || s.name.contains("DES40")
    });
    if !sweet32_suites.is_empty() {
        findings.push(finding(
            "3DES-CIPHER",
            "接受 64 位分组密码（3DES/DES）",
            "64 位分组密码受 Sweet32 生日攻击影响，长期会话可在数小时内泄露明文内容。",
            "MEDIUM",
            "cipher",
            format!("握手探测确认服务器接受了 64 位分组密码套件：{}", sweet32_suites.join("、")),
            vec!["CVE-2016-2183"],
            sweet32_suites.clone(),
            "禁用 3DES 和 DES 套件，改用 AES-GCM 或 ChaCha20-Poly1305。",
            vec!["https://sweet32.info/", "https://nvd.nist.gov/vuln/detail/CVE-2016-2183"],
            "C",
        ));
    }

    // ---- TLS 压缩（CRIME 前提，直接观测 ServerHello 压缩方法）----

    if compression_supported {
        findings.push(finding(
            "TLS-COMPRESSION",
            "启用 TLS 级压缩",
            "TLS 压缩是 CRIME 信息泄露攻击的前提，攻击者可据此恢复会话 Cookie 等明文内容。",
            "HIGH",
            "protocol",
            "握手探测中服务器在 ServerHello 中选择了 zlib 压缩方法".to_string(),
            vec!["CVE-2012-4929"],
            vec!["TLS 压缩".to_string()],
            "禁用 TLS 压缩。Apache：SSLCompression off；Nginx：ssl_compression off。",
            vec!["https://nvd.nist.gov/vuln/detail/CVE-2012-4929"],
            "C",
        ));
    }

    // ---- 密钥交换类发现 ----

    let rsa_kex_suites = matched_suites(&|s| {
        s.name.starts_with("TLS_RSA_") || s.name.starts_with("SSL_RSA_")
    });
    if !rsa_kex_suites.is_empty() {
        findings.push(finding(
            "RSA-KEY-EXCHANGE",
            "接受无前向保密的静态 RSA 密钥交换",
            "静态 RSA 密钥交换不具备前向保密性：服务器私钥一旦泄露，历史流量可被完整解密。",
            "MEDIUM",
            "key-exchange",
            format!("握手探测确认服务器接受了静态 RSA 密钥交换套件：{}", rsa_kex_suites.join("、")),
            vec![],
            rsa_kex_suites.clone(),
            "仅保留 ECDHE/DHE 密钥交换套件以启用前向保密。",
            vec!["https://datatracker.ietf.org/doc/html/rfc7525"],
            "B",
        ));
    }

    // ---- 证书类发现 ----

    let days_until_expiry = chrono::DateTime::parse_from_rfc3339(&cert.valid_to)
        .ok()
        .map(|valid_to| {
            (valid_to.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days()
        });

    if let Some(days) = days_until_expiry {
        if days < 0 {
            findings.push(finding(
                "EXPIRED-CERT",
                "证书已过期",
                "过期的证书会导致所有现代浏览器直接拒绝连接。",
                "CRITICAL",
                "certificate",
                format!("证书已于 {} 过期（{} 天前）", cert.valid_to, -days),
                vec![],
                vec![format!("有效期至 {}", cert.valid_to)],
                "立即更换证书。建议使用 Let's Encrypt certbot 等工具实现自动化续期。",
                vec![],
                "F",
            ));
        } else if days < 30 {
            findings.push(finding(
                "EXPIRING-CERT",
                "证书即将过期",
                "证书将在 30 天内过期，需安排续期以避免服务中断。",
                "MEDIUM",
                "certificate",
                format!("证书将于 {} 天后过期（{}）", days, cert.valid_to),
                vec![],
                vec![format!("有效期至 {}", cert.valid_to)],
                "尽快续期证书，并配置到期监控或自动续期。",
                vec![],
                "",
            ));
        }
    }

    let sig_lower = cert.signature_algorithm.to_lowercase();
    if sig_lower.contains("md5") {
        findings.push(finding(
            "MD5-SIG",
            "使用已破解的 MD5 签名算法",
            "MD5 签名可被伪造（已有选择前缀碰撞的实践案例），攻击者可制作假证书。",
            "CRITICAL",
            "certificate",
            format!("证书签名为 {}", cert.signature_algorithm),
            vec![],
            vec![cert.signature_algorithm.clone()],
            "立即更换为 SHA-256 或更强的签名算法。",
            vec![],
            "F",
        ));
    } else if sig_lower.contains("sha1") && !sig_lower.contains("sha256") {
        findings.push(finding(
            "SHA1-SIG",
            "使用已弃用的 SHA-1 签名算法",
            "SHA-1 已可被构造碰撞，所有主流浏览器自 2017 年起拒绝 SHA-1 证书。",
            "HIGH",
            "certificate",
            format!("证书签名为 {}", cert.signature_algorithm),
            vec![],
            vec![cert.signature_algorithm.clone()],
            "更换为 SHA-256 或更强的签名算法。",
            vec![],
            "C",
        ));
    }

    if let Some(key_size) = cert.key_size {
        let weak = match cert.public_key_algorithm.as_str() {
            "RSA" => key_size < 2048,
            "EC" => key_size < 256,
            _ => false,
        };
        if weak {
            let description = format!(
                "{} 位{}密钥不符合当前安全标准（RSA 至少 2048 位 / EC 至少 256 位）。",
                key_size, cert.public_key_algorithm
            );
            findings.push(finding(
                "WEAK-KEY",
                "证书密钥长度不足",
                &description,
                "HIGH",
                "certificate",
                format!(
                    "证书使用 {} 密钥，长度 {} 位",
                    cert.public_key_algorithm, key_size
                ),
                vec![],
                vec![format!("{} 密钥 {} 位", cert.public_key_algorithm, key_size)],
                match cert.public_key_algorithm.as_str() {
                    "RSA" => "更换为至少 2048 位的 RSA 密钥，推荐 3072 位以上。",
                    _ => "更换为至少 256 位的 ECC 密钥（如 P-256）。",
                },
                vec!["https://www.keylength.com/"],
                "C",
            ));
        }
    }

    findings
}

/// 生效的等级封顶说明
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GradeCapInfo {
    /// 触发封顶的发现项名称
    pub finding: String,
    /// 封顶等级（如 "B"）
    pub cap: String,
    /// 证据/原因
    pub reason: String,
}

/// 统一安全评估结果：Overview 的 SSL Labs 评级与 Security 页的分数同源
#[derive(Debug)]
struct SecurityAssessment {
    certificate_score: u32,
    protocol_score: u32,
    key_exchange_score: u32,
    cipher_strength_score: u32,
    /// 封顶后的最终分数
    score: u32,
    grade: String,
    has_warnings: bool,
    has_errors: bool,
    applied_caps: Vec<GradeCapInfo>,
    details: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SslLabsRating {
    pub grade: String,
    pub score: u32,
    pub has_warnings: bool,
    pub has_errors: bool,
    pub certificate_score: u32,
    pub protocol_score: u32,
    pub key_exchange_score: u32,
    pub cipher_strength_score: u32,
    pub applied_caps: Vec<GradeCapInfo>,
    pub details: String,
}

/// 统一评分引擎：四类加权分 + 发现项等级封顶。
/// 发现项不再逐条扣分，而是对总分设置上限（对应等级区间的上界），
/// 同一根因只封顶一次，避免同一问题被多条发现重复惩罚。
fn assess_security(
    cert: &SslCertificate,
    ssl_versions: &[String],
    cipher_suites: &[CipherSuite],
    findings: &[SecurityVulnerability],
) -> SecurityAssessment {
    let certificate_score = calculate_certificate_score(cert);
    let protocol_score = calculate_protocol_score(ssl_versions);
    let key_exchange_score = calculate_key_exchange_score(cipher_suites);
    let cipher_strength_score = calculate_cipher_strength_score(cipher_suites);

    // 权重沿用 SSL Labs：证书 30% / 协议 30% / 密钥交换 30% / 套件强度 10%
    let weighted_score = ((certificate_score as f32 * 0.30)
        + (protocol_score as f32 * 0.30)
        + (key_exchange_score as f32 * 0.30)
        + (cipher_strength_score as f32 * 0.10)) as u32;

    let mut applied_caps: Vec<GradeCapInfo> = findings
        .iter()
        .filter(|f| !f.grade_cap.is_empty())
        .map(|f| GradeCapInfo {
            finding: f.name.clone(),
            cap: f.grade_cap.clone(),
            reason: f.evidence.clone(),
        })
        .collect();
    // 按封顶严重程度排序（F 在前）
    applied_caps.sort_by_key(|c| cap_score_ceiling(&c.cap));

    let mut score = weighted_score;
    for cap in &applied_caps {
        score = score.min(cap_score_ceiling(&cap.cap));
    }

    let has_errors = findings.iter().any(|f| f.severity == "CRITICAL") || score < 50;
    let has_warnings = findings.iter().any(|f| f.severity == "HIGH" || f.severity == "MEDIUM")
        || !applied_caps.is_empty()
        || score < 80;

    let details = if applied_caps.is_empty() && weighted_score >= 80 {
        "未发现封顶级安全问题".to_string()
    } else if applied_caps.is_empty() {
        format!("加权得分 {}，未触发等级封顶", weighted_score)
    } else {
        format!(
            "加权得分 {}，受 {} 项发现封顶至 {}",
            weighted_score,
            applied_caps.len(),
            score
        )
    };

    SecurityAssessment {
        certificate_score,
        protocol_score,
        key_exchange_score,
        cipher_strength_score,
        score,
        grade: grade_from_score(score),
        has_warnings,
        has_errors,
        applied_caps,
        details,
    }
}

/// 封顶等级对应的分数上限（即该等级区间的上界）
fn cap_score_ceiling(cap: &str) -> u32 {
    match cap {
        "F" => 30,
        "D" => 59,
        "C" => 69,
        "B" => 79,
        "A-" => 89,
        "A" => 94,
        _ => 100,
    }
}

/// 从分数映射等级：A+ ≥95 / A ≥90 / A- ≥80 / B ≥70 / C ≥60 / D ≥50 / 否则 F
fn grade_from_score(score: u32) -> String {
    match score {
        95..=100 => "A+",
        90..=94 => "A",
        80..=89 => "A-",
        70..=79 => "B",
        60..=69 => "C",
        50..=59 => "D",
        _ => "F",
    }
    .to_string()
}

/// 从证据驱动的发现项生成 Security 页的漏洞/建议文案列表
fn derive_security_lists(
    findings: &[SecurityVulnerability],
    score: u32,
) -> (Vec<String>, Vec<String>) {
    let vulnerabilities = findings
        .iter()
        .map(|f| format!("{}：{}", f.name, f.evidence))
        .collect();

    let mut recommendations: Vec<String> = findings
        .iter()
        .filter(|f| !f.remediation.is_empty())
        .map(|f| f.remediation.clone())
        .collect();

    if score >= 90 {
        recommendations.push("SSL/TLS 配置优秀，建议定期复查。".to_string());
    } else if score >= 70 {
        recommendations.push("SSL/TLS 配置良好，仍有优化空间。".to_string());
    } else if score >= 50 {
        recommendations.push("SSL/TLS 配置需要改进，存在安全风险。".to_string());
    } else {
        recommendations.push("SSL/TLS 配置存在严重安全风险，需要立即修复。".to_string());
    }
    recommendations.push("启用 HSTS：Strict-Transport-Security: max-age=31536000; includeSubDomains; preload".to_string());
    recommendations.push("配置 OCSP Stapling 提高证书验证性能：ssl_stapling on; ssl_stapling_verify on;".to_string());
    recommendations.push("仅启用 TLS 1.2 和 TLS 1.3，禁用 SSL 2.0/3.0 与 TLS 1.0/1.1。".to_string());

    (vulnerabilities, recommendations)
}

// Individual scoring components
fn calculate_certificate_score(cert: &SslCertificate) -> u32 {
    let mut score = 100u32;

    // Certificate validity
    if let Ok(valid_to) = chrono::DateTime::parse_from_rfc3339(&cert.valid_to) {
        let now = chrono::Utc::now();
        let valid_to_utc = valid_to.with_timezone(&chrono::Utc);
        let days_until_expiry = (valid_to_utc - now).num_days();

        if days_until_expiry < 0 {
            score = 0; // Expired certificate = 0 points
        } else if days_until_expiry < 7 {
            score = score.saturating_sub(50);
        } else if days_until_expiry < 30 {
            score = score.saturating_sub(20);
        }
    }

    // Key size scoring (more granular)
    if let Some(key_size) = cert.key_size {
        let key_penalty = match cert.public_key_algorithm.as_str() {
            "RSA" => match key_size {
                size if size < 1024 => 60, // Severely penalize weak keys
                size if size < 2048 => 30, // Penalize 1024-bit keys
                size if size < 3072 => 5,  // Minor penalty for 2048-bit
                _ => 0,
            },
            "EC" | "ECDSA" => match key_size {
                size if size < 256 => 40,
                size if size < 384 => 10,
                _ => 0,
            },
            _ => 0,
        };
        score = score.saturating_sub(key_penalty);
    }

    // Signature algorithm scoring
    let sig_penalty = if cert.signature_algorithm.to_lowercase().contains("sha1") {
        if cert.signature_algorithm.to_lowercase().contains("rsa") {
            40 // SHA-1 with RSA is particularly bad
        } else {
            30 // SHA-1 in general
        }
    } else if cert.signature_algorithm.to_lowercase().contains("md5") {
        50 // MD5 is completely broken
    } else if cert.signature_algorithm.to_lowercase().contains("sha256") {
        0 // SHA-256 is good
    } else if cert.signature_algorithm.to_lowercase().contains("sha384")
        || cert.signature_algorithm.to_lowercase().contains("sha512")
    {
        0 // SHA-384/512 are excellent
    } else {
        10 // Unknown algorithm, small penalty
    };
    score = score.saturating_sub(sig_penalty);

    score
}

fn calculate_protocol_score(ssl_versions: &[String]) -> u32 {
    let mut score = 100u32;

    let has_ssl2 = ssl_versions.contains(&"SSL 2.0".to_string());
    let has_ssl3 = ssl_versions.contains(&"SSL 3.0".to_string());
    let has_tls10 = ssl_versions.contains(&"TLS 1.0".to_string());
    let has_tls11 = ssl_versions.contains(&"TLS 1.1".to_string());
    let has_tls12 = ssl_versions.contains(&"TLS 1.2".to_string());
    let has_tls13 = ssl_versions.contains(&"TLS 1.3".to_string());

    // Severe penalties for deprecated protocols
    if has_ssl2 {
        score = 0; // SSL 2.0 = immediate F grade
    } else if has_ssl3 {
        score = score.saturating_sub(80); // SSL 3.0 is very bad
    }

    // Penalties for weak TLS versions
    if has_tls10 {
        score = score.saturating_sub(20);
    }
    if has_tls11 {
        score = score.saturating_sub(10);
    }

    // Bonus for modern protocols
    if has_tls13 {
        score = std::cmp::min(100, score + 5); // Small bonus for TLS 1.3
    }

    // Must support at least TLS 1.2
    if !has_tls12 && !has_tls13 {
        score = score.saturating_sub(50);
    }

    score
}

fn calculate_key_exchange_score(cipher_suites: &[CipherSuite]) -> u32 {
    let mut score = 100u32;

    let has_dhe = cipher_suites.iter().any(|cs| cs.name.contains("DHE"));
    let has_ecdhe = cipher_suites.iter().any(|cs| cs.name.contains("ECDHE"));
    // TLS 1.3 强制使用 (EC)DHE 临时密钥交换，必然具备前向保密
    let has_tls13 = cipher_suites
        .iter()
        .any(|cs| cs.versions.iter().any(|v| v == "TLS 1.3"));
    let has_rsa_key_exchange = cipher_suites.iter().any(|cs| {
        cs.name.contains("TLS_RSA_")
            || (cs.name.contains("RSA") && !cs.name.contains("ECDHE") && !cs.name.contains("DHE"))
    });

    // Forward secrecy is critical
    if !has_dhe && !has_ecdhe && !has_tls13 {
        score = score.saturating_sub(40); // No forward secrecy
    }

    // RSA key exchange without PFS is penalized
    if has_rsa_key_exchange {
        score = score.saturating_sub(20);
    }

    // Bonus for ECDHE (preferred over DHE)
    if has_ecdhe {
        score = std::cmp::min(100, score + 5);
    }

    score
}

fn calculate_cipher_strength_score(cipher_suites: &[CipherSuite]) -> u32 {
    if cipher_suites.is_empty() {
        return 0;
    }

    let weak_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "WEAK")
        .count();
    let medium_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "MEDIUM")
        .count();
    let high_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "HIGH")
        .count();
    let total_count = cipher_suites.len();

    // Calculate percentage-based score
    let weak_penalty = (weak_count as f32 / total_count as f32 * 60.0) as u32;
    let medium_penalty = (medium_count as f32 / total_count as f32 * 20.0) as u32;

    let mut score = 100u32;
    score = score.saturating_sub(weak_penalty);
    score = score.saturating_sub(medium_penalty);

    // Bonus for having high-strength ciphers
    if high_count > 0 {
        let high_bonus = std::cmp::min(10, (high_count as f32 / total_count as f32 * 10.0) as u32);
        score = std::cmp::min(100, score + high_bonus);
    }

    score
}

#[tauri::command]
pub async fn check_ssl_info(domain: String) -> Result<SslInfo, String> {
    let domain = domain.trim().to_lowercase();

    if domain.is_empty() {
        return Err("域名不能为空".to_string());
    }

    // Resolve IP address
    let server_ip = match resolve_domain_ip(&domain) {
        Ok(ip) => Some(ip.to_string()),
        Err(_) => None,
    };

    // Get server info
    let server_info = get_https_server_info(&domain)
        .await
        .or_else(|| get_server_info(&domain, 443))
        .or_else(|| get_server_info(&domain, 80));

    // Check protocol support and get certificate
    let (
        certificate,
        certificate_chain,
        ssl_versions,
        cipher_suites,
        protocol_support,
        server_cipher_order,
        security_score,
        ssl_labs_rating,
        vulnerabilities,
        recommendations,
        cve_vulnerabilities,
        http2_support,
        spdy_support,
        http3_support,
        alpn_protocols,
    ) = match check_tls_connection(&domain, 443).await {
        Ok((cert_der, negotiated_suites, cert_chain_ders)) => {
            match parse_certificate(&cert_der) {
                Ok(cert) => {
                    // Build certificate chain（与 CertificateViewer 共用链级别判定）
                    let certificate_chain = match build_certificate_chain(&cert_chain_ders) {
                        Ok(chain) => Some(chain),
                        Err(_) => None,
                    };

                    // Get detailed protocol support（含各版本真实套件枚举与压缩观测）
                    let (protocol_support, compression_supported) =
                        match check_protocol_support(&domain, 443).await {
                            Ok((support, zlib)) => (Some(support), zlib),
                            Err(_) => (None, false),
                        };

                    // Get server cipher order preference
                    let server_cipher_order = match check_server_cipher_order(&domain, 443).await {
                        Ok(order) => Some(order),
                        Err(_) => None,
                    };

                    // Check HTTP/2 support
                    let http2_support = match check_http2_support(&domain, 443).await {
                        Ok(supported) => Some(supported),
                        Err(_) => None,
                    };

                    // Check SPDY support
                    let spdy_support = match check_spdy_support(&domain, 443).await {
                        Ok(supported) => Some(supported),
                        Err(_) => None,
                    };

                    // Check HTTP/3 support
                    let http3_support = match check_http3_support(&domain, 443).await {
                        Ok(supported) => Some(supported),
                        Err(_) => None,
                    };

                    // Check ALPN support
                    let alpn_protocols = match check_alpn_support(&domain, 443).await {
                        Ok(protocols) => Some(protocols),
                        Err(_) => None,
                    };

                    // Extract supported versions from protocol support
                    let supported_versions: Vec<String> = protocol_support
                        .as_ref()
                        .map(|support| {
                            support
                                .iter()
                                .filter(|p| p.supported)
                                .map(|p| p.version.clone())
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["TLS 1.2".to_string(), "TLS 1.3".to_string()]);

                    // 聚合各协议版本真实探测到的套件 + rustls 协商的套件
                    let per_version: Vec<(String, Vec<CipherSuite>)> = protocol_support
                        .as_ref()
                        .map(|support| {
                            support
                                .iter()
                                .filter(|p| p.supported)
                                .map(|p| (p.version.clone(), p.cipher_suites.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let cipher_suites =
                        aggregate_cipher_suites(per_version, negotiated_suites.first().cloned());

                    // 仅证据判定的安全发现（每条都有真实探测证据）
                    let findings = detect_cve_vulnerabilities(
                        &cert,
                        &supported_versions,
                        &cipher_suites,
                        compression_supported,
                    );

                    // 统一评分：四类加权分 + 等级封顶，Overview 与 Security 页共用
                    let assessment =
                        assess_security(&cert, &supported_versions, &cipher_suites, &findings);
                    let (vulns, recs) = derive_security_lists(&findings, assessment.score);

                    let rating = SslLabsRating {
                        grade: assessment.grade.clone(),
                        score: assessment.score,
                        has_warnings: assessment.has_warnings,
                        has_errors: assessment.has_errors,
                        certificate_score: assessment.certificate_score,
                        protocol_score: assessment.protocol_score,
                        key_exchange_score: assessment.key_exchange_score,
                        cipher_strength_score: assessment.cipher_strength_score,
                        applied_caps: assessment.applied_caps.clone(),
                        details: assessment.details,
                    };

                    (
                        Some(cert),
                        certificate_chain,
                        Some(supported_versions),
                        Some(cipher_suites),
                        protocol_support,
                        server_cipher_order,
                        Some(assessment.score),
                        Some(rating),
                        Some(vulns),
                        Some(recs),
                        Some(findings),
                        http2_support,
                        spdy_support,
                        http3_support,
                        alpn_protocols,
                    )
                }
                Err(_) => (
                    None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                    None,
                ),
            }
        }
        Err(_) => (
            None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        ),
    };

    Ok(SslInfo {
        domain,
        server_ip,
        server_info,
        certificate,
        certificate_chain,
        ssl_versions,
        cipher_suites,
        protocol_support,
        server_cipher_order,
        security_score,
        ssl_labs_rating,
        vulnerabilities,
        recommendations,
        cve_vulnerabilities,
        http2_support,
        spdy_support,
        http3_support,
        alpn_protocols,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 套件强度分类 =====

    #[test]
    fn test_cipher_strength_classification() {
        // AES_128_GCM 曾因包含 "aes128" 被误判为 MEDIUM，回归验证
        assert_eq!(
            classify_cipher_suite_strength("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"),
            "HIGH"
        );
        assert_eq!(classify_cipher_suite_strength("TLS_AES_128_GCM_SHA256"), "HIGH");
        assert_eq!(
            classify_cipher_suite_strength("TLS_AES_256_GCM_SHA384"),
            "HIGH"
        );
        assert_eq!(
            classify_cipher_suite_strength("TLS_CHACHA20_POLY1305_SHA256"),
            "HIGH"
        );
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_WITH_AES_128_CBC_SHA"), "MEDIUM");
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_WITH_3DES_EDE_CBC_SHA"), "MEDIUM");
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_WITH_RC4_128_SHA"), "WEAK");
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_WITH_DES_CBC_SHA"), "WEAK");
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_EXPORT_WITH_RC4_40_MD5"), "WEAK");
        assert_eq!(classify_cipher_suite_strength("TLS_RSA_WITH_NULL_SHA"), "WEAK");
    }

    // ===== 等级映射与封顶 =====

    #[test]
    fn test_grade_from_score_boundaries() {
        assert_eq!(grade_from_score(100), "A+");
        assert_eq!(grade_from_score(95), "A+");
        assert_eq!(grade_from_score(94), "A");
        assert_eq!(grade_from_score(90), "A");
        assert_eq!(grade_from_score(89), "A-");
        assert_eq!(grade_from_score(80), "A-");
        assert_eq!(grade_from_score(79), "B");
        assert_eq!(grade_from_score(70), "B");
        assert_eq!(grade_from_score(69), "C");
        assert_eq!(grade_from_score(60), "C");
        assert_eq!(grade_from_score(59), "D");
        assert_eq!(grade_from_score(50), "D");
        assert_eq!(grade_from_score(49), "F");
        assert_eq!(grade_from_score(0), "F");
    }

    #[test]
    fn test_cap_score_ceiling() {
        assert_eq!(cap_score_ceiling("F"), 30);
        assert_eq!(cap_score_ceiling("C"), 69);
        assert_eq!(cap_score_ceiling("B"), 79);
        assert_eq!(cap_score_ceiling("A-"), 89);
        assert_eq!(cap_score_ceiling(""), 100);
    }

    fn sample_cert() -> SslCertificate {
        SslCertificate {
            subject: "CN=example.test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2026-01-01T00:00:00+08:00".to_string(),
            valid_to: "2027-01-01T00:00:00+08:00".to_string(),
            fingerprint: "aa".to_string(),
            serial_number: "01".to_string(),
            signature_algorithm: "SHA256withRSA".to_string(),
            public_key_algorithm: "RSA".to_string(),
            key_size: Some(2048),
            san_domains: None,
        }
    }

    fn suite(name: &str, versions: &[&str], strength: &str) -> CipherSuite {
        CipherSuite {
            name: name.to_string(),
            versions: versions.iter().map(|s| s.to_string()).collect(),
            strength: strength.to_string(),
            server_order: false,
        }
    }

    fn vuln(id: &str, severity: &str, cap: &str) -> SecurityVulnerability {
        SecurityVulnerability {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            severity: severity.to_string(),
            category: "protocol".to_string(),
            evidence: "test evidence".to_string(),
            cve_ids: vec![],
            affected_components: vec![],
            remediation: String::new(),
            references: vec![],
            grade_cap: cap.to_string(),
        }
    }

    #[test]
    fn test_assess_security_caps_applied_once() {
        // 加权分接近满分，但 TLS 1.0（封顶 B）与 3DES（封顶 C）并存 → 取更低封顶 69 → C
        let cert = sample_cert();
        let versions = vec!["TLS 1.2".to_string(), "TLS 1.3".to_string()];
        let suites = vec![
            suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH"),
            suite("TLS_AES_256_GCM_SHA384", &["TLS 1.3"], "HIGH"),
        ];
        let findings = vec![
            vuln("TLS-1.0", "MEDIUM", "B"),
            vuln("3DES-CIPHER", "MEDIUM", "C"),
        ];
        let assessment = assess_security(&cert, &versions, &suites, &findings);
        assert_eq!(assessment.score, 69);
        assert_eq!(assessment.grade, "C");
        assert_eq!(assessment.applied_caps.len(), 2);
        // 封顶按严重程度排序，C 在 B 前
        assert_eq!(assessment.applied_caps[0].cap, "C");
    }

    #[test]
    fn test_assess_security_no_findings_keeps_weighted() {
        let cert = sample_cert();
        let versions = vec!["TLS 1.3".to_string()];
        let suites = vec![suite("TLS_AES_128_GCM_SHA256", &["TLS 1.3"], "HIGH")];
        let assessment = assess_security(&cert, &versions, &suites, &[]);
        assert_eq!(assessment.score, assessment.weighted_hint());
        assert_eq!(assessment.grade, "A+");
        assert!(assessment.applied_caps.is_empty());
    }

    impl SecurityAssessment {
        // 测试辅助：从四类分重算加权分，验证封顶前的期望值
        fn weighted_hint(&self) -> u32 {
            ((self.certificate_score as f32 * 0.30)
                + (self.protocol_score as f32 * 0.30)
                + (self.key_exchange_score as f32 * 0.30)
                + (self.cipher_strength_score as f32 * 0.10)) as u32
        }
    }

    // ===== ServerHello 解析 =====

    /// 构造一个合成 ServerHello 记录
    fn synth_server_hello(version: [u8; 2], random: [u8; 32], cipher: u16, compression: u8, tls13_ext: bool) -> Vec<u8> {
        let mut hs = Vec::new();
        hs.extend_from_slice(&version); // ServerHello legacy_version
        hs.extend_from_slice(&random);
        hs.push(0x00); // session_id 长度 0
        hs.extend_from_slice(&cipher.to_be_bytes());
        hs.push(compression);
        if tls13_ext {
            // supported_versions 扩展：(3,4)。扩展总长 = 4 字节头 + 4 字节体 = 8
            hs.extend_from_slice(&[0x00, 0x08]);
            hs.extend_from_slice(&0x002bu16.to_be_bytes());
            hs.extend_from_slice(&[0x00, 0x02, 0x03, 0x04]);
        }
        let mut record = vec![0x16u8, 0x03, 0x01];
        record.extend_from_slice(&((hs.len() + 4) as u16).to_be_bytes());
        record.push(0x02); // ServerHello
        let hs_len = (hs.len() as u32).to_be_bytes();
        record.extend_from_slice(&hs_len[1..4]);
        record.extend_from_slice(&hs);
        record
    }

    #[test]
    fn test_parse_server_hello_tls12() {
        let record = synth_server_hello([3, 3], [0u8; 32], 0xC02F, 0, false);
        let info = parse_server_hello(&record).expect("应解析成功");
        assert_eq!(info.legacy_version, Some((3, 3)));
        assert_eq!(info.cipher_suite, Some(0xC02F));
        assert_eq!(info.compression_method, Some(0));
        assert_eq!(info.selected_version, None);
        assert!(!info.is_hello_retry_request);
    }

    #[test]
    fn test_parse_server_hello_tls13_with_hrr_detection() {
        let record = synth_server_hello([3, 3], HELLO_RETRY_REQUEST_RANDOM, 0x1301, 0, true);
        let info = parse_server_hello(&record).expect("应解析成功");
        assert_eq!(info.selected_version, Some((3, 4)));
        assert_eq!(info.cipher_suite, Some(0x1301));
        assert!(info.is_hello_retry_request);
    }

    #[test]
    fn test_parse_server_hello_rejects_alert_record() {
        // alert 记录（0x15）与握手类型非 ServerHello 的记录应被拒绝
        let mut alert = vec![0x15u8, 0x03, 0x01, 0x00, 0x02, 0x00, 0x2E];
        assert!(parse_server_hello(&alert).is_none());
        let mut not_sh = synth_server_hello([3, 3], [0u8; 32], 0x002F, 0, false);
        not_sh[5] = 0x0B; // Certificate 类型
        assert!(parse_server_hello(&not_sh).is_none());
        alert.clear();
    }

    // ===== ClientHello 构造 =====

    #[test]
    fn test_build_client_hello_structure() {
        let opts = ClientHelloOptions {
            probe: PROBE_TLS12,
            cipher_codes: vec![0xC02F],
            offer_zlib: false,
            include_sni: true,
            include_sig_algs: true,
            tls13_negotiation: false,
        };
        let hello = build_client_hello("example.com", &opts);
        assert_eq!(hello[0], 0x16); // handshake 记录
        assert_eq!(&hello[1..3], &[0x03, 0x01]); // 记录版本 3.1
        assert_eq!(hello[5], 0x01); // ClientHello
        assert_eq!(&hello[9..11], &[0x03, 0x03]); // client_version 3.3
        // SNI 扩展应包含域名
        let text = hello.windows(11).any(|w| w == b"example.com");
        assert!(text, "ClientHello 应包含 SNI 域名");
    }

    #[test]
    fn test_build_client_hello_ssl30_has_no_extensions() {
        let opts = ClientHelloOptions {
            probe: PROBE_SSL30,
            cipher_codes: vec![0x002F],
            offer_zlib: false,
            include_sni: true,
            include_sig_algs: false,
            tls13_negotiation: false,
        };
        let hello = build_client_hello("example.com", &opts);
        assert_eq!(&hello[1..3], &[0x03, 0x00]); // 记录版本 3.0
        assert!(!hello.windows(11).any(|w| w == b"example.com"), "SSL 3.0 不应携带 SNI 扩展");
    }

    #[test]
    fn test_build_client_hello_tls13_negotiation() {
        let opts = ClientHelloOptions {
            probe: PROBE_TLS13,
            cipher_codes: vec![0x1301],
            offer_zlib: false,
            include_sni: true,
            include_sig_algs: true,
            tls13_negotiation: true,
        };
        let hello = build_client_hello("example.com", &opts);
        assert_eq!(&hello[9..11], &[0x03, 0x03]); // legacy_version 3.3
        // supported_versions 扩展：类型 0x002b + 长度 0x0003 + 列表长 0x02 + 0x0304
        let marker = [0x00u8, 0x2b, 0x00, 0x03, 0x02, 0x03, 0x04];
        assert!(hello.windows(7).any(|w| w == marker), "应包含 TLS 1.3 supported_versions 扩展");
    }

    // ===== 套件聚合 =====

    #[test]
    fn test_aggregate_cipher_suites_merges_versions() {
        let per_version = vec![
            (
                "TLS 1.0".to_string(),
                vec![suite("TLS_RSA_WITH_AES_128_CBC_SHA", &["TLS 1.0"], "MEDIUM")],
            ),
            (
                "TLS 1.2".to_string(),
                vec![suite("TLS_RSA_WITH_AES_128_CBC_SHA", &["TLS 1.2"], "MEDIUM")],
            ),
        ];
        let negotiated = suite("TLS_AES_128_GCM_SHA256", &["TLS 1.3"], "HIGH");
        let merged = aggregate_cipher_suites(per_version, Some(negotiated));
        assert_eq!(merged.len(), 2);
        let cbc = merged.iter().find(|s| s.name.contains("CBC")).unwrap();
        assert_eq!(cbc.versions, vec!["TLS 1.0", "TLS 1.2"]);
    }

    // ===== CVE 检测（仅证据判定）=====

    #[test]
    fn test_detect_no_findings_for_modern_config() {
        let cert = sample_cert();
        let versions = vec!["TLS 1.2".to_string(), "TLS 1.3".to_string()];
        let suites = vec![
            suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH"),
            suite("TLS_AES_128_GCM_SHA256", &["TLS 1.3"], "HIGH"),
        ];
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        assert!(findings.is_empty(), "现代配置不应产生发现项，实际: {:?}", findings.iter().map(|f| &f.id).collect::<Vec<_>>());
    }

    #[test]
    fn test_detect_no_heartbleed_false_positive() {
        // 旧逻辑：支持 TLS 1.2 而无 1.3 会被误判为 Heartbleed 受影响（CRITICAL）
        // 新逻辑：无远程证据时不得出现任何发现
        let cert = sample_cert();
        let versions = vec!["TLS 1.2".to_string()];
        let suites = vec![suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH")];
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_detect_tls10_with_cbc_evidence() {
        let cert = sample_cert();
        let versions = vec!["TLS 1.0".to_string(), "TLS 1.2".to_string()];
        let suites = vec![
            suite("TLS_RSA_WITH_AES_128_CBC_SHA", &["TLS 1.0", "TLS 1.2"], "MEDIUM"),
            suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH"),
        ];
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        let tls10 = findings.iter().find(|f| f.id == "TLS-1.0").expect("应报告 TLS 1.0");
        assert_eq!(tls10.severity, "MEDIUM");
        assert_eq!(tls10.grade_cap, "B");
        assert!(tls10.cve_ids.contains(&"CVE-2011-3389".to_string()));
        assert!(tls10.evidence.contains("CBC"), "证据应注明 CBC 套件");
        // 无 RSA 静态交换以外的弱套件时，不应产生其他套件类发现
        assert!(findings.iter().all(|f| f.id != "RC4-CIPHER"));
    }

    #[test]
    fn test_detect_weak_suite_findings() {
        let cert = sample_cert();
        let versions = vec!["TLS 1.2".to_string()];
        let suites = vec![
            suite("TLS_RSA_WITH_RC4_128_SHA", &["TLS 1.2"], "WEAK"),
            suite("TLS_RSA_WITH_3DES_EDE_CBC_SHA", &["TLS 1.2"], "MEDIUM"),
            suite("TLS_RSA_EXPORT_WITH_RC4_40_MD5", &["TLS 1.0"], "WEAK"),
        ];
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RC4-CIPHER"));
        assert!(ids.contains(&"3DES-CIPHER"));
        assert!(ids.contains(&"EXPORT-CIPHER"));
        assert!(ids.contains(&"RSA-KEY-EXCHANGE"), "TLS_RSA_ 开头套件应触发无前向保密发现");
        let rc4 = findings.iter().find(|f| f.id == "RC4-CIPHER").unwrap();
        assert_eq!(rc4.grade_cap, "F");
        assert!(!rc4.evidence.is_empty());
    }

    #[test]
    fn test_detect_weak_certificate_findings() {
        let mut cert = sample_cert();
        cert.key_size = Some(1024);
        cert.signature_algorithm = "SHA1withRSA".to_string();
        cert.valid_to = "2020-01-01T00:00:00+08:00".to_string();
        let versions = vec!["TLS 1.2".to_string()];
        let suites = vec![suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH")];
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"EXPIRED-CERT"));
        assert!(ids.contains(&"SHA1-SIG"));
        assert!(ids.contains(&"WEAK-KEY"));
        let expired = findings.iter().find(|f| f.id == "EXPIRED-CERT").unwrap();
        assert_eq!(expired.grade_cap, "F");
    }

    #[test]
    fn test_detect_crime_requires_observed_compression() {
        let cert = sample_cert();
        let versions = vec!["TLS 1.2".to_string()];
        let suites = vec![suite("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", &["TLS 1.2"], "HIGH")];
        // 未观测到压缩：不报告
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, false);
        assert!(findings.iter().all(|f| f.id != "TLS-COMPRESSION"));
        // 观测到 zlib：报告并封顶 C
        let findings = detect_cve_vulnerabilities(&cert, &versions, &suites, true);
        let crime = findings.iter().find(|f| f.id == "TLS-COMPRESSION").expect("应报告 TLS 压缩");
        assert_eq!(crime.grade_cap, "C");
    }

    // ===== 端到端冒烟测试（需真实网络，默认忽略）=====
    // 运行：cargo test --lib ssl_checker -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn smoke_check_real_domain() {
        let info = check_ssl_info("github.com".to_string()).await.expect("检测失败");
        eprintln!("=== {} ===", info.domain);
        eprintln!("协议: {:?}", info.ssl_versions);
        eprintln!("评级: {:?}", info.ssl_labs_rating.as_ref().map(|r| (&r.grade, r.score)));
        eprintln!("套件数: {}", info.cipher_suites.as_ref().map(|s| s.len()).unwrap_or(0));
        eprintln!("套件样例: {:?}", info.cipher_suites.as_ref().map(|s| {
            s.iter().take(5).map(|c| (c.name.clone(), c.versions.clone())).collect::<Vec<_>>()
        }));
        if let Some(chain) = &info.certificate_chain {
            eprintln!("链: {} 张, complete={}, root_in_chain={}, anchor={:?}, status={}",
                chain.chain_length, chain.is_complete, chain.root_in_chain,
                chain.trust_anchor_info, chain.chain_validation_status);
            for n in &chain.certificates {
                eprintln!("  L{} {:?} {}", n.chain_level, n.trust_status, n.certificate.subject);
            }
        }
        eprintln!("发现: {:?}", info.cve_vulnerabilities.as_ref().map(|f| {
            f.iter().map(|v| (v.id.clone(), v.severity.clone(), v.grade_cap.clone())).collect::<Vec<_>>()
        }));
        eprintln!("封顶: {:?}", info.ssl_labs_rating.as_ref().map(|r| r.applied_caps.clone()));
        assert!(info.certificate_chain.is_some(), "应能构建证书链");
        let chain = info.certificate_chain.unwrap();
        assert!(chain.is_complete, "github.com 的链应可锚定到受信任根");
        assert!(chain.trust_anchor_info.is_some(), "应匹配到 webpki 信任锚");
    }

    // ===== 证书链分析 =====

    fn pem_to_der(pem: &str) -> Vec<u8> {
        let (_, p) = x509_parser::pem::parse_x509_pem(pem.as_bytes())
            .map_err(|e| panic!("PEM 解析失败: {:?}", e))
            .unwrap();
        p.contents
    }

    fn chain_from_pems(pems: &[&str]) -> CertificateChain {
        let ders: Vec<Vec<u8>> = pems.iter().map(|p| pem_to_der(p)).collect();
        build_certificate_chain(&ders).expect("链构建失败")
    }

    // 以下证书由 OpenSSL 生成：root（自签名 CA）→ inter（pathlen:0）→ leaf；
    // cross_g2 由 root 交叉签发的另一根形态证书（无 pathlen）→ inter_g2（pathlen:0）
    const ROOT_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/root.pem");
    const INTER_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/inter.pem");
    const LEAF_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/leaf.pem");
    const CROSS_G2_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/cross_g2.pem");
    const INTER_G2_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/inter_g2.pem");
    const LEAF_G2_PEM: &str = include_str!("../../tests/fixtures/ssl_chain/leaf_g2.pem");

    #[test]
    fn test_complete_chain_levels_and_roles() {
        let chain = chain_from_pems(&[LEAF_PEM, INTER_PEM, ROOT_PEM]);
        assert_eq!(chain.certificates.len(), 3);
        // 排序后：叶子 → 中间 → 根
        let levels: Vec<u32> = chain.certificates.iter().map(|n| n.chain_level).collect();
        assert_eq!(levels, vec![0, 1, 2]);
        assert_eq!(chain.certificates[0].trust_status, "end-entity");
        assert_eq!(chain.certificates[1].trust_status, "intermediate");
        assert_eq!(chain.certificates[2].trust_status, "self-signed");
        assert!(chain.certificates[0].is_leaf && !chain.certificates[0].is_root);
        assert!(chain.certificates[2].is_root && !chain.certificates[2].is_leaf);
        assert!(chain.is_complete, "含自签名根的链应完整");
        assert!(chain.root_in_chain);
        assert!(chain.trust_anchor_info.is_none(), "自建测试 CA 不在 webpki 信任库中");
    }

    #[test]
    fn test_chain_order_is_normalized() {
        // 服务器乱序发送 [根, 中间, 叶子] 时应被纠正
        let chain = chain_from_pems(&[ROOT_PEM, INTER_PEM, LEAF_PEM]);
        assert_eq!(chain.certificates[0].chain_level, 0);
        assert_eq!(chain.certificates[2].chain_level, 2);
        assert!(chain.is_complete);
        assert!(chain.chain_errors.is_empty());
    }

    #[test]
    fn test_missing_root_detected() {
        // 标准服务器链 [叶子, 中间]：中间 CA 的 issuer（root）不在链内且有 pathlen
        // → 判为中间 CA，链不完整（缺少根）
        let chain = chain_from_pems(&[LEAF_PEM, INTER_PEM]);
        assert_eq!(chain.certificates[1].chain_level, 1);
        assert_eq!(chain.certificates[1].trust_status, "intermediate");
        assert!(!chain.is_complete, "缺少根时链不应判完整");
        assert!(!chain.root_in_chain);
        let info = chain.root_ca_info.unwrap();
        assert!(info.contains("未能匹配到受信任的根 CA"), "root_ca_info 应说明未匹配信任锚: {}", info);
    }

    #[test]
    fn test_cross_signed_root_chain_is_complete() {
        // [叶子, G2中间CA, 交叉签名根]：cross_g2 非自签名、无 pathlen、issuer 不在链内
        // → 按 CertificateViewer 同样的逻辑判为根 CA（level 2），链完整
        let chain = chain_from_pems(&[LEAF_G2_PEM, INTER_G2_PEM, CROSS_G2_PEM]);
        let levels: Vec<u32> = chain.certificates.iter().map(|n| n.chain_level).collect();
        assert_eq!(levels, vec![0, 1, 2]);
        assert_eq!(chain.certificates[2].trust_status, "root-ca");
        assert!(chain.is_complete, "链顶交叉签名形式的根应视为链完整");
        // 真正的旧根（issuer）不在链内，仍可通过信任锚补全语义表达
        assert!(chain.chain_errors.is_empty(), "不应有链中断错误");
    }

}


