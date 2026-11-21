//! 密码生成与强度评分。
//!
//! - 字符集拼接顺序：大写 26 → 小写 26 → 数字 10 → 自定义符号（与历史版本一致）；
//! - 随机来源：openssl CSPRNG，每字符取一个 u32 对字符数取模
//!   （与历史版本的随机取模语义一致）；
//! - 强度 score =（长度 ≥12 得 2，≥8 得 1，否则 0）+ 命中字符类型数
//!   （大写/小写/数字/其他符号，0–4）；≥5 非常强、≥4 强、≥3 中等、否则弱。

use openssl::rand::rand_bytes;

pub const UPPERCASE_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWERCASE_CHARS: &str = "abcdefghijklmnopqrstuvwxyz";
pub const DIGIT_CHARS: &str = "0123456789";
/// 默认自定义特殊字符（与历史版本一致）。
pub const DEFAULT_CUSTOM_SYMBOLS: &str = "!@#$%^&*()_+-=[]{}|;:,.<>?";
/// 所有字符类型都未勾选时的提示文案（与历史版本一致）。
pub const NO_CHARSET_MSG: &str = "请至少选择一种字符类型";

/// 按选项构建可用字符集（大写 → 小写 → 数字 → 自定义符号依次拼接）。
/// 全不选时返回 `None`。
pub fn charset(
    include_uppercase: bool,
    include_lowercase: bool,
    include_numbers: bool,
    include_symbols: bool,
    custom_symbols: &str,
) -> Option<Vec<char>> {
    let mut chars = Vec::with_capacity(94);
    if include_uppercase {
        chars.extend(UPPERCASE_CHARS.chars());
    }
    if include_lowercase {
        chars.extend(LOWERCASE_CHARS.chars());
    }
    if include_numbers {
        chars.extend(DIGIT_CHARS.chars());
    }
    if include_symbols {
        chars.extend(custom_symbols.chars());
    }
    if chars.is_empty() {
        None
    } else {
        Some(chars)
    }
}

/// 生成随机密码；所有字符类型都未勾选时返回 `None`。
pub fn generate(
    length: usize,
    include_uppercase: bool,
    include_lowercase: bool,
    include_numbers: bool,
    include_symbols: bool,
    custom_symbols: &str,
) -> Option<String> {
    let chars = charset(
        include_uppercase,
        include_lowercase,
        include_numbers,
        include_symbols,
        custom_symbols,
    )?;
    if length == 0 {
        return Some(String::new());
    }

    let mut bytes = vec![0u8; length.checked_mul(4).unwrap_or(usize::MAX / 2)];
    rand_bytes(&mut bytes).expect("openssl rand_bytes 不应失败");
    let mut password = String::with_capacity(length.saturating_mul(4));
    for chunk in bytes.as_chunks::<4>().0 {
        let r = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        // 随机取模（与历史版本一致）
        password.push(chars[r as usize % chars.len()]);
    }
    Some(password)
}

/// 密码强度等级。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl Strength {
    /// 中文名（与历史版本一致）。
    pub fn label(self) -> &'static str {
        match self {
            Strength::VeryStrong => "非常强",
            Strength::Strong => "强",
            Strength::Medium => "中等",
            Strength::Weak => "弱",
        }
    }
}

/// 强度评分 = 长度分（≥12 得 2，≥8 得 1，否则 0）+ 命中字符类型数（0–4）。
/// 字符类型判定与历史版本正则等价：/[A-Z]/、/[a-z]/、/[0-9]/、/[^A-Za-z0-9]/。
pub fn score(password: &str) -> u32 {
    let len = password.chars().count();
    let length_score = if len >= 12 {
        2
    } else if len >= 8 {
        1
    } else {
        0
    };
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password
        .chars()
        .any(|c| !(c.is_ascii_uppercase() || c.is_ascii_lowercase() || c.is_ascii_digit()));
    length_score
        + [has_upper, has_lower, has_digit, has_symbol]
            .into_iter()
            .filter(|hit| *hit)
            .count() as u32
}

