use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use openssl::pkey::{HasPublic, Id, PKey};
use openssl::x509::X509Req;
use serde::Serialize;
use x509_parser::pem::Pem;
use x509_parser::prelude::*;

use crate::utils::crypto::CryptoUtils;

/// 主题（Subject）中的单个属性
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectEntry {
    /// OID 点分字符串，如 2.5.4.3
    pub oid: String,
    /// 属性友好名称，如 通用名称 (CN)
    pub name: String,
    /// 属性值
    pub value: String,
}

/// CSR 中请求的公钥详情
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyDetails {
    /// 密钥类型：RSA / ECC / Ed25519 / DSA 等
    pub key_type: String,
    /// 密钥长度（位）
    pub key_size: Option<u32>,
    /// ECC 曲线名称（仅 ECC 密钥）
    pub curve_name: Option<String>,
    /// 公钥的 SPKI PEM 文本（-----BEGIN PUBLIC KEY-----）
    pub spki_pem: String,
    /// 公钥 DER 的 SHA-256 指纹（大写 hex），可用于比对私钥与 CSR 是否匹配
    pub fingerprint_sha256: String,
}

/// 可选输入的私钥校验结果（支持 PKCS#8 / PKCS#1 / SEC1 格式）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateKeyCheck {
    /// 私钥封装格式：PKCS#8 / PKCS#1 / SEC1 / DER
    pub format: String,
    /// 密钥类型：RSA / ECC / Ed25519 等
    pub key_type: String,
    /// 密钥长度（位）
    pub key_size: Option<u32>,
    /// ECC 曲线名称（仅 ECC 密钥）
    pub curve_name: Option<String>,
    /// 私钥对应公钥的 SHA-256 指纹（大写 hex），与 CSR 公钥指纹一致即匹配
    pub fingerprint_sha256: String,
    /// 私钥与 CSR 中的公钥是否为同一把密钥
    pub matches_csr: bool,
}

/// CSR 中请求的单个扩展
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsrExtensionInfo {
    /// 扩展友好名称，如 Subject Alternative Name
    pub name: String,
    /// 扩展值（人类可读格式）
    pub value: String,
    /// 是否标记为 critical
    pub critical: bool,
}

/// CSR 解析结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsrInfo {
    /// PKCS#10 版本号（通常为 0，即 v1）
    pub version: u32,
    /// 主题（Subject）属性列表，按 CSR 中出现顺序排列
    pub subject: Vec<SubjectEntry>,
    /// 请求的公钥信息
    pub public_key: PublicKeyDetails,
    /// 可选：随 CSR 一起提供的私钥校验结果（用于确认私钥与 CSR 是否匹配）
    pub private_key: Option<PrivateKeyCheck>,
    /// CSR 的签名算法
    pub signature_algorithm: String,
    /// 请求的扩展列表（SAN、Key Usage 等）
    pub requested_extensions: Vec<CsrExtensionInfo>,
    /// CSR 自签名是否验证通过（用请求中的公钥验证 CSR 签名）
    pub signature_valid: bool,
    /// DER 编码字节数
    pub der_size: usize,
    /// 规范化后的 PEM 文本
    pub pem: String,
}

/// 解析证书签名请求（CSR / PKCS#10）
///
/// 支持两种输入（二选一）：
/// - `csr_content`: PEM 文本（`BEGIN CERTIFICATE REQUEST` 或 IIS 的 `BEGIN NEW CERTIFICATE REQUEST`），
///   或不带标记的 Base64 编码 DER
/// - `csr_der`: DER 编码的原始字节（如上传的二进制 .csr 文件）
///
/// `private_key` 为可选的配套私钥（PKCS#8 / PKCS#1 / SEC1 PEM，或 Base64 编码 DER），
/// 提供时将校验其与 CSR 公钥是否匹配
#[tauri::command]
pub fn parse_csr(
    csr_content: Option<String>,
    csr_der: Option<Vec<u8>>,
    private_key: Option<String>,
) -> Result<CsrInfo, String> {
    let der = match (csr_content, csr_der) {
        (Some(content), _) if !content.trim().is_empty() => extract_csr_der_from_text(&content)?,
        (_, Some(data)) if !data.is_empty() => data,
        _ => return Err("请提供 CSR 内容：粘贴 PEM 文本或上传 CSR 文件".to_string()),
    };

    parse_csr_der(&der, private_key.as_deref())
}

