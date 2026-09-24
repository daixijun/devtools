use std::net::IpAddr;

use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::stack::Stack;
use openssl::x509::extension::SubjectAlternativeName;
use openssl::x509::{X509Extension, X509NameBuilder, X509Req, X509ReqBuilder};

/// CSR 生成请求参数
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsrGenerateRequest {
    /// 通用名称（CN），通常为域名，必填
    pub common_name: String,
    /// 国家代码（C），两位字母，如 CN
    pub country: Option<String>,
    /// 省份 / 州（ST）
    pub state: Option<String>,
    /// 城市（L）
    pub locality: Option<String>,
    /// 组织 / 公司（O）
    pub organization: Option<String>,
    /// 部门（OU）
    pub org_unit: Option<String>,
    /// 邮箱地址
    pub email: Option<String>,
    /// 密钥类型："rsa" 或 "ec"
    pub key_type: String,
    /// RSA 密钥长度（位）：2048 / 3072 / 4096
    pub key_size: Option<u32>,
    /// ECC 曲线：P-256 / P-384 / P-521
    pub curve: Option<String>,
    /// SAN 列表：域名或 IP 地址（IP 自动识别）
    pub sans: Option<Vec<String>>,
}

/// 生成的 CSR 与配套密钥
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCsr {
    /// CSR 的 PEM 文本（-----BEGIN CERTIFICATE REQUEST-----）
    pub csr_pem: String,
    /// 配套私钥的 PKCS#8 PEM（-----BEGIN PRIVATE KEY-----），务必妥善保管
    pub private_key_pem: String,
    /// 公钥的 SPKI PEM（-----BEGIN PUBLIC KEY-----）
    pub public_key_pem: String,
    /// 密钥类型：RSA / ECC
    pub key_type: String,
    /// 密钥长度（位）
    pub key_size: Option<u32>,
    /// ECC 曲线名称（仅 ECC 密钥）
    pub curve_name: Option<String>,
    /// 主题的可读形式，如 "C=CN, O=DevTools, CN=example.com"
    pub subject: String,
    /// 实际写入 CSR 的 SAN 列表（"DNS:x" / "IP:y" 格式）
    pub sans: Vec<String>,
}

