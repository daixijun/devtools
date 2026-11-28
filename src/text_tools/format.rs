//! JSON 格式化 + JSON/YAML/TOML 互转纯函数（含单元测试）。
//!
//! 错误文案必须与历史版本一致（前缀由 UI 层拼接）。

use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// JSON 格式化 / 压缩 / 去除转义
// ---------------------------------------------------------------------------

/// 校验并美化 JSON（缩进 2）。失败返回 `输入的不是有效的JSON格式`（错误前缀由 UI 加）。
pub fn json_pretty(s: &str) -> Result<String, String> {
    let value: Json = parse_json(s)?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

/// 校验并压缩 JSON（去除全部空白）。失败返回 `输入的不是有效的JSON格式`（错误前缀由 UI 加）。
pub fn json_minify(s: &str) -> Result<String, String> {
    let value: Json = parse_json(s)?;
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

/// 去除转义：解析输入，若结果本身是字符串则再解析一次并输出压缩 JSON，
/// 否则输出美化 JSON；解析链失败回退为纯文本替换 `\"` → `"`、`\\` → `\`
/// （保持历史版本 replace 顺序）；回退后必然成功，Err 仅保留给 UI 前缀逻辑。
pub fn json_unescape(s: &str) -> Result<String, String> {
    if let Ok(value) = serde_json::from_str::<Json>(s) {
        match &value {
            Json::String(inner) => {
                // 字符串值：再 parse 一次，输出压缩 JSON
                if let Ok(v2) = serde_json::from_str::<Json>(inner) {
                    return serde_json::to_string(&v2).map_err(|e| e.to_string());
                }
            }
            _ => {
                return serde_json::to_string_pretty(&value).map_err(|e| e.to_string());
            }
        }
    }
    // 回退：纯文本替换，顺序与历史版本一致（先 \" → "，再 \\ → \）
    Ok(s.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn parse_json(s: &str) -> Result<Json, String> {
    serde_json::from_str::<Json>(s).map_err(|_| "输入的不是有效的JSON格式".to_string())
}

// ---------------------------------------------------------------------------
// 格式转换（JSON / YAML / TOML）
// ---------------------------------------------------------------------------

/// 支持的格式（select 的 value 字符串：json / yaml / toml）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Json,
    Yaml,
    Toml,
}

impl Format {
    /// 从 select 的 value 字符串解析。
    pub fn from_value(value: &str) -> Option<Self> {
        match value {
            "json" => Some(Format::Json),
            "yaml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    /// 展示标签。
    pub fn label(self) -> &'static str {
        match self {
            Format::Json => "JSON",
            Format::Yaml => "YAML",
            Format::Toml => "TOML",
        }
    }

    /// 语法高亮语言名（syntect 可识别的小写形式）。
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
        }
    }
}

/// 在 JSON / YAML / TOML 之间转换。
///
/// 解析统一落到中间 `serde_json::Value`；YAML 用 serde_yaml（默认缩进 2），
/// TOML 解析用 toml crate、序列化用手写序列化器（见 `to_toml_string`）。
/// 所有异常统一包裹为 `转换失败: {msg}`；
/// `from == to` 返回 `输入和输出格式相同，无需转换`（正常由 UI 拦截，这里兜底）。
pub fn convert(input: &str, from: Format, to: Format) -> Result<String, String> {
    if from == to {
        return Err("输入和输出格式相同，无需转换".to_string());
    }
    inner_convert(input, from, to).map_err(|msg| format!("转换失败: {msg}"))
}

fn inner_convert(input: &str, from: Format, to: Format) -> Result<String, String> {
    let value: Json = match from {
        Format::Json => serde_json::from_str(input).map_err(|e| e.to_string())?,
        Format::Yaml => {
            let yaml: serde_yaml::Value = serde_yaml::from_str(input).map_err(|e| e.to_string())?;
            yaml_to_json(yaml)?
        }
        Format::Toml => {
            let toml_value: toml::Value = toml::from_str(input).map_err(|e| e.to_string())?;
            toml_to_json(toml_value)
        }
    };
    match to {
        Format::Json => serde_json::to_string_pretty(&value).map_err(|e| e.to_string()),
        Format::Yaml => serde_yaml::to_string(&value).map_err(|e| e.to_string()),
        Format::Toml => Ok(to_toml_string(&value)),
    }
}

/// serde_yaml::Value → serde_json::Value（非标量映射键字符串化，复杂键报错）。
fn yaml_to_json(value: serde_yaml::Value) -> Result<Json, String> {
    Ok(match value {
        serde_yaml::Value::Null => Json::Null,
        serde_yaml::Value::Bool(b) => Json::Bool(b),
        serde_yaml::Value::Number(n) => Json::Number(yaml_number_to_json(&n)?),
        serde_yaml::Value::String(s) => Json::String(s),
        serde_yaml::Value::Sequence(seq) => Json::Array(
            seq.into_iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Number(n) => yaml_number_string(&n),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    serde_yaml::Value::Null => "null".to_string(),
                    other => return Err(format!("不支持的 YAML 映射键类型: {other:?}")),
                };
                obj.insert(key, yaml_to_json(v)?);
            }
            Json::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value)?,
    })
}