/// 由评分映射强度：≥5 非常强、≥4 强、≥3 中等、否则弱。
pub fn strength(password: &str) -> Strength {
    match score(password) {
        0..=2 => Strength::Weak,
        3 => Strength::Medium,
        4 => Strength::Strong,
        _ => Strength::VeryStrong,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_concat_order() {
        let all = charset(true, true, true, true, "!@#").unwrap();
        assert_eq!(
            all,
            UPPERCASE_CHARS
                .chars()
                .chain(LOWERCASE_CHARS.chars())
                .chain(DIGIT_CHARS.chars())
                .chain("!@#".chars())
                .collect::<Vec<_>>()
        );
        assert_eq!(all.len(), 26 + 26 + 10 + 3);

        // 仅勾选部分类型
        assert_eq!(charset(true, false, false, false, "").unwrap().len(), 26);
        assert_eq!(
            charset(false, true, false, true, "$€").unwrap(),
            LOWERCASE_CHARS
                .chars()
                .chain("$€".chars())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn charset_all_disabled_is_none() {
        assert!(charset(false, false, false, false, "!@#").is_none());
        // 勾选符号但自定义串为空 → 等效全不选
        assert!(charset(false, false, false, true, "").is_none());
    }

    #[test]
    fn generate_none_when_all_disabled() {
        assert!(generate(16, false, false, false, false, "!@#").is_none());
    }

    #[test]
    fn generate_length_and_charset_membership() {
        let pw = generate(32, true, true, true, true, DEFAULT_CUSTOM_SYMBOLS).unwrap();
        assert_eq!(pw.chars().count(), 32);
        let allowed = charset(true, true, true, true, DEFAULT_CUSTOM_SYMBOLS).unwrap();
        assert!(pw.chars().all(|c| allowed.contains(&c)));

        let digits = generate(10, false, false, true, false, "").unwrap();
        assert_eq!(digits.len(), 10);
        assert!(digits.chars().all(|c| c.is_ascii_digit()));

        // 自定义符号含多字节字符（货币符号）
        let money = generate(50, false, false, false, true, "$€£¥¢").unwrap();
        assert_eq!(money.chars().count(), 50);
        assert!(money.chars().all(|c| "$€£¥¢".contains(c)));
    }

    #[test]
    fn generate_zero_length_is_empty() {
        assert_eq!(
            generate(0, true, true, true, true, DEFAULT_CUSTOM_SYMBOLS).unwrap(),
            ""
        );
    }

    #[test]
    fn score_and_strength() {
        // 长度分 + 类型分
        assert_eq!(score("a"), 1); // 0 + lower
        assert_eq!(score("abc"), 1); // 0 + lower
        assert_eq!(score("abc123"), 2); // 0 + lower + digit
        assert_eq!(score("abcd1234"), 3); // 1(len≥8) + lower + digit
        assert_eq!(score("Abcd1234"), 4); // 1 + upper + lower + digit
        assert_eq!(score("Abcd1234!@"), 5); // 1 + 4 types
        assert_eq!(score("Abcd1234!@#$"), 6); // 2(len≥12) + 4 types
        assert_eq!(score(""), 0);
        assert_eq!(score("€€€"), 1); // 仅符号类型
    }

    #[test]
    fn strength_labels() {
        assert_eq!(strength("abc"), Strength::Weak);
        assert_eq!(strength("abcd1234"), Strength::Medium);
        assert_eq!(strength("Abcd1234"), Strength::Strong);
        assert_eq!(strength("Abcd1234!@"), Strength::VeryStrong);
        assert_eq!(Strength::VeryStrong.label(), "非常强");
        assert_eq!(Strength::Strong.label(), "强");
        assert_eq!(Strength::Medium.label(), "中等");
        assert_eq!(Strength::Weak.label(), "弱");
    }

    #[test]
    fn generate_is_random() {
        let a = generate(24, true, true, true, true, DEFAULT_CUSTOM_SYMBOLS).unwrap();
        let b = generate(24, true, true, true, true, DEFAULT_CUSTOM_SYMBOLS).unwrap();
        // 24 字符随机密码完全相同的概率可忽略
        assert_ne!(a, b);
    }
}
