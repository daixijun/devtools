//! EN 词条：格式转换页（json_to_go / sql_to_go / sql_to_ent / format_converter）。

/// 格式转换页词条表。
pub const EN_PAGES_CONVERT: &[(&str, &str)] = &[
    // json_to_go
    (
        "粘贴 JSON 数据，自动生成 Go 结构体…",
        "Paste JSON data to auto-generate Go structs…",
    ),
    ("结构体名称", "Struct name"),
    ("转换出错", "Conversion error"),
    ("导出字段", "Exported fields"),
    ("标签选择:", "Tags:"),
    ("JSON 输入", "JSON Input"),
    ("Go 结构体输出", "Go Struct Output"),
    // sql_to_go / sql_to_ent
    (
        "输入 CREATE TABLE SQL 语句…",
        "Enter CREATE TABLE SQL statements…",
    ),
    ("无", "None"),
    ("复数表名", "Plural table names"),
    ("JSON 处理:", "JSON handling:"),
    ("标签:", "Tags:"),
    ("SQL 输入", "SQL Input"),
    ("转换失败", "Conversion failed"),
    ("生成边缘关系", "Generate edges"),
    ("生成 Mixin", "Generate Mixin"),
    ("生成 Hooks", "Generate Hooks"),
    ("生成 Policy", "Generate Policy"),
    ("UUID 主键", "UUID primary key"),
    ("软删除", "Soft delete"),
    ("包名:", "Package name:"),
    ("Go Ent Schema 输出", "Go Ent Schema Output"),
    // format_converter
    (
        "粘贴 JSON、YAML 或 TOML 内容...",
        "Paste JSON, YAML or TOML content...",
    ),
    (
        "转换结果将显示在这里...",
        "Converted result will appear here...",
    ),
    (
        "输入和输出格式相同，无需转换",
        "Input and output formats are the same, no conversion needed",
    ),
    ("已复制", "Copied"),
    ("输入格式:", "Input format:"),
    ("输出格式:", "Output format:"),
    ("复制结果", "Copy Result"),
    // 通用（与 en_shell 等通用表保持一致）
    ("示例", "Example"),
    ("清空", "Clear"),
];
