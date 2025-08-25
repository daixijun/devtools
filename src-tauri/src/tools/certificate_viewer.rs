use ::time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use x509_parser::certificate::X509Certificate;
use x509_parser::extensions::ParsedExtension;
use x509_parser::oid_registry::Oid;
use x509_parser::pem::Pem;
use x509_parser::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CertificateInfo {
    pub subject: HashMap<String, String>,
    pub issuer: HashMap<String, String>,
    pub validity: ValidityInfo,
    pub serial_number: String,
    pub signature_algorithm: String,
    pub public_key_info: PublicKeyInfo,
    pub extensions: Vec<ExtensionInfo>,
    pub sans: Vec<String>,
    pub chain_level: usize,
    pub certificate_type: Option<String>,
    pub brand: Option<String>,
    pub sha1_fingerprint: Option<String>,
    pub sha256_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateChainInfo {
    pub certificates: Vec<CertificateInfo>,
    pub missing_certificates: Vec<MissingCertificateInfo>,
    pub is_full_chain: bool,
    pub chain_status: String,
    pub ca_download_urls: Vec<String>,
    pub missing_ca_info: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MissingCertificateInfo {
    pub subject_name: String,
    pub issuer_name: String,
    pub certificate_type: String,
    pub chain_level: usize,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidityInfo {
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicKeyInfo {
    pub key_type: String,
    pub key_size: Option<u32>,
    pub algorithm: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionInfo {
    pub name: String,
    pub value: String,
    pub critical: bool,
}

/// 检查两个证书是否为同一个证书（通过序列号和指纹比较）
fn is_duplicate_certificate(cert1: &CertificateInfo, cert2: &CertificateInfo) -> bool {
    // 首先比较序列号
    if cert1.serial_number == cert2.serial_number {
        return true;
    }

    // 如果有SHA256指纹，比较指纹
    if let (Some(fp1), Some(fp2)) = (&cert1.sha256_fingerprint, &cert2.sha256_fingerprint) {
        if fp1 == fp2 {
            return true;
        }
    }

    // 如果有SHA1指纹，比较指纹
    if let (Some(fp1), Some(fp2)) = (&cert1.sha1_fingerprint, &cert2.sha1_fingerprint) {
        if fp1 == fp2 {
            return true;
        }
    }

    false
}

/// 去除重复的证书
fn deduplicate_certificates(certificates: &[CertificateInfo]) -> Vec<CertificateInfo> {
    let mut deduped = Vec::new();

    for cert in certificates {
        // 检查是否已经存在相同的证书
        let is_duplicate = deduped
            .iter()
            .any(|existing| is_duplicate_certificate(existing, cert));

        if !is_duplicate {
            deduped.push(cert.clone());
        }
    }

    deduped
}

#[tauri::command]
pub fn parse_pfx_certificate(
    pfx_data: Vec<u8>,
    password: Option<String>,
) -> Result<CertificateChainInfo, String> {
    use openssl::pkcs12::Pkcs12;

    if pfx_data.is_empty() {
        return Err("PFX文件内容不能为空".to_string());
    }

    let password_str = password.as_deref().unwrap_or("");

    // 解析PFX文件
    let pkcs12 = Pkcs12::from_der(&pfx_data).map_err(|e| format!("无效的PFX文件格式: {}", e))?;

    let parsed = pkcs12
        .parse2(password_str)
        .map_err(|e| format!("PFX文件解析失败: {}。可能原因：密码错误或PFX文件损坏", e))?;

    let mut certificates = Vec::new();

    // 处理主证书
    if let Some(cert) = parsed.cert {
        let cert_der = cert.to_der().map_err(|e| format!("证书转换失败: {}", e))?;

        let (_, x509_cert) =
            X509Certificate::from_der(&cert_der).map_err(|e| format!("证书解析失败: {}", e))?;

        let cert_info = parse_certificate(&x509_cert)?;
        certificates.push(cert_info);
    }

    // 处理CA证书链
    if let Some(ca_certs) = parsed.ca {
        for cert in ca_certs {
            let cert_der = cert
                .to_der()
                .map_err(|e| format!("CA证书转换失败: {}", e))?;

            let (_, x509_cert) = X509Certificate::from_der(&cert_der)
                .map_err(|e| format!("CA证书解析失败: {}", e))?;

            let cert_info = parse_certificate(&x509_cert)?;
            certificates.push(cert_info);
        }
    }

    if certificates.is_empty() {
        return Err("未在PFX文件中找到任何证书".to_string());
    }

    // 去除重复的证书（PFX文件中可能包含重复的中间CA证书）
    let certificates = deduplicate_certificates(&certificates);

    // 分析证书链
    let chain_analysis = analyze_certificate_chain(&certificates);
    Ok(chain_analysis)
}

#[tauri::command]
pub fn parse_pem_certificate(pem_content: String) -> Result<CertificateChainInfo, String> {
    // 清理PEM内容
    let cleaned_content = pem_content
        .replace("\r\n", "\n")
        .replace("\r", "\n")
        .trim()
        .to_string();

    // 验证PEM格式 - 更严格的检查
    if !cleaned_content.contains("-----BEGIN CERTIFICATE-----")
        || !cleaned_content.contains("-----END CERTIFICATE-----")
    {
        return Err("无效的PEM证书格式：缺少正确的BEGIN/END标记".to_string());
    }

    // 检查是否包含有效的Base64内容
    let lines: Vec<&str> = cleaned_content.lines().collect();
    let mut in_cert = false;
    let mut has_valid_content = false;

    for line in &lines {
        let line = line.trim();
        if line == "-----BEGIN CERTIFICATE-----" {
            in_cert = true;
            continue;
        }
        if line == "-----END CERTIFICATE-----" {
            in_cert = false;
            continue;
        }
        if in_cert && !line.is_empty() {
            has_valid_content = true;
            break;
        }
    }

    if !has_valid_content {
        return Err("PEM证书内容为空或格式错误".to_string());
    }

    // 使用x509-parser解析多个证书
    let mut certificates = Vec::new();
    let mut pem_contents = Vec::new();

    for (_index, pem_result) in Pem::iter_from_buffer(cleaned_content.as_bytes()).enumerate() {
        let pem = match pem_result {
            Ok(pem) => pem,
            Err(e) => return Err(format!("PEM解析失败: {}", e)),
        };

        if pem.label == "CERTIFICATE" {
            // 克隆PEM内容以避免生命周期问题
            let contents = pem.contents.clone();
            pem_contents.push(contents);
        }
    }

    if pem_contents.is_empty() {
        return Err("未找到有效的证书".to_string());
    }

    // 解析证书并构建信息结构
    for (index, contents) in pem_contents.iter().enumerate() {
        let (_, cert) = X509Certificate::from_der(contents)
            .map_err(|e| format!("第{}个证书解析失败: {}", index + 1, e))?;

        let cert_info = parse_certificate(&cert)?;
        certificates.push(cert_info);
    }

    // 去除重复的证书（PEM文件中也可能包含重复证书）
    let certificates = deduplicate_certificates(&certificates);

    // 分析证书链
    let chain_analysis = analyze_certificate_chain(&certificates);

    Ok(chain_analysis)
}

fn parse_certificate(cert: &X509Certificate) -> Result<CertificateInfo, String> {
    // 解析主题
    let subject = parse_name(&cert.subject)?;

    // 解析颁发者
    let issuer = parse_name(&cert.issuer)?;

    // 获取有效期信息
    let not_before = cert.validity.not_before.to_datetime();
    let not_after = cert.validity.not_after.to_datetime();

    // 计算剩余天数
    let now = OffsetDateTime::now_utc();
    let duration = not_after - now;
    let days_until_expiry = duration.whole_days();

    let validity = ValidityInfo {
        not_before: not_before
            .format(&::time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| not_before.to_string()),
        not_after: not_after
            .format(&::time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| not_after.to_string()),
        days_until_expiry,
    };

    // 序列号 - 移除冒号分隔符
    let serial_bytes = cert.tbs_certificate.serial.to_bytes_be();
    let serial_number = hex::encode(&serial_bytes).to_uppercase();

    // 签名算法 - 显示用户友好的名称
    let signature_algorithm = match cert.signature_algorithm.algorithm.to_string().as_str() {
        "1.2.840.113549.1.1.11" => "SHA256WithRSA".to_string(),
        "1.2.840.113549.1.1.5" => "SHA1WithRSA".to_string(),
        "1.2.840.113549.1.1.4" => "MD5WithRSA".to_string(),
        "1.2.840.113549.1.1.13" => "SHA512WithRSA".to_string(),
        "1.2.840.113549.1.1.12" => "SHA384WithRSA".to_string(),
        "1.2.840.10045.4.3.2" => "ECDSAWithSHA256".to_string(),
        "1.2.840.10045.4.3.3" => "ECDSAWithSHA384".to_string(),
        "1.2.840.10045.4.3.4" => "ECDSAWithSHA512".to_string(),
        "1.2.840.10045.4.1" => "ECDSAWithSHA1".to_string(),
        "1.3.101.112" => "Ed25519".to_string(),
        "1.2.840.10040.4.3" => "DSAWithSHA1".to_string(),
        "2.16.840.1.101.3.4.3.1" => "DSAWithSHA224".to_string(),
        "2.16.840.1.101.3.4.3.2" => "DSAWithSHA256".to_string(),
        _ => cert.signature_algorithm.algorithm.to_id_string(),
    };

    // 公钥信息
    let public_key_info = parse_public_key(&cert.tbs_certificate.subject_pki)?;

    // 提取域名信息
    let sans = extract_sans(cert)?;

    // 扩展信息
    let extensions = parse_extensions(cert);

    // 确定证书链级别
    let chain_level = determine_chain_level(cert);

    // 计算指纹
    let sha1_fingerprint = Some(calculate_sha1_fingerprint(cert)?);
    let sha256_fingerprint = Some(calculate_sha256_fingerprint(cert)?);

    // 确定证书类型和品牌
    let certificate_type = determine_certificate_type(cert);
    let brand = determine_certificate_brand(cert);

    Ok(CertificateInfo {
        subject,
        issuer,
        validity,
        serial_number,
        signature_algorithm,
        public_key_info,
        extensions,
        sans,
        chain_level,
        certificate_type,
        brand,
        sha1_fingerprint,
        sha256_fingerprint,
    })
}

fn parse_name(name: &x509_parser::x509::X509Name) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();

    // Pre-define OIDs for comparison
    let cn_oid = Oid::from(&[2, 5, 4, 3]).unwrap();
    let o_oid = Oid::from(&[2, 5, 4, 10]).unwrap();
    let ou_oid = Oid::from(&[2, 5, 4, 11]).unwrap();
    let c_oid = Oid::from(&[2, 5, 4, 6]).unwrap();
    let st_oid = Oid::from(&[2, 5, 4, 8]).unwrap();
    let l_oid = Oid::from(&[2, 5, 4, 7]).unwrap();
    let email_oid = Oid::from(&[1, 2, 840, 113549, 1, 9, 1]).unwrap();

    for rdn in name.iter() {
        for attr in rdn.iter() {
            let key = if attr.attr_type() == &cn_oid {
                "通用名称 (CN)"
            } else if attr.attr_type() == &o_oid {
                "组织名称 (O)"
            } else if attr.attr_type() == &ou_oid {
                "组织单位 (OU)"
            } else if attr.attr_type() == &c_oid {
                "国家 (C)"
            } else if attr.attr_type() == &st_oid {
                "省份 (ST)"
            } else if attr.attr_type() == &l_oid {
                "城市 (L)"
            } else if attr.attr_type() == &email_oid {
                "邮箱地址"
            } else {
                continue;
            };

            let value = attr.as_str().unwrap_or("").to_string();

            result.insert(key.to_string(), value);
        }
    }

    Ok(result)
}

fn parse_public_key(
    public_key: &x509_parser::x509::SubjectPublicKeyInfo,
) -> Result<PublicKeyInfo, String> {
    let rsa_oid = Oid::from(&[1, 2, 840, 113549, 1, 1, 1]).unwrap();
    let ecc_oid = Oid::from(&[1, 2, 840, 10045, 2, 1]).unwrap();
    let dsa_oid = Oid::from(&[1, 2, 840, 10040, 4, 1]).unwrap();
    let ed25519_oid = Oid::from(&[1, 3, 101, 112]).unwrap();

    let key_type = if public_key.algorithm.algorithm == rsa_oid {
        "RSA"
    } else if public_key.algorithm.algorithm == ecc_oid {
        "ECC"
    } else if public_key.algorithm.algorithm == dsa_oid {
        "DSA"
    } else if public_key.algorithm.algorithm == ed25519_oid {
        "Ed25519"
    } else {
        "未知"
    }
    .to_string();

    let key_size = public_key.parsed().map(|pk| pk.key_size()).unwrap_or(0) as u32;

    // 算法名称 - 使用用户友好的名称而不是OID
    let algorithm = if public_key.algorithm.algorithm == rsa_oid {
        "RSA Encryption".to_string()
    } else if public_key.algorithm.algorithm == ecc_oid {
        "Elliptic Curve Public Key".to_string()
    } else if public_key.algorithm.algorithm == dsa_oid {
        "DSA Public Key".to_string()
    } else if public_key.algorithm.algorithm == ed25519_oid {
        "Ed25519 Public Key".to_string()
    } else {
        // 对于其他算法，尝试提供更友好的名称
        match public_key.algorithm.algorithm.to_string().as_str() {
            "1.2.840.113549.1.1.1" => "RSA Encryption".to_string(),
            "1.2.840.10045.2.1" => "Elliptic Curve Public Key".to_string(),
            "1.2.840.10040.4.1" => "DSA Public Key".to_string(),
            "1.3.101.112" => "Ed25519 Public Key".to_string(),
            "1.3.101.113" => "Ed448 Public Key".to_string(),
            _ => "Public Key Algorithm".to_string(),
        }
    };

    Ok(PublicKeyInfo {
        key_type,
        key_size: if key_size > 0 { Some(key_size) } else { None },
        algorithm,
    })
}

fn extract_sans(cert: &X509Certificate) -> Result<Vec<String>, String> {
    let mut sans = Vec::new();

    // 从Subject获取Common Name
    if let Some(cn) = cert.subject().iter_common_name().next() {
        if let Ok(cn_str) = cn.as_str() {
            sans.push(cn_str.to_string());
        }
    }

    // 从Subject Alternative Name扩展中提取
    let san_oid = Oid::from(&[2, 5, 29, 17]).unwrap();
    if let Some(san_extension) = cert.extensions().iter().find(|ext| {
        ext.oid == san_oid // SAN OID
    }) {
        if let ParsedExtension::SubjectAlternativeName(san) = san_extension.parsed_extension() {
            for name in &san.general_names {
                match name {
                    GeneralName::DNSName(dns) => {
                        if !dns.is_empty() {
                            sans.push(dns.to_string());
                        }
                    }
                    GeneralName::IPAddress(ip) => {
                        if !ip.is_empty() {
                            let ip_str = match ip.len() {
                                4 => {
                                    // IPv4
                                    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
                                }
                                16 => {
                                    // IPv6 - 改进格式化
                                    let mut ip_str = String::new();
                                    for i in (0..16).step_by(2) {
                                        if i > 0 {
                                            ip_str.push(':');
                                        }
                                        let part = ((ip[i] as u16) << 8) | (ip[i + 1] as u16);
                                        ip_str.push_str(&format!("{:x}", part));
                                    }
                                    ip_str
                                }
                                _ => format!("{:?}", ip),
                            };
                            sans.push(ip_str);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 去重
    sans.sort();
    sans.dedup();

    Ok(sans)
}

fn parse_extensions(cert: &X509Certificate) -> Vec<ExtensionInfo> {
    let mut extensions = Vec::new();

    // Pre-define OIDs for comparison
    let san_oid = Oid::from(&[2, 5, 29, 17]).unwrap();
    let key_usage_oid = Oid::from(&[2, 5, 29, 15]).unwrap();
    let ext_key_usage_oid = Oid::from(&[2, 5, 29, 37]).unwrap();
    let subject_key_id_oid = Oid::from(&[2, 5, 29, 14]).unwrap();
    let authority_key_id_oid = Oid::from(&[2, 5, 29, 35]).unwrap();
    let crl_dp_oid = Oid::from(&[2, 5, 29, 31]).unwrap();
    let cert_policies_oid = Oid::from(&[2, 5, 29, 32]).unwrap();
    let subject_info_access_oid = Oid::from(&[1, 3, 6, 1, 4, 1, 311, 21, 10]).unwrap();

    for ext in cert.extensions() {
        let name = if ext.oid == san_oid {
            "Subject Alternative Name"
        } else if ext.oid == key_usage_oid {
            "Key Usage"
        } else if ext.oid == ext_key_usage_oid {
            "Extended Key Usage"
        } else if ext.oid == subject_key_id_oid {
            "Subject Key Identifier"
        } else if ext.oid == authority_key_id_oid {
            "Authority Key Identifier"
        } else if ext.oid == crl_dp_oid {
            "CRL Distribution Points"
        } else if ext.oid == cert_policies_oid {
            "Certificate Policies"
        } else if ext.oid == subject_info_access_oid {
            "Subject Information Access"
        } else {
            "Unknown Extension"
        };

        let value = format!("{:?}", ext.parsed_extension());
        let critical = ext.critical;

        extensions.push(ExtensionInfo {
            name: name.to_string(),
            value,
            critical,
        });
    }

    extensions
}

fn determine_chain_level(cert: &X509Certificate) -> usize {
    // 检查Basic Constraints扩展
    let basic_constraints_oid = Oid::from(&[2, 5, 29, 19]).unwrap();

    if let Some(bc_ext) = cert
        .extensions()
        .iter()
        .find(|ext| ext.oid == basic_constraints_oid)
    {
        if let ParsedExtension::BasicConstraints(bc) = bc_ext.parsed_extension() {
            if bc.ca {
                // 是CA证书，检查路径长度
                match bc.path_len_constraint {
                    Some(path_len) => {
                        // 有路径长度限制，通常是中间CA
                        if path_len == 0 {
                            1 // 中间CA，路径长度为0
                        } else {
                            // 路径长度大于0，可能是根CA或高级中间CA
                            // 检查是否为自签名来进一步区分
                            if is_self_signed(cert) {
                                2 // 自签名根CA
                            } else {
                                1 // 中间CA
                            }
                        }
                    }
                    None => {
                        // 没有路径长度限制，检查是否为自签名
                        if is_self_signed(cert) {
                            2 // 自签名根CA
                        } else {
                            1 // 中间CA
                        }
                    }
                }
            } else {
                0 // 不是CA，终端证书
            }
        } else {
            // 无法解析Basic Constraints，回退到原有逻辑
            fallback_determine_chain_level(cert)
        }
    } else {
        // 没有Basic Constraints扩展，回退到原有逻辑
        fallback_determine_chain_level(cert)
    }
}

fn is_self_signed(cert: &X509Certificate) -> bool {
    let subject = cert.subject();
    let issuer = cert.issuer();

    let subject_cn = subject
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    let issuer_cn = issuer
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    subject_cn == issuer_cn && !subject_cn.is_empty()
}

fn fallback_determine_chain_level(cert: &X509Certificate) -> usize {
    // 原有的判断逻辑作为回退
    let subject = cert.subject();
    let issuer = cert.issuer();

    let subject_cn = subject
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    let issuer_cn = issuer
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    let is_self_signed = subject_cn == issuer_cn && !subject_cn.is_empty();

    // 检查基本约束（这里简化处理）
    if is_self_signed {
        2 // 可能是根CA
    } else {
        // 简单的启发式判断
        if issuer_cn.contains("Root") || issuer_cn.contains("CA") {
            0 // 可能是终端证书
        } else {
            1 // 可能是中间CA
        }
    }
}

fn calculate_sha1_fingerprint(cert: &X509Certificate) -> Result<String, String> {
    // 使用CryptoUtils计算SHA1指纹
    let der_data = cert.as_ref();
    Ok(crate::utils::crypto::CryptoUtils::calculate_sha1_fingerprint(der_data))
}

fn calculate_sha256_fingerprint(cert: &X509Certificate) -> Result<String, String> {
    // 使用CryptoUtils计算SHA256指纹
    let der_data = cert.as_ref();
    Ok(crate::utils::crypto::CryptoUtils::calculate_sha256_fingerprint(der_data))
}

fn determine_certificate_type(cert: &X509Certificate) -> Option<String> {
    let issuer = cert.issuer();
    let subject = cert.subject();

    // 获取颁发者和主题信息
    let issuer_cn = issuer
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    let subject_o = subject
        .iter_organization()
        .next()
        .and_then(|o| o.as_str().ok())
        .unwrap_or("");

    // 基于颁发者和主题信息确定证书类型
    if issuer_cn.contains("EV") || issuer_cn.contains("Extended Validation") {
        return Some("EV".to_string());
    }

    if !subject_o.is_empty() && !subject_o.contains("Unknown") {
        return Some("OV".to_string());
    }

    // 基于常见CA的默认类型
    if issuer_cn.contains("Let's Encrypt") {
        return Some("DV".to_string());
    }

    Some("DV".to_string())
}

fn determine_certificate_brand(cert: &X509Certificate) -> Option<String> {
    let issuer = cert.issuer();

    // 获取颁发者通用名称和组织名称
    let issuer_cn = issuer
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let issuer_o = issuer
        .iter_organization()
        .next()
        .and_then(|o| o.as_str().ok())
        .unwrap_or("")
        .to_lowercase();

    // 组合颁发者信息进行匹配
    let issuer_info = format!("{} {}", issuer_cn, issuer_o);

    // 基于颁发者确定证书品牌 - 更全面的品牌识别
    let brand = if issuer_info.contains("digicert") {
        "DigiCert"
    } else if issuer_info.contains("globalsign") {
        "GlobalSign"
    } else if issuer_info.contains("let's encrypt") || issuer_info.contains("letsencrypt") {
        "Let's Encrypt"
    } else if issuer_info.contains("sectigo") || issuer_info.contains("comodo") {
        "Sectigo"
    } else if issuer_info.contains("amazon") || issuer_info.contains("aws") {
        "Amazon Trust Services"
    } else if issuer_info.contains("google") || issuer_info.contains("gts") {
        "Google Trust Services"
    } else if issuer_info.contains("microsoft") {
        "Microsoft"
    } else if issuer_info.contains("apple") {
        "Apple"
    } else if issuer_info.contains("entrust") {
        "Entrust"
    } else if issuer_info.contains("godaddy") {
        "GoDaddy"
    } else if issuer_info.contains("rapidssl") {
        "RapidSSL"
    } else if issuer_info.contains("thawte") {
        "Thawte"
    } else if issuer_info.contains("verisign") {
        "VeriSign"
    } else if issuer_info.contains("symantec") {
        "Symantec"
    } else if issuer_info.contains("geotrust") {
        "GeoTrust"
    } else if issuer_info.contains("trustwave") {
        "Trustwave"
    } else if issuer_info.contains("ssl.com") {
        "SSL.com"
    } else if issuer_info.contains("buypass") {
        "Buypass"
    } else if issuer_info.contains("certum") {
        "Certum"
    } else if issuer_info.contains("startcom") || issuer_info.contains("startssl") {
        "StartCom"
    } else if issuer_info.contains("wosign") {
        "WoSign"
    } else if issuer_info.contains("cfca") {
        "CFCA"
    } else if issuer_info.contains("trustasia") {
        "TrustAsia"
    } else if issuer_info.contains("zerossl") {
        "ZeroSSL"
    } else if issuer_info.contains("cloudflare") {
        "Cloudflare"
    } else if issuer_info.contains("fastly") {
        "Fastly"
    } else if issuer_info.contains("baltimore") {
        "Baltimore CyberTrust"
    } else if issuer_info.contains("identrust") {
        "IdenTrust"
    } else {
        "Unknown"
    };

    Some(brand.to_string())
}

fn analyze_certificate_chain(cert_infos: &[CertificateInfo]) -> CertificateChainInfo {
    let mut is_full_chain = false;
    let mut ca_download_urls = Vec::new();
    let mut missing_ca_info = None;
    let chain_status;

    // 改进的证书链排序逻辑
    let certificates = build_certificate_chain(cert_infos);

    // 识别缺失的CA证书
    let missing_certificates = identify_missing_certificates(&certificates);

    if certificates.len() == 1 {
        // 只有一个证书，检查是否为自签名根证书
        let cert = &certificates[0];
        if cert.chain_level == 2 {
            is_full_chain = true;
            chain_status = "完整证书链：自签名根证书".to_string();
        } else {
            // 单个终端证书或中间CA证书是不完整的
            chain_status = "不完整证书链：缺少上级CA证书".to_string();
            missing_ca_info = Some("此证书需要上级CA证书才能形成完整链路".to_string());
            ca_download_urls = suggest_ca_download_urls(cert);
        }
    } else {
        // 多个证书，分析链路完整性
        let has_root_ca = certificates.iter().any(|cert| cert.chain_level == 2);
        let has_intermediate_ca = certificates.iter().any(|cert| cert.chain_level == 1);
        let has_end_entity = certificates.iter().any(|cert| cert.chain_level == 0);

        // 更严格的证书链完整性检查
        if has_end_entity {
            // 有终端证书的情况
            if has_root_ca {
                // 有终端证书和根证书
                if has_intermediate_ca {
                    // 终端 + 中间 + 根 = 完整链
                    is_full_chain = true;
                    chain_status = "完整证书链：包含终端证书、中间CA和根证书".to_string();
                } else {
                    // 终端 + 根，但可能缺少中间CA
                    // 需要检查终端证书的颁发者是否直接是根证书
                    let end_cert = certificates
                        .iter()
                        .find(|cert| cert.chain_level == 0)
                        .unwrap();
                    let root_cert = certificates
                        .iter()
                        .find(|cert| cert.chain_level == 2)
                        .unwrap();

                    if certificates_match_issuer_subject(&root_cert.subject, &end_cert.issuer) {
                        is_full_chain = true;
                        chain_status = "完整证书链：终端证书由根CA直接颁发".to_string();
                    } else {
                        chain_status = "不完整证书链：缺少中间CA证书".to_string();
                        missing_ca_info = Some("终端证书和根证书之间缺少中间CA证书".to_string());
                        ca_download_urls = suggest_ca_download_urls(&certificates[0]);
                    }
                }
            } else if has_intermediate_ca {
                // 有终端证书和中间CA，但缺少根证书
                chain_status = "不完整证书链：缺少根CA证书".to_string();
                missing_ca_info = Some("证书链缺少根CA证书，但包含必要的中间CA".to_string());
                ca_download_urls = suggest_ca_download_urls(&certificates[0]);
            } else {
                // 只有终端证书，缺少所有CA
                chain_status = "不完整证书链：缺少所有CA证书".to_string();
                missing_ca_info = Some("此证书需要完整的CA证书链才能正常使用".to_string());
                ca_download_urls = suggest_ca_download_urls(&certificates[0]);
            }
        } else if has_intermediate_ca && has_root_ca {
            // 只有CA证书，没有终端证书
            is_full_chain = true;
            chain_status = "CA证书链：包含中间CA和根证书".to_string();
        } else if has_root_ca {
            // 只有根证书
            is_full_chain = true;
            chain_status = "根证书：自签名根CA证书".to_string();
        } else if has_intermediate_ca {
            // 只有中间CA，缺少根证书
            chain_status = "不完整CA链：缺少根CA证书".to_string();
            missing_ca_info = Some("中间CA证书需要对应的根CA证书".to_string());
            ca_download_urls = suggest_ca_download_urls(&certificates[0]);
        } else {
            chain_status = "证书链结构异常".to_string();
        }
    }

    CertificateChainInfo {
        certificates,
        missing_certificates,
        is_full_chain,
        chain_status,
        ca_download_urls,
        missing_ca_info,
    }
}

/// 构建正确排序的证书链
/// 处理缺少根CA但有多个中间CA的情况，确保证书按正确的颁发顺序排列
fn build_certificate_chain(cert_infos: &[CertificateInfo]) -> Vec<CertificateInfo> {
    let mut certificates = cert_infos.to_vec();

    // 如果只有一个证书，直接返回
    if certificates.len() <= 1 {
        return certificates;
    }

    // 尝试按照颁发关系进行排序
    let ordered = order_certificates_by_chain(&certificates);
    if !ordered.is_empty() {
        return ordered;
    }

    // 回退到按级别排序
    certificates.sort_by(|a, b| b.chain_level.cmp(&a.chain_level));
    certificates
}

/// 识别缺失的CA证书
fn identify_missing_certificates(certificates: &[CertificateInfo]) -> Vec<MissingCertificateInfo> {
    let mut missing = Vec::new();

    if certificates.is_empty() {
        return missing;
    }

    let has_root_ca = certificates.iter().any(|cert| cert.chain_level == 2);
    let has_intermediate_ca = certificates.iter().any(|cert| cert.chain_level == 1);
    let has_end_entity = certificates.iter().any(|cert| cert.chain_level == 0);

    // 如果有终端证书但没有任何CA证书
    if has_end_entity && !has_intermediate_ca && !has_root_ca {
        let end_cert = certificates
            .iter()
            .find(|cert| cert.chain_level == 0)
            .unwrap();
        let empty_string = String::new();
        let issuer_cn = end_cert
            .issuer
            .get("通用名称 (CN)")
            .unwrap_or(&empty_string);
        let issuer_o = end_cert.issuer.get("组织名称 (O)").unwrap_or(&empty_string);

        let issuer_name = if !issuer_cn.is_empty() {
            issuer_cn.clone()
        } else if !issuer_o.is_empty() {
            issuer_o.clone()
        } else {
            "未知CA".to_string()
        };

        missing.push(MissingCertificateInfo {
            subject_name: issuer_name.clone(),
            issuer_name: "未知".to_string(),
            certificate_type: "中间CA或根CA".to_string(),
            chain_level: 1, // 假设是中间CA
            description: format!("缺少颁发终端证书的CA: {}", issuer_name),
        });
    }

    // 如果有中间CA但没有根CA
    if has_intermediate_ca && !has_root_ca {
        // 找到链级别最高的中间CA
        if let Some(top_intermediate) = certificates
            .iter()
            .filter(|cert| cert.chain_level == 1)
            .max_by_key(|cert| {
                // 简单的启发式：选择序列号最小的（通常是最早颁发的）
                &cert.serial_number
            })
        {
            let empty_string = String::new();
            let issuer_cn = top_intermediate
                .issuer
                .get("通用名称 (CN)")
                .unwrap_or(&empty_string);
            let issuer_o = top_intermediate
                .issuer
                .get("组织名称 (O)")
                .unwrap_or(&empty_string);

            let issuer_name = if !issuer_cn.is_empty() {
                issuer_cn.clone()
            } else if !issuer_o.is_empty() {
                issuer_o.clone()
            } else {
                "未知根CA".to_string()
            };

            missing.push(MissingCertificateInfo {
                subject_name: issuer_name.clone(),
                issuer_name: issuer_name.clone(), // 根CA是自签名的
                certificate_type: "根CA".to_string(),
                chain_level: 2,
                description: format!("缺少根CA证书: {}", issuer_name),
            });
        }
    }

    // 检查证书链中是否有断裂（即某个证书的颁发者不在链中）
    for cert in certificates {
        if cert.chain_level == 0 || cert.chain_level == 1 {
            let empty_string = String::new();
            let issuer_cn = cert.issuer.get("通用名称 (CN)").unwrap_or(&empty_string);
            let issuer_o = cert.issuer.get("组织名称 (O)").unwrap_or(&empty_string);

            // 检查是否存在对应的颁发者证书
            let has_issuer = certificates.iter().any(|other_cert| {
                certificates_match_issuer_subject(&other_cert.subject, &cert.issuer)
            });

            if !has_issuer && !is_cert_self_signed(cert) {
                let issuer_name = if !issuer_cn.is_empty() {
                    issuer_cn.clone()
                } else if !issuer_o.is_empty() {
                    issuer_o.clone()
                } else {
                    "未知CA".to_string()
                };

                // 避免重复添加
                if !missing.iter().any(|m| m.subject_name == issuer_name) {
                    missing.push(MissingCertificateInfo {
                        subject_name: issuer_name.clone(),
                        issuer_name: "未知".to_string(),
                        certificate_type: if cert.chain_level == 0 {
                            "中间CA"
                        } else {
                            "根CA或上级中间CA"
                        }
                        .to_string(),
                        chain_level: cert.chain_level + 1,
                        description: format!("缺少颁发证书的CA: {}", issuer_name),
                    });
                }
            }
        }
    }

    missing
}

/// 按照证书链的颁发关系进行排序
/// 从根CA开始，到终端证书结束
fn order_certificates_by_chain(certificates: &[CertificateInfo]) -> Vec<CertificateInfo> {
    let mut ordered = Vec::new();
    let mut remaining: Vec<_> = certificates.iter().collect();

    // 首先找到根证书（自签名的证书）
    if let Some((index, root)) = remaining
        .iter()
        .enumerate()
        .find(|(_, cert)| is_cert_self_signed(cert))
    {
        ordered.push((*root).clone());
        remaining.remove(index);
    }

    // 如果没有找到根证书，从链级别最高的证书开始
    if ordered.is_empty() {
        if let Some((index, highest)) = remaining
            .iter()
            .enumerate()
            .max_by_key(|(_, cert)| cert.chain_level)
        {
            ordered.push((*highest).clone());
            remaining.remove(index);
        }
    }

    // 逐步找到被当前证书颁发的下一级证书
    while !remaining.is_empty() && !ordered.is_empty() {
        let current_issuer = &ordered.last().unwrap().subject;

        // 寻找由当前证书颁发的证书（即subject匹配current的issuer）
        let next_index = remaining
            .iter()
            .position(|cert| certificates_match_issuer_subject(current_issuer, &cert.issuer));

        if let Some(index) = next_index {
            let next_cert = remaining.remove(index);
            ordered.push(next_cert.clone());
        } else {
            // 如果找不到匹配的，按链级别选择最低的证书
            if let Some((index, lowest)) = remaining
                .iter()
                .enumerate()
                .min_by_key(|(_, cert)| cert.chain_level)
            {
                ordered.push((*lowest).clone());
                remaining.remove(index);
            } else {
                break;
            }
        }
    }

    // 如果还有剩余证书，按链级别添加
    while !remaining.is_empty() {
        if let Some((index, cert)) = remaining
            .iter()
            .enumerate()
            .min_by_key(|(_, cert)| cert.chain_level)
        {
            ordered.push((*cert).clone());
            remaining.remove(index);
        }
    }

    ordered
}

/// 检查证书是否为自签名证书
fn is_cert_self_signed(cert: &CertificateInfo) -> bool {
    let empty_string = String::new();
    let subject_cn = cert.subject.get("通用名称 (CN)").unwrap_or(&empty_string);
    let issuer_cn = cert.issuer.get("通用名称 (CN)").unwrap_or(&empty_string);

    let subject_o = cert.subject.get("组织名称 (O)").unwrap_or(&empty_string);
    let issuer_o = cert.issuer.get("组织名称 (O)").unwrap_or(&empty_string);

    // 比较CN和O字段
    (!subject_cn.is_empty() && subject_cn == issuer_cn)
        || (!subject_o.is_empty() && subject_o == issuer_o)
}

/// 检查两个证书的颁发关系（证书A的subject是否与证书B的issuer匹配）
fn certificates_match_issuer_subject(
    subject: &std::collections::HashMap<String, String>,
    issuer: &std::collections::HashMap<String, String>,
) -> bool {
    let empty_string = String::new();

    // 比较CN字段
    let subject_cn = subject.get("通用名称 (CN)").unwrap_or(&empty_string);
    let issuer_cn = issuer.get("通用名称 (CN)").unwrap_or(&empty_string);

    if !subject_cn.is_empty() && !issuer_cn.is_empty() && subject_cn == issuer_cn {
        return true;
    }

    // 比较O字段
    let subject_o = subject.get("组织名称 (O)").unwrap_or(&empty_string);
    let issuer_o = issuer.get("组织名称 (O)").unwrap_or(&empty_string);

    if !subject_o.is_empty() && !issuer_o.is_empty() && subject_o == issuer_o {
        return true;
    }

    // 比较OU字段
    let subject_ou = subject.get("组织单位 (OU)").unwrap_or(&empty_string);
    let issuer_ou = issuer.get("组织单位 (OU)").unwrap_or(&empty_string);

    if !subject_ou.is_empty() && !issuer_ou.is_empty() && subject_ou == issuer_ou {
        return true;
    }

    false
}

fn suggest_ca_download_urls(cert: &CertificateInfo) -> Vec<String> {
    let mut urls = Vec::new();

    // 根据证书颁发者信息提供CA下载建议 - 更新和优化URL
    // 同时检查通用名称和组织名称获得更好的匹配
    let issuer_cn = cert
        .issuer
        .get("通用名称 (CN)")
        .unwrap_or(&String::new())
        .to_lowercase();
    let issuer_o = cert
        .issuer
        .get("组织名称 (O)")
        .unwrap_or(&String::new())
        .to_lowercase();
    let issuer_info = format!("{} {}", issuer_cn, issuer_o);

    if issuer_info.contains("let's encrypt") || issuer_info.contains("letsencrypt") {
        urls.push("https://letsencrypt.org/certificates/".to_string());
        urls.push("https://cert.int-x3.letsencrypt.org/".to_string());
        urls.push("https://letsencrypt.org/certs/isrgrootx1.pem".to_string());
    } else if issuer_info.contains("digicert") {
        urls.push("https://www.digicert.com/kb/digicert-root-certificates.htm".to_string());
        urls.push("https://cacerts.digicert.com/DigiCertGlobalRootCA.crt".to_string());
        urls.push("https://cacerts.digicert.com/DigiCertHighAssuranceEVRootCA.crt".to_string());
        urls.push("https://cacerts.digicert.com/DigiCertAssuredIDRootCA.crt".to_string());
    } else if issuer_info.contains("globalsign") {
        urls.push("https://www.globalsign.com/en/repository/".to_string());
        urls.push("https://secure.globalsign.com/cacert/root-r1.crt".to_string());
        urls.push("https://secure.globalsign.com/cacert/root-r3.crt".to_string());
        urls.push("https://secure.globalsign.com/cacert/gsorganizationvalsha2g2r1.crt".to_string());
    } else if issuer_info.contains("sectigo") || issuer_info.contains("comodo") {
        urls.push("https://www.sectigo.com/knowledge-base/detail/Sectigo-Intermediate-Certificates/kA03l000000vBXH".to_string());
        urls.push(
            "https://crt.sectigo.com/SectigoRSADomainValidationSecureServerCA.crt".to_string(),
        );
        urls.push(
            "https://crt.sectigo.com/SectigoRSAOrganizationValidationSecureServerCA.crt"
                .to_string(),
        );
        urls.push(
            "https://crt.sectigo.com/SectigoRSAExtendedValidationSecureServerCA.crt".to_string(),
        );
    } else if issuer_info.contains("amazon") || issuer_info.contains("aws") {
        urls.push("https://www.amazontrust.com/repository/".to_string());
        urls.push("https://www.amazontrust.com/repository/AmazonRootCA1.pem".to_string());
        urls.push("https://www.amazontrust.com/repository/AmazonRootCA2.pem".to_string());
        urls.push("https://www.amazontrust.com/repository/AmazonRootCA3.pem".to_string());
        urls.push("https://www.amazontrust.com/repository/AmazonRootCA4.pem".to_string());
    } else if issuer_info.contains("google") || issuer_info.contains("gts") {
        urls.push("https://pki.goog/repository/".to_string());
        urls.push("https://pki.goog/roots.pem".to_string());
        urls.push("https://pki.goog/gsr2/GTS1O1.crt".to_string());
        urls.push("https://pki.goog/gsr4/GTS1C3.crt".to_string());
    } else if issuer_info.contains("microsoft") {
        urls.push("https://www.microsoft.com/pki/mscorp/cps/default.htm".to_string());
        urls.push("https://www.microsoft.com/pkiops/certs/Microsoft%20RSA%20Root%20Certificate%20Authority%202017.crt".to_string());
        urls.push(
            "https://www.microsoft.com/pkiops/certs/MicRooCerAut2011_2011_03_22.crt".to_string(),
        );
    } else if issuer_info.contains("apple") {
        urls.push("https://www.apple.com/certificateauthority/".to_string());
        urls.push("https://www.apple.com/appleca/AppleIncRootCertificate.cer".to_string());
        urls.push("https://developer.apple.com/certificationauthority/AppleWWDRCA.cer".to_string());
    } else if issuer_info.contains("entrust") {
        urls.push("https://www.entrust.com/root-certificates/".to_string());
        urls.push("https://web.entrust.com/root-certificates/entrust_l1k.cer".to_string());
        urls.push("https://web.entrust.com/root-certificates/entrust_2048.cer".to_string());
    } else if issuer_info.contains("godaddy") {
        urls.push("https://certs.godaddy.com/repository/".to_string());
        urls.push("https://certificates.godaddy.com/repository/gd_bundle-g2-g1.crt".to_string());
    } else if issuer_info.contains("rapidssl") {
        urls.push("https://www.digicert.com/kb/digicert-root-certificates.htm".to_string());
        urls.push("https://cacerts.digicert.com/DigiCertGlobalRootCA.crt".to_string());
    } else if issuer_info.contains("thawte") {
        urls.push("https://www.digicert.com/kb/digicert-root-certificates.htm".to_string());
        urls.push("https://cacerts.digicert.com/ThawteRSACA2018.crt".to_string());
    } else if issuer_info.contains("geotrust") {
        urls.push("https://www.digicert.com/kb/digicert-root-certificates.htm".to_string());
        urls.push("https://cacerts.digicert.com/GeoTrustRSACA2018.crt".to_string());
    } else if issuer_info.contains("ssl.com") {
        urls.push(
            "https://www.ssl.com/how-to/ssl-com-root-and-intermediate-ca-certificate-downloads/"
                .to_string(),
        );
    } else if issuer_info.contains("buypass") {
        urls.push("https://www.buypass.com/ssl/resources/downloads".to_string());
    } else if issuer_info.contains("certum") {
        urls.push("https://www.certum.eu/certum/cert,offer_en_open-source_ca.xml".to_string());
    } else if issuer_info.contains("trustasia") {
        urls.push(
            "https://www.trustasia.com/knowledge-center/root-intermediate-certificates".to_string(),
        );
    } else if issuer_info.contains("zerossl") {
        urls.push("https://zerossl.com/downloads/".to_string());
    } else if issuer_info.contains("cloudflare") {
        urls.push(
            "https://developers.cloudflare.com/ssl/origin-configuration/origin-ca/".to_string(),
        );
    } else if issuer_info.contains("baltimore") {
        urls.push("https://cacerts.digicert.com/BaltimoreCyberTrustRoot.crt".to_string());
    } else if issuer_info.contains("identrust") {
        urls.push("https://www.identrust.com/certificates/trustid/root-download-x3".to_string());
    } else if issuer_info.contains("cfca") {
        urls.push("http://www.cfca.com.cn/".to_string());
    } else {
        // 通用CA证书资源
        urls.push("https://curl.se/docs/caextract.html".to_string());
        urls.push("https://wiki.mozilla.org/CA".to_string());
        urls.push(
            "https://ccadb-public.secure.force.com/mozilla/IncludedCACertificateReport".to_string(),
        );
    }

    urls
}
