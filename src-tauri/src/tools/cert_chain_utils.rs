//! 证书链角色判定工具，由 CertificateViewer 与在线 SSL 检测工具共享。
//!
//! 判定分两层：
//! 1. 是否为 CA：BasicConstraints (2.5.29.19) 优先，缺失时用 KeyUsage (2.5.29.15) 的
//!    keyCertSign；两者都缺（如 v1 老证书）走启发式回退；
//! 2. 根 CA 还是中间 CA：完整 DN（DER 字节级）自签名为根；发行者存在于证书集合中为
//!    中间 CA；链顶非自签名 CA 按 pathlen 区分交叉签名根（无 pathlen）与中间 CA（有 pathlen）。

use x509_parser::certificate::X509Certificate;
use x509_parser::extensions::ParsedExtension;
use x509_parser::oid_registry::Oid;

/// 链级别：0 = 终端证书，1 = 中间 CA，2 = 根 CA
pub fn determine_chain_level(cert: &X509Certificate, all_certs: &[X509Certificate]) -> usize {
    // 第一层：根据证书自身功能（Basic Constraints / Key Usage）判断是否为 CA
    // 第二层：根据在链中的位置（发行者是否在证书集合中）区分根 CA 与中间 CA
    let basic_constraints_oid = Oid::from(&[2, 5, 29, 19]).unwrap();

    if let Some(bc_ext) = cert
        .extensions()
        .iter()
        .find(|ext| ext.oid == basic_constraints_oid)
    {
        if let ParsedExtension::BasicConstraints(bc) = bc_ext.parsed_extension() {
            if !bc.ca {
                return 0; // 不是CA，终端证书
            }
            return determine_ca_chain_level(cert, all_certs, bc.path_len_constraint);
        }
    }

    // 没有 Basic Constraints 扩展时，用 Key Usage 的 keyCertSign 判断是否具有签发证书的功能
    if has_key_cert_sign(cert) {
        return determine_ca_chain_level(cert, all_certs, None);
    }

    // 既无 Basic Constraints 也无法确认 keyCertSign（如 v1 老证书），回退到启发式
    fallback_determine_chain_level(cert)
}

/// 已确认证书是 CA 后，结合链中位置与路径长度约束区分根 CA / 中间 CA
pub fn determine_ca_chain_level(
    cert: &X509Certificate,
    all_certs: &[X509Certificate],
    path_len_constraint: Option<u32>,
) -> usize {
    // 完整 DN 一致的自签名证书是真根 CA
    if is_self_signed(cert) {
        return 2;
    }

    // 发行者存在于当前证书集合中，说明它上面还有签发者，是中间 CA
    if issuer_in_chain(cert, all_certs) {
        return 1;
    }

    // 位于链顶（发行者不在集合中）的非自签名 CA：
    // - 无路径长度约束：无法与交叉签名根证书区分（如 DigiCert Global Root G2 被旧根
    //   交叉签名的证书），按链顶信任锚的功能处理为根 CA
    // - 带路径长度约束：形态更接近中间 CA（根 CA 通常不设置 pathlen），
    //   保持中间 CA 并由链分析提示缺少上级证书
    match path_len_constraint {
        Some(_) => 1,
        None => 2,
    }
}

/// 检查证书的发行者是否存在于当前证书集合中（完整 DN 的 DER 字节级比较）
pub fn issuer_in_chain(cert: &X509Certificate, all_certs: &[X509Certificate]) -> bool {
    let issuer_raw = cert.issuer().as_raw();
    all_certs
        .iter()
        .any(|other| other.subject().as_raw() == issuer_raw)
}

/// 检查 Key Usage 扩展中是否启用了 keyCertSign
pub fn has_key_cert_sign(cert: &X509Certificate) -> bool {
    let key_usage_oid = Oid::from(&[2, 5, 29, 15]).unwrap();
    cert.extensions()
        .iter()
        .find(|ext| ext.oid == key_usage_oid)
        .and_then(|ext| match ext.parsed_extension() {
            ParsedExtension::KeyUsage(ku) => Some(ku.key_cert_sign()),
            _ => None,
        })
        .unwrap_or(false)
}

/// 通过完整 DN 比较（DER 字节级）判断证书是否为自签名
pub fn is_self_signed(cert: &X509Certificate) -> bool {
    let subject_raw = cert.subject().as_raw();
    !subject_raw.is_empty() && subject_raw == cert.issuer().as_raw()
}

pub fn fallback_determine_chain_level(cert: &X509Certificate) -> usize {
    // v1 老证书等缺少扩展信息的回退判断
    if is_self_signed(cert) {
        return 2; // 自签名根CA
    }

    let issuer_cn = cert
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("");

    // 简单的启发式判断
    if issuer_cn.contains("Root") || issuer_cn.contains("CA") {
        0 // 由CA颁发的证书，可能是终端证书
    } else {
        1 // 可能是中间CA
    }
}