fn yaml_number_to_json(n: &serde_yaml::Number) -> Result<serde_json::Number, String> {
    if let Some(i) = n.as_i64() {
        Ok(serde_json::Number::from(i))
    } else if let Some(u) = n.as_u64() {
        Ok(serde_json::Number::from(u))
    } else {
        let f = n.as_f64().ok_or_else(|| "无法解析 YAML 数字".to_string())?;
        serde_json::Number::from_f64(f).ok_or_else(|| format!("无法表示的 YAML 数字: {f}"))
    }
}

fn yaml_number_string(n: &serde_yaml::Number) -> String {
    if let Some(i) = n.as_i64() {
        i.to_string()
    } else if let Some(u) = n.as_u64() {
        u.to_string()
    } else {
        n.as_f64()
            .map(|f| f.to_string())
            .unwrap_or_else(|| "0".to_string())
    }
}

/// toml::Value → serde_json::Value（Datetime 字符串化）。
fn toml_to_json(value: toml::Value) -> Json {
    match value {
        toml::Value::String(s) => Json::String(s),
        toml::Value::Integer(i) => Json::Number(i.into()),
        toml::Value::Float(f) => Json::Number(
            serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
        ),
        toml::Value::Boolean(b) => Json::Bool(b),
        toml::Value::Datetime(d) => Json::String(d.to_string()),
        toml::Value::Array(arr) => Json::Array(arr.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => Json::Object(
            table
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect(),
        ),
    }
}

// ---------------------------------------------------------------------------
// 手写 TOML 序列化器
// ---------------------------------------------------------------------------

/// 手写 TOML 序列化器，规则：
/// - 顶层标量/数组直接按值输出；对象按表拆分；
/// - 数组：空 → `[]`；元素同类型且非对象 → 内联 `[a, b]`；对象数组/混合数组 → 多行 `[\n  <递归>,\n]`
///   （JS `typeof null === 'object'`、`typeof [] === 'object'`，两者都走多行分支）；
/// - 对象：先简单键（进入子表时输出 `[table.path]` 表头）+ 空行分隔，再递归子表 `[parent.child]`；
/// - 键含 `[^a-zA-Z0-9_-]` 时双引号包裹（仅转义 `"`）；
/// - 字符串值双引号包裹，转义顺序保持历史版本：先 `"` → `\"` 再 `\` → `\\`；
/// - null 输出字面 `null`（非合法 TOML，仅为对齐旧行为）。
fn to_toml_string(value: &Json) -> String {
    match value {
        Json::Object(map) => serialize_toml_table(map, ""),
        other => format_toml_value(other),
    }
}

/// 对象分支（`currentTable` → `path`）。
fn serialize_toml_table(table: &serde_json::Map<String, Json>, path: &str) -> String {
    let mut simple: Vec<(&String, &Json)> = Vec::new();
    let mut subtables: Vec<(&String, &Json)> = Vec::new();
    for (key, value) in table {
        if matches!(value, Json::Object(_)) {
            subtables.push((key, value));
        } else {
            simple.push((key, value));
        }
    }

    let mut lines: Vec<String> = Vec::new();
    if !simple.is_empty() {
        if !path.is_empty() {
            lines.push(format!("[{path}]"));
        }
        for (key, value) in &simple {
            lines.push(format!(
                "{} = {}",
                format_toml_key(key),
                format_toml_value(value)
            ));
        }
        if !subtables.is_empty() {
            lines.push(String::new()); // 空行分隔简单键与子表
        }
    }

    for (key, value) in &subtables {
        let Json::Object(sub) = value else {
            continue;
        };
        let sub_path = if path.is_empty() {
            (*key).clone()
        } else {
            format!("{path}.{key}")
        };
        let content = serialize_toml_table(sub, &sub_path);
        if !content.trim().is_empty() {
            lines.push(content);
        }
    }

    lines.join("\n")
}

/// 数组分支。
fn format_toml_value(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.to_string(),
        Json::String(s) => format!("\"{}\"", escape_toml_string(s)),
        Json::Array(items) => format_toml_array(items),
        Json::Object(map) => serialize_toml_table(map, ""),
    }
}

