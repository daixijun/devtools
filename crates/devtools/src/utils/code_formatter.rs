/// 代码格式化工具集
/// 统一处理各种代码的格式化操作
pub struct CodeFormatter;

impl CodeFormatter {
    /// 格式化 Go 代码
    ///
    /// 大括号块（struct/func）与圆括号块（`import (`、`var (`、`const (`）
    /// 均按 Go 惯例使用制表符缩进；圆括号块内行相对块首行 +1 层。
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
        // 圆括号块深度：块内行输出时相对当前层级再 +1 层缩进
        let mut paren_depth: i32 = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                formatted.push(String::new());
                continue;
            }

            // 圆括号块内：`)` 行结束块（与块首同级），其余行 +1 层
            if paren_depth > 0 {
                if trimmed.starts_with(')') {
                    paren_depth -= 1;
                    formatted.push(Self::format_line_with_indent(trimmed, indent_level));
                    continue;
                }
                formatted.push(Self::format_line_with_indent(trimmed, indent_level + 1));
                continue;
            }

            // 进入圆括号块：块首行保持原级，内容行 +1 层
            if (trimmed.starts_with("import ")
                || trimmed.starts_with("var ")
                || trimmed.starts_with("const "))
                && trimmed.ends_with('(')
            {
                paren_depth += 1;
                formatted.push(trimmed.to_string());
                continue;
            }

            // 处理闭合大括号
            indent_level = Self::adjust_indent_for_closing_brace(trimmed, indent_level);

            // 添加缩进
            let indented = Self::format_line_with_indent(trimmed, indent_level);
            formatted.push(indented);

            // 处理开放大括号
            indent_level = Self::adjust_indent_for_opening_brace(trimmed, indent_level);
        }

        formatted.join("\n")
    }

    /// 闭合大括号行：先降一层缩进，使 `}` 与其配对的开括号语句同级。
    fn adjust_indent_for_closing_brace(trimmed: &str, indent_level: i32) -> i32 {
        if trimmed.starts_with('}') {
            indent_level.saturating_sub(1)
        } else {
            indent_level
        }
    }

    /// 按当前层级输出一行（Go 惯例：制表符缩进）。
    fn format_line_with_indent(trimmed: &str, indent_level: i32) -> String {
        let mut line = String::new();
        for _ in 0..indent_level.max(0) {
            line.push('\t');
        }
        line.push_str(trimmed);
        line
    }

    /// 开放大括号行：行尾 `{` 后升一层缩进。
    fn adjust_indent_for_opening_brace(trimmed: &str, indent_level: i32) -> i32 {
        if trimmed.ends_with('{') {
            indent_level + 1
        } else {
            indent_level
        }
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
    fn test_format_go_import_block() {
        let input = "package schema\n\nimport (\n\"entgo.io/ent\"\n\"time\"\n)\n";
        let output = CodeFormatter::format_go_code(input);
        // import 块首行与 `)` 无缩进，内容行 +1 层制表符
        assert!(output.contains("import ("));
        assert!(output.contains("\t\"entgo.io/ent\""));
        assert!(output.contains("\t\"time\""));
        assert!(output.contains("\n)"));
    }

    #[test]
    fn test_format_go_const_block_inside_func() {
        let input = "func f() int {\nconst (\nA = 1\nB = 2\n)\nreturn A\n}";
        let output = CodeFormatter::format_go_code(input);
        // 函数内 const 块：内容行相对函数体再 +1 层
        assert!(output.contains("\n\t\tA = 1"));
        assert!(output.contains("\n\t\tB = 2"));
        assert!(output.contains("\n\t)\n"));
        assert!(output.contains("\treturn A"));
    }
}