/// 从文本输入中提取 CSR 的 DER 字节：优先按 PEM 解析，否则按 Base64 解码
fn extract_csr_der_from_text(content: &str) -> Result<Vec<u8>, String> {
    let cleaned = content.replace("\r\n", "\n").replace('\r', "\n").trim().to_string();

    if cleaned.contains("-----BEGIN") {
        for pem_result in Pem::iter_from_buffer(cleaned.as_bytes()) {
            let pem = pem_result.map_err(|e| format!("PEM 解析失败: {}", e))?;
            // IIS 导出的 CSR 使用 NEW CERTIFICATE REQUEST 标记
            if pem.label == "CERTIFICATE REQUEST" || pem.label == "NEW CERTIFICATE REQUEST" {
                return Ok(pem.contents);
            }
        }
        return Err(
            "未找到有效的 CSR：PEM 标记应为 CERTIFICATE REQUEST（或 IIS 的 NEW CERTIFICATE REQUEST）"
                .to_string(),
        );
    }

    // 无 PEM 标记，尝试按 Base64 解码为 DER
    let compact: String = cleaned.chars().filter(|c| !c.is_whitespace()).collect();
    BASE64_STANDARD
        .decode(compact.as_bytes())
        .map_err(|_| "无法识别的 CSR 格式：应为 PEM 文本（含 BEGIN/END 标记）或 Base64 编码的 DER 数据".to_string())
}

/// 解析 DER 编码的 CSR
fn parse_csr_der(der: &[u8], private_key_text: Option<&str>) -> Result<CsrInfo, String> {
    let (_, csr) = X509CertificationRequest::from_der(der)
        .map_err(|e| format!("CSR 解析失败: {}。请确认文件是有效的 PKCS#10 证书请求", e))?;

    // openssl 第二次解析用于提取公钥 PEM/指纹和签名验证
    let req = X509Req::from_der(der).map_err(|e| format!("CSR 解析失败: {}", e))?;

    let subject = parse_subject(&csr.certification_request_info.subject);
    let signature_algorithm = signature_algorithm_name(&csr.signature_algorithm);
    let requested_extensions = parse_requested_extensions(&csr);

    let pkey = req.public_key().map_err(|e| format!("提取公钥失败: {}", e))?;
    let public_key = parse_public_key(&pkey)?;

    let signature_valid = req.verify(&pkey).unwrap_or(false);

    // 可选：解析配套私钥并校验与 CSR 公钥是否匹配
    let private_key = match private_key_text.map(str::trim).filter(|s| !s.is_empty()) {
        Some(text) => Some(parse_private_key_check(text, &pkey)?),
        None => None,
    };

    Ok(CsrInfo {
        version: csr.certification_request_info.version.0,
        subject,
        public_key,
        private_key,
        signature_algorithm,
        requested_extensions,
        signature_valid,
        der_size: der.len(),
        pem: der_to_pem("CERTIFICATE REQUEST", der),
    })
}

/// 解析主题属性列表（保留顺序，支持同类型多个值）
fn parse_subject(subject: &X509Name) -> Vec<SubjectEntry> {
    let mut entries = Vec::new();

    for rdn in subject.iter() {
        for attr in rdn.iter() {
            let oid = attr.attr_type().to_string();
            let name = subject_attr_friendly_name(&oid);
            let value = attr
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "<无法解码的值>".to_string());
            entries.push(SubjectEntry { oid, name, value });
        }
    }

    entries
}

