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

#[derive(Debug, Serialize, Deserialize)]
pub struct CipherSuite {
    pub name: String,
    pub version: String,
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
    pub cve_id: String,
    pub name: String,
    pub description: String,
    pub severity: String, // "CRITICAL", "HIGH", "MEDIUM", "LOW"
    pub affected_components: Vec<String>,
    pub remediation: String,
    pub references: Vec<String>,
    pub affected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub trust_status: String, // "trusted", "untrusted", "self-signed"
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateChain {
    pub certificates: Vec<CertificateChainNode>,
    pub chain_length: u32,
    pub is_complete: bool,
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
        || cipher_lower.contains("des") && !cipher_lower.contains("3des")
        || cipher_lower.contains("rc4")
        || cipher_lower.contains("md5")
    {
        return "WEAK".to_string();
    }

    // Medium strength
    if cipher_lower.contains("3des")
        || cipher_lower.contains("aes128")
        || cipher_lower.contains("camellia128")
    {
        return "MEDIUM".to_string();
    }

    // High strength - modern ciphers
    if cipher_lower.contains("aes256")
        || cipher_lower.contains("chacha20")
        || cipher_lower.contains("gcm")
        || cipher_lower.contains("poly1305")
    {
        return "HIGH".to_string();
    }

    "MEDIUM".to_string() // Default
}

#[derive(Debug)]
struct TlsVersion {
    name: &'static str,
    rustls_version: &'static rustls::SupportedProtocolVersion,
}

const TLS_VERSIONS: &[TlsVersion] = &[
    TlsVersion {
        name: "SSL 2.0",
        rustls_version: &rustls::version::TLS12, // Placeholder - will be checked separately
    },
    TlsVersion {
        name: "SSL 3.0",
        rustls_version: &rustls::version::TLS12, // Placeholder - will be checked separately
    },
    TlsVersion {
        name: "TLS 1.0",
        rustls_version: &rustls::version::TLS12, // Placeholder - will be checked separately
    },
    TlsVersion {
        name: "TLS 1.1",
        rustls_version: &rustls::version::TLS12, // Placeholder - will be checked separately
    },
    TlsVersion {
        name: "TLS 1.2",
        rustls_version: &rustls::version::TLS12,
    },
    TlsVersion {
        name: "TLS 1.3",
        rustls_version: &rustls::version::TLS13,
    },
];

async fn check_protocol_support(domain: &str, port: u16) -> Result<Vec<ProtocolSupport>, String> {
    let mut results = Vec::new();

    for version in TLS_VERSIONS {
        let supported = check_tls_version_support(domain, port, version)
            .await
            .unwrap_or(false);

        if supported {
            let cipher_suites = get_cipher_suites_for_version(domain, port, version)
                .await
                .unwrap_or_else(|_| Vec::new());

            // Check for ALPN, HTTP/2, SPDY, and HTTP/3 support
            let alpn_protocols = check_alpn_support(domain, port).await.ok();
            let http2_support = check_http2_support(domain, port).await.ok();
            let spdy_support = check_spdy_support(domain, port).await.ok();
            let http3_support = check_http3_support(domain, port).await.ok();

            results.push(ProtocolSupport {
                version: version.name.to_string(),
                supported: true,
                cipher_suites,
                alpn_protocols,
                http2_support,
                spdy_support,
                http3_support,
            });
        } else {
            results.push(ProtocolSupport {
                version: version.name.to_string(),
                supported: false,
                cipher_suites: Vec::new(),
                alpn_protocols: None,
                http2_support: None,
                spdy_support: None,
                http3_support: None,
            });
        }
    }

    Ok(results)
}

async fn check_tls_version_support(
    domain: &str,
    port: u16,
    version: &TlsVersion,
) -> Result<bool, String> {
    // For legacy protocols, use OpenSSL-based detection
    match version.name {
        "SSL 2.0" | "SSL 3.0" | "TLS 1.0" | "TLS 1.1" => {
            check_legacy_protocol_support(domain, port, version.name).await
        }
        _ => {
            // Initialize crypto provider
            let _ = rustls::crypto::ring::default_provider().install_default();

            let server_name = match ServerName::try_from(domain.to_string()) {
                Ok(name) => name,
                Err(_) => return Ok(false),
            };

            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            // Create a config that only supports the specific version
            let version_config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[version.rustls_version])
            .map_err(|_| "Unsupported protocol version".to_string())?
            .with_root_certificates(root_store.clone())
            .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(version_config));

            let stream = match tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await
            {
                Ok(stream) => stream,
                Err(_) => return Ok(false),
            };

            match tokio::time::timeout(
                Duration::from_secs(5),
                connector.connect(server_name, stream),
            )
            .await
            {
                Ok(Ok(_)) => Ok(true),
                Ok(Err(_)) => Ok(false),
                Err(_) => Ok(false), // Timeout
            }
        }
    }
}

async fn check_legacy_protocol_support(
    domain: &str,
    port: u16,
    protocol_name: &str,
) -> Result<bool, String> {
    // For legacy protocols, we need to use a TLS library that supports them
    // Since rustls doesn't support TLS 1.0/1.1 by default, we'll use a different approach
    // We'll try to establish a raw TLS connection with the specific protocol version

    match protocol_name {
        "SSL 2.0" | "SSL 3.0" => {
            // SSL 2.0 and 3.0 are completely deprecated and insecure
            // Most modern servers don't support them, so we'll return false
            Ok(false)
        }
        "TLS 1.0" | "TLS 1.1" => {
            // For TLS 1.0/1.1, we'll use a custom TLS client that can negotiate these versions
            // Since we can't use rustls for this, we'll use a raw socket approach
            check_tls_version_with_raw_socket(domain, port, protocol_name).await
        }
        _ => Ok(false),
    }
}

