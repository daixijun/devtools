//! URL 编码/解码。
//!
//! 编码/解码行为（与历史版本一致）：
//! - encode：保留 `A-Za-z0-9-_.!~*'()`，其余字符按 UTF-8 字节逐个 `%XX`
//!   （大写十六进制，与 JS 输出一致）；
//! - decode：按 RFC3986 解析 `%XX`，非法转义序列或解码结果不是合法 UTF-8
//!   时返回 `URI malformed`（历史版本同款错误文案）。

/// 非法转义序列 / 非法 UTF-8 的错误文案。
pub const URI_MALFORMED_MSG: &str = "URI malformed";

/// URL 编码（永不失败；Rust `&str` 保证是合法 UTF-8，
/// 不存在 JS 因代理项孤儿抛错的情况）。
pub fn encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            // JS 不转义：A-Za-z0-9 - _ . ! ~ * ' ( )
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(hex_hi(b));
                out.push(hex_lo(b));
            }
        }
    }
    out
}

/// 等价 JS `decodeURIComponent`：非法 `%` 序列或结果非合法 UTF-8 → [`URI_MALFORMED_MSG`]。
pub fn decode_component(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = hex_val(bytes.get(i + 1).copied()).ok_or(URI_MALFORMED_MSG)?;
            let lo = hex_val(bytes.get(i + 2).copied()).ok_or(URI_MALFORMED_MSG)?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| URI_MALFORMED_MSG.to_string())
}

fn hex_hi(b: u8) -> char {
    hex_digit(b >> 4)
}

fn hex_lo(b: u8) -> char {
    hex_digit(b & 0xF)
}

fn hex_digit(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        _ => (b'A' + (v - 10)) as char,
    }
}

fn hex_val(b: Option<u8>) -> Option<u8> {
    match b? {
        c @ b'0'..=b'9' => Some(c - b'0'),
        c @ b'a'..=b'f' => Some(c - b'a' + 10),
        c @ b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_keeps_unreserved() {
        assert_eq!(encode_component(""), "");
        assert_eq!(
            encode_component("abcXYZ0123456789-_.!~*'()"),
            "abcXYZ0123456789-_.!~*'()"
        );
    }

    #[test]
    fn encode_escapes_reserved_and_non_ascii() {
        assert_eq!(encode_component("hello world"), "hello%20world");
        assert_eq!(encode_component("a+b=c"), "a%2Bb%3Dc");
        assert_eq!(encode_component("a/b?c#d"), "a%2Fb%3Fc%23d");
        assert_eq!(encode_component("你好"), "%E4%BD%A0%E5%A5%BD");
        assert_eq!(encode_component("<>\"&"), "%3C%3E%22%26");
        assert_eq!(encode_component("🦀"), "%F0%9F%A6%80");
    }

    #[test]
    fn decode_basic() {
        assert_eq!(decode_component("").unwrap(), "");
        assert_eq!(decode_component("hello%20world").unwrap(), "hello world");
        assert_eq!(decode_component("a%2Bb").unwrap(), "a+b");
        // '+' 不按空格解码（decodeURIComponent 语义）
        assert_eq!(decode_component("a+b").unwrap(), "a+b");
        assert_eq!(decode_component("%E4%BD%A0%E5%A5%BD").unwrap(), "你好");
        assert_eq!(decode_component("%F0%9F%A6%80").unwrap(), "🦀");
        assert_eq!(decode_component("%2F%3F%23").unwrap(), "/?#");
    }

    #[test]
    fn decode_invalid_escape() {
        assert_eq!(decode_component("%"), Err(URI_MALFORMED_MSG.to_string()));
        assert_eq!(decode_component("abc%"), Err(URI_MALFORMED_MSG.to_string()));
        assert_eq!(decode_component("%2"), Err(URI_MALFORMED_MSG.to_string()));
        assert_eq!(decode_component("%G1"), Err(URI_MALFORMED_MSG.to_string()));
        assert_eq!(
            decode_component("%E4%BD"),
            Err(URI_MALFORMED_MSG.to_string())
        );
    }

    #[test]
    fn decode_invalid_utf8_bytes() {
        assert_eq!(decode_component("%FF"), Err(URI_MALFORMED_MSG.to_string()));
        // 0xC3 0x28：非法 UTF-8 序列
        assert_eq!(
            decode_component("%C3%28"),
            Err(URI_MALFORMED_MSG.to_string())
        );
        // 截断的多字节序列
        assert_eq!(
            decode_component("你%E4"),
            Err(URI_MALFORMED_MSG.to_string())
        );
    }

    #[test]
    fn roundtrip() {
        let cases = [
            "https://example.com/path?q=值&tag=a b#c",
            "a+b=c&d=e",
            "你好，世界！🦀",
            "-_.!~*'()",
        ];
        for case in cases {
            assert_eq!(decode_component(&encode_component(case)).unwrap(), case);
        }
    }
}