/// 主题属性 OID 的中文友好名称
fn subject_attr_friendly_name(oid: &str) -> String {
    let name = match oid {
        "2.5.4.3" => "通用名称 (CN)",
        "2.5.4.5" => "序列号 (serialNumber)",
        "2.5.4.6" => "国家 (C)",
        "2.5.4.7" => "城市 (L)",
        "2.5.4.8" => "省份 (ST)",
        "2.5.4.10" => "组织名称 (O)",
        "2.5.4.11" => "组织单位 (OU)",
        "1.2.840.113549.1.9.1" => "邮箱地址",
        "0.9.2342.19200300.100.1.25" => "域名组件 (DC)",
        _ => return format!("其他属性 ({})", oid),
    };
    name.to_string()
}

/// CSR 签名算法的友好名称（OID 映射与证书查看器保持一致）
fn signature_algorithm_name(algorithm: &AlgorithmIdentifier) -> String {
    let name = match algorithm.algorithm.to_string().as_str() {
        "1.2.840.113549.1.1.5" => "SHA1WithRSA",
        "1.2.840.113549.1.1.11" => "SHA256WithRSA",
        "1.2.840.113549.1.1.12" => "SHA384WithRSA",
        "1.2.840.113549.1.1.13" => "SHA512WithRSA",
        "1.2.840.113549.1.1.4" => "MD5WithRSA",
        "1.2.840.10045.4.1" => "ECDSAWithSHA1",
        "1.2.840.10045.4.3.2" => "ECDSAWithSHA256",
        "1.2.840.10045.4.3.3" => "ECDSAWithSHA384",
        "1.2.840.10045.4.3.4" => "ECDSAWithSHA512",
        "1.3.101.112" => "Ed25519",
        "1.2.840.10040.4.3" => "DSAWithSHA1",
        "2.16.840.1.101.3.4.3.2" => "DSAWithSHA256",
        _ => return algorithm.algorithm.to_id_string(),
    };
    name.to_string()
}

/// 解析 CSR 中请求的扩展（extReq / msExtReq 属性）
fn parse_requested_extensions(csr: &X509CertificationRequest) -> Vec<CsrExtensionInfo> {
    let mut extensions = Vec::new();

    for attr in csr.certification_request_info.iter_attributes() {
        if let ParsedCriAttribute::ExtensionRequest(requested) = &attr.parsed_attribute() {
            for ext in requested.extensions.iter() {
                let oid = ext.oid.to_string();
                let name = extension_friendly_name(&oid);
                let value = format_extension_value(&ext.parsed_extension());
                extensions.push(CsrExtensionInfo {
                    name,
                    value,
                    critical: ext.critical,
                });
            }
        }
    }

    extensions
}

/// 扩展 OID 的友好名称
fn extension_friendly_name(oid: &str) -> String {
    let name = match oid {
        "2.5.29.17" => "Subject Alternative Name",
        "2.5.29.15" => "Key Usage",
        "2.5.29.37" => "Extended Key Usage",
        "2.5.29.19" => "Basic Constraints",
        "2.5.29.14" => "Subject Key Identifier",
        "2.5.29.35" => "Authority Key Identifier",
        "2.5.29.31" => "CRL Distribution Points",
        "2.5.29.32" => "Certificate Policies",
        _ => return format!("其他扩展 ({})", oid),
    };
    name.to_string()
}