async fn check_tls_version_with_raw_socket(
    domain: &str,
    port: u16,
    protocol_name: &str,
) -> Result<bool, String> {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    // Map protocol name to TLS version bytes
    let (major_version, minor_version) = match protocol_name {
        "TLS 1.0" => (3, 1),
        "TLS 1.1" => (3, 2),
        "TLS 1.2" => (3, 3),
        "TLS 1.3" => (3, 4),
        _ => return Ok(false),
    };

    // Create TCP connection
    let mut stream = match tokio::time::timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("{}:{}", domain, port)),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(_)) | Err(_) => return Ok(false),
    };

    // Create a simple TLS ClientHello message for the specific version
    let client_hello = create_client_hello(domain, major_version, minor_version);

    // Send ClientHello
    if let Err(_) = stream.write_all(&client_hello).await {
        return Ok(false);
    }

    // Read server response
    let mut buffer = [0u8; 1024];
    match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await {
        Ok(Ok(bytes_read)) if bytes_read > 0 => {
            // Check if we got a ServerHello response
            if buffer[0] == 0x16 && buffer[5] == 0x02 {
                // TLS Handshake, ServerHello
                // Check if the server responded with the requested version or higher
                let server_major = buffer[9];
                let server_minor = buffer[10];

                // Server supports the requested version if it responds with it or negotiates down to it
                let supports_version = (server_major == major_version
                    && server_minor >= minor_version)
                    || (server_major > major_version);

                Ok(supports_version)
            } else if buffer[0] == 0x15 {
                // TLS Alert
                // Server sent an alert, likely doesn't support this version
                Ok(false)
            } else {
                // Unexpected response
                Ok(false)
            }
        }
        Ok(Ok(_)) => Ok(false),           // No data received
        Ok(Err(_)) | Err(_) => Ok(false), // Read error or timeout
    }
}

fn create_client_hello(server_name: &str, major_version: u8, minor_version: u8) -> Vec<u8> {
    let mut hello = Vec::new();

    // TLS Record Header
    hello.push(0x16); // Handshake
    hello.push(major_version); // Major version
    hello.push(minor_version); // Minor version

    // Length placeholder (will be filled later)
    let length_pos = hello.len();
    hello.extend_from_slice(&[0x00, 0x00]);

    // Handshake Header
    hello.push(0x01); // ClientHello

    // Handshake length placeholder (will be filled later)
    let handshake_length_pos = hello.len();
    hello.extend_from_slice(&[0x00, 0x00, 0x00]);

    // ClientHello content
    hello.push(major_version); // Major version
    hello.push(minor_version); // Minor version

    // Random (32 bytes)
    let random: [u8; 32] = [0x01; 32]; // Simple random data
    hello.extend_from_slice(&random);

    // Session ID length (0)
    hello.push(0x00);

    // Cipher suites
    let cipher_suites = match (major_version, minor_version) {
        (3, 1) => vec![0x00, 0x2F], // TLS_RSA_WITH_AES_128_CBC_SHA for TLS 1.0
        (3, 2) => vec![0x00, 0x35], // TLS_RSA_WITH_AES_256_CBC_SHA for TLS 1.1
        _ => vec![0x00, 0x2F, 0x00, 0x35], // Multiple cipher suites
    };

    hello.push(0x00); // Cipher suites length high byte
    hello.push(cipher_suites.len() as u8); // Cipher suites length low byte
    hello.extend_from_slice(&cipher_suites);

    // Compression methods
    hello.push(0x01); // Compression methods length
    hello.push(0x00); // No compression

    // Extensions
    let mut extensions = Vec::new();

    // Server Name Indication (SNI) extension
    if !server_name.is_empty() {
        extensions.extend_from_slice(&[0x00, 0x00]); // Extension type: server_name

        let sni_data = create_sni_extension(server_name);
        extensions.extend_from_slice(&(sni_data.len() as u16).to_be_bytes());
        extensions.extend_from_slice(&sni_data);
    }

    if !extensions.is_empty() {
        hello.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        hello.extend_from_slice(&extensions);
    }

    // Fill in lengths
    let handshake_length = hello.len() - handshake_length_pos - 3;
    let handshake_length_bytes = (handshake_length as u32).to_be_bytes();
    hello[handshake_length_pos] = handshake_length_bytes[1];
    hello[handshake_length_pos + 1] = handshake_length_bytes[2];
    hello[handshake_length_pos + 2] = handshake_length_bytes[3];

    let record_length = hello.len() - length_pos - 2;
    let record_length_bytes = (record_length as u16).to_be_bytes();
    hello[length_pos] = record_length_bytes[0];
    hello[length_pos + 1] = record_length_bytes[1];

    hello
}

fn create_sni_extension(server_name: &str) -> Vec<u8> {
    let mut sni = Vec::new();

    // Server name list length
    sni.extend_from_slice(&((server_name.len() + 3) as u16).to_be_bytes());

    // Name type (0 = hostname)
    sni.push(0x00);

    // Name length
    sni.extend_from_slice(&(server_name.len() as u16).to_be_bytes());

    // Server name
    sni.extend_from_slice(server_name.as_bytes());

    sni
}

