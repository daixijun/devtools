use openssl::pkcs12::Pkcs12;
use openssl::pkey::{PKey, Private};
use openssl::x509::X509;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PfxConversionResult {
    pub certificates: Vec<String>,
    pub private_keys: Vec<String>,
    pub combined_pem: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PemToPfxResult {
    pub pfx_data: Vec<u8>,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn convert_pfx_to_pem(
    pfx_data: Vec<u8>,
    password: Option<String>,
) -> Result<PfxConversionResult, String> {
    convert_pfx_internal(pfx_data, password)
}

fn convert_pfx_internal(
    pfx_data: Vec<u8>,
    password: Option<String>,
) -> Result<PfxConversionResult, String> {
    if pfx_data.is_empty() {
        return Err("PFX文件内容不能为空".to_string());
    }

    let password_str = password.as_deref().unwrap_or("");

    // 尝试解析PFX文件
    let pkcs12 = Pkcs12::from_der(&pfx_data)
        .map_err(|e| format!("无效的PFX文件格式: {}。请确保上传的是有效的PFX/P12文件", e))?;

    // 解析PFX内容
    let parsed = pkcs12
        .parse2(password_str)
        .map_err(|e| format!("PFX文件解析失败: {}。可能原因：密码错误或PFX文件损坏", e))?;

    let mut certificates = Vec::new();
    let mut private_keys = Vec::new();
    let mut combined_pem = String::new();

    // 处理证书链
    if let Some(cert) = parsed.cert {
        let cert_pem = cert_to_pem(&cert)?;
        certificates.push(cert_pem.clone());
        combined_pem.push_str(&cert_pem);
        combined_pem.push_str("\n");
    }

    // 处理额外证书
    if let Some(ca_certs) = parsed.ca {
        for cert in ca_certs {
            let cert_pem = cert_to_pem(&cert)?;
            certificates.push(cert_pem.clone());
            combined_pem.push_str(&cert_pem);
            combined_pem.push_str("\n");
        }
    }

    // 处理私钥
    if let Some(pkey) = parsed.pkey {
        let key_pem = private_key_to_pem(&pkey)?;
        private_keys.push(key_pem.clone());
        combined_pem.push_str(&key_pem);
        combined_pem.push_str("\n");
    }

    // 检查是否找到任何内容
    if certificates.is_empty() && private_keys.is_empty() {
        return Err("未在PFX文件中找到证书和私钥".to_string());
    }

    Ok(PfxConversionResult {
        certificates,
        private_keys,
        combined_pem,
        success: true,
        error: None,
    })
}

#[tauri::command]
pub async fn convert_pem_to_pfx(
    cert_pem: String,
    private_key_pem: Option<String>,
    password: String,
    private_key_password: Option<String>,
) -> Result<PemToPfxResult, String> {
    convert_pem_to_pfx_internal(cert_pem, private_key_pem, password, private_key_password)
}

fn convert_pem_to_pfx_internal(
    cert_pem: String,
    private_key_pem: Option<String>,
    password: String,
    private_key_password: Option<String>,
) -> Result<PemToPfxResult, String> {
    // 验证输入参数
    if cert_pem.trim().is_empty() {
        return Err("证书内容不能为空".to_string());
    }

    if password.trim().is_empty() {
        return Err("PFX密码不能为空".to_string());
    }

    // 检查证书格式
    if !cert_pem.contains("-----BEGIN CERTIFICATE-----") {
        return Err("证书格式不正确：缺少-----BEGIN CERTIFICATE-----标记".to_string());
    }

    // 解析证书
    let cert = X509::from_pem(cert_pem.as_bytes()).map_err(|e| {
        format!(
            "证书解析失败: {}。请确保证书内容为PEM格式，以-----BEGIN CERTIFICATE-----开头",
            e
        )
    })?;

    // 解析私钥（如果提供）
    let private_key = if let Some(key_pem) = private_key_pem {
        let key_pem = key_pem.trim();
        if key_pem.is_empty() {
            return Err("私钥内容不能为空".to_string());
        }

        // 检查私钥格式
        let has_private_key_header = key_pem.contains("-----BEGIN PRIVATE KEY-----")
            || key_pem.contains("-----BEGIN RSA PRIVATE KEY-----")
            || key_pem.contains("-----BEGIN EC PRIVATE KEY-----")
            || key_pem.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----");

        if !has_private_key_header {
            return Err(
                "私钥格式不正确：缺少BEGIN PRIVATE KEY、BEGIN RSA PRIVATE KEY或BEGIN EC PRIVATE KEY标记".to_string(),
            );
        }

        let key_bytes = key_pem.as_bytes();

        let parsed_key = if let Some(key_password) = private_key_password {
            // 有密码的私钥 - 先尝试 PEM 格式，再尝试 PKCS8 格式
            PKey::private_key_from_pem_passphrase(key_bytes, key_password.as_bytes())
                .or_else(|_| PKey::private_key_from_pkcs8_passphrase(key_bytes, key_password.as_bytes()))
                .map_err(|e| format!("私钥密码错误或解析失败: {}。请检查私钥密码是否正确，或私钥格式是否为PEM/PKCS8格式", e))?
        } else {
            // 无密码的私钥 - 先尝试 PEM 格式，再尝试 PKCS8 格式
            PKey::private_key_from_pem(key_bytes)
                .or_else(|_| PKey::private_key_from_pkcs8(key_bytes))
                .map_err(|e| format!("私钥解析失败: {}。请检查私钥格式是否为PEM/PKCS8格式，以-----BEGIN PRIVATE KEY-----、-----BEGIN RSA PRIVATE KEY-----或-----BEGIN EC PRIVATE KEY-----开头", e))?
        };
        Some(parsed_key)
    } else {
        None
    };

    // 创建PKCS12
    let mut builder = Pkcs12::builder();
    builder.name("certificate");
    builder.cert(&cert);

    if let Some(private_key) = &private_key {
        builder.pkey(private_key);
    }

    let p12 = builder.build2(password.as_str()).map_err(|e| {
        format!(
            "PKCS12构建失败: {}。可能原因：证书和私钥不匹配、证书格式错误或缺少必要的证书信息",
            e
        )
    })?;

    // 导出为DER格式
    let pfx_der = p12.to_der().map_err(|e| format!("PKCS12导出失败: {}", e))?;

    Ok(PemToPfxResult {
        pfx_data: pfx_der,
        success: true,
        error: None,
    })
}

fn cert_to_pem(cert: &X509) -> Result<String, String> {
    cert.to_pem()
        .map_err(|e| format!("证书转换失败: {}。可能原因：证书格式不支持或数据损坏", e))
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|e| format!("证书编码转换失败: {}。可能原因：包含非UTF8字符", e))
        })
}

fn private_key_to_pem(pkey: &PKey<Private>) -> Result<String, String> {
    pkey.private_key_to_pem_pkcs8()
        .map_err(|e| format!("私钥转换失败: {}。可能原因：私钥格式不支持或数据损坏", e))
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|e| format!("私钥编码转换失败: {}。可能原因：包含非UTF8字符", e))
        })
}