/// 将扩展解析结果格式化为人类可读文本
fn format_extension_value(parsed: &ParsedExtension) -> String {
    match parsed {
        ParsedExtension::SubjectAlternativeName(san) => {
            let names: Vec<String> = san
                .general_names
                .iter()
                .map(|gn| match gn {
                    GeneralName::DNSName(dns) => format!("DNS:{}", dns),
                    GeneralName::IPAddress(ip) => format!("IP:{}", format_ip_address(ip)),
                    GeneralName::URI(uri) => format!("URI:{}", uri),
                    GeneralName::RFC822Name(email) => format!("email:{}", email),
                    other => format!("{:?}", other),
                })
                .collect();
            if names.is_empty() {
                "<空>".to_string()
            } else {
                names.join(", ")
            }
        }
        ParsedExtension::KeyUsage(ku) => {
            let mut usages = Vec::new();
            if ku.digital_signature() {
                usages.push("digitalSignature");
            }
            if ku.non_repudiation() {
                usages.push("nonRepudiation");
            }
            if ku.key_encipherment() {
                usages.push("keyEncipherment");
            }
            if ku.data_encipherment() {
                usages.push("dataEncipherment");
            }
            if ku.key_agreement() {
                usages.push("keyAgreement");
            }
            if ku.key_cert_sign() {
                usages.push("keyCertSign");
            }
            if ku.crl_sign() {
                usages.push("cRLSign");
            }
            if ku.encipher_only() {
                usages.push("encipherOnly");
            }
            if ku.decipher_only() {
                usages.push("decipherOnly");
            }
            if usages.is_empty() {
                "<空>".to_string()
            } else {
                usages.join(", ")
            }
        }
        ParsedExtension::ExtendedKeyUsage(eku) => {
            let mut usages: Vec<String> = Vec::new();
            if eku.server_auth {
                usages.push("serverAuth (TLS Web Server Authentication)".to_string());
            }
            if eku.client_auth {
                usages.push("clientAuth (TLS Web Client Authentication)".to_string());
            }
            if eku.code_signing {
                usages.push("codeSigning".to_string());
            }
            if eku.email_protection {
                usages.push("emailProtection".to_string());
            }
            if eku.time_stamping {
                usages.push("timeStamping".to_string());
            }
            if eku.ocsp_signing {
                usages.push("OCSPSigning".to_string());
            }
            if eku.any {
                usages.push("anyExtendedKeyUsage".to_string());
            }
            for oid in &eku.other {
                usages.push(oid.to_string());
            }
            if usages.is_empty() {
                "<空>".to_string()
            } else {
                usages.join(", ")
            }
        }
        ParsedExtension::BasicConstraints(bc) => match bc.path_len_constraint {
            Some(path_len) => format!("CA={}, pathlen={}", bc.ca, path_len),
            None => format!("CA={}", bc.ca),
        },
        other => format!("{:?}", other),
    }
}

/// 格式化 IP 地址字节（IPv4 点分十进制 / IPv6 冒号十六进制）
fn format_ip_address(ip: &[u8]) -> String {
    match ip.len() {
        4 => format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]),
        16 => {
            let groups: Vec<String> = (0..16)
                .step_by(2)
                .map(|i| format!("{:x}", ((ip[i] as u16) << 8) | (ip[i + 1] as u16)))
                .collect();
            groups.join(":")
        }
        _ => hex::encode(ip),
    }
}

/// 从 openssl 密钥（公钥或私钥的公钥部分）提取类型 / 长度 / 曲线 / PEM / 指纹
fn parse_public_key<T: HasPublic>(pkey: &PKey<T>) -> Result<PublicKeyDetails, String> {
    let key_type;
    let mut key_size = None;
    let mut curve_name = None;

    match pkey.id() {
        Id::RSA => {
            key_type = "RSA".to_string();
            let rsa = pkey.rsa().map_err(|e| format!("读取 RSA 密钥参数失败: {}", e))?;
            key_size = Some(rsa.n().num_bits() as u32);
        }
        Id::EC => {
            key_type = "ECC".to_string();
            let ec = pkey
                .ec_key()
                .map_err(|e| format!("读取 ECC 密钥参数失败: {}", e))?;
            let group = ec.group();
            curve_name = group
                .curve_name()
                .and_then(|nid| nid.short_name().ok())
                .map(|s| s.to_string());
            key_size = Some(group.degree() * 8);
        }
        Id::ED25519 => {
            key_type = "Ed25519".to_string();
            key_size = Some(256);
        }
        Id::DSA => {
            key_type = "DSA".to_string();
        }
        other => {
            key_type = format!("其他 ({:?})", other);
        }
    }

    let spki_pem = String::from_utf8(
        pkey.public_key_to_pem()
            .map_err(|e| format!("公钥转 PEM 失败: {}", e))?,
    )
    .map_err(|e| format!("公钥 PEM 编码异常: {}", e))?;

    let pubkey_der = pkey
        .public_key_to_der()
        .map_err(|e| format!("公钥转 DER 失败: {}", e))?;
    let fingerprint_sha256 = CryptoUtils::calculate_sha256_fingerprint(&pubkey_der);

    Ok(PublicKeyDetails {
        key_type,
        key_size,
        curve_name,
        spki_pem,
        fingerprint_sha256,
    })
}