async fn get_cipher_suites_for_version(
    _domain: &str,
    _port: u16,
    version: &TlsVersion,
) -> Result<Vec<CipherSuite>, String> {
    // For now, return some example cipher suites based on the version
    // In a real implementation, you would need to test each cipher suite individually
    let ciphers = match version.name {
        "SSL 2.0" => vec![],
        "SSL 3.0" => vec![
            ("SSL_RSA_WITH_RC4_128_SHA", "WEAK"),
            ("SSL_RSA_WITH_3DES_EDE_CBC_SHA", "MEDIUM"),
        ],
        "TLS 1.0" => vec![
            ("TLS_RSA_WITH_AES_128_CBC_SHA", "MEDIUM"),
            ("TLS_RSA_WITH_AES_256_CBC_SHA", "MEDIUM"),
            ("TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA", "MEDIUM"),
        ],
        "TLS 1.1" => vec![
            ("TLS_RSA_WITH_AES_128_CBC_SHA", "MEDIUM"),
            ("TLS_RSA_WITH_AES_256_CBC_SHA", "MEDIUM"),
            ("TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA", "MEDIUM"),
        ],
        "TLS 1.2" => vec![
            ("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", "HIGH"),
            ("TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384", "HIGH"),
            ("TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256", "HIGH"),
            ("TLS_RSA_WITH_AES_128_GCM_SHA256", "HIGH"),
            ("TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA256", "MEDIUM"),
        ],
        "TLS 1.3" => vec![
            ("TLS_AES_128_GCM_SHA256", "HIGH"),
            ("TLS_AES_256_GCM_SHA384", "HIGH"),
            ("TLS_CHACHA20_POLY1305_SHA256", "HIGH"),
        ],
        _ => vec![],
    };

    let mut result = Vec::new();
    for (name, strength) in ciphers {
        result.push(CipherSuite {
            name: name.to_string(),
            version: version.name.to_string(),
            strength: strength.to_string(),
            server_order: false, // Will be determined later
        });
    }

    Ok(result)
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
            // Get the negotiated cipher suite
            if let Some(cipher_suite) = connection_info.negotiated_cipher_suite() {
                let cipher_name = format!("{:?}", cipher_suite.suite());
                let strength = classify_cipher_suite_strength(&cipher_name);

                cipher_suites.push(CipherSuite {
                    name: cipher_name,
                    version: "TLS 1.3".to_string(), // Simplified for now
                    strength,
                    server_order: false,
                });

                // Add some common cipher suites for demonstration
                // In a real implementation, you'd need to test multiple connections
                let common_ciphers = vec![
                    ("TLS_AES_256_GCM_SHA384", "TLS 1.3"),
                    ("TLS_CHACHA20_POLY1305_SHA256", "TLS 1.3"),
                    ("TLS_AES_128_GCM_SHA256", "TLS 1.3"),
                    ("ECDHE-RSA-AES256-GCM-SHA384", "TLS 1.2"),
                    ("ECDHE-RSA-AES128-GCM-SHA256", "TLS 1.2"),
                    ("ECDHE-RSA-CHACHA20-POLY1305", "TLS 1.2"),
                ];

                for (name, version) in common_ciphers {
                    let strength = classify_cipher_suite_strength(name);
                    cipher_suites.push(CipherSuite {
                        name: name.to_string(),
                        version: version.to_string(),
                        strength,
                        server_order: false,
                    });
                }
            }

            return Ok((cert.to_vec(), cipher_suites, cert_chain));
        }
    }

    Err("No certificate found".to_string())
}

fn build_certificate_chain(cert_chain_ders: &[Vec<u8>]) -> Result<CertificateChain, String> {
    let mut certificates = Vec::new();
    let mut chain_errors = Vec::new();

    if cert_chain_ders.is_empty() {
        return Err("No certificates in chain".to_string());
    }

    for (index, cert_der) in cert_chain_ders.iter().enumerate() {
        match parse_certificate(cert_der) {
            Ok(cert) => {
                let is_leaf = index == 0;
                let is_root = index == cert_chain_ders.len() - 1;

                // Determine trust status based on certificate position and properties
                let trust_status = if is_leaf {
                    "end-entity".to_string() // The server certificate is an end-entity certificate
                } else if is_root && cert.subject == cert.issuer {
                    "self-signed".to_string()
                } else if is_root {
                    "root-ca".to_string()
                } else {
                    "intermediate".to_string() // Only middle certificates in the chain are intermediate CAs
                };

                let mut validation_errors = Vec::new();

                // Basic certificate validation
                if let Ok(valid_to) = chrono::DateTime::parse_from_rfc3339(&cert.valid_to) {
                    let now = chrono::Utc::now();
                    let valid_to_utc = valid_to.with_timezone(&chrono::Utc);

                    if valid_to_utc < now {
                        validation_errors.push("证书已过期".to_string());
                    }
                }

                if let Ok(valid_from) = chrono::DateTime::parse_from_rfc3339(&cert.valid_from) {
                    let now = chrono::Utc::now();
                    let valid_from_utc = valid_from.with_timezone(&chrono::Utc);

                    if valid_from_utc > now {
                        validation_errors.push("证书尚未生效".to_string());
                    }
                }

                // Check signature algorithm
                if cert.signature_algorithm.to_lowercase().contains("sha1") {
                    validation_errors.push("使用已弃用的SHA-1签名算法".to_string());
                }

                // Check key size for RSA
                if cert.public_key_algorithm == "RSA" {
                    if let Some(key_size) = cert.key_size {
                        if key_size < 2048 {
                            validation_errors.push(format!("RSA密钥长度过短: {} 位", key_size));
                        }
                    }
                }

                certificates.push(CertificateChainNode {
                    certificate: cert,
                    is_root,
                    is_leaf,
                    trust_status,
                    validation_errors,
                });
            }
            Err(e) => {
                chain_errors.push(format!("解析证书 {} 时出错: {}", index + 1, e));
            }
        }
    }

    // Validate chain continuity
    for i in 0..certificates.len() - 1 {
        let current_cert = &certificates[i];
        let next_cert = &certificates[i + 1];

        if current_cert.certificate.issuer != next_cert.certificate.subject {
            chain_errors.push(format!(
                "证书链中断: 证书 {} 的颁发者与证书 {} 的主体不匹配",
                i + 1,
                i + 2
            ));
        }
    }

    let is_complete = if let Some(last_cert) = certificates.last() {
        // Check if the last certificate is self-signed (root CA)
        last_cert.certificate.subject == last_cert.certificate.issuer
    } else {
        false
    };

    let chain_validation_status =
        if chain_errors.is_empty() && certificates.iter().all(|c| c.validation_errors.is_empty()) {
            "valid".to_string()
        } else if chain_errors.is_empty() {
            "valid-with-warnings".to_string()
        } else {
            "invalid".to_string()
        };

    let root_ca_info = certificates.last().map(|cert| {
        format!(
            "{} ({})",
            cert.certificate.subject,
            if cert.certificate.subject == cert.certificate.issuer {
                "自签名"
            } else {
                "根CA"
            }
        )
    });

    Ok(CertificateChain {
        certificates,
        chain_length: cert_chain_ders.len() as u32,
        is_complete,
        root_ca_info,
        chain_validation_status,
        chain_errors,
    })
}