#[tauri::command]
pub async fn generate_csr(request: CsrGenerateRequest) -> Result<GeneratedCsr, String> {
    // RSA 4096 位密钥生成可能耗时数秒, 放到阻塞线程池避免占用异步运行时
    tokio::task::spawn_blocking(move || generate_csr_internal(request))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

fn generate_csr_internal(request: CsrGenerateRequest) -> Result<GeneratedCsr, String> {
    let common_name = request.common_name.trim().to_string();
    if common_name.is_empty() {
        return Err("通用名称（CN）不能为空".to_string());
    }

    let country = normalize_optional(&request.country);
    if let Some(code) = &country {
        if code.len() != 2 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(format!(
                "国家代码（C）应为两位字母（如 CN），当前为「{}」",
                code
            ));
        }
    }
    let state = normalize_optional(&request.state);
    let locality = normalize_optional(&request.locality);
    let organization = normalize_optional(&request.organization);
    let org_unit = normalize_optional(&request.org_unit);
    let email = normalize_optional(&request.email);

    // 1. 生成密钥对
    let (pkey, key_type, key_size, curve_name) =
        generate_key(&request.key_type, request.key_size, request.curve.as_deref())?;

    // 2. 组装主题（按 C/ST/L/O/OU/CN/email 的惯例顺序写入）
    let mut name_builder = X509NameBuilder::new().map_err(|e| format!("初始化主题失败: {}", e))?;
    append_subject_entry(&mut name_builder, "C", &country)?;
    append_subject_entry(&mut name_builder, "ST", &state)?;
    append_subject_entry(&mut name_builder, "L", &locality)?;
    append_subject_entry(&mut name_builder, "O", &organization)?;
    append_subject_entry(&mut name_builder, "OU", &org_unit)?;
    name_builder
        .append_entry_by_text("CN", &common_name)
        .map_err(|e| format!("写入主题属性 CN={} 失败: {}", common_name, e))?;
    append_subject_entry(&mut name_builder, "emailAddress", &email)?;
    let subject_name = name_builder.build();

    // 主题的可读形式（与写入顺序一致）
    let mut subject_parts: Vec<String> = Vec::new();
    for (text, value) in [
        ("C", &country),
        ("ST", &state),
        ("L", &locality),
        ("O", &organization),
        ("OU", &org_unit),
    ] {
        if let Some(v) = value {
            subject_parts.push(format!("{}={}", text, v));
        }
    }
    subject_parts.push(format!("CN={}", common_name));
    if let Some(v) = &email {
        subject_parts.push(format!("emailAddress={}", v));
    }
    let subject = subject_parts.join(", ");

    // 3. 归一化 SAN：去空白、去重（域名忽略大小写）
    let mut sans: Vec<String> = Vec::new();
    if let Some(list) = &request.sans {
        for item in list {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !sans.iter().any(|s| s.eq_ignore_ascii_case(trimmed)) {
                sans.push(trimmed.to_string());
            }
        }
    }

    // 4. 构建 CSR 并签名
    let mut req_builder = X509ReqBuilder::new().map_err(|e| format!("初始化 CSR 失败: {}", e))?;
    req_builder
        .set_subject_name(&subject_name)
        .map_err(|e| format!("设置主题失败: {}", e))?;
    req_builder
        .set_pubkey(&pkey)
        .map_err(|e| format!("设置公钥失败: {}", e))?;

    if !sans.is_empty() {
        let ctx = req_builder.x509v3_context(None);
        let mut san_builder = SubjectAlternativeName::new();
        for entry in &sans {
            if entry.parse::<IpAddr>().is_ok() {
                san_builder.ip(entry);
            } else {
                san_builder.dns(entry);
            }
        }
        let san = san_builder
            .build(&ctx)
            .map_err(|e| format!("构建 SAN 扩展失败: {}。请检查 SAN 条目格式", e))?;
        let mut extensions: Stack<X509Extension> =
            Stack::new().map_err(|e| format!("初始化扩展失败: {}", e))?;
        extensions
            .push(san)
            .map_err(|e| format!("添加 SAN 扩展失败: {}", e))?;
        req_builder
            .add_extensions(&extensions)
            .map_err(|e| format!("添加扩展失败: {}", e))?;
    }

    req_builder
        .sign(&pkey, MessageDigest::sha256())
        .map_err(|e| format!("CSR 签名失败: {}", e))?;
    let req = req_builder.build();

    // 生成后自检: 配套私钥必须能验证 CSR 签名。
    // OpenSSL 3.x 对内存中刚构建的请求直接 verify 会报 unsupported version,
    // 因此先转 DER 重新解析（与 CSR 查看器的解析路径一致）再验证
    let der = req.to_der().map_err(|e| format!("CSR 编码失败: {}", e))?;
    let req = X509Req::from_der(&der).map_err(|e| format!("CSR 编码异常: {}", e))?;
    if !req.verify(&pkey).unwrap_or(false) {
        return Err("生成的 CSR 签名自检未通过，请重试".to_string());
    }

    let csr_pem = pem_to_string(req.to_pem().map_err(|e| format!("导出 CSR 失败: {}", e))?)?;
    let private_key_pem = pem_to_string(
        pkey.private_key_to_pem_pkcs8()
            .map_err(|e| format!("导出 PKCS#8 私钥失败: {}", e))?,
    )?;
    let public_key_pem = pem_to_string(
        pkey.public_key_to_pem()
            .map_err(|e| format!("导出公钥失败: {}", e))?,
    )?;

    let san_display: Vec<String> = sans
        .iter()
        .map(|s| {
            if s.parse::<IpAddr>().is_ok() {
                format!("IP:{}", s)
            } else {
                format!("DNS:{}", s)
            }
        })
        .collect();

    Ok(GeneratedCsr {
        csr_pem,
        private_key_pem,
        public_key_pem,
        key_type,
        key_size,
        curve_name,
        subject,
        sans: san_display,
    })
}

