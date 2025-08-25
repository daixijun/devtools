/// 代码格式化工具集
/// 统一处理各种代码的格式化操作
pub struct CodeFormatter;

#[allow(dead_code)]
impl CodeFormatter {
    /// 格式化 Go 代码
    ///
    /// # Arguments
    /// * `code` - 待格式化的Go代码
    ///
    /// # Returns
    /// 格式化后的Go代码字符串
    pub fn format_go_code(code: &str) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let mut formatted = Vec::new();
        let mut indent_level: i32 = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                formatted.push(String::new());
                continue;
            }

            // 处理闭合大括号
            indent_level = Self::adjust_indent_for_closing_brace(trimmed, indent_level);

            // 添加缩进
            let indented = Self::format_line_with_indent(trimmed, &formatted, indent_level);
            formatted.push(indented);

            // 处理开放大括号
            indent_level = Self::adjust_indent_for_opening_brace(trimmed, indent_level);
        }

        formatted.join("\n")
    }

    /// 格式化JSON代码
    ///
    /// # Arguments
    /// * `json` - 待格式化的JSON字符串
    ///
    /// # Returns
    /// 格式化后的JSON字符串
    pub fn format_json_code(json: &str) -> Result<String, String> {
        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => match serde_json::to_string_pretty(&value) {
                Ok(formatted) => Ok(formatted),
                Err(e) => Err(format!("JSON格式化失败: {}", e)),
            },
            Err(e) => Err(format!("JSON解析失败: {}", e)),
        }
    }

    /// 格式化SQL代码
    ///
    /// # Arguments
    /// * `sql` - 待格式化的SQL字符串
    ///
    /// # Returns
    /// 格式化后的SQL字符串
    pub fn format_sql_code(sql: &str) -> String {
        let mut formatted = String::new();
        let mut in_string = false;
        let mut quote_char = '"';
        let mut prev_char = ' ';

        for ch in sql.chars() {
            match ch {
                '"' | '\'' if !in_string => {
                    in_string = true;
                    quote_char = ch;
                    formatted.push(ch);
                }
                ch if ch == quote_char && in_string && prev_char != '\\' => {
                    in_string = false;
                    formatted.push(ch);
                }
                _ if in_string => {
                    formatted.push(ch);
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if !formatted.ends_with(' ') && !formatted.is_empty() {
                        formatted.push(' ');
                    }
                }
                _ => {
                    formatted.push(ch);
                }
            }
            prev_char = ch;
        }

        // 简单的关键词大写处理
        Self::capitalize_sql_keywords(&formatted)
    }

    /// 调整闭合大括号的缩进
    fn adjust_indent_for_closing_brace(line: &str, current_indent: i32) -> i32 {
        if line == "}" || line.starts_with('}') {
            current_indent.saturating_sub(1)
        } else {
            current_indent
        }
    }

    /// 调整开放大括号的缩进
    fn adjust_indent_for_opening_brace(line: &str, current_indent: i32) -> i32 {
        if Self::should_increase_indent(line) {
            current_indent + 1
        } else {
            current_indent
        }
    }

    /// 格式化行缩进
    fn format_line_with_indent(
        line: &str,
        formatted_lines: &[String],
        indent_level: i32,
    ) -> String {
        if Self::is_import_block_delimiter(line) {
            // import 块分隔符不需要缩进
            line.to_string()
        } else if Self::is_import_statement(line, formatted_lines) {
            // import 语句使用一个制表符
            format!("\t{}", line)
        } else if Self::is_top_level_declaration(line) {
            // 顶级声明（type、func等）不需要缩进
            line.to_string()
        } else {
            // 结构体字段、函数体等需要缩进
            if indent_level > 0 {
                format!("{}{}", "\t".repeat(indent_level as usize), line)
            } else {
                line.to_string()
            }
        }
    }

    /// 检查是否是 import 块分隔符
    fn is_import_block_delimiter(line: &str) -> bool {
        line.starts_with("import (") || line == ")"
    }

    /// 检查是否是 import 语句
    fn is_import_statement(line: &str, formatted_lines: &[String]) -> bool {
        line.starts_with('"')
            && formatted_lines
                .last()
                .map_or(false, |l| l.contains("import ("))
    }

    /// 检查是否是顶级声明
    fn is_top_level_declaration(line: &str) -> bool {
        line.starts_with("type ")
            || line.starts_with("func ")
            || line.starts_with("var ")
            || line.starts_with("const ")
            || line.starts_with("package ")
            || line.starts_with("//")
            || line.starts_with("/*")
    }

    /// 检查是否应该增加缩进
    fn should_increase_indent(line: &str) -> bool {
        line.ends_with('{')
            || line == "import ("
            || (line.contains("struct {") && !line.ends_with('}'))
    }

    /// SQL关键词大写处理
    fn capitalize_sql_keywords(sql: &str) -> String {
        let keywords = [
            "select",
            "from",
            "where",
            "and",
            "or",
            "not",
            "in",
            "like",
            "insert",
            "into",
            "values",
            "update",
            "set",
            "delete",
            "create",
            "table",
            "alter",
            "drop",
            "index",
            "primary",
            "key",
            "foreign",
            "references",
            "constraint",
            "null",
            "not null",
            "auto_increment",
            "default",
            "unique",
            "varchar",
            "int",
            "bigint",
            "text",
            "datetime",
            "timestamp",
            "boolean",
            "join",
            "left",
            "right",
            "inner",
            "outer",
            "on",
            "group",
            "by",
            "having",
            "order",
            "asc",
            "desc",
            "limit",
            "offset",
            "union",
            "all",
            "distinct",
        ];

        let mut result = sql.to_string();
        for keyword in &keywords {
            let regex_pattern = format!(r"\b{}\b", regex::escape(keyword));
            if let Ok(re) = regex::Regex::new(&regex_pattern) {
                result = re.replace_all(&result, keyword.to_uppercase()).to_string();
            }
        }

        result
    }

    /// 格式化XML/HTML代码
    pub fn format_xml_code(xml: &str) -> String {
        let mut formatted = String::new();
        let mut indent_level = 0;
        let mut _in_tag = false; // 标记为不使用，避免编译警告
        let mut tag_content = String::new();

        for ch in xml.chars() {
            match ch {
                '<' => {
                    if !tag_content.trim().is_empty() {
                        formatted.push_str(&format!(
                            "{}{}\n",
                            "  ".repeat(indent_level),
                            tag_content.trim()
                        ));
                        tag_content.clear();
                    }
                    _in_tag = true;
                    tag_content.push(ch);
                }
                '>' => {
                    _in_tag = false;
                    tag_content.push(ch);

                    let tag_str = tag_content.trim();
                    if tag_str.starts_with("</") {
                        // 闭合标签
                        indent_level = indent_level.saturating_sub(1);
                        formatted.push_str(&format!("{}{}\n", "  ".repeat(indent_level), tag_str));
                    } else if tag_str.ends_with("/>") {
                        // 自闭合标签
                        formatted.push_str(&format!("{}{}\n", "  ".repeat(indent_level), tag_str));
                    } else {
                        // 开放标签
                        formatted.push_str(&format!("{}{}\n", "  ".repeat(indent_level), tag_str));
                        indent_level += 1;
                    }
                    tag_content.clear();
                }
                _ => {
                    tag_content.push(ch);
                }
            }
        }

        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_go_code() {
        let input = "package main\nfunc main() {\nfmt.Println(\"Hello\")\n}";
        let output = CodeFormatter::format_go_code(input);
        assert!(output.contains("\tfmt.Println"));
    }

    #[test]
    fn test_format_go_struct() {
        let input = "type User struct {\nID int32\nName string\n}";
        let output = CodeFormatter::format_go_code(input);
        println!("Input: {}", input);
        println!("Output: {}", output);

        // 验证type声明不缩进
        assert!(output.contains("type User struct {"));
        // 验证字段使用制表符缩进
        assert!(output.contains("\tID int32"));
        assert!(output.contains("\tName string"));
        // 验证结构体结束大括号不缩进
        assert!(output.contains("\n}"));
        // 验证没有给type添加缩进
        assert!(!output.contains("\ttype User"));
    }

    #[test]
    fn test_format_go_complete_struct() {
        let input = "package main\n\ntype User struct {\nID int32 `json:\"id\"`\nName string `json:\"name\"`\n}\n\nfunc (u User) TableName() string {\nreturn \"users\"\n}";
        let output = CodeFormatter::format_go_code(input);
        println!("Complete struct output:\n{}", output);

        // 验证package不缩进
        assert!(output.starts_with("package main"));
        // 验证type不缩进
        assert!(output.contains("\ntype User struct {"));
        // 验证字段缩进
        assert!(output.contains("\tID int32"));
        assert!(output.contains("\tName string"));
        // 验证func不缩进
        assert!(output.contains("\nfunc (u User) TableName() string {"));
        // 验证函数体缩进
        assert!(output.contains("\treturn \"users\""));
    }

    #[test]
    fn test_format_json_code() {
        let input = r#"{"name":"test","value":123}"#;
        let result = CodeFormatter::format_json_code(input);
        assert!(result.is_ok());
        let formatted = result.unwrap();
        assert!(formatted.contains("  \"name\""));
    }

    #[test]
    fn test_format_sql_code() {
        let input = "select name,age from users where id=1";
        let output = CodeFormatter::format_sql_code(input);
        assert!(output.contains("SELECT"));
        assert!(output.contains("FROM"));
    }
}