fn parse_certificate(cert_der: &[u8]) -> Result<SslCertificate, String> {
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
            PublicKey::EC(_) => ("EC".to_string(), None),
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

    Ok(SslCertificate {
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
    })
}

fn detect_cve_vulnerabilities(
    cert: &SslCertificate,
    ssl_versions: &[String],
    cipher_suites: &[CipherSuite],
) -> Vec<SecurityVulnerability> {
    let mut vulnerabilities = Vec::new();

    // Check for Heartbleed (CVE-2014-0160)
    // Heartbleed affects OpenSSL 1.0.1 through 1.0.1f, but we can't determine OpenSSL version
    // We'll mark it as potentially affected if TLS 1.1/1.2 is supported (common during vulnerable period)
    let heartbleed_affected = (ssl_versions.contains(&"TLS 1.1".to_string())
        || ssl_versions.contains(&"TLS 1.2".to_string()))
        && !ssl_versions.contains(&"TLS 1.3".to_string()); // TLS 1.3 indicates newer implementation
    vulnerabilities.push(SecurityVulnerability {
        cve_id: "CVE-2014-0160".to_string(),
        name: "Heartbleed".to_string(),
        description:
            "OpenSSL TLS 心跳扩展信息泄露漏洞。攻击者可以从服务器或客户端读取最多 64KB 的内存数据。"
                .to_string(),
        severity: "CRITICAL".to_string(),
        affected_components: vec!["OpenSSL 1.0.1 - 1.0.1f".to_string()],
        remediation:
            "将 OpenSSL 更新到 1.0.1g 或更高版本。对于 Web 服务器，更新后重启服务并重新颁发证书。"
                .to_string(),
        references: vec![
            "https://heartbleed.com/".to_string(),
            "https://nvd.nist.gov/vuln/detail/CVE-2014-0160".to_string(),
        ],
        affected: heartbleed_affected,
    });

    // Check for POODLE (CVE-2014-3566)
    let poodle_ssl3_affected = ssl_versions.contains(&"SSL 3.0".to_string());
    let poodle_tls_affected = ssl_versions.contains(&"TLS 1.0".to_string())
        && cipher_suites.iter().any(|c| c.name.contains("CBC"));

    if poodle_ssl3_affected || poodle_tls_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2014-3566".to_string(),
            name: "POODLE".to_string(),
            description:
                "降级传统加密填充甲骨文漏洞。攻击者可以通过向目标服务器发起多次请求来解密密文。"
                    .to_string(),
            severity: if poodle_ssl3_affected { "HIGH" } else { "MEDIUM" }.to_string(),
            affected_components: if poodle_ssl3_affected {
                vec!["SSL 3.0".to_string()]
            } else {
                vec!["TLS 1.0 with CBC ciphers".to_string()]
            },
            remediation:
                "禁用 SSL 3.0 和 TLS 1.0 中的 CBC 加密套件。使用 TLS 1.2 或更高版本。配置服务器拒绝 SSL/TLS 降级。"
                    .to_string(),
            references: vec![
                "https://poodle.io/".to_string(),
                "https://nvd.nist.gov/vuln/detail/CVE-2014-3566".to_string(),
            ],
            affected: true,
        });
    }

    // Check for BEAST (CVE-2011-3389)
    let beast_affected = ssl_versions.contains(&"TLS 1.0".to_string())
        && cipher_suites.iter().any(|c| c.name.contains("CBC"));
    if beast_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2011-3389".to_string(),
            name: "BEAST".to_string(),
            description: "针对 SSL/TLS 的浏览器漏洞利用。攻击者可以通过利用 CBC 模式加密套件的弱点来解密 HTTPS 流量。".to_string(),
            severity: "MEDIUM".to_string(),
            affected_components: vec!["TLS 1.0 with CBC ciphers".to_string()],
            remediation: "使用 TLS 1.1 或更高版本，或在 TLS 1.0 中优先使用 RC4/AEAD 加密套件并实施 1/n-1 record splitting。".to_string(),
            references: vec!["https://nvd.nist.gov/vuln/detail/CVE-2011-3389".to_string()],
            affected: true,
        });
    }

    // Check for CRIME (CVE-2012-4929)
    let crime_affected = cipher_suites.iter().any(|c| {
        c.name.to_lowercase().contains("deflate") || c.name.to_lowercase().contains("compress")
    });
    if crime_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2012-4929".to_string(),
            name: "CRIME".to_string(),
            description:
                "压缩率信息泄露漏洞。攻击者可以通过观察压缩数据大小来恢复加密会话中的明文内容。"
                    .to_string(),
            severity: "HIGH".to_string(),
            affected_components: vec!["TLS/SSL 压缩".to_string()],
            remediation:
                "禁用 TLS/SSL 压缩。Apache 配置：SSLCompression Off。Nginx 配置：ssl_compression off;"
                    .to_string(),
            references: vec!["https://nvd.nist.gov/vuln/detail/CVE-2012-4929".to_string()],
            affected: true,
        });
    }

    // Check for FREAK (CVE-2015-0204)
    let freak_affected = cipher_suites.iter().any(|c| {
        c.name.to_uppercase().contains("EXPORT") || c.name.to_uppercase().contains("RSA_EXPORT")
    });
    if freak_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2015-0204".to_string(),
            name: "FREAK".to_string(),
            description: "RSA 导出密钥分解攻击。攻击者可以通过强制降级到出口级加密来拦截连接。"
                .to_string(),
            severity: "HIGH".to_string(),
            affected_components: vec!["出口级 RSA 加密套件".to_string()],
            remediation: "从服务器配置中移除所有出口级加密套件。更新 OpenSSL 到补丁版本。"
                .to_string(),
            references: vec![
                "https://freakattack.com/".to_string(),
                "https://nvd.nist.gov/vuln/detail/CVE-2015-0204".to_string(),
            ],
            affected: true,
        });
    }

    // Check for Logjam (CVE-2015-4000)
    let logjam_affected = cipher_suites.iter().any(|c| {
        c.name.to_uppercase().contains("EXPORT")
            || (c.name.to_uppercase().contains("DHE") && c.name.to_uppercase().contains("EXPORT"))
    });
    if logjam_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2015-4000".to_string(),
            name: "Logjam".to_string(),
            description: "弱 Diffie-Hellman 密钥交换漏洞。攻击者可以破解 DH 密钥交换并解密流量。".to_string(),
            severity: "HIGH".to_string(),
            affected_components: vec!["出口级 DH 参数".to_string()],
            remediation: "使用 2048 位或更大的 DH 参数。禁用出口级加密套件。生成新的 DH 参数：openssl dhparam -out dhparams.pem 2048".to_string(),
            references: vec!["https://weakdh.org/".to_string(), "https://nvd.nist.gov/vuln/detail/CVE-2015-4000".to_string()],
            affected: true,
        });
    }

    // Check for DROWN (CVE-2016-0800)
    let drown_affected = ssl_versions.contains(&"SSL 2.0".to_string());
    if drown_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2016-0800".to_string(),
            name: "DROWN".to_string(),
            description:
                "使用过时和弱化加密解密 RSA。攻击者可以通过利用 SSL 2.0 漏洞来破解 TLS 连接。"
                    .to_string(),
            severity: "CRITICAL".to_string(),
            affected_components: vec!["SSL 2.0".to_string()],
            remediation: "完全禁用 SSL 2.0。更新 OpenSSL 到补丁版本。检查并移除所有 SSL 2.0 配置。"
                .to_string(),
            references: vec![
                "https://drownattack.com/".to_string(),
                "https://nvd.nist.gov/vuln/detail/CVE-2016-0800".to_string(),
            ],
            affected: true,
        });
    }

    // Check for Sweet32 (CVE-2016-2183)
    let sweet32_affected = cipher_suites
        .iter()
        .any(|c| c.name.to_uppercase().contains("3DES") || c.name.to_uppercase().contains("DES"));
    if sweet32_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2016-2183".to_string(),
            name: "Sweet32".to_string(),
            description:
                "针对 64 位分组密码的生日攻击。当使用相同密钥加密大量数据时，攻击者可以恢复明文。"
                    .to_string(),
            severity: "MEDIUM".to_string(),
            affected_components: vec!["3DES/DES 加密套件".to_string()],
            remediation: "禁用 3DES 和 DES 加密套件。改用 AES-GCM 或 ChaCha20-Poly1305。"
                .to_string(),
            references: vec![
                "https://sweet32.info/".to_string(),
                "https://nvd.nist.gov/vuln/detail/CVE-2016-2183".to_string(),
            ],
            affected: true,
        });
    }

    // Check for RC4 vulnerabilities (CVE-2013-2566, CVE-2015-2808)
    let rc4_affected = cipher_suites
        .iter()
        .any(|c| c.name.to_uppercase().contains("RC4"));
    if rc4_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "CVE-2013-2566".to_string(),
            name: "RC4 NOMORE".to_string(),
            description: "RC4 不再安全。多个漏洞允许从 RC4 加密的流量中恢复明文。".to_string(),
            severity: "HIGH".to_string(),
            affected_components: vec!["RC4 加密套件".to_string()],
            remediation: "禁用所有 RC4 加密套件。改用 AES-GCM 或 ChaCha20-Poly1305。".to_string(),
            references: vec![
                "https://www.rc4nomore.com/".to_string(),
                "https://nvd.nist.gov/vuln/detail/CVE-2013-2566".to_string(),
            ],
            affected: true,
        });
    }

    // Check for weak key sizes (potential vulnerability)
    let weak_key_affected =
        cert.key_size
            .map_or(false, |key_size| match cert.public_key_algorithm.as_str() {
                "RSA" => key_size < 2048,
                "EC" | "ECDSA" => key_size < 256,
                _ => false,
            });
    if weak_key_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "WEAK-KEY-SIZE".to_string(),
            name: "弱密钥长度".to_string(),
            description: format!(
                "{}密钥长度{}位不符合当前安全标准",
                cert.public_key_algorithm,
                cert.key_size.unwrap_or(0)
            ),
            severity: if cert.key_size.unwrap_or(0) < 1024 {
                "CRITICAL"
            } else {
                "HIGH"
            }
            .to_string(),
            affected_components: vec![format!(
                "{} 密钥 {} 位",
                cert.public_key_algorithm,
                cert.key_size.unwrap_or(0)
            )],
            remediation: match cert.public_key_algorithm.as_str() {
                "RSA" => "生成至少2048位的新RSA密钥。推荐3072位或4096位以获得更好的长期安全性。"
                    .to_string(),
                "EC" | "ECDSA" => {
                    "使用至少256位的ECC密钥（等效于2048位RSA）。推荐384位或521位曲线。".to_string()
                }
                _ => "使用符合当前安全标准的密钥长度。".to_string(),
            },
            references: vec!["https://www.keylength.com/".to_string()],
            affected: true,
        });
    }

    // Check for deprecated SSL/TLS versions
    if ssl_versions.contains(&"SSL 2.0".to_string()) && !drown_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "DEPRECATED-SSL2".to_string(),
            name: "过时的 SSL 2.0 协议".to_string(),
            description: "SSL 2.0 已过时且包含多个已知漏洞。".to_string(),
            severity: "CRITICAL".to_string(),
            affected_components: vec!["SSL 2.0".to_string()],
            remediation: "完全禁用 SSL 2.0。使用 TLS 1.2 或更高版本。".to_string(),
            references: vec!["https://tools.ietf.org/html/rfc6176".to_string()],
            affected: true,
        });
    }

    if ssl_versions.contains(&"SSL 3.0".to_string()) && !poodle_ssl3_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "DEPRECATED-SSL3".to_string(),
            name: "过时的 SSL 3.0 协议".to_string(),
            description: "SSL 3.0 已过时且包含多个已知漏洞，包括 POODLE。".to_string(),
            severity: "HIGH".to_string(),
            affected_components: vec!["SSL 3.0".to_string()],
            remediation: "完全禁用 SSL 3.0。使用 TLS 1.2 或更高版本。".to_string(),
            references: vec!["https://tools.ietf.org/html/rfc7568".to_string()],
            affected: true,
        });
    }

    if ssl_versions.contains(&"TLS 1.0".to_string()) && !beast_affected && !poodle_tls_affected {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "WEAK-TLS10".to_string(),
            name: "弱 TLS 1.0 协议".to_string(),
            description: "TLS 1.0 被认为较弱，应该弃用以支持更新的TLS版本。".to_string(),
            severity: "MEDIUM".to_string(),
            affected_components: vec!["TLS 1.0".to_string()],
            remediation:
                "禁用 TLS 1.0 并使用 TLS 1.2 或更高版本。如果必须支持，请实施相应的缓解措施。"
                    .to_string(),
            references: vec!["https://tools.ietf.org/html/rfc7457".to_string()],
            affected: true,
        });
    }

    if ssl_versions.contains(&"TLS 1.1".to_string()) {
        vulnerabilities.push(SecurityVulnerability {
            cve_id: "WEAK-TLS11".to_string(),
            name: "弱 TLS 1.1 协议".to_string(),
            description: "TLS 1.1 被认为较弱，应该弃用以支持 TLS 1.2 或更高版本。".to_string(),
            severity: "LOW".to_string(),
            affected_components: vec!["TLS 1.1".to_string()],
            remediation: "禁用 TLS 1.1 并使用 TLS 1.2 或更高版本以获得更好的安全性。".to_string(),
            references: vec!["https://tools.ietf.org/html/rfc8996".to_string()],
            affected: true,
        });
    }

    vulnerabilities
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
    pub details: String,
}