/// 按参数生成密钥对，返回（私钥, 类型, 长度, 曲线名）
fn generate_key(
    key_type: &str,
    key_size: Option<u32>,
    curve: Option<&str>,
) -> Result<(PKey<Private>, String, Option<u32>, Option<String>), String> {
    match key_type {
        "rsa" => {
            let bits = key_size.unwrap_or(2048);
            if !matches!(bits, 2048 | 3072 | 4096) {
                return Err(format!(
                    "不支持的 RSA 密钥长度: {}。仅支持 2048、3072、4096 位",
                    bits
                ));
            }
            let rsa = Rsa::generate(bits).map_err(|e| format!("RSA 密钥生成失败: {}", e))?;
            let pkey = PKey::from_rsa(rsa).map_err(|e| format!("构建 RSA 密钥失败: {}", e))?;
            Ok((pkey, "RSA".to_string(), Some(bits), None))
        }
        "ec" => {
            let (nid, curve_name, bits) = match curve.unwrap_or("P-256") {
                "P-256" | "prime256v1" => (Nid::X9_62_PRIME256V1, "prime256v1", 256),
                "P-384" | "secp384r1" => (Nid::SECP384R1, "secp384r1", 384),
                "P-521" | "secp521r1" => (Nid::SECP521R1, "secp521r1", 521),
                other => {
                    return Err(format!(
                        "不支持的 ECC 曲线: {}。支持 P-256、P-384、P-521",
                        other
                    ))
                }
            };
            let group = EcGroup::from_curve_name(nid)
                .map_err(|e| format!("初始化 ECC 曲线失败: {}", e))?;
            let ec_key =
                EcKey::generate(&group).map_err(|e| format!("ECC 密钥生成失败: {}", e))?;
            let pkey = PKey::from_ec_key(ec_key).map_err(|e| format!("构建 ECC 密钥失败: {}", e))?;
            Ok((
                pkey,
                "ECC".to_string(),
                Some(bits),
                Some(curve_name.to_string()),
            ))
        }
        other => Err(format!("不支持的密钥类型: {}。仅支持 rsa / ec", other)),
    }
}

/// 写入单个主题属性（空值跳过）
fn append_subject_entry(
    builder: &mut X509NameBuilder,
    text: &str,
    value: &Option<String>,
) -> Result<(), String> {
    if let Some(v) = value {
        builder
            .append_entry_by_text(text, v)
            .map_err(|e| format!("写入主题属性 {}={} 失败: {}", text, v, e))?;
    }
    Ok(())
}