/// 解析用户提供的私钥文本，并校验其与 CSR 公钥是否匹配
fn parse_private_key_check(
    text: &str,
    csr_pkey: &PKey<openssl::pkey::Public>,
) -> Result<PrivateKeyCheck, String> {
    let (pkey, format) = parse_private_key_input(text)?;
    let details = parse_public_key(&pkey)?;
    let matches_csr = csr_pkey.public_eq(&pkey);

    Ok(PrivateKeyCheck {
        format: format.to_string(),
        key_type: details.key_type,
        key_size: details.key_size,
        curve_name: details.curve_name,
        fingerprint_sha256: details.fingerprint_sha256,
        matches_csr,
    })
}

/// 解析私钥文本，返回（私钥, 封装格式）
///
/// 支持 PKCS#8（`BEGIN PRIVATE KEY`）、PKCS#1（`BEGIN RSA PRIVATE KEY`）、
/// SEC1（`BEGIN EC PRIVATE KEY`）PEM，以及不带标记的 Base64 编码 DER
fn parse_private_key_input(content: &str) -> Result<(PKey<openssl::pkey::Private>, &'static str), String> {
    let cleaned = content.replace("\r\n", "\n").replace('\r', "\n").trim().to_string();

    if cleaned.contains("-----BEGIN") {
        for pem_result in Pem::iter_from_buffer(cleaned.as_bytes()) {
            let pem = pem_result.map_err(|e| format!("私钥 PEM 解析失败: {}", e))?;
            let format = match pem.label.as_str() {
                "PRIVATE KEY" => "PKCS#8",
                "RSA PRIVATE KEY" => "PKCS#1",
                "EC PRIVATE KEY" => "SEC1",
                "ENCRYPTED PRIVATE KEY" => {
                    return Err(
                        "暂不支持加密的私钥（ENCRYPTED PRIVATE KEY），请提供未加密的 PKCS#8 私钥"
                            .to_string(),
                    )
                }
                _ => continue,
            };
            let pkey = PKey::private_key_from_der(&pem.contents)
                .map_err(|e| format!("私钥解析失败: {}。请确认提供的是有效的私钥", e))?;
            return Ok((pkey, format));
        }
        return Err(
            "未找到有效的私钥：PEM 标记应为 PRIVATE KEY（PKCS#8）、RSA PRIVATE KEY 或 EC PRIVATE KEY"
                .to_string(),
        );
    }

    // 无 PEM 标记，尝试按 Base64 解码为 DER
    let compact: String = cleaned.chars().filter(|c| !c.is_whitespace()).collect();
    let der = BASE64_STANDARD
        .decode(compact.as_bytes())
        .map_err(|_| "无法识别的私钥格式：应为 PEM 文本（含 BEGIN/END 标记）或 Base64 编码的 DER 数据".to_string())?;
    let pkey = PKey::private_key_from_der(&der)
        .map_err(|e| format!("私钥解析失败: {}。请确认提供的是有效的私钥", e))?;
    Ok((pkey, "DER"))
}