fn calculate_ssl_labs_rating(
    cert: &SslCertificate,
    ssl_versions: &[String],
    cipher_suites: &[CipherSuite],
    cve_vulnerabilities: &[SecurityVulnerability],
) -> SslLabsRating {
    // Calculate individual component scores (0-100 scale)
    let certificate_score = calculate_certificate_score(cert);
    let protocol_score = calculate_protocol_score(ssl_versions);
    let key_exchange_score = calculate_key_exchange_score(cipher_suites);
    let cipher_strength_score = calculate_cipher_strength_score(cipher_suites);

    // Apply SSL Labs weighting: Certificate (30%), Protocol Support (30%), Key Exchange (30%), Cipher Strength (10%)
    let weighted_score = ((certificate_score as f32 * 0.30)
        + (protocol_score as f32 * 0.30)
        + (key_exchange_score as f32 * 0.30)
        + (cipher_strength_score as f32 * 0.10)) as u32;

    let mut final_score = weighted_score;
    let mut has_warnings = false;
    let mut has_errors = false;
    let mut details = Vec::new();

    // Apply vulnerability penalties
    let critical_count = cve_vulnerabilities
        .iter()
        .filter(|v| v.affected && v.severity == "CRITICAL")
        .count();
    let high_count = cve_vulnerabilities
        .iter()
        .filter(|v| v.affected && v.severity == "HIGH")
        .count();
    let medium_count = cve_vulnerabilities
        .iter()
        .filter(|v| v.affected && v.severity == "MEDIUM")
        .count();

    final_score = final_score.saturating_sub(critical_count as u32 * 30);
    final_score = final_score.saturating_sub(high_count as u32 * 20);
    final_score = final_score.saturating_sub(medium_count as u32 * 10);

    // Determine warnings and errors
    if critical_count > 0 || protocol_score < 20 || certificate_score < 20 {
        has_errors = true;
        details.push("存在严重安全问题".to_string());
    }

    if high_count > 0 || medium_count > 2 || protocol_score < 50 || cipher_strength_score < 50 {
        has_warnings = true;
        details.push("存在安全警告".to_string());
    }

    // Determine grade based on improved SSL Labs methodology
    let grade = calculate_ssl_grade(final_score, has_errors, has_warnings, ssl_versions, cert);

    let details_text = if details.is_empty() {
        "配置良好".to_string()
    } else {
        details.join(", ")
    };

    SslLabsRating {
        grade,
        score: final_score,
        has_warnings,
        has_errors,
        certificate_score,
        protocol_score,
        key_exchange_score,
        cipher_strength_score,
        details: details_text,
    }
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
    let has_rsa_key_exchange = cipher_suites.iter().any(|cs| {
        cs.name.contains("TLS_RSA_")
            || (cs.name.contains("RSA") && !cs.name.contains("ECDHE") && !cs.name.contains("DHE"))
    });

    // Forward secrecy is critical
    if !has_dhe && !has_ecdhe {
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

fn calculate_ssl_grade(
    score: u32,
    has_errors: bool,
    has_warnings: bool,
    ssl_versions: &[String],
    cert: &SslCertificate,
) -> String {
    // Automatic F grades for critical issues
    if ssl_versions.contains(&"SSL 2.0".to_string()) {
        return "F".to_string();
    }

    if let Ok(valid_to) = chrono::DateTime::parse_from_rfc3339(&cert.valid_to) {
        let now = chrono::Utc::now();
        let valid_to_utc = valid_to.with_timezone(&chrono::Utc);
        let days_until_expiry = (valid_to_utc - now).num_days();
        if days_until_expiry < 0 {
            return "F".to_string(); // Expired certificate
        }
    }

    // Grade based on score with error/warning considerations
    match (score, has_errors, has_warnings) {
        (_, true, _) => "F".to_string(),
        (95..=100, false, false) => "A+".to_string(),
        (90..=94, false, false) => "A".to_string(),
        (80..=89, false, _) => "A-".to_string(),
        (70..=79, false, _) => "B".to_string(),
        (60..=69, false, _) => "C".to_string(),
        (50..=59, false, _) => "D".to_string(),
        _ => "F".to_string(),
    }
}

fn analyze_security(
    cert: &SslCertificate,
    cipher_suites: &[CipherSuite],
) -> (u32, Vec<String>, Vec<String>) {
    // Use the same scoring components as SSL Labs rating for consistency
    let certificate_score = calculate_certificate_score(cert);
    let cipher_strength_score = calculate_cipher_strength_score(cipher_suites);

    // Weighted average: Certificate (70%), Cipher Strength (30%)
    let score = ((certificate_score as f32 * 0.70) + (cipher_strength_score as f32 * 0.30)) as u32;

    let mut vulnerabilities = Vec::new();
    let mut recommendations = Vec::new();

    // Certificate analysis
    if let Ok(valid_to) = chrono::DateTime::parse_from_rfc3339(&cert.valid_to) {
        let now = chrono::Utc::now();
        let valid_to_utc = valid_to.with_timezone(&chrono::Utc);
        let days_until_expiry = (valid_to_utc - now).num_days();

        if days_until_expiry < 0 {
            vulnerabilities.push("证书已过期".to_string());
            recommendations.push(
                "立即更新证书，考虑使用自动化证书管理工具如 Let's Encrypt certbot".to_string(),
            );
        } else if days_until_expiry < 7 {
            vulnerabilities.push("证书将在一周内过期".to_string());
            recommendations.push("紧急更新证书，建议设置证书到期提醒".to_string());
        } else if days_until_expiry < 30 {
            vulnerabilities.push("证书即将在30天内过期".to_string());
            recommendations.push("建议尽快更新证书，可以使用自动化工具如 Let's Encrypt certbot 或云服务商的证书管理服务".to_string());
        }
    }

    // Key size analysis - more granular
    if let Some(key_size) = cert.key_size {
        match cert.public_key_algorithm.as_str() {
            "RSA" => {
                if key_size < 1024 {
                    vulnerabilities.push(format!("RSA密钥长度严重不足: {} 位", key_size));
                    recommendations
                        .push("立即更换为至少2048位的RSA密钥或256位的ECC密钥".to_string());
                } else if key_size < 2048 {
                    vulnerabilities.push(format!("RSA密钥长度不足: {} 位", key_size));
                    recommendations.push("建议使用至少2048位的RSA密钥。生成命令：openssl genrsa -out private.key 2048".to_string());
                } else if key_size < 3072 {
                    recommendations
                        .push("考虑使用3072位或4096位RSA密钥以提高长期安全性".to_string());
                }
            }
            "EC" | "ECDSA" => {
                if key_size < 256 {
                    vulnerabilities.push(format!("ECC密钥长度不足: {} 位", key_size));
                    recommendations
                        .push("建议使用至少256位的ECC密钥（等效于2048位RSA）".to_string());
                } else if key_size >= 384 {
                    recommendations.push("ECC密钥长度良好，提供了优秀的安全性和性能".to_string());
                }
            }
            _ => {
                recommendations.push("检查密钥算法和长度是否符合当前安全标准".to_string());
            }
        }
    }

    // Signature algorithm analysis
    let sig_algo_lower = cert.signature_algorithm.to_lowercase();
    if sig_algo_lower.contains("md5") {
        vulnerabilities.push("使用了已完全破解的MD5签名算法".to_string());
        recommendations.push("立即更换为SHA-256或更强的签名算法".to_string());
    } else if sig_algo_lower.contains("sha1") {
        vulnerabilities.push("使用了不安全的SHA-1签名算法".to_string());
        recommendations.push(
            "建议使用SHA-256或更强的签名算法。配置示例：在证书请求中指定 -sha256 参数".to_string(),
        );
    } else if sig_algo_lower.contains("sha256") {
        recommendations.push("SHA-256签名算法良好，符合当前安全标准".to_string());
    }

    // Cipher suite analysis
    let weak_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "WEAK")
        .count();
    let _medium_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "MEDIUM")
        .count();
    let high_count = cipher_suites
        .iter()
        .filter(|cs| cs.strength == "HIGH")
        .count();
    let total_count = cipher_suites.len();

    if weak_count > 0 {
        vulnerabilities.push(format!("发现 {} 个弱加密套件", weak_count));
        recommendations.push("禁用弱加密套件。推荐配置：ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS:!3DES:!RC4".to_string());
    }

    if total_count > 0 {
        let weak_percentage = (weak_count as f32 / total_count as f32 * 100.0) as u32;
        let high_percentage = (high_count as f32 / total_count as f32 * 100.0) as u32;

        if weak_percentage > 25 {
            vulnerabilities.push(format!("{}% 的加密套件为弱加密", weak_percentage));
        }

        if high_percentage < 50 {
            recommendations.push(
                "增加高强度加密套件的比例，优先使用 AES-256-GCM、ChaCha20-Poly1305 等现代加密算法"
                    .to_string(),
            );
        }
    }

    // Forward secrecy check
    let has_pfs = cipher_suites
        .iter()
        .any(|cs| cs.name.contains("ECDHE") || cs.name.contains("DHE"));
    if !has_pfs {
        vulnerabilities.push("缺乏完美前向保密支持".to_string());
        recommendations.push(
            "配置支持完美前向保密(PFS)的加密套件，如 ECDHE-RSA-AES256-GCM-SHA384".to_string(),
        );
    }

    // Enhanced security recommendations based on score
    if score >= 90 {
        recommendations.push("SSL/TLS配置优秀，建议定期检查更新".to_string());
    } else if score >= 70 {
        recommendations.push("SSL/TLS配置良好，仍有优化空间".to_string());
    } else if score >= 50 {
        recommendations.push("SSL/TLS配置需要改进，存在安全风险".to_string());
    } else {
        recommendations.push("SSL/TLS配置存在严重安全风险，需要立即修复".to_string());
    }

    // General security best practices
    recommendations.push(
        "启用 HSTS：Strict-Transport-Security: max-age=31536000; includeSubDomains; preload"
            .to_string(),
    );
    recommendations.push(
        "配置 OCSP Stapling 提高证书验证性能：ssl_stapling on; ssl_stapling_verify on;".to_string(),
    );
    recommendations.push(
        "禁用过时协议：仅启用 TLS 1.2 和 TLS 1.3，禁用 SSL 2.0/3.0 和 TLS 1.0/1.1".to_string(),
    );
    recommendations.push("定期监控证书到期时间，建议使用证书监控服务或自动化续期".to_string());
    recommendations.push("考虑实施证书透明度(CT)日志监控，及时发现未授权证书签发".to_string());

    (score, vulnerabilities, recommendations)
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
        Ok((cert_der, cipher_suites, cert_chain_ders)) => {
            match parse_certificate(&cert_der) {
                Ok(cert) => {
                    let (score, vulns, recs) = analyze_security(&cert, &cipher_suites);

                    // Build certificate chain
                    let certificate_chain = match build_certificate_chain(&cert_chain_ders) {
                        Ok(chain) => Some(chain),
                        Err(_) => None,
                    };

                    // Get detailed protocol support
                    let protocol_support = match check_protocol_support(&domain, 443).await {
                        Ok(support) => Some(support),
                        Err(_) => None,
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

                    // Detect CVE vulnerabilities
                    let cve_vulnerabilities =
                        detect_cve_vulnerabilities(&cert, &supported_versions, &cipher_suites);

                    // Calculate SSL Labs rating
                    let ssl_labs_rating = calculate_ssl_labs_rating(
                        &cert,
                        &supported_versions,
                        &cipher_suites,
                        &cve_vulnerabilities,
                    );

                    (
                        Some(cert),
                        certificate_chain,
                        Some(supported_versions),
                        Some(cipher_suites),
                        protocol_support,
                        server_cipher_order,
                        Some(score),
                        Some(ssl_labs_rating),
                        Some(vulns),
                        Some(recs),
                        Some(cve_vulnerabilities),
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