fn format_toml_array(items: &[Json]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let first_kind = js_typeof(&items[0]);
    let homogeneous = items.iter().all(|item| js_typeof(item) == first_kind);
    if homogeneous && first_kind != "object" {
        // 基本类型数组：内联
        let inner: Vec<String> = items.iter().map(format_toml_value).collect();
        format!("[{}]", inner.join(", "))
    } else {
        // 对象数组或混合数组：多行
        let inner: Vec<String> = items
            .iter()
            .map(|item| format!("  {}", to_toml_string(item)))
            .collect();
        format!("[\n{}\n]", inner.join(",\n"))
    }
}

/// 模拟 JS `typeof`（null 与数组都归为 "object"）。
fn js_typeof(value: &Json) -> &'static str {
    match value {
        Json::Null | Json::Array(_) | Json::Object(_) => "object",
        Json::Bool(_) => "boolean",
        Json::Number(_) => "number",
        Json::String(_) => "string",
    }
}

/// 键名格式化：含 `[^a-zA-Z0-9_-]` 时双引号包裹（仅转义 `"`）。
fn format_toml_key(key: &str) -> String {
    let needs_quoting = key
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    if needs_quoting {
        format!("\"{}\"", key.replace('"', "\\\""))
    } else {
        key.to_string()
    }
}

