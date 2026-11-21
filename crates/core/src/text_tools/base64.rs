//! Base64 编解码。
//!
//! 契约见 docs/plans/tool-specs/batch1.md 算法节：
//! - 编码：UTF-8 字节 → 标准 Base64；
//! - 解码先做 canonical 校验（「s 是某字节序列的
//!   标准 Base64 编码」，再把字节按 UTF-8 解码。
//! - 校验失败文案 `输入的不是有效的Base64格式`；字节非合法 UTF-8 时文案
//!   `URI malformed`，错误前缀由 UI 层拼接。

use base64::Engine as _;

/// 输入不是合法（canonical）标准 Base64 的错误文案（与历史版本一致）。
pub const INVALID_BASE64_MSG: &str = "输入的不是有效的Base64格式";
/// 解码出的字节不是合法 UTF-8 的错误文案（与历史版本一致）。
pub const URI_MALFORMED_MSG: &str = "URI malformed";

/// 编码：UTF-8 字节 → 标准 Base64（带 padding）。
pub fn encode(input: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

/// 解码：标准 Base64 → UTF-8 文本。
///
/// - 空/空白输入或非 canonical（编码/解码往返不等）→ [`INVALID_BASE64_MSG`]；
/// - 解码字节不是合法 UTF-8 → [`URI_MALFORMED_MSG`]。
pub fn decode(input: &str) -> Result<String, String> {
    if input.trim().is_empty() {
        return Err(INVALID_BASE64_MSG.to_string());
    }
    let bytes = decode_canonical(input).ok_or(INVALID_BASE64_MSG)?;
    String::from_utf8(bytes).map_err(|_| URI_MALFORMED_MSG.to_string())
}

/// 判定 `s` 是否为 canonical 标准 Base64（编码/解码往返相等），
/// 成立时返回解码后的字节，否则返回 `None`。
///
/// 规则：
/// - 长度为 4 的倍数且非空；
/// - 仅含标准字母表字符，`=` 只能作为末尾 1–2 个 padding；
/// - 末尾被丢弃的比特必须为 0（如 `AB==` 非法：解码丢弃非零比特，回编码得 `AA==`）；
/// - 含任何 ASCII 空白直接非法（标准 Base64 输出不含空白，回串必不相等）。
fn decode_canonical(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return None;
    }

    let pad = bytes.iter().rev().take_while(|&&c| c == b'=').count();
    if pad > 2 {
        return None;
    }
    let data_end = bytes.len() - pad;
    if data_end == 0 {
        return None;
    }

    let value = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut acc = 0u32;
    let mut group = 0usize; // 当前 4 字符组内已累计的 6-bit 数
    for &c in &bytes[..data_end] {
        acc = (acc << 6) | value(c)?;
        group += 1;
        if group == 4 {
            out.extend_from_slice(&[(acc >> 16) as u8, (acc >> 8) as u8, acc as u8]);
            acc = 0;
            group = 0;
        }
    }
    match group {
        0 => {}
        // 剩 2 个数据字符（pad == 2）→ 1 字节，末尾 4 bit 必须为 0
        2 => {
            if acc & 0b1111 != 0 {
                return None;
            }
            out.push((acc >> 4) as u8);
        }
        // 剩 3 个数据字符（pad == 1）→ 2 字节，末尾 2 bit 必须为 0
        3 => {
            if acc & 0b11 != 0 {
                return None;
            }
            out.push((acc >> 10) as u8);
            out.push((acc >> 2) as u8);
        }
        // 长度是 4 的倍数且 pad ≤ 2，剩余 1 个数据字符不可能出现
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ascii() {
        assert_eq!(encode(""), "");
        assert_eq!(encode("f"), "Zg==");
        assert_eq!(encode("fo"), "Zm8=");
        assert_eq!(encode("foo"), "Zm9v");
        assert_eq!(encode("foobar"), "Zm9vYmFy");
        assert_eq!(encode("Hello"), "SGVsbG8=");
    }

    #[test]
    fn encode_utf8_roundtrip() {
        let cases = [
            "Hello, World! 你好，世界！",
            "emoji 🦀 test",
            "line1\nline2\ttab",
            "特殊字符 !@#$%^&*()",
        ];
        for case in cases {
            assert_eq!(decode(&encode(case)).unwrap(), case);
        }
    }

    #[test]
    fn decode_ok() {
        assert_eq!(decode("SGVsbG8=").unwrap(), "Hello");
        assert_eq!(decode("Zm9vYmFy").unwrap(), "foobar");
        assert_eq!(decode("QQ==").unwrap(), "A");
        assert_eq!(decode("AA==").unwrap(), "\0");
        // 编码端示例：标准 Base64("Hello, World! 你好，世界！")
        assert_eq!(
            decode("SGVsbG8sIFdvcmxkISDkvaDlpb3vvIzkuJbnlYzvvIE=").unwrap(),
            "Hello, World! 你好，世界！"
        );
    }

    #[test]
    fn decode_rejects_empty_and_bad_length() {
        assert_eq!(decode(""), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode("   \t\n "), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode("Zg="), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode("Zm9vY"), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode("Q"), Err(INVALID_BASE64_MSG.to_string()));
    }

    #[test]
    fn decode_rejects_bad_charset() {
        assert_eq!(decode("!!!!"), Err(INVALID_BASE64_MSG.to_string()));
        // URL 安全字母表不属于标准 Base64
        assert_eq!(decode("Zm8-"), Err(INVALID_BASE64_MSG.to_string()));
        // 中文等非 ASCII 字符
        assert_eq!(decode("中文"), Err(INVALID_BASE64_MSG.to_string()));
        // 内嵌空白：解码会忽略空白但往返不等 → 判为非法
        assert_eq!(decode("QQ\n=="), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode(" QQ== "), Err(INVALID_BASE64_MSG.to_string()));
        // '=' 出现在中间
        assert_eq!(decode("Z=8="), Err(INVALID_BASE64_MSG.to_string()));
        // '=' 过多
        assert_eq!(decode("Z===="), Err(INVALID_BASE64_MSG.to_string()));
        assert_eq!(decode("===="), Err(INVALID_BASE64_MSG.to_string()));
    }

    #[test]
    fn decode_rejects_non_canonical_padding_bits() {
        // 解码可接受但往返回串不同 → 判为非法
        // 'B'=1，末 4 bit 非零
        assert_eq!(decode("AB=="), Err(INVALID_BASE64_MSG.to_string()));
        // 'C'=2，末 2 bit 非零
        assert_eq!(decode("ABC="), Err(INVALID_BASE64_MSG.to_string()));
        // canonical 对照
        assert_eq!(decode("AA==").unwrap(), "\0");
        assert_eq!(decode("AQ==").unwrap(), "\u{1}");
    }

    #[test]
    fn decode_invalid_utf8_yields_uri_malformed() {
        // 0xFF 的 canonical 编码
        assert_eq!(decode("/w=="), Err(URI_MALFORMED_MSG.to_string()));
        // 0xC3 0x28（非法 UTF-8 序列）
        assert_eq!(decode("wyg="), Err(URI_MALFORMED_MSG.to_string()));
    }
}