/// 去除首尾空白, 空字符串归一为 None
fn normalize_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn pem_to_string(pem: Vec<u8>) -> Result<String, String> {
    String::from_utf8(pem).map_err(|e| format!("PEM 编码异常: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::csr_parser::parse_csr;

    fn base_request(common_name: &str) -> CsrGenerateRequest {
        CsrGenerateRequest {
            common_name: common_name.to_string(),
            country: None,
            state: None,
            locality: None,
            organization: None,
            org_unit: None,
            email: None,
            key_type: "rsa".to_string(),
            key_size: Some(2048),
            curve: None,
            sans: None,
        }
    }

    #[test]
    fn test_generate_rsa_csr_with_san() {
        let mut request = base_request("api.example.com");
        request.country = Some("CN".to_string());
        request.state = Some("Shanghai".to_string());
        request.organization = Some("DevTools".to_string());
        request.email = Some("admin@example.com".to_string());
        request.sans = Some(vec![
            "api.example.com".to_string(),
            "10.0.0.1".to_string(),
            "  ".to_string(), // 空白条目应被忽略
        ]);

        let result = generate_csr_internal(request).unwrap();
        assert!(result.csr_pem.contains("BEGIN CERTIFICATE REQUEST"));
        assert!(result.private_key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(result.public_key_pem.contains("BEGIN PUBLIC KEY"));
        assert_eq!(result.key_type, "RSA");
        assert_eq!(result.key_size, Some(2048));
        assert!(result.curve_name.is_none());
        assert_eq!(
            result.subject,
            "C=CN, ST=Shanghai, O=DevTools, CN=api.example.com, emailAddress=admin@example.com"
        );
        assert_eq!(result.sans, vec!["DNS:api.example.com", "IP:10.0.0.1"]);

        // 与 CSR 查看器的解析管线联动验证（含私钥匹配）
        let info = parse_csr(
            Some(result.csr_pem.clone()),
            None,
            Some(result.private_key_pem.clone()),
        )
        .unwrap();
        assert!(info.signature_valid);
        assert_eq!(info.public_key.key_type, "RSA");
        assert_eq!(info.public_key.key_size, Some(2048));
        assert!(info.private_key.as_ref().expect("应包含私钥校验结果").matches_csr);

        let san = info
            .requested_extensions
            .iter()
            .find(|e| e.name == "Subject Alternative Name")
            .expect("应包含 SAN 扩展");
        assert!(san.value.contains("DNS:api.example.com"));
        assert!(san.value.contains("IP:10.0.0.1"));
    }

    #[test]
    fn test_generate_ec_csr() {
        let mut request = base_request("example.com");
        request.key_type = "ec".to_string();
        request.key_size = None;
        request.curve = Some("P-384".to_string());

        let result = generate_csr_internal(request).unwrap();
        assert_eq!(result.key_type, "ECC");
        assert_eq!(result.curve_name.as_deref(), Some("secp384r1"));
        assert_eq!(result.key_size, Some(384));
        assert_eq!(result.subject, "CN=example.com");
        assert!(result.sans.is_empty());

        let info = parse_csr(Some(result.csr_pem), None, None).unwrap();
        assert!(info.signature_valid);
        assert_eq!(info.public_key.key_type, "ECC");
        assert_eq!(info.public_key.curve_name.as_deref(), Some("secp384r1"));
        assert_eq!(info.signature_algorithm, "ECDSAWithSHA256");
    }

    #[test]
    fn test_san_dedup_ignoring_case() {
        let mut request = base_request("example.com");
        request.sans = Some(vec![
            "Example.COM".to_string(),
            "example.com".to_string(),
            "www.example.com".to_string(),
        ]);

        let result = generate_csr_internal(request).unwrap();
        assert_eq!(result.sans, vec!["DNS:Example.COM", "DNS:www.example.com"]);
    }

    #[test]
    fn test_invalid_requests() {
        // CN 为空
        assert!(generate_csr_internal(base_request("   ")).is_err());

        // 国家代码非法
        let mut request = base_request("example.com");
        request.country = Some("CHN".to_string());
        assert!(generate_csr_internal(request).is_err());

        // 密钥类型非法
        let mut request = base_request("example.com");
        request.key_type = "dsa".to_string();
        assert!(generate_csr_internal(request).is_err());

        // RSA 长度非法
        let mut request = base_request("example.com");
        request.key_size = Some(1024);
        assert!(generate_csr_internal(request).is_err());

        // ECC 曲线非法
        let mut request = base_request("example.com");
        request.key_type = "ec".to_string();
        request.key_size = None;
        request.curve = Some("P-999".to_string());
        assert!(generate_csr_internal(request).is_err());
    }

    #[test]
    fn test_serde_json_keys_are_camel_case() {
        let result = generate_csr_internal(base_request("example.com")).unwrap();
        let json = serde_json::to_value(&result).unwrap();

        assert!(json.get("csrPem").is_some());
        assert!(json.get("privateKeyPem").is_some());
        assert!(json.get("publicKeyPem").is_some());
        assert!(json.get("keyType").is_some());
        assert!(json.get("keySize").is_some());
        assert!(json.get("curveName").is_some());
        assert!(json.get("subject").is_some());
        assert!(json.get("sans").is_some());
    }
}
