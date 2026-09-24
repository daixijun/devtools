use crate::tools::cert_chain_utils::determine_chain_level;
use base64::{engine::general_purpose, Engine as _};
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
    pub order_check: Option<ChainOrderCheck>,
}

/// 证书链顺序检测中的单个证书条目
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainOrderEntry {
    /// 角色名称：终端证书 / 中间CA证书 / 根CA证书
    pub role: String,
    /// 主题 CN（缺省时为空），用于区分同角色的多张证书
    pub subject_cn: String,
    pub serial_number: String,
}

/// 用户输入的证书段落顺序检测结果。
/// 部署规范顺序为：终端证书 → 中间CA → 根CA（如 Nginx/Apache 的 fullchain 文件）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainOrderCheck {
    pub is_correct: bool,
    /// 按用户输入顺序排列
    pub actual_order: Vec<ChainOrderEntry>,
    /// 部署规范顺序
    pub expected_order: Vec<ChainOrderEntry>,
    /// 中文提示信息
    pub message: String,
    /// 顺序错误时提供按规范序重排后的完整 PEM 内容
    pub corrected_pem: Option<String>,
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

    let mut cert_ders = Vec::new();

    // 处理主证书
    if let Some(cert) = parsed.cert {
        let cert_der = cert.to_der().map_err(|e| format!("证书转换失败: {}", e))?;
        cert_ders.push(cert_der);
    }

    // 处理CA证书链
    if let Some(ca_certs) = parsed.ca {
        for cert in ca_certs {
            let cert_der = cert
                .to_der()
                .map_err(|e| format!("CA证书转换失败: {}", e))?;
            cert_ders.push(cert_der);
        }
    }

    if cert_ders.is_empty() {
        return Err("未在PFX文件中找到任何证书".to_string());
    }

    // 先解析全部证书，链级别判定需要完整的证书集合作为上下文
    // （X509Certificate 借用 DER 数据，cert_ders 需在证书使用期间保活）
    let x509_certs: Vec<X509Certificate> = cert_ders
        .iter()
        .enumerate()
        .map(|(index, der)| {
            X509Certificate::from_der(der)
                .map(|(_, cert)| cert)
                .map_err(|e| format!("第{}个证书解析失败: {}", index + 1, e))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let certificates = x509_certs
        .iter()
        .map(|cert| parse_certificate(cert, &x509_certs))
        .collect::<Result<Vec<_>, String>>()?;

    // 去重、分析证书链，并检测输入的段落顺序（顺序错误时生成修正后的 PEM）
    Ok(finalize_certificate_chain(&certificates, &cert_ders))
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

    // 先解析全部证书，链级别判定需要完整的证书集合作为上下文
    // （X509Certificate 借用 DER 数据，pem_contents 需在证书使用期间保活）
    let x509_certs: Vec<X509Certificate> = pem_contents
        .iter()
        .enumerate()
        .map(|(index, contents)| {
            X509Certificate::from_der(contents)
                .map(|(_, cert)| cert)
                .map_err(|e| format!("第{}个证书解析失败: {}", index + 1, e))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let certificates = x509_certs
        .iter()
        .map(|cert| parse_certificate(cert, &x509_certs))
        .collect::<Result<Vec<_>, String>>()?;

    // 去重、分析证书链，并检测输入的段落顺序（顺序错误时生成修正后的 PEM）
    Ok(finalize_certificate_chain(&certificates, &pem_contents))
}

fn parse_certificate(
    cert: &X509Certificate,
    all_certs: &[X509Certificate],
) -> Result<CertificateInfo, String> {
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

    // 确定证书链级别（需要完整的证书集合作为上下文）
    let chain_level = determine_chain_level(cert, all_certs);

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

// 证书链角色判定（determine_chain_level / determine_ca_chain_level / issuer_in_chain /
// has_key_cert_sign / is_self_signed / fallback_determine_chain_level）已迁移至共享模块
// tools::cert_chain_utils，由 CertificateViewer 与在线 SSL 检测工具共用同一套逻辑。

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

    // 检测用户输入的段落顺序（cert_infos 保存着重排前的输入顺序）
    let order_check = check_input_cert_order(cert_infos, &certificates);

    // 识别缺失的CA证书
    let missing_certificates = identify_missing_certificates(&certificates);

    if certificates.len() == 1 {
        // 只有一个证书，检查是否为根证书
        let cert = &certificates[0];
        if cert.chain_level == 2 {
            is_full_chain = true;
            if is_cert_self_signed(cert) {
                chain_status = "完整证书链：自签名根证书".to_string();
            } else {
                // 交叉签名形式的根证书（如 DigiCert Global Root G2 的交叉签名版本）
                chain_status = "根证书：交叉签名形式的根CA证书".to_string();
            }
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

    // 链顶根证书非自签名（交叉签名形式）时补充说明，
    // 例如使用 DigiCert Global Root G2 交叉签名证书 terminated 的完整链
    let chain_status = if is_full_chain
        && certificates
            .iter()
            .any(|cert| cert.chain_level == 2 && !is_cert_self_signed(cert))
    {
        format!(
            "{}（链顶根证书为交叉签名证书，其上级根未包含在文件中，通常不影响证书验证）",
            chain_status
        )
    } else {
        chain_status
    };

    CertificateChainInfo {
        certificates,
        missing_certificates,
        is_full_chain,
        chain_status,
        ca_download_urls,
        missing_ca_info,
        order_check,
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

    // 没有自签名根时，优先选择位于链顶的根 CA（如交叉签名根证书）：
    // 其发行者不匹配任何其他证书的主题
    if ordered.is_empty() {
        if let Some((index, root)) = remaining.iter().enumerate().find(|(_, cert)| {
            cert.chain_level == 2
                && !remaining.iter().any(|other| {
                    certificates_match_issuer_subject(&other.subject, &cert.issuer)
                })
        }) {
            ordered.push((*root).clone());
            remaining.remove(index);
        }
    }

    // 如果还没有找到根证书，从链级别最高的证书开始
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

/// 检查证书是否为自签名证书（subject 与 issuer 完整 DN 一致）
fn is_cert_self_signed(cert: &CertificateInfo) -> bool {
    // parse_name 为 subject/issuer 生成相同的键集合，
    // 因此 HashMap 整体相等即等价于完整 DN 的 DER 级比较，
    // 避免仅凭 CN 或 O 相同就误判为自签名
    !cert.subject.is_empty() && cert.subject == cert.issuer
}

/// 检查两个证书的颁发关系（证书A的subject是否与证书B的issuer一致）
/// RFC 5280 要求 issuer DN 与签发者的 subject DN 完全一致，
/// 因此这里做完整的映射相等比较，
/// 避免仅凭 CN/O/OU 单字段碰撞就判定颁发关系成立
fn certificates_match_issuer_subject(
    subject: &std::collections::HashMap<String, String>,
    issuer: &std::collections::HashMap<String, String>,
) -> bool {
    if issuer.is_empty() {
        return false;
    }

    // 至少要有一个非空字段参与比较
    let has_non_empty = issuer.values().any(|value| !value.is_empty());
    if !has_non_empty {
        return false;
    }

    subject == issuer
}

/// 去重并分析证书链，同时检测输入的段落顺序；
/// 顺序错误时按部署规范序（终端证书在前、根CA在最后）生成修正后的 PEM。
/// ders_in_input_order 需与 certificates_in_input_order 同序（提供原始 DER 用于重排）
fn finalize_certificate_chain(
    certificates_in_input_order: &[CertificateInfo],
    ders_in_input_order: &[Vec<u8>],
) -> CertificateChainInfo {
    let deduped = deduplicate_certificates(certificates_in_input_order);
    let mut analysis = analyze_certificate_chain(&deduped);

    if let Some(check) = analysis.order_check.as_mut() {
        if !check.is_correct {
            let der_by_serial: HashMap<&str, &Vec<u8>> = certificates_in_input_order
                .iter()
                .zip(ders_in_input_order.iter())
                .map(|(cert, der)| (cert.serial_number.as_str(), der))
                .collect();
            check.corrected_pem = assemble_corrected_pem(&check.expected_order, &der_by_serial);
        }
    }

    analysis
}

/// 按部署规范顺序将各证书的 DER 重新组装为标准 PEM 文本
fn assemble_corrected_pem(
    expected_order: &[ChainOrderEntry],
    der_by_serial: &HashMap<&str, &Vec<u8>>,
) -> Option<String> {
    let mut blocks = Vec::with_capacity(expected_order.len());
    for entry in expected_order {
        let der = der_by_serial.get(entry.serial_number.as_str())?;
        blocks.push(der_to_pem(der));
    }
    Some(blocks.join("\n"))
}

/// DER 编码的证书转标准 PEM 文本（Base64 按 64 字符换行）
fn der_to_pem(der: &[u8]) -> String {
    let encoded = general_purpose::STANDARD.encode(der);
    let mut pem = String::with_capacity(encoded.len() + encoded.len() / 64 + 64);
    pem.push_str("-----BEGIN CERTIFICATE-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).expect("base64 输出必为 UTF-8"));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----");
    pem
}

/// 证书链角色的中文名称（与前端展示文案一致）
fn certificate_role_name(chain_level: usize) -> &'static str {
    match chain_level {
        0 => "终端证书",
        1 => "中间CA证书",
        2 => "根CA证书",
        _ => "未知类型",
    }
}

/// 从证书信息生成顺序检测条目
fn chain_order_entry(cert: &CertificateInfo) -> ChainOrderEntry {
    let empty = String::new();
    let subject_cn = cert
        .subject
        .get("通用名称 (CN)")
        .or_else(|| cert.subject.get("组织名称 (O)"))
        .unwrap_or(&empty)
        .clone();
    ChainOrderEntry {
        role: certificate_role_name(cert.chain_level).to_string(),
        subject_cn,
        serial_number: cert.serial_number.clone(),
    }
}

/// 顺序条目的展示文本
fn format_order_entry(entry: &ChainOrderEntry) -> String {
    if entry.subject_cn.is_empty() {
        entry.role.clone()
    } else {
        format!("{}（{}）", entry.role, entry.subject_cn)
    }
}

fn format_order_entries(entries: &[ChainOrderEntry]) -> String {
    entries
        .iter()
        .map(format_order_entry)
        .collect::<Vec<_>>()
        .join(" → ")
}

/// 检测用户输入的证书段落顺序是否符合部署规范（终端证书在前、根CA证书在最后）。
/// canonical_root_first 为 build_certificate_chain 的输出（根在前），部署规范序即其逆序。
/// 少于 2 张证书时无需检测，返回 None
fn check_input_cert_order(
    input_order: &[CertificateInfo],
    canonical_root_first: &[CertificateInfo],
) -> Option<ChainOrderCheck> {
    if input_order.len() < 2 {
        return None;
    }

    let expected: Vec<ChainOrderEntry> = canonical_root_first
        .iter()
        .rev()
        .map(chain_order_entry)
        .collect();
    let actual: Vec<ChainOrderEntry> = input_order.iter().map(chain_order_entry).collect();

    // 仅当规范序中相邻证书的位置可被唯一确定（层级递增或存在真实颁发关系）时
    // 才做严格的逐位比较；断裂链等无法确定同级先后顺序的场景回退为宽松判定：
    // 输入的层级序列非降即视为正确，避免误报
    let order_unambiguous = canonical_root_first.windows(2).all(|pair| {
        pair[1].chain_level > pair[0].chain_level
            || certificates_match_issuer_subject(&pair[0].subject, &pair[1].issuer)
    });

    let is_correct = if order_unambiguous {
        input_order
            .iter()
            .zip(expected.iter())
            .all(|(cert, entry)| cert.serial_number == entry.serial_number)
    } else {
        input_order
            .windows(2)
            .all(|pair| pair[1].chain_level >= pair[0].chain_level)
    };

    let message = if is_correct {
        "证书链顺序符合部署规范：终端证书在前，根CA证书在最后".to_string()
    } else {
        format!(
            "证书链顺序错误：当前文件中证书排列为「{}」，正确顺序应为「{}」。\
服务器部署（如 Nginx/Apache 的 fullchain 文件）要求终端证书在最前、根CA证书在最后，请调整各证书段落的先后顺序。",
            format_order_entries(&actual),
            format_order_entries(&expected)
        )
    };

    Some(ChainOrderCheck {
        is_correct,
        actual_order: actual,
        expected_order: expected,
        message,
        corrected_pem: None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 测试 fixture：openssl 生成的自建测试链，无敏感信息 =====
    // 模拟 DigiCert G2 交叉签名场景：
    //   root（自签名根，CA:TRUE 无 pathlen）
    //   └─ cross_g2（subject=Test Root G2，由 root 交叉签发，CA:TRUE 无 pathlen —— 交叉签名根形态）
    //       └─ inter_g2（G2 中间 CA，CA:TRUE pathlen:0）
    //           └─ leaf（终端证书，CA:FALSE）
    // 另有 inter（由 root 直接签发的普通中间 CA，pathlen:0）用于不完整链场景
    const ROOT_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDYzCCAkugAwIBAgIUaorjagVzzwX3RWb9XZAteER1spkwDQYJKoZIhvcNAQEL\n",
        "BQAwOTEeMBwGA1UEAwwVRGV2VG9vbHMgVGVzdCBSb290IENBMRcwFQYDVQQKDA5E\n",
        "ZXZUb29scyBUZXN0czAeFw0yNjA5MTQxNjAyNDVaFw0zNjA5MTExNjAyNDVaMDkx\n",
        "HjAcBgNVBAMMFURldlRvb2xzIFRlc3QgUm9vdCBDQTEXMBUGA1UECgwORGV2VG9v\n",
        "bHMgVGVzdHMwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDDN4srjevC\n",
        "/3OuJq9kfAb+8kOW4Dll42tjIcDyFKsdec3iLHGANT7j7sIfLwl6h4/+lmh74MmN\n",
        "2BrLdKiFixNO2bUuKKE1MSWesbxPhYrz0IeQMd/W5L33Px8KFUtK1dpDjhn0SftY\n",
        "W9YuHf3HMprqHZ00jhUkDRy1r4LW2ekQDhXyqFWkg+PPP24N5+RPxgRdUkO3Uzuh\n",
        "snCPwVckPNmg/xcp6GuSiHwvcsSfek8xejblG4W6aICDJUoFbABBbRoG108W+yEs\n",
        "ygfbhYC06Q5KYUGZhCkUDfWk70dEe3ZdhJ3hWk8kLLDuXLuJpn86d3hckahfOqMs\n",
        "CZlBHRvcurqzAgMBAAGjYzBhMB0GA1UdDgQWBBTBfjvUf1q0bzc+dLSsM17qKgpH\n",
        "qzAfBgNVHSMEGDAWgBTBfjvUf1q0bzc+dLSsM17qKgpHqzAPBgNVHRMBAf8EBTAD\n",
        "AQH/MA4GA1UdDwEB/wQEAwIBBjANBgkqhkiG9w0BAQsFAAOCAQEAf7Oo0usIdD4o\n",
        "vc9+juh+D+5INtKYVllNa5TMQV4/LyIVJDN9VCnYJw7PyGu2JYxBpYJAip9cX7eB\n",
        "BBHXVdG2uK9nh3WxuzfS9h0aIqCgyaU7Quox2VTntZE8zgOH230/rQ9U6YwRH8n3\n",
        "Ga9RnTPEnQ9vqIyFxaIegZhNLPPZCseSY630yvb+Eh9RlHKvwjRReY5nBWew0MNg\n",
        "oZKH871PBkJK9gqxYdznBhncjCECo59O6fT/rGYvEp1RONxMhk+aFIRLKe4Ocyg/\n",
        "Ygszss8DlbNn+oyyob4PiC8JuXF1LVpkKIoHBLshDJiv/VR8b+Ro+mSu42hRzrsC\n",
        "6rAe9beXnw==\n",
        "-----END CERTIFICATE-----\n",
    );

    const INTER_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDbjCCAlagAwIBAgIUVHLGHhuV2WTtsBHq2InF3CnIWdswDQYJKoZIhvcNAQEL\n",
        "BQAwOTEeMBwGA1UEAwwVRGV2VG9vbHMgVGVzdCBSb290IENBMRcwFQYDVQQKDA5E\n",
        "ZXZUb29scyBUZXN0czAeFw0yNjA5MTQxNjAyNDVaFw0zNjA5MTExNjAyNDVaMEEx\n",
        "JjAkBgNVBAMMHURldlRvb2xzIFRlc3QgSW50ZXJtZWRpYXRlIENBMRcwFQYDVQQK\n",
        "DA5EZXZUb29scyBUZXN0czCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEB\n",
        "AO0jUicO8mn1Q3hEPw+9cqcd0wOYJqCXkQmIIk9VmKBDDS8gHSbue5/xUStpsSo8\n",
        "ydCyN9BUAvEBH/+THRQfyIwxHjbFYLrmoQUMJtL9Qi31+wIAFCGrtk1p7GJ5S0T7\n",
        "4f/UtUKcD8kcigbp4D2G+KgbINv7F3kXTBPVJ+gGAR/5DsB2cmV8Xp6bkjFpYRfN\n",
        "+xgM0qZCLBL2R0oO7QkMkIUr7js5mDUXuiPrixBmnJ6U737QmpC0W+Qvc00JthXt\n",
        "V5ZAnqIW086wchV2/EEapfQv9rwqLL7KfRJNCzdyEBNhOqGjh/sRr502/iYDAXrs\n",
        "SNDuPankdulgEf8/X9R/8UECAwEAAaNmMGQwEgYDVR0TAQH/BAgwBgEB/wIBADAO\n",
        "BgNVHQ8BAf8EBAMCAgQwHQYDVR0OBBYEFI+TDGm+EtydGGF5rLdA3BmupAFdMB8G\n",
        "A1UdIwQYMBaAFMF+O9R/WrRvNz50tKwzXuoqCkerMA0GCSqGSIb3DQEBCwUAA4IB\n",
        "AQC4xjPgoGMHXQQ+urlJRWjZg7I0h93q7ZBjSPdmJ34MhuknQUuqiT/Sde0ouWLe\n",
        "5rscMIET0UyN5tW/kWjCeWsmsxwiiqJkSVQc09pN8Drrdn4ZpGLGpgoF4lKSqFNj\n",
        "+yuvYkZM62Eo//N3zy8RZhcwJqi9JUYi8CysbFdALMb3JyAkhBW5SYhqPQY56WKF\n",
        "sYEpbnDg4AdXk9HXcehT93QmQBWLFNG4pkXquPkF/tDI/fFBTce7zr6P75F46/Sp\n",
        "1f1obBcaFumOKty8SNhQo/voXWfTLtB0kyUcGd3asIHdX9z9muFNu5RHzC1wmT2Y\n",
        "9S4orCmX5uLNa2IqQOjXOfvO\n",
        "-----END CERTIFICATE-----\n",
    );

    const LEAF_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDXDCCAkSgAwIBAgIUFhAHh2dK/iIyWiEEIUtme0Tl9SQwDQYJKoZIhvcNAQEL\n",
        "BQAwQTEmMCQGA1UEAwwdRGV2VG9vbHMgVGVzdCBJbnRlcm1lZGlhdGUgQ0ExFzAV\n",
        "BgNVBAoMDkRldlRvb2xzIFRlc3RzMB4XDTI2MDkxNDE2MDI0NVoXDTM2MDkxMTE2\n",
        "MDI0NVowMDEVMBMGA1UEAwwMZXhhbXBsZS50ZXN0MRcwFQYDVQQKDA5EZXZUb29s\n",
        "cyBUZXN0czCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAKtcxnfpYiq2\n",
        "GZOMVhLaTFuYA++ikzZktRkPOLkBZq2TW73+4FKMMSm0z7Afgh6qyqKE9dCOUqs9\n",
        "RSvBeQaU7/2kMT1SutM9rMMF0Js1MFC0KEkQwuE2BJQmnQURmJNlRqqdyW+GHYhT\n",
        "fL6HbHvoNz7sYAtaq0Fg0GRRXfj6DdtT2v3Jg0bPMuaz7boL7xKsYJHc3z1lEu76\n",
        "ILyXbJmdBrZeKJVM10RHdrF5TRlEC260XOt8rjQWuEVPrcKyw8HcfXhLJhX+rW3u\n",
        "jzDfCtB5w/mXYvaT6BxxNxu3G+oWuFRQ5YumCkrvxN3JgDRldS6pamn3xS6+d2Fb\n",
        "PJU6iVbwk88CAwEAAaNdMFswDAYDVR0TAQH/BAIwADALBgNVHQ8EBAMCBaAwHQYD\n",
        "VR0OBBYEFOpUedRyWD3MIhPSakthygYBbsfYMB8GA1UdIwQYMBaAFI+TDGm+Etyd\n",
        "GGF5rLdA3BmupAFdMA0GCSqGSIb3DQEBCwUAA4IBAQBHG48j0/hBIdgY3PSMxEp1\n",
        "uMqFnX8Wb8Z/Nu/cDCfuOE9YkALEbf7N2NPP+0G65VkimQGHGv81IY7NNhAc9gE+\n",
        "aD1k1M2qIHTWwqZXrIA2jRt/JXKwAyQvRaLGHYjpIGkhBr+kQbNBoOIl3aU8ntU6\n",
        "W0lXtXvoD58pBu09FC79Ar8nZJB7V2Z2+X9MSdG9Q4k1E7JVRTYpa/E8GtGqYLGk\n",
        "8RIEqIw9QG/ONN8XGdadiDNrEF+C70p39CTG2qXCtcK80Z0KVUbyVaw+IHGRJFWp\n",
        "FHIPbuh4VTcFgt9knZP4bLu2NU0jrJ+oFdgd6PV7MwebVK/5EdHxriNH645hEqJR\n",
        "-----END CERTIFICATE-----\n",
    );

    const CROSS_G2_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDYzCCAkugAwIBAgIUVHLGHhuV2WTtsBHq2InF3CnIWdwwDQYJKoZIhvcNAQEL\n",
        "BQAwOTEeMBwGA1UEAwwVRGV2VG9vbHMgVGVzdCBSb290IENBMRcwFQYDVQQKDA5E\n",
        "ZXZUb29scyBUZXN0czAeFw0yNjA5MTQxNjAyNDVaFw0zNjA5MTExNjAyNDVaMDkx\n",
        "HjAcBgNVBAMMFURldlRvb2xzIFRlc3QgUm9vdCBHMjEXMBUGA1UECgwORGV2VG9v\n",
        "bHMgVGVzdHMwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCpaBtrtXnn\n",
        "rYIHO4CDBSCWHkdglRcJatWMJ8rft/PUbqVD+HcLk6P9GAsFKVM84OEmmbyPG3Xc\n",
        "5DOGS2QH5okrk9SxF5CueockQU3IwQnMWluU5gOd/nHyBnFu2Dzjvssdu1sWMkvW\n",
        "H2517uFwFMnZg0CsXDXT4V7IpNBMXJdHIR6iwywOGfsgb4Djx2UtRKrzL2Hi3Tk8\n",
        "JBle28X+AlQOnHGtt3KFywal06TCVA1XDxZLqTjIp1n3BjrtM48WSFkzB7DI2HFl\n",
        "vbrlxTj5Zm8ydhM7/u+GdHpRDoD1Wz09wx6a/hIYfyf4ucw81K/ukWjUXyw/LoK6\n",
        "rnxMz0HZw1NZAgMBAAGjYzBhMA8GA1UdEwEB/wQFMAMBAf8wDgYDVR0PAQH/BAQD\n",
        "AgIEMB0GA1UdDgQWBBRMpzGoHsiPdJFQkbtw3l1MFfYKhzAfBgNVHSMEGDAWgBTB\n",
        "fjvUf1q0bzc+dLSsM17qKgpHqzANBgkqhkiG9w0BAQsFAAOCAQEANWkuudkEioZH\n",
        "cbeXxRL0kkVk+Ztr/g6cpEOuS7rqUx1BWtlcJrR9nB3kmlQc2CvMuUIU0V4cgJSB\n",
        "OyU1YjB7Vp9ZZKVY33YtccRAdk1CCrdCeU76l/VwU+wfsQXLsSt44gzw8/nlWr/m\n",
        "endWnuFufc4nmIugNY8NFJhC6So4xFVY4XpbwdZnfJE+VjFMGWoYBqWMiFMfc9bd\n",
        "YqPQe5l4j7PzM9LFXvtM4VG1GRZQI+qhqxISzO+EFu0OZ8vupiZwjyf9lDfl/AwU\n",
        "m0qa+imEzoDLL9VYxJHpVh4/pzvJ8jSOi/9eq7Ohk6hkMN0WDbZ+BeFgwqlzByz4\n",
        "HedvBTY+Eg==\n",
        "-----END CERTIFICATE-----\n",
    );

    const INTER_G2_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDaDCCAlCgAwIBAgIUQ4paTG2QIq5V/73op0v2HfL2+jcwDQYJKoZIhvcNAQEL\n",
        "BQAwOTEeMBwGA1UEAwwVRGV2VG9vbHMgVGVzdCBSb290IEcyMRcwFQYDVQQKDA5E\n",
        "ZXZUb29scyBUZXN0czAeFw0yNjA5MTQxNjAyNDVaFw0zNjA5MTExNjAyNDVaMDsx\n",
        "IDAeBgNVBAMMF0RldlRvb2xzIFRlc3QgRzIgVExTIENBMRcwFQYDVQQKDA5EZXZU\n",
        "b29scyBUZXN0czCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALSrIu7U\n",
        "HURJ6ewNRYNr3DgCg0rTQOMpD5EYrJUulCkDdmZcdTSt61yuOcvVj74Tcfh8eR/z\n",
        "xZridOzat9nIO58pFI+RrxyfwQ1O5jKFIXAoSvY/LGHdZHqGK4opADkKMFLiwRrB\n",
        "5JULrdNIGDdl1e8sxbomgH+yN4VahZmSQQqRc0WFpEFcrzL9HZ5kOTER9gHa6jsA\n",
        "DQ3jG7Gb42kfTFn0jR+DBWtG05BAPSVmV+9FOLVpICOonvBrqwh04gTKb8JBVKq2\n",
        "rj2Cj2HJKYU8RXIC7I86r6w+IaE7GSWRa6ACRRO6gKsdyjHOWiqtjugRAM6G91rT\n",
        "6ems9nUqDr8nlosCAwEAAaNmMGQwEgYDVR0TAQH/BAgwBgEB/wIBADAOBgNVHQ8B\n",
        "Af8EBAMCAgQwHQYDVR0OBBYEFF11bajCkCiUl8ABPRZC5Ku4/gd+MB8GA1UdIwQY\n",
        "MBaAFEynMageyI90kVCRu3DeXUwV9gqHMA0GCSqGSIb3DQEBCwUAA4IBAQA6lueC\n",
        "EMCjRdcFy7/9ntf302k0CY1CJriXvt5AwsQ1tynwsVJJaIywCpOhNUxrTz32ShqM\n",
        "vcclBE4tSo0bQ1lGeyVIBM+b+Qmbskcs/KSM01dSMWu7K5DKYKkHS3w63riP1RAU\n",
        "BzQmKM9uwyoS+7Viw26w3q6LiJazV2XKRSwo5VmJJxUW5GUCADxDFM072TJ6vI8y\n",
        "ohOWkLIH9xuX6quEcWa4I+XXohjw2QlPYiUfixV17DR5lIBzECn3egtyJmEWGnoy\n",
        "RTQ0zaB+lTnOa3Jk/NGag/PGZqLaKIg57T0n1gMix86M/f+oa7w7AZAktUUp5ZqI\n",
        "jvBaCR4Ty3T906Oz\n",
        "-----END CERTIFICATE-----\n",
    );

    const LEAF_G2_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDWTCCAkGgAwIBAgIUE6i821EpSouofXrG7aIXcieWvlkwDQYJKoZIhvcNAQEL\n",
        "BQAwOzEgMB4GA1UEAwwXRGV2VG9vbHMgVGVzdCBHMiBUTFMgQ0ExFzAVBgNVBAoM\n",
        "DkRldlRvb2xzIFRlc3RzMB4XDTI2MDkxNDE2MjEwNFoXDTM2MDkxMTE2MjEwNFow\n",
        "MzEYMBYGA1UEAwwPZXhhbXBsZS1nMi50ZXN0MRcwFQYDVQQKDA5EZXZUb29scyBU\n",
        "ZXN0czCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAJrbAzyU48qDf3Uv\n",
        "zU9d23tJ1pyVuF3WBMGw8dTM0s3PEwxt2+3JIrM0tCnyAYlUsg3+Hp0uOVfNTNqz\n",
        "TFkEx2pTjAVhACT3kdEiZGKvBRDh6QIHj2tEqT2Q7yPYbH0ZxEScfoBXaRunqJ97\n",
        "Gx8i9gBR4GEbzHvbjUrY7W4xSRCDi5kH/lWRs5xs8Vfbxqa5VshaVURp0T/AkjR/\n",
        "ENy8BkT/MkYAJZwLUXmkGnnQBy9Faxl99r/XNQqS/1eB+pgl6CTTW/Iflean6IYf\n",
        "i5aFZfou1KlF3WXlN5r54gKagIowWFII5HpGsSRI3MrpCG8X92JYlkmxnL2air0X\n",
        "4venEC8CAwEAAaNdMFswDAYDVR0TAQH/BAIwADALBgNVHQ8EBAMCBaAwHQYDVR0O\n",
        "BBYEFOfGh5j1ynaWK1NKFUPkhab2xYraMB8GA1UdIwQYMBaAFF11bajCkCiUl8AB\n",
        "PRZC5Ku4/gd+MA0GCSqGSIb3DQEBCwUAA4IBAQApelUuQHGx3Qa8T0dleOuy6KDx\n",
        "dTNv/Gz1Isk0+inUE97qZZE4twLGLU0jMMlpRhDKS5e9Yg+6WWTnCBAeEHQOU+nM\n",
        "Sblskhow7kro0+Su1Hrc05p/LU2R8xT95Pp0ATqPX/Z4IodzYwwIO4bcd94K+xJf\n",
        "KU/h/hUOCmDxdOL+sQvWTKmZy5TI1z4YHFWb5TLKT/Afegf/sdxAXpIKGVej2wOO\n",
        "Yhfplzbo2g+Yok56DmQCHb+O8IBvEDz0SFZnjy4Do109Re1H/Iz4EVTYArn6m6Hm\n",
        "rMGU5QtKR24u3AsBrgs9Q2Z2PJQvNXfycKB4bME4jtXIgh2UDvyJswu7fUJK\n",
        "-----END CERTIFICATE-----\n",
    );

    /// 解析单个 PEM 证书。DER 数据被故意泄漏（'static），
    /// 以绕过 X509Certificate 对 DER 缓冲区的借用约束，仅用于测试
    fn fixture_cert(pem_text: &str) -> X509Certificate<'static> {
        let (pem, _) = Pem::read(std::io::Cursor::new(pem_text)).expect("PEM 解析失败");
        let der: &'static [u8] = Box::leak(pem.contents.into_boxed_slice());
        X509Certificate::from_der(der)
            .expect("证书解析失败")
            .1
    }

    fn parse_chain(pems: &[&str]) -> CertificateChainInfo {
        let combined = pems.concat();
        parse_pem_certificate(combined).expect("证书链解析失败")
    }

    fn level_of_cn(chain: &CertificateChainInfo, cn: &str) -> usize {
        chain
            .certificates
            .iter()
            .find(|cert| {
                cert.subject
                    .get("通用名称 (CN)")
                    .map(|value| value == cn)
                    .unwrap_or(false)
            })
            .expect("证书链中未找到指定证书")
            .chain_level
    }

    #[test]
    fn test_cross_signed_root_chain_is_complete() {
        // 回归场景：leaf + G2中间CA + 交叉签名根，不应再误报"缺少根CA证书"
        let chain = parse_chain(&[LEAF_G2_PEM, INTER_G2_PEM, CROSS_G2_PEM]);

        assert_eq!(chain.certificates.len(), 3);
        assert_eq!(level_of_cn(&chain, "example-g2.test"), 0);
        assert_eq!(level_of_cn(&chain, "DevTools Test G2 TLS CA"), 1);
        assert_eq!(level_of_cn(&chain, "DevTools Test Root G2"), 2);

        assert!(chain.is_full_chain, "chain_status: {}", chain.chain_status);
        assert!(chain.chain_status.contains("完整证书链"));
        assert!(chain.chain_status.contains("交叉签名"));
        assert!(chain.missing_certificates.is_empty());
        // 排序后链顶应为根 CA
        assert_eq!(chain.certificates[0].chain_level, 2);
    }

    #[test]
    fn test_input_order_leaf_first_is_correct() {
        // 终端 → 中间 → 根：符合部署规范（Nginx/Apache fullchain 顺序）
        let chain = parse_chain(&[LEAF_PEM, INTER_PEM, ROOT_PEM]);

        let check = chain.order_check.expect("应返回顺序检测结果");
        assert!(check.is_correct, "message: {}", check.message);
        assert!(check.corrected_pem.is_none());
        assert_eq!(check.actual_order[0].role, "终端证书");
        assert_eq!(check.expected_order[0].role, "终端证书");
        // 顺序检测不影响链完整性判定
        assert!(chain.is_full_chain, "chain_status: {}", chain.chain_status);
    }

    #[test]
    fn test_input_order_reversed_reports_error_and_correction() {
        // 根 → 中间 → 终端：三段位置全部放错，应提示并给出修正后的 PEM
        let chain = parse_chain(&[ROOT_PEM, INTER_PEM, LEAF_PEM]);

        let check = chain.order_check.expect("应返回顺序检测结果");
        assert!(!check.is_correct);
        assert!(check.message.contains("证书链顺序错误"), "{}", check.message);
        assert_eq!(check.actual_order[0].role, "根CA证书");
        assert_eq!(check.expected_order[0].role, "终端证书");

        // 修正后的 PEM 应包含全部 3 张证书
        let corrected = check.corrected_pem.expect("应提供修正后的 PEM");
        assert_eq!(
            corrected.matches("-----BEGIN CERTIFICATE-----").count(),
            3
        );

        // 用修正后的 PEM 重新解析，顺序检测应通过
        let reparsed = parse_pem_certificate(corrected).expect("修正后的 PEM 应可解析");
        assert_eq!(reparsed.certificates.len(), 3);
        let reparsed_check = reparsed.order_check.expect("应返回顺序检测结果");
        assert!(
            reparsed_check.is_correct,
            "message: {}",
            reparsed_check.message
        );
    }

    #[test]
    fn test_input_order_middle_swapped_reports_error() {
        // 终端 → 根 → 中间：根与中间CA位置放错
        let chain = parse_chain(&[LEAF_PEM, ROOT_PEM, INTER_PEM]);

        let check = chain.order_check.expect("应返回顺序检测结果");
        assert!(!check.is_correct);
        assert!(check.corrected_pem.is_some());
    }

    #[test]
    fn test_cross_signed_chain_leaf_first_order_correct() {
        // 交叉签名根场景下 leaf-first 输入同样应判定为顺序正确
        let chain = parse_chain(&[LEAF_G2_PEM, INTER_G2_PEM, CROSS_G2_PEM]);

        let check = chain.order_check.expect("应返回顺序检测结果");
        assert!(check.is_correct, "message: {}", check.message);
    }

    #[test]
    fn test_broken_chain_falls_back_to_lenient_level_check() {
        // 链断裂（缺少中间CA）：leaf → root 输入层级非降，不应误报顺序错误
        let chain = parse_chain(&[LEAF_PEM, ROOT_PEM]);

        let check = chain.order_check.expect("应返回顺序检测结果");
        assert!(check.is_correct, "message: {}", check.message);
    }

    #[test]
    fn test_single_cert_has_no_order_check() {
        let chain = parse_chain(&[ROOT_PEM]);
        assert!(chain.order_check.is_none());
    }

    #[test]
    fn test_incomplete_chain_with_pathlen_intermediate_still_warns() {
        // leaf + 带 pathlen 的中间CA（发行者不在链中）仍应提示缺少根CA
        let chain = parse_chain(&[LEAF_PEM, INTER_PEM]);

        assert!(!chain.is_full_chain);
        assert!(chain.chain_status.contains("缺少根CA证书"));
        assert!(!chain.missing_certificates.is_empty());
    }

    #[test]
    fn test_self_signed_root_single_cert() {
        let chain = parse_chain(&[ROOT_PEM]);

        assert_eq!(chain.certificates[0].chain_level, 2);
        assert!(chain.is_full_chain);
        assert!(chain.chain_status.contains("自签名根证书"));
    }

    #[test]
    fn test_cross_signed_root_alone_is_root() {
        let chain = parse_chain(&[CROSS_G2_PEM]);

        assert_eq!(chain.certificates[0].chain_level, 2);
        assert!(chain.is_full_chain);
        assert!(chain.chain_status.contains("交叉签名"));
    }

    #[test]
    fn test_determine_chain_level_depends_on_context() {
        let all = vec![
            fixture_cert(ROOT_PEM),
            fixture_cert(INTER_PEM),
            fixture_cert(CROSS_G2_PEM),
            fixture_cert(LEAF_PEM),
        ];
        let (root, inter, cross_g2, leaf) = (&all[0], &all[1], &all[2], &all[3]);

        // 单张证书的上下文
        assert_eq!(determine_chain_level(root, std::slice::from_ref(root)), 2);

        // 交叉签名根单独出现：链顶 + 无 pathlen => 根 CA
        assert_eq!(
            determine_chain_level(cross_g2, std::slice::from_ref(cross_g2)),
            2
        );

        // 带 pathlen 的链顶 CA => 中间 CA（提示缺少上级）
        assert_eq!(determine_chain_level(inter, std::slice::from_ref(inter)), 1);

        assert_eq!(determine_chain_level(leaf, std::slice::from_ref(leaf)), 0);

        // 完整上下文中：发行者存在时交叉签名根是"中间 CA"
        assert_eq!(determine_chain_level(root, &all), 2);
        assert_eq!(determine_chain_level(cross_g2, &all), 1);
        assert_eq!(determine_chain_level(inter, &all), 1);
        assert_eq!(determine_chain_level(leaf, &all), 0);
    }

    fn cert_info(subject: HashMap<String, String>, issuer: HashMap<String, String>) -> CertificateInfo {
        CertificateInfo {
            subject,
            issuer,
            validity: ValidityInfo {
                not_before: String::new(),
                not_after: String::new(),
                days_until_expiry: 0,
            },
            serial_number: "01".to_string(),
            signature_algorithm: "SHA256WithRSA".to_string(),
            public_key_info: PublicKeyInfo {
                key_type: "RSA".to_string(),
                key_size: Some(2048),
                algorithm: "rsaEncryption".to_string(),
            },
            extensions: Vec::new(),
            sans: Vec::new(),
            chain_level: 0,
            certificate_type: None,
            brand: None,
            sha1_fingerprint: None,
            sha256_fingerprint: None,
        }
    }

    fn name_map(fields: &[(&str, &str)]) -> HashMap<String, String> {
        fields
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn test_is_cert_self_signed_requires_full_dn() {
        // subject == issuer（完整 DN 一致）=> 自签名
        let identical = name_map(&[("通用名称 (CN)", "Test Root"), ("组织名称 (O)", "Test Org")]);
        assert!(is_cert_self_signed(&cert_info(identical.clone(), identical)));

        // 仅 O 相同（CN 不同）=> 不再误判为自签名
        let subject = name_map(&[("通用名称 (CN)", "Test Root"), ("组织名称 (O)", "Test Org")]);
        let issuer = name_map(&[("通用名称 (CN)", "Test Issuer"), ("组织名称 (O)", "Test Org")]);
        assert!(!is_cert_self_signed(&cert_info(subject, issuer)));
    }

    #[test]
    fn test_match_issuer_subject_requires_full_dn() {
        let subject = name_map(&[
            ("通用名称 (CN)", "DigiCert Global Root G2"),
            ("组织名称 (O)", "DigiCert Inc"),
            ("组织单位 (OU)", "www.digicert.com"),
        ]);

        // 完整 DN 一致 => 匹配
        let issuer = name_map(&[
            ("通用名称 (CN)", "DigiCert Global Root G2"),
            ("组织名称 (O)", "DigiCert Inc"),
            ("组织单位 (OU)", "www.digicert.com"),
        ]);
        assert!(certificates_match_issuer_subject(&subject, &issuer));

        // 仅 O 碰撞 => 不匹配
        let o_collision = name_map(&[("通用名称 (CN)", "Other CA"), ("组织名称 (O)", "DigiCert Inc")]);
        assert!(!certificates_match_issuer_subject(&subject, &o_collision));

        // 仅 OU 碰撞 => 不匹配
        let ou_collision = name_map(&[
            ("通用名称 (CN)", "Other CA"),
            ("组织单位 (OU)", "www.digicert.com"),
        ]);
        assert!(!certificates_match_issuer_subject(&subject, &ou_collision));

        // issuer 字段在 subject 中缺失 => 不匹配
        let missing_field = name_map(&[("通用名称 (CN)", "DigiCert Global Root G2")]);
        assert!(!certificates_match_issuer_subject(&subject, &missing_field));

        // 空 issuer => 不匹配
        assert!(!certificates_match_issuer_subject(
            &subject,
            &HashMap::new()
        ));
    }
}
