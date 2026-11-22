use regex::Regex;

/// 字符串处理工具集
/// 统一处理字符串转换、清理和格式化操作
pub struct StringUtils;

impl StringUtils {
    /// 转换为 PascalCase
    ///
    /// # Examples
    /// ```
    /// # use devtools::utils::string_utils::StringUtils;
    /// assert_eq!(StringUtils::to_pascal_case("hello_world"), "HelloWorld");
    /// assert_eq!(StringUtils::to_pascal_case("user-name"), "UserName");
    /// ```
    pub fn to_pascal_case(s: &str) -> String {
        Self::convert_case(s, true)
    }

    /// 转换为 camelCase
    ///
    /// # Examples
    /// ```
    /// # use devtools::utils::string_utils::StringUtils;
    /// assert_eq!(StringUtils::to_camel_case("hello_world"), "helloWorld");
    /// assert_eq!(StringUtils::to_camel_case("user-name"), "userName");
    /// ```
    pub fn to_camel_case(s: &str) -> String {
        Self::convert_case(s, false)
    }

    /// 通用大小写转换
    fn convert_case(s: &str, capitalize_first: bool) -> String {
        if s.is_empty() {
            return String::new();
        }

        let words = Self::split_into_words(s);
        if words.is_empty() {
            return String::new();
        }

        Self::build_case_string(words, capitalize_first)
    }

    /// 清理和标准化字段名
    ///
    /// # Arguments
    /// * `key` - 原始字段名
    /// * `fallback` - 当key无效时使用的后备名称
    ///
    /// # Examples
    /// ```
    /// # use devtools::utils::string_utils::StringUtils;
    /// assert_eq!(StringUtils::sanitize_field_name("user@name", "Field"), "user_name");
    /// assert_eq!(StringUtils::sanitize_field_name("123name", "Field"), "Field_123name");
    /// ```
    pub fn sanitize_field_name(key: &str, fallback: &str) -> String {
        if key.is_empty() {
            return fallback.to_string();
        }

        let mut result = String::new();
        let mut chars = key.chars().peekable();

        // 处理第一个字符
        if let Some(first) = chars.next() {
            if first.is_alphabetic() || first == '_' {
                result.push(first);
            } else if first.is_ascii_digit() {
                result.push_str(&format!("{}_", fallback));
                result.push(first);
            } else {
                result.push_str(&format!("{}_", fallback));
                if first.is_alphanumeric() {
                    result.push(first);
                }
            }
        }

        // 处理其余字符
        while let Some(ch) = chars.next() {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
            } else {
                if !result.ends_with('_') {
                    result.push('_');
                }
                // 跳过连续的特殊字符
                while let Some(next) = chars.peek() {
                    if next.is_alphanumeric() || *next == '_' {
                        break;
                    }
                    chars.next();
                }
            }
        }

        // 移除末尾的下划线
        while result.ends_with('_') {
            result.pop();
        }

        if result.is_empty() || result.chars().all(|c| c == '_') {
            return fallback.to_string();
        }

        result
    }

    /// 按分隔符拆分单词
    fn split_into_words(s: &str) -> Vec<&str> {
        if let Ok(re) = Regex::new(r"[_\-\s]+") {
            re.split(s).filter(|word| !word.is_empty()).collect()
        } else {
            // 如果正则表达式失败，使用简单的空格分割
            s.split_whitespace().collect()
        }
    }

    /// 构建指定大小写的字符串
    fn build_case_string(words: Vec<&str>, capitalize_first: bool) -> String {
        let mut result = String::new();

        for (i, word) in words.iter().enumerate() {
            if i == 0 && !capitalize_first {
                result.push_str(&word.to_lowercase());
            } else {
                result.push_str(&Self::capitalize_word(word));
            }
        }

        result
    }

    /// 首字母大写
    fn capitalize_word(word: &str) -> String {
        let mut chars = word.chars();
        let mut result = String::new();

        if let Some(first) = chars.next() {
            result.push(first.to_uppercase().next().unwrap_or(first));
            result.push_str(&chars.as_str().to_lowercase());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(StringUtils::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(StringUtils::to_pascal_case("user-name"), "UserName");
        assert_eq!(StringUtils::to_pascal_case("api_key"), "ApiKey");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(StringUtils::to_camel_case("hello_world"), "helloWorld");
        assert_eq!(StringUtils::to_camel_case("user-name"), "userName");
        assert_eq!(StringUtils::to_camel_case("api_key"), "apiKey");
    }

    #[test]
    fn test_sanitize_field_name() {
        assert_eq!(
            StringUtils::sanitize_field_name("valid_name", "Field"),
            "valid_name"
        );
        assert_eq!(
            StringUtils::sanitize_field_name("123name", "Field"),
            "Field_123name"
        );
        assert_eq!(
            StringUtils::sanitize_field_name("user@name", "Field"),
            "user_name"
        );
        assert_eq!(StringUtils::sanitize_field_name("", "Field"), "Field");
    }
}
