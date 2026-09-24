use base64::{engine::general_purpose, Engine as _};
use openssl::pkey::PKey;
use openssl::rsa::Rsa;

/// RSA 密钥生成请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsaGenerateRequest {
    /// 密钥长度(位): 仅支持 2048/3072/4096
    pub key_size: u32,
    /// OpenSSH 公钥末尾的注释, 留空使用默认值
    pub comment: Option<String>,
}

/// 生成的 RSA 密钥对
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsaKeyPair {
    /// PKCS#8 格式私钥 PEM (-----BEGIN PRIVATE KEY-----)
    pub private_pkcs8_pem: String,
    /// PKCS#1 格式私钥 PEM (-----BEGIN RSA PRIVATE KEY-----)
    pub private_pkcs1_pem: String,
    /// SPKI 格式公钥 PEM (-----BEGIN PUBLIC KEY-----)
    pub public_spki_pem: String,
    /// PKCS#1 格式公钥 PEM (-----BEGIN RSA PUBLIC KEY-----)
    pub public_pkcs1_pem: String,
    /// OpenSSH 格式公钥 (ssh-rsa AAAA...)
    pub public_openssh: String,
}

const DEFAULT_COMMENT: &str = "devtools-generated";

#[tauri::command]
pub async fn generate_rsa_keypair(request: RsaGenerateRequest) -> Result<RsaKeyPair, String> {
    let key_size = request.key_size;
    let comment = request.comment;
    // 密钥生成是 CPU 密集操作, 放到阻塞线程池避免占用异步运行时
    tokio::task::spawn_blocking(move || generate_rsa_keypair_internal(key_size, comment))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

fn generate_rsa_keypair_internal(key_size: u32, comment: Option<String>) -> Result<RsaKeyPair, String> {
    if !matches!(key_size, 2048 | 3072 | 4096) {
        return Err(format!(
            "不支持的密钥长度: {}。仅支持 2048、3072、4096 位",
            key_size
        ));
    }

    let rsa = Rsa::generate(key_size).map_err(|e| format!("RSA 密钥生成失败: {}", e))?;

    // n/e 的大端字节序原始数据, 用于拼装 OpenSSH 公钥
    // (需在 rsa 被 PKey::from_rsa 消耗前取出)
    let n_bytes = rsa.n().to_vec();
    let e_bytes = rsa.e().to_vec();

    let private_pkcs1_pem = pem_to_string(
        rsa.private_key_to_pem()
            .map_err(|e| format!("导出 PKCS#1 私钥失败: {}", e))?,
    )?;
    // PKCS#1 公钥 (BEGIN RSA PUBLIC KEY), 同样借用 rsa 导出
    let public_pkcs1_pem = pem_to_string(
        rsa.public_key_to_pem_pkcs1()
            .map_err(|e| format!("导出 PKCS#1 公钥失败: {}", e))?,
    )?;

    // PKCS#8 与 SPKI 需经 PKey 导出; from_rsa 会消耗 Rsa,
    // 因此放在借用操作(n/e 取值、PKCS#1 导出)之后
    let pkey = PKey::from_rsa(rsa).map_err(|e| format!("构建 PKey 失败: {}", e))?;
    let private_pkcs8_pem = pem_to_string(
        pkey.private_key_to_pem_pkcs8()
            .map_err(|e| format!("导出 PKCS#8 私钥失败: {}", e))?,
    )?;
    let public_spki_pem = pem_to_string(
        pkey.public_key_to_pem()
            .map_err(|e| format!("导出公钥失败: {}", e))?,
    )?;

    let public_openssh = format_openssh_public_key(&n_bytes, &e_bytes, comment.as_deref());

    Ok(RsaKeyPair {
        private_pkcs8_pem,
        private_pkcs1_pem,
        public_spki_pem,
        public_pkcs1_pem,
        public_openssh,
    })
}

fn pem_to_string(pem: Vec<u8>) -> Result<String, String> {
    String::from_utf8(pem).map_err(|e| format!("PEM 编码异常: {}", e))
}

fn format_openssh_public_key(n: &[u8], e: &[u8], comment: Option<&str>) -> String {
    // RFC 4253: string "ssh-rsa", mpint e, mpint n (注意指数在前)
    let mut blob = Vec::new();
    blob.extend_from_slice(&ssh_string(b"ssh-rsa"));
    blob.extend_from_slice(&ssh_mpint(e));
    blob.extend_from_slice(&ssh_mpint(n));

    let encoded = general_purpose::STANDARD.encode(&blob);
    let comment = comment
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .unwrap_or(DEFAULT_COMMENT);
    format!("ssh-rsa {} {}", encoded, comment)
}

/// SSH wire format 的 string 类型: 4 字节大端长度 + 数据
fn ssh_string(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    out
}

/// SSH wire format 的 mpint 类型: 大端补码整数, 去除多余前导零,
/// 最高位为 1 时补 0x00 以标识正数
fn ssh_mpint(data: &[u8]) -> Vec<u8> {
    let start = data.iter().take_while(|&&b| b == 0).count();
    let stripped = &data[start..];
    let needs_pad = !stripped.is_empty() && stripped[0] & 0x80 != 0;

    let mut payload = Vec::with_capacity(stripped.len() + 1);
    if needs_pad {
        payload.push(0x00);
    }
    payload.extend_from_slice(stripped);
    ssh_string(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_leading_zeros(bytes: &[u8]) -> &[u8] {
        let start = bytes.iter().take_while(|&&b| b == 0).count();
        &bytes[start..]
    }

    /// 解析 OpenSSH 公钥行, 返回 (n, e)
    fn decode_openssh_fields(public_openssh: &str) -> (Vec<u8>, Vec<u8>) {
        let b64 = public_openssh.split_whitespace().nth(1).unwrap();
        let blob = general_purpose::STANDARD.decode(b64).unwrap();
        let mut pos = 0usize;
        let mut read_field = |blob: &[u8]| -> Vec<u8> {
            let len =
                u32::from_be_bytes([blob[pos], blob[pos + 1], blob[pos + 2], blob[pos + 3]]) as usize;
            pos += 4;
            let field = blob[pos..pos + len].to_vec();
            pos += len;
            field
        };
        assert_eq!(read_field(&blob), b"ssh-rsa");
        let e = read_field(&blob);
        let n = read_field(&blob);
        assert_eq!(pos, blob.len(), "OpenSSH blob 应在读取 n 后结束");
        (n, e)
    }

    #[test]
    fn test_generate_2048_formats_and_roundtrip() {
        let result =
            generate_rsa_keypair_internal(2048, Some("test@example.com".to_string())).unwrap();

        assert!(result.private_pkcs8_pem.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(result.private_pkcs1_pem.starts_with("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(result.public_spki_pem.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(result.public_pkcs1_pem.starts_with("-----BEGIN RSA PUBLIC KEY-----"));
        assert!(result.public_openssh.starts_with("ssh-rsa AAAA"));
        assert!(result.public_openssh.ends_with("test@example.com"));

        // 回读私钥, 校验与公钥参数一致
        // (mpint 解码结果可能带符号填充的 0x00 前缀, 比较前统一剥离)
        let rsa = Rsa::private_key_from_pem(result.private_pkcs8_pem.as_bytes()).unwrap();
        let (openssh_n, openssh_e) = decode_openssh_fields(&result.public_openssh);
        assert_eq!(
            strip_leading_zeros(&openssh_n),
            strip_leading_zeros(&rsa.n().to_vec())
        );
        assert_eq!(
            strip_leading_zeros(&openssh_e),
            strip_leading_zeros(&rsa.e().to_vec())
        );

        // 公钥 PEM 也能解析回相同参数
        let pkey = PKey::public_key_from_pem(result.public_spki_pem.as_bytes()).unwrap();
        let pub_rsa = pkey.rsa().unwrap();
        assert_eq!(pub_rsa.n().to_vec(), rsa.n().to_vec());
        assert_eq!(pub_rsa.e().to_vec(), rsa.e().to_vec());

        let pkcs1_pub = Rsa::public_key_from_pem_pkcs1(result.public_pkcs1_pem.as_bytes()).unwrap();
        assert_eq!(pkcs1_pub.n().to_vec(), rsa.n().to_vec());
        assert_eq!(pkcs1_pub.e().to_vec(), rsa.e().to_vec());
    }

    #[tokio::test]
    async fn test_command_wrapper_4096_default_comment() {
        let request = RsaGenerateRequest {
            key_size: 4096,
            comment: Some("   ".to_string()),
        };
        let result = generate_rsa_keypair(request).await.unwrap();
        assert!(result.private_pkcs8_pem.contains("BEGIN PRIVATE KEY"));
        // 空白注释回退到默认值
        assert!(result.public_openssh.ends_with(DEFAULT_COMMENT));
    }

    #[test]
    fn test_invalid_key_size_rejected() {
        assert!(generate_rsa_keypair_internal(1024, None).is_err());
        assert!(generate_rsa_keypair_internal(0, None).is_err());
    }
}
