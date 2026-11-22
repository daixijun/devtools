//! AES 加密/解密页共享的选项常量与契约转换。

use devtools::crypto::{AesMode, AesPadding};

/// 模式下拉选项（value 为历史契约字符串）。
pub const MODE_OPTIONS: &[(&str, &str)] = &[
    ("CBC", "CBC"),
    ("ECB", "ECB"),
    ("CFB", "CFB"),
    ("OFB", "OFB"),
    ("CTR", "CTR"),
];

/// 密钥长度下拉选项。
pub const KEY_LEN_OPTIONS: &[(&str, &str)] =
    &[("128", "128 位"), ("192", "192 位"), ("256", "256 位")];

/// 填充下拉选项（value 为历史契约字符串）。
pub const PADDING_OPTIONS: &[(&str, &str)] = &[
    ("Pkcs7", "PKCS#7"),
    ("Iso97971", "ISO/IEC 9797-1"),
    ("AnsiX923", "ANSI X9.23"),
    ("Iso10126", "ISO 10126"),
    ("ZeroPadding", "Zero Padding"),
    ("NoPadding", "No Padding"),
];

/// 历史契约 AesMode 字符串 → core 枚举。
pub fn aes_mode(value: &str) -> AesMode {
    match value {
        "ECB" => AesMode::Ecb,
        "CFB" => AesMode::Cfb,
        "OFB" => AesMode::Ofb,
        "CTR" => AesMode::Ctr,
        _ => AesMode::Cbc,
    }
}

/// 历史契约 AesPadding 字符串 → core 枚举。
pub fn aes_padding(value: &str) -> AesPadding {
    match value {
        "Iso97971" => AesPadding::Iso97971,
        "AnsiX923" => AesPadding::AnsiX923,
        "Iso10126" => AesPadding::Iso10126,
        "ZeroPadding" => AesPadding::Zero,
        "NoPadding" => AesPadding::NoPadding,
        _ => AesPadding::Pkcs7,
    }
}

/// 密钥长度字符串 → 位数值。
pub fn key_len_bits(value: &str) -> usize {
    value.parse().unwrap_or(128)
}