/// 字符串值转义，保持历史版本顺序：先 `"` → `\"`，再 `\` → `\\`。
fn escape_toml_string(s: &str) -> String {
    s.replace('"', "\\\"").replace('\\', "\\\\")
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ----- JSON 格式化 / 压缩 -----

    #[test]
    fn json_pretty_formats_with_indent_2() {
        let out = json_pretty("{\"a\":1,\"b\":[1,2]}").unwrap();
        assert_eq!(out, "{\n  \"a\": 1,\n  \"b\": [\n    1,\n    2\n  ]\n}");
    }

    #[test]
    fn json_minify_strips_whitespace() {
        let out = json_minify("{\n  \"a\": 1,\n  \"b\": \"x\"\n}").unwrap();
        assert_eq!(out, "{\"a\":1,\"b\":\"x\"}");
    }

    #[test]
    fn json_pretty_rejects_invalid_input() {
        assert_eq!(
            json_pretty("not json"),
            Err("输入的不是有效的JSON格式".to_string())
        );
        assert_eq!(
            json_minify("{a:1}"),
            Err("输入的不是有效的JSON格式".to_string())
        );
    }

    // ----- 去除转义 -----

    #[test]
    fn json_unescape_string_value_outputs_minified() {
        // 输入是一个 JSON 字符串值，内容本身是合法 JSON
        let out = json_unescape(r#""{\"a\": 1, \"b\": [1, 2]}""#).unwrap();
        assert_eq!(out, r#"{"a":1,"b":[1,2]}"#);
    }

    #[test]
    fn json_unescape_object_outputs_pretty() {
        let out = json_unescape("{\"a\":1}").unwrap();
        assert_eq!(out, "{\n  \"a\": 1\n}");
    }

    #[test]
    fn json_unescape_falls_back_to_text_replace() {
        // 非 JSON 文本且字符串内容也非 JSON → 回退纯文本替换
        let out = json_unescape(r#"hello \"world\""#).unwrap();
        assert_eq!(out, r#"hello "world""#);
    }

    #[test]
    fn json_unescape_replaces_double_backslash_last() {
        // 保持历史版本顺序：先 \" → " 再 \\ → \
        let out = json_unescape(r#"C:\\path\\file"#).unwrap();
        assert_eq!(out, r"C:\path\file");
    }

    // ----- 格式转换：往返 -----

    #[test]
    fn convert_json_yaml_json_round_trip() {
        let src = json!({
            "name": "张三",
            "age": 30,
            "active": true,
            "tags": ["a", "b"],
            "nested": { "x": 1.5, "y": null }
        });
        let text = serde_json::to_string_pretty(&src).unwrap();
        let yaml = convert(&text, Format::Json, Format::Yaml).unwrap();
        let back = convert(&yaml, Format::Yaml, Format::Json).unwrap();
        let back_value: Json = serde_json::from_str(&back).unwrap();
        assert_eq!(back_value, src);
    }

    #[test]
    fn convert_json_toml_json_round_trip() {
        // 不含 null 与对象数组（手写序列化器此场景输出合法 TOML）
        let src = json!({
            "title": "demo",
            "count": 3,
            "rate": 1.5,
            "enabled": true,
            "tags": ["a", "b", "c"],
            "db": { "host": "localhost", "port": 5432 }
        });
        let text = serde_json::to_string_pretty(&src).unwrap();
        let toml_text = convert(&text, Format::Json, Format::Toml).unwrap();
        let back = convert(&toml_text, Format::Toml, Format::Json).unwrap();
        let back_value: Json = serde_json::from_str(&back).unwrap();
        assert_eq!(back_value, src);
    }

    #[test]
    fn convert_toml_json_toml_round_trip() {
        let toml_src = "title = \"demo\"\n\n[owner]\nname = \"dev\"\nport = 8080\n";
        let json_text = convert(toml_src, Format::Toml, Format::Json).unwrap();
        let back = convert(&json_text, Format::Json, Format::Toml).unwrap();
        let a: toml::Value = toml::from_str(toml_src).unwrap();
        let b: toml::Value = toml::from_str(&back).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn convert_yaml_to_json_maps_types() {
        let out = convert("a: 1\nb: hello\nc: true\n", Format::Yaml, Format::Json).unwrap();
        let value: Json = serde_json::from_str(&out).unwrap();
        assert_eq!(value, json!({"a": 1, "b": "hello", "c": true}));
    }

    #[test]
    fn convert_json_to_yaml_known_output() {
        let out = convert("{\"a\":1}", Format::Json, Format::Yaml).unwrap();
        assert_eq!(out, "a: 1\n");
    }

    // ----- 格式转换：错误 -----

    #[test]
    fn convert_same_format_is_rejected() {
        assert_eq!(
            convert("{}", Format::Json, Format::Json),
            Err("输入和输出格式相同，无需转换".to_string())
        );
    }

    #[test]
    fn convert_wraps_parse_errors() {
        let err = convert("not json", Format::Json, Format::Yaml).unwrap_err();
        assert!(err.starts_with("转换失败: "), "got: {err}");
    }

    // ----- 手写 TOML 序列化器（结构化用例） -----

    #[test]
    fn toml_simple_keys_then_subtable() {
        // 简单键在前，空行分隔，子表在后
        let src = json!({
            "title": "hello",
            "db": { "host": "localhost", "port": 5432 }
        });
        let expected = "title = \"hello\"\n\n[db]\nhost = \"localhost\"\nport = 5432";
        assert_eq!(to_toml_string(&src), expected);
    }

    #[test]
    fn toml_basic_and_nested_tables() {
        let src = json!({
            "count": 3,
            "mixed": [1, "x"],
            "tags": ["a", "b"],
            "title": "hello",
            "db": { "host": "localhost", "port": 5432 }
        });
        let expected = "count = 3\n\
                        mixed = [\n  1,\n  \"x\"\n]\n\
                        tags = [\"a\", \"b\"]\n\
                        title = \"hello\"\n\
                        \n\
                        [db]\n\
                        host = \"localhost\"\n\
                        port = 5432";
        assert_eq!(to_toml_string(&src), expected);
    }

    #[test]
    fn toml_deep_nesting_uses_table_path_header() {
        let src = json!({ "server": { "logging": { "level": "info" } } });
        assert_eq!(to_toml_string(&src), "[server.logging]\nlevel = \"info\"");
    }

    #[test]
    fn toml_inline_array_of_scalars() {
        let src = json!({ "nums": [1, 2, 3] });
        assert_eq!(to_toml_string(&src), "nums = [1, 2, 3]");
    }

    #[test]
    fn toml_empty_array_is_inline_empty() {
        let src = json!({ "empty": [] });
        assert_eq!(to_toml_string(&src), "empty = []");
    }

    #[test]
    fn toml_object_array_is_multiline() {
        let src = json!({ "users": [ { "name": "a" }, { "name": "b" } ] });
        let expected = "users = [\n  name = \"a\",\n  name = \"b\"\n]";
        assert_eq!(to_toml_string(&src), expected);
    }

    #[test]
    fn toml_null_outputs_null_literal() {
        let src = json!({ "x": null });
        assert_eq!(to_toml_string(&src), "x = null");
    }

    #[test]
    fn toml_string_escape_order_matches_legacy() {
        // 值 a\b"c：先 " → \"（a\b\"c），再 \ → \\（插入的反斜杠也被翻倍）
        // → a\\b\\"c。该输出经 node 模拟旧 TSX 链式 replace 验证一致。
        let src = json!({ "s": "a\\b\"c" });
        assert_eq!(to_toml_string(&src), "s = \"a\\\\b\\\\\"c\"");
    }

    #[test]
    fn toml_special_keys_are_quoted() {
        // serde_json 开启 preserve_order：键保持插入序
        let src = json!({ "my key": 1, "normal": 2, "dash-key": 3, "under_key": 4 });
        let expected = "\"my key\" = 1\nnormal = 2\ndash-key = 3\nunder_key = 4";
        assert_eq!(to_toml_string(&src), expected);
    }

    #[test]
    fn toml_scalar_root_is_value_only() {
        assert_eq!(to_toml_string(&json!("text")), "\"text\"");
        assert_eq!(to_toml_string(&json!(42)), "42");
    }
}
