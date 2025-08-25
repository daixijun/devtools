use crate::utils::error::{DevToolError, DevToolResult};

/// 输入验证工具集
/// 统一处理各种输入数据的验证逻辑
#[allow(dead_code)]
pub struct InputValidator;

#[allow(dead_code)]
impl InputValidator {
    /// 验证非空字符串
    ///
    /// # Arguments
    /// * `input` - 待验证的字符串
    /// * `field_name` - 字段名称，用于错误消息
    #[allow(dead_code)]
    pub fn validate_non_empty(input: &str, field_name: &str) -> DevToolResult<()> {
        if input.trim().is_empty() {
            return Err(DevToolError::EmptyInput(format!("{}不能为空", field_name)));
        }
        Ok(())
    }

    /// 验证文件内容
    ///
    /// # Arguments
    /// * `content` - 文件内容字节数组
    /// * `file_type` - 文件类型名称，用于错误消息
    pub fn validate_file_content(content: &[u8], file_type: &str) -> DevToolResult<()> {
        if content.is_empty() {
            return Err(DevToolError::FileError(format!(
                "{}文件内容不能为空",
                file_type
            )));
        }
        Ok(())
    }

    /// 验证证书格式
    ///
    /// # Arguments
    /// * `content` - 证书内容
    /// * `cert_type` - 证书类型
    pub fn validate_certificate_format(content: &str, cert_type: &str) -> DevToolResult<()> {
        Self::validate_non_empty(content, &format!("{}内容", cert_type))?;

        let expected_headers = match cert_type {
            "PEM证书" => vec!["-----BEGIN CERTIFICATE-----"],
            "PEM私钥" => vec![
                "-----BEGIN PRIVATE KEY-----",
                "-----BEGIN RSA PRIVATE KEY-----",
                "-----BEGIN EC PRIVATE KEY-----",
                "-----BEGIN ENCRYPTED PRIVATE KEY-----",
            ],
            _ => return Ok(()),
        };

        if !expected_headers
            .iter()
            .any(|header| content.contains(header))
        {
            return Err(DevToolError::ValidationError(format!(
                "{}格式不正确：缺少正确的BEGIN标记",
                cert_type
            )));
        }

        Ok(())
    }

    /// 验证JSON格式
    pub fn validate_json(input: &str) -> DevToolResult<()> {
        Self::validate_non_empty(input, "JSON内容")?;

        match serde_json::from_str::<serde_json::Value>(input) {
            Ok(_) => Ok(()),
            Err(e) => Err(DevToolError::ValidationError(format!(
                "JSON格式错误: {}",
                e
            ))),
        }
    }

    /// 验证域名格式
    pub fn validate_domain(domain: &str) -> DevToolResult<()> {
        Self::validate_non_empty(domain, "域名")?;

        // 基本的域名格式验证
        let domain_regex = regex::Regex::new(
            r"^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$",
        )
        .map_err(|_| DevToolError::ValidationError("域名验证正则表达式错误".to_string()))?;

        if !domain_regex.is_match(&domain.to_lowercase()) {
            return Err(DevToolError::ValidationError("域名格式不正确".to_string()));
        }

        Ok(())
    }

    /// 验证IP地址格式
    pub fn validate_ip_address(ip: &str) -> DevToolResult<()> {
        Self::validate_non_empty(ip, "IP地址")?;

        match ip.parse::<std::net::IpAddr>() {
            Ok(_) => Ok(()),
            Err(_) => Err(DevToolError::ValidationError(
                "IP地址格式不正确".to_string(),
            )),
        }
    }

    /// 验证端口号
    pub fn validate_port(port: u16) -> DevToolResult<()> {
        if port == 0 {
            return Err(DevToolError::ValidationError("端口号不能为0".to_string()));
        }
        Ok(())
    }

    /// 验证Base64格式
    pub fn validate_base64(input: &str) -> DevToolResult<()> {
        Self::validate_non_empty(input, "Base64内容")?;

        use base64::{engine::general_purpose, Engine as _};
        match general_purpose::STANDARD.decode(input) {
            Ok(_) => Ok(()),
            Err(e) => Err(DevToolError::ValidationError(format!(
                "Base64格式错误: {}",
                e
            ))),
        }
    }

    /// 验证SQL语句
    pub fn validate_sql(sql: &str) -> DevToolResult<()> {
        Self::validate_non_empty(sql, "SQL语句")?;

        // 基本的SQL关键词检查
        let sql_lower = sql.to_lowercase();
        let sql_keywords = [
            "select", "create", "alter", "insert", "update", "delete", "drop",
        ];

        if !sql_keywords
            .iter()
            .any(|keyword| sql_lower.contains(keyword))
        {
            return Err(DevToolError::ValidationError(
                "不是有效的SQL语句".to_string(),
            ));
        }

        Ok(())
    }

    /// 验证时间戳
    pub fn validate_timestamp(timestamp: &str) -> DevToolResult<()> {
        Self::validate_non_empty(timestamp, "时间戳")?;

        // 尝试解析为数字
        match timestamp.parse::<i64>() {
            Ok(ts) => {
                // 检查时间戳合理性（1970年到2100年之间）
                if ts < 0 || ts > 4102444800 {
                    return Err(DevToolError::ValidationError(
                        "时间戳超出合理范围".to_string(),
                    ));
                }
                Ok(())
            }
            Err(_) => Err(DevToolError::ValidationError(
                "时间戳格式错误，必须为数字".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_non_empty() {
        assert!(InputValidator::validate_non_empty("test", "字段").is_ok());
        assert!(InputValidator::validate_non_empty("", "字段").is_err());
        assert!(InputValidator::validate_non_empty("   ", "字段").is_err());
    }

    #[test]
    fn test_validate_json() {
        assert!(InputValidator::validate_json(r#"{"key": "value"}"#).is_ok());
        assert!(InputValidator::validate_json(r#"[1, 2, 3]"#).is_ok());
        assert!(InputValidator::validate_json(r#"invalid json"#).is_err());
    }

    #[test]
    fn test_validate_domain() {
        assert!(InputValidator::validate_domain("example.com").is_ok());
        assert!(InputValidator::validate_domain("sub.example.com").is_ok());
        assert!(InputValidator::validate_domain("invalid domain").is_err());
    }

    #[test]
    fn test_validate_ip_address() {
        assert!(InputValidator::validate_ip_address("127.0.0.1").is_ok());
        assert!(InputValidator::validate_ip_address("::1").is_ok());
        assert!(InputValidator::validate_ip_address("invalid ip").is_err());
    }
}