/// DER 编码转 PEM 文本（64 字符换行）
fn der_to_pem(label: &str, der: &[u8]) -> String {
    let encoded = BASE64_STANDARD.encode(der);
    let lines: Vec<String> = encoded
        .as_bytes()
        .chunks(64)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ec::{EcGroup, EcKey};
    use openssl::hash::MessageDigest;
    use openssl::nid::Nid;
    use openssl::pkey::Private;
    use openssl::rsa::Rsa;
    use openssl::stack::Stack;
    use openssl::x509::extension::SubjectAlternativeName;
    use openssl::x509::{X509Extension, X509NameBuilder, X509ReqBuilder};

    /// 用 openssl 动态生成一个带 SAN 扩展的测试 CSR（PEM 格式）
    fn generate_test_csr_pem() -> String {
        let rsa = Rsa::generate(2048).unwrap();
        let pkey = PKey::from_rsa(rsa).unwrap();

        let mut builder = X509ReqBuilder::new().unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "test.example.com").unwrap();
        name.append_entry_by_text("O", "Test Org").unwrap();
        let name = name.build();
        builder.set_subject_name(&name).unwrap();
        builder.set_pubkey(&pkey).unwrap();

        let ctx = builder.x509v3_context(None);
        let san = SubjectAlternativeName::new()
            .dns("test.example.com")
            .dns("api.example.com")
            .build(&ctx)
            .unwrap();
        let mut extensions: Stack<X509Extension> = Stack::new().unwrap();
        extensions.push(san).unwrap();
        builder.add_extensions(&extensions).unwrap();

        builder.sign(&pkey, MessageDigest::sha256()).unwrap();
        String::from_utf8(builder.build().to_pem().unwrap()).unwrap()
    }

    fn generate_test_key() -> PKey<Private> {
        PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap()
    }

    /// 用指定私钥生成一个最小可用的 CSR（PEM）
    fn build_csr_for_key(pkey: &PKey<Private>) -> String {
        let mut builder = X509ReqBuilder::new().unwrap();
        builder.set_pubkey(pkey).unwrap();
        builder.sign(pkey, MessageDigest::sha256()).unwrap();
        String::from_utf8(builder.build().to_pem().unwrap()).unwrap()
    }

    #[test]
    fn test_private_key_pkcs8_match() {
        let pkey = generate_test_key();
        let csr_pem = build_csr_for_key(&pkey);
        let pkcs8 = String::from_utf8(pkey.private_key_to_pem_pkcs8().unwrap()).unwrap();

        let info = parse_csr(Some(csr_pem), None, Some(pkcs8)).unwrap();
        let check = info.private_key.as_ref().expect("应包含私钥校验结果");
        assert_eq!(check.format, "PKCS#8");
        assert_eq!(check.key_type, "RSA");
        assert_eq!(check.key_size, Some(2048));
        assert!(check.matches_csr);
        assert_eq!(check.fingerprint_sha256, info.public_key.fingerprint_sha256);

        // 前端 TypeScript 接口依赖 camelCase 键名
        let json = serde_json::to_value(&info).unwrap();
        let pk = json.get("privateKey").unwrap();
        assert_eq!(pk.get("format").unwrap(), "PKCS#8");
        assert_eq!(pk.get("matchesCsr").unwrap(), true);
    }

    #[test]
    fn test_private_key_mismatch() {
        let key_a = generate_test_key();
        let key_b = generate_test_key();
        let csr_pem = build_csr_for_key(&key_a);
        let pkcs8_b = String::from_utf8(key_b.private_key_to_pem_pkcs8().unwrap()).unwrap();

        let info = parse_csr(Some(csr_pem), None, Some(pkcs8_b)).unwrap();
        let check = info.private_key.as_ref().expect("应包含私钥校验结果");
        assert!(!check.matches_csr);
        assert!(!check.fingerprint_sha256.is_empty());
    }

    #[test]
    fn test_private_key_pkcs1_format() {
        let pkey = generate_test_key();
        let csr_pem = build_csr_for_key(&pkey);
        let pkcs1 = String::from_utf8(pkey.rsa().unwrap().private_key_to_pem().unwrap()).unwrap();
        assert!(pkcs1.contains("BEGIN RSA PRIVATE KEY"));

        let info = parse_csr(Some(csr_pem), None, Some(pkcs1)).unwrap();
        let check = info.private_key.as_ref().expect("应包含私钥校验结果");
        assert_eq!(check.format, "PKCS#1");
        assert!(check.matches_csr);
    }

    #[test]
    fn test_private_key_ec_sec1() {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ec_key = EcKey::generate(&group).unwrap();
        // 传统格式导出 ECC 私钥即 SEC1（BEGIN EC PRIVATE KEY）
        let sec1 = String::from_utf8(ec_key.private_key_to_pem().unwrap()).unwrap();
        let pkey = PKey::from_ec_key(ec_key).unwrap();
        let csr_pem = build_csr_for_key(&pkey);
        assert!(sec1.contains("BEGIN EC PRIVATE KEY"));

        let info = parse_csr(Some(csr_pem), None, Some(sec1)).unwrap();
        let check = info.private_key.expect("应包含私钥校验结果");
        assert_eq!(check.format, "SEC1");
        assert_eq!(check.key_type, "ECC");
        assert_eq!(check.curve_name.as_deref(), Some("prime256v1"));
        assert!(check.matches_csr);
    }

    #[test]
    fn test_private_key_invalid_input() {
        let csr_pem = build_csr_for_key(&generate_test_key());
        assert!(parse_csr(Some(csr_pem.clone()), None, Some("not a key".to_string())).is_err());

        // 用 CSR 的 PEM 充当私钥输入时应报错（PEM 标记不匹配）
        assert!(parse_csr(Some(csr_pem), None, Some(generate_test_csr_pem())).is_err());
    }

    #[test]
    fn test_no_private_key_serializes_null() {
        let info = parse_csr(Some(generate_test_csr_pem()), None, None).unwrap();
        assert!(info.private_key.is_none());
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("privateKey").unwrap().is_null());
    }

    #[test]
    fn test_parse_pem_csr() {
        let pem = generate_test_csr_pem();
        let info = parse_csr(Some(pem), None, None).unwrap();

        assert_eq!(info.version, 0);
        assert_eq!(info.public_key.key_type, "RSA");
        assert_eq!(info.public_key.key_size, Some(2048));
        assert!(info.signature_valid);
        assert!(info.subject.iter().any(|e| e.value == "test.example.com"));
        assert!(info.subject.iter().any(|e| e.value == "Test Org"));
        assert_eq!(info.signature_algorithm, "SHA256WithRSA");
        assert!(info.pem.contains("BEGIN CERTIFICATE REQUEST"));
        assert!(info.public_key.spki_pem.contains("BEGIN PUBLIC KEY"));
        assert!(!info.public_key.fingerprint_sha256.is_empty());
    }

    #[test]
    fn test_requested_extensions_contains_san() {
        let pem = generate_test_csr_pem();
        let info = parse_csr(Some(pem), None, None).unwrap();

        let san = info
            .requested_extensions
            .iter()
            .find(|e| e.name == "Subject Alternative Name")
            .expect("CSR 应包含 SAN 扩展");
        assert!(san.value.contains("DNS:test.example.com"));
        assert!(san.value.contains("DNS:api.example.com"));
    }

    #[test]
    fn test_parse_base64_der_csr() {
        let pem = generate_test_csr_pem();
        let der = extract_csr_der_from_text(&pem).unwrap();
        let base64_der = BASE64_STANDARD.encode(&der);

        let info = parse_csr(Some(base64_der), None, None).unwrap();
        assert!(info.signature_valid);
        assert_eq!(info.der_size, der.len());
    }

    #[test]
    fn test_parse_binary_der_csr() {
        let pem = generate_test_csr_pem();
        let der = extract_csr_der_from_text(&pem).unwrap();

        let info = parse_csr(None, Some(der), None).unwrap();
        assert!(info.signature_valid);
    }

    #[test]
    fn test_iis_pem_label() {
        let pem = generate_test_csr_pem();
        // 模拟 IIS 的 NEW CERTIFICATE REQUEST 标记
        let der = extract_csr_der_from_text(&pem).unwrap();
        let iis_pem = der_to_pem("NEW CERTIFICATE REQUEST", &der);

        let info = parse_csr(Some(iis_pem), None, None).unwrap();
        assert!(info.signature_valid);
    }

    #[test]
    fn test_signature_valid_matches_signing_key() {
        // 用另一把私钥生成 CSR 后，签名校验仍应通过；公钥指纹应与私钥公钥一致
        let pkey = generate_test_key();
        let mut builder = X509ReqBuilder::new().unwrap();
        builder.set_pubkey(&pkey).unwrap();
        builder.sign(&pkey, MessageDigest::sha256()).unwrap();

        let pem = String::from_utf8(builder.build().to_pem().unwrap()).unwrap();
        let info = parse_csr(Some(pem), None, None).unwrap();

        let expected_fp = CryptoUtils::calculate_sha256_fingerprint(&pkey.public_key_to_der().unwrap());
        assert!(info.signature_valid);
        assert_eq!(info.public_key.fingerprint_sha256, expected_fp);
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(parse_csr(None, None, None).is_err());
        assert!(parse_csr(Some("  ".to_string()), None, None).is_err());
        assert!(parse_csr(Some("not a csr".to_string()), None, None).is_err());

        // PEM 标记错误时应给出明确错误
        let err = parse_csr(Some("-----BEGIN FOO-----\nYWJj\n-----END FOO-----".to_string()), None, None);
        assert!(err.is_err());
    }

    #[test]
    fn test_subject_order_preserved() {
        let pem = generate_test_csr_pem();
        let info = parse_csr(Some(pem), None, None).unwrap();

        let values: Vec<&str> = info.subject.iter().map(|e| e.value.as_str()).collect();
        let cn_pos = values.iter().position(|v| *v == "test.example.com").unwrap();
        let o_pos = values.iter().position(|v| *v == "Test Org").unwrap();
        assert!(cn_pos < o_pos, "主题属性应保持 CSR 中的顺序");
    }

    #[test]
    fn test_serde_json_keys_are_camel_case() {
        // 前端 TypeScript 接口依赖 camelCase 键名, 防止 rename_all 失效
        let pem = generate_test_csr_pem();
        let info = parse_csr(Some(pem), None, None).unwrap();
        let json = serde_json::to_value(&info).unwrap();

        assert!(json.get("version").is_some());
        assert!(json.get("subject").is_some());
        assert!(json.get("signatureAlgorithm").is_some());
        assert!(json.get("requestedExtensions").is_some());
        assert!(json.get("signatureValid").is_some());
        assert!(json.get("derSize").is_some());
        assert!(json.get("pem").is_some());

        let public_key = json.get("publicKey").unwrap();
        assert!(public_key.get("keyType").is_some());
        assert!(public_key.get("keySize").is_some());
        assert!(public_key.get("curveName").is_some());
        assert!(public_key.get("spkiPem").is_some());
        assert!(public_key.get("fingerprintSha256").is_some());
    }

    #[test]
    fn test_parse_real_cli_generated_csr() {
        // 用 openssl 命令行生成的真实 CSR 文件验证（文件不存在时跳过，兼容 CI）
        let path = std::path::Path::new("/tmp/devtools_csr_test.csr");
        if !path.exists() {
            return;
        }
        let content = std::fs::read_to_string(path).unwrap();

        let info = parse_csr(Some(content), None, None).unwrap();
        assert!(info.signature_valid);
        assert_eq!(info.public_key.key_type, "RSA");
        assert_eq!(info.public_key.key_size, Some(2048));
        assert!(info.subject.iter().any(|e| e.value == "example.test.com"));
        assert!(info.subject.iter().any(|e| e.value == "DevTools Test"));
        assert!(info.requested_extensions.iter().any(|e| e.name == "Subject Alternative Name"
            && e.value.contains("DNS:example.test.com")
            && e.value.contains("DNS:www.test.com")));
        assert!(info
            .requested_extensions
            .iter()
            .any(|e| e.name == "Key Usage" && e.value.contains("digitalSignature")));
        assert_eq!(info.signature_algorithm, "SHA256WithRSA");
    }
}
