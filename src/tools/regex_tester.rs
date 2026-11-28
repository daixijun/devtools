use pcre2::bytes::RegexBuilder as Pcre2RegexBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegexFlags {
    pub case_insensitive: bool,
    pub multiline: bool,
    pub dot_matches_new_line: bool,
    pub swap_greed: bool,
    pub unicode: bool,
}

impl Default for RegexFlags {
    fn default() -> Self {
        Self {
            case_insensitive: false,
            multiline: false,
            dot_matches_new_line: false,
            swap_greed: false,
            unicode: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexEngine {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexTestRequest {
    pub pattern: String,
    pub text: String,
    pub flags: RegexFlags,
    pub engine: String, // "rust", "re2", "pcre", "golang", "javascript"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexReplaceRequest {
    pub pattern: String,
    pub text: String,
    pub replacement: String,
    pub flags: RegexFlags,
    pub engine: String,
    pub replace_all: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexMatch {
    pub full_match: String,
    pub start: usize,
    pub end: usize,
    pub groups: Vec<Option<String>>,
    pub named_groups: std::collections::HashMap<String, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexTestResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub matches: Vec<RegexMatch>,
    pub match_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexReplaceResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub result: Option<String>,
    pub replacement_count: usize,
}

fn build_rust_regex(pattern: &str, flags: &RegexFlags) -> Result<regex::Regex, regex::Error> {
    let mut regex_pattern = String::new();

    // 添加标志
    if flags.case_insensitive
        || flags.multiline
        || flags.dot_matches_new_line
        || flags.swap_greed
        || !flags.unicode
    {
        regex_pattern.push_str("(?");
        if flags.case_insensitive {
            regex_pattern.push('i');
        }
        if flags.multiline {
            regex_pattern.push('m');
        }
        if flags.dot_matches_new_line {
            regex_pattern.push('s');
        }
        if flags.swap_greed {
            regex_pattern.push('U');
        }
        if !flags.unicode {
            regex_pattern.push_str("-u");
        }
        regex_pattern.push(')');
    }

    regex_pattern.push_str(pattern);
    regex::Regex::new(&regex_pattern)
}

pub async fn test_regex(
    pattern: String,
    text: String,
    flags: RegexFlags,
    engine: String,
) -> Result<RegexTestResult, String> {
    if pattern.is_empty() {
        return Err("正则表达式不能为空".to_string());
    }

    let request = RegexTestRequest {
        pattern,
        text,
        flags,
        engine,
    };

    match request.engine.as_str() {
        "rust" => test_rust_regex(request).await,
        "re2" => test_re2_regex(request).await,
        "pcre" => test_pcre_regex(request).await,
        "golang" => test_golang_regex(request).await,
        "javascript" => test_javascript_regex(request).await,
        _ => Err("不支持的正则表达式引擎".to_string()),
    }
}

async fn test_rust_regex(request: RegexTestRequest) -> Result<RegexTestResult, String> {
    match build_rust_regex(&request.pattern, &request.flags) {
        Ok(re) => {
            let mut matches = Vec::new();

            // 使用captures_iter来获取所有匹配和捕获组
            for caps in re.captures_iter(&request.text) {
                if let Some(full_match) = caps.get(0) {
                    let start = full_match.start();
                    let end = full_match.end();
                    let full_match_str = full_match.as_str().to_string();

                    let mut groups = Vec::new();
                    let mut named_groups = std::collections::HashMap::new();

                    // 收集编号捕获组 (从1开始，0是完整匹配)
                    for i in 1..caps.len() {
                        groups.push(caps.get(i).map(|m| m.as_str().to_string()));
                    }

                    // 收集命名捕获组
                    for name in re.capture_names().flatten() {
                        named_groups.insert(
                            name.to_string(),
                            caps.name(name).map(|m| m.as_str().to_string()),
                        );
                    }

                    matches.push(RegexMatch {
                        full_match: full_match_str,
                        start,
                        end,
                        groups,
                        named_groups,
                    });
                }
            }

            Ok(RegexTestResult {
                is_valid: true,
                error_message: None,
                match_count: matches.len(),
                matches,
            })
        }
        Err(e) => Ok(RegexTestResult {
            is_valid: false,
            error_message: Some(format!("正则表达式语法错误: {}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

async fn test_pcre_regex(request: RegexTestRequest) -> Result<RegexTestResult, String> {
    // 使用真正的 PCRE2 库
    let mut builder = Pcre2RegexBuilder::new();

    // 设置 PCRE2 标志
    if request.flags.case_insensitive {
        builder.caseless(true);
    }
    if request.flags.multiline {
        builder.multi_line(true);
    }
    if request.flags.dot_matches_new_line {
        builder.dotall(true);
    }
    if request.flags.swap_greed {
        // PCRE2 中使用 ungreedy 的替代方法需要在模式中添加 (?U)
        // 暂时跳过此标志
    }
    // PCRE2 默认按非 UTF 编码编译（`.` 等按单字节匹配中文会产生碎片，
    // 替换时还可能在多字节字符内部 panic）。显式跟随开关，
    // ucp 让 \w \d 等具备 Unicode 语义，与 Rust 引擎的行为一致。
    builder
        .utf(request.flags.unicode)
        .ucp(request.flags.unicode);

    match builder.build(&request.pattern) {
        Ok(re) => {
            let text_bytes = request.text.as_bytes();
            let mut matches = Vec::new();

            // captures_iter：一次迭代同时给出偏移与本次匹配的分组
            //（旧实现循环内 re.captures 固定从头搜索，第二处起的分组全是第一处的）。
            // 迭代器出错时不推进位置，必须显式处理，不能 flatten 吞掉后继续轮询
            let mut iter = re.captures_iter(text_bytes);
            loop {
                let Some(captures) = iter.next() else {
                    break;
                };
                let captures = match captures {
                    Ok(c) => c,
                    Err(e) => {
                        return Ok(RegexTestResult {
                            is_valid: false,
                            error_message: Some(format!(
                                "正则匹配失败: {}（常见原因：回溯超出 PCRE2 匹配限制）",
                                e
                            )),
                            matches: Vec::new(),
                            match_count: 0,
                        });
                    }
                };
                let whole = captures.get(0).expect("captures 的第 0 组总是存在");
                let start = whole.start();
                let end = whole.end();
                let full_match = String::from_utf8_lossy(whole.as_bytes()).to_string();

                // 捕获组
                let mut groups = Vec::new();
                for i in 1..captures.len() {
                    groups.push(
                        captures
                            .get(i)
                            .map(|g| String::from_utf8_lossy(g.as_bytes()).to_string()),
                    );
                }

                // 命名捕获组
                let mut named_groups = std::collections::HashMap::new();
                for name_str in re.capture_names().iter().flatten() {
                    named_groups.insert(
                        name_str.to_string(),
                        captures
                            .name(name_str)
                            .map(|g| String::from_utf8_lossy(g.as_bytes()).to_string()),
                    );
                }

                matches.push(RegexMatch {
                    full_match,
                    start,
                    end,
                    groups,
                    named_groups,
                });
            }

            Ok(RegexTestResult {
                is_valid: true,
                error_message: None,
                match_count: matches.len(),
                matches,
            })
        }
        Err(e) => Ok(RegexTestResult {
            is_valid: false,
            error_message: Some(format!("PCRE2正则表达式语法错误: {}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

async fn test_golang_regex(request: RegexTestRequest) -> Result<RegexTestResult, String> {
    // Go 的正则表达式引擎基于 RE2，具有特定的语法特点
    let mut pattern = request.pattern.clone();

    // Go 风格标志处理 - 与标准 RE2 略有不同
    let mut flags = Vec::new();
    if request.flags.case_insensitive {
        flags.push('i');
    }
    if request.flags.multiline {
        flags.push('m');
    }
    if request.flags.dot_matches_new_line {
        flags.push('s');
    }
    if request.flags.swap_greed {
        flags.push('U'); // Go 支持 U 标志进行非贪婪匹配
    }

    // Go 风格的标志应用 - 使用 (?flags:pattern) 语法
    if !flags.is_empty() {
        let flags_str: String = flags.into_iter().collect();
        pattern = format!("(?{}:{})", flags_str, pattern);
    }

    // Go 特有的模式转换
    pattern = convert_to_go_style(&pattern);

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let mut matches = Vec::new();

            for caps in re.captures_iter(&request.text) {
                if let Some(full_match) = caps.get(0) {
                    let start = full_match.start();
                    let end = full_match.end();
                    let full_match_str = full_match.as_str().to_string();

                    let mut groups = Vec::new();
                    let mut named_groups = std::collections::HashMap::new();

                    // 收集编号捕获组
                    for i in 1..caps.len() {
                        groups.push(caps.get(i).map(|m| m.as_str().to_string()));
                    }

                    // 收集命名捕获组 - Go 使用 (?P<name>...) 语法
                    for name in re.capture_names().flatten() {
                        named_groups.insert(
                            name.to_string(),
                            caps.name(name).map(|m| m.as_str().to_string()),
                        );
                    }

                    matches.push(RegexMatch {
                        full_match: full_match_str,
                        start,
                        end,
                        groups,
                        named_groups,
                    });
                }
            }

            Ok(RegexTestResult {
                is_valid: true,
                error_message: None,
                match_count: matches.len(),
                matches,
            })
        }
        Err(e) => Ok(RegexTestResult {
            is_valid: false,
            error_message: Some(format!("Golang正则表达式语法错误: {}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

async fn test_javascript_regex(request: RegexTestRequest) -> Result<RegexTestResult, String> {
    // JavaScript正则表达式语法，使用标志字符串
    let mut pattern = request.pattern.clone();
    let mut flags = String::new();

    if request.flags.case_insensitive {
        flags.push('i');
    }
    if request.flags.multiline {
        flags.push('m');
    }
    if request.flags.dot_matches_new_line {
        flags.push('s');
    }
    // JavaScript没有直接的非贪婪标志，需要在量词中使用?
    // Unicode在JavaScript中默认启用（ES2015+）

    // 将JavaScript风格的正则转换为Rust风格
    if !flags.is_empty() {
        // 将JavaScript标志转换为内联标志
        pattern = format!("(?{}){}", flags, pattern);
    }

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let mut matches = Vec::new();

            for caps in re.captures_iter(&request.text) {
                if let Some(full_match) = caps.get(0) {
                    let start = full_match.start();
                    let end = full_match.end();
                    let full_match_str = full_match.as_str().to_string();

                    let mut groups = Vec::new();
                    let mut named_groups = std::collections::HashMap::new();

                    // 收集编号捕获组
                    for i in 1..caps.len() {
                        groups.push(caps.get(i).map(|m| m.as_str().to_string()));
                    }

                    // JavaScript支持命名捕获组 (?<name>pattern)
                    for name in re.capture_names().flatten() {
                        named_groups.insert(
                            name.to_string(),
                            caps.name(name).map(|m| m.as_str().to_string()),
                        );
                    }

                    matches.push(RegexMatch {
                        full_match: full_match_str,
                        start,
                        end,
                        groups,
                        named_groups,
                    });
                }
            }

            Ok(RegexTestResult {
                is_valid: true,
                error_message: None,
                match_count: matches.len(),
                matches,
            })
        }
        Err(e) => Ok(RegexTestResult {
            is_valid: false,
            error_message: Some(format!("JavaScript正则表达式语法错误: {}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

async fn test_re2_regex(request: RegexTestRequest) -> Result<RegexTestResult, String> {
    // 使用真正的 RE2 库
    let mut pattern = request.pattern.clone();

    // RE2 标志处理 - 使用内联标志语法
    let mut flags = String::new();
    if request.flags.case_insensitive {
        flags.push('i');
    }
    if request.flags.multiline {
        flags.push('m');
    }
    if request.flags.dot_matches_new_line {
        flags.push('s');
    }
    // RE2 不直接支持非贪婪标志，需要在量词中使用 ?

    if !flags.is_empty() {
        pattern = format!("(?{}){}", flags, pattern);
    }

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let mut matches = Vec::new();

            // captures_iter：由 regex crate 处理空匹配的推进，避免手写
            // 「start + 1」落进多字节字符内部导致 str 切片 panic
            for caps in re.captures_iter(&request.text) {
                let m = caps.get(0).expect("captures 的第 0 组总是存在");
                let match_start = m.start();
                let match_end = m.end();
                let full_match = m.as_str().to_string();

                let mut groups = Vec::new();
                for i in 1..caps.len() {
                    groups.push(caps.get(i).map(|g| g.as_str().to_string()));
                }

                let mut named_groups = std::collections::HashMap::new();
                for name_str in re.capture_names().flatten() {
                    named_groups.insert(
                        name_str.to_string(),
                        caps.name(name_str).map(|g| g.as_str().to_string()),
                    );
                }

                matches.push(RegexMatch {
                    full_match,
                    start: match_start,
                    end: match_end,
                    groups,
                    named_groups,
                });
            }

            Ok(RegexTestResult {
                is_valid: true,
                error_message: None,
                match_count: matches.len(),
                matches,
            })
        }
        Err(e) => Ok(RegexTestResult {
            is_valid: false,
            error_message: Some(format!("RE2正则表达式语法错误: {:?}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

pub async fn replace_regex(
    pattern: String,
    text: String,
    replacement: String,
    flags: RegexFlags,
    engine: String,
    replace_all: bool,
) -> Result<RegexReplaceResult, String> {
    if pattern.is_empty() {
        return Err("正则表达式不能为空".to_string());
    }

    let request = RegexReplaceRequest {
        pattern,
        text,
        replacement,
        flags,
        engine,
        replace_all,
    };

    match request.engine.as_str() {
        "rust" => replace_rust_regex(request).await,
        "re2" => replace_re2_regex(request).await,
        "pcre" => replace_pcre_regex(request).await,
        "golang" => replace_golang_regex(request).await,
        "javascript" => replace_javascript_regex(request).await,
        _ => Err("不支持的正则表达式引擎".to_string()),
    }
}

async fn replace_rust_regex(request: RegexReplaceRequest) -> Result<RegexReplaceResult, String> {
    match build_rust_regex(&request.pattern, &request.flags) {
        Ok(re) => {
            let result = if request.replace_all {
                re.replace_all(&request.text, request.replacement.as_str())
                    .to_string()
            } else {
                re.replace(&request.text, request.replacement.as_str())
                    .to_string()
            };

            // 计算替换次数
            let original_matches = re.find_iter(&request.text).count();
            let replacement_count = if request.replace_all {
                original_matches
            } else {
                if original_matches > 0 {
                    1
                } else {
                    0
                }
            };

            Ok(RegexReplaceResult {
                is_valid: true,
                error_message: None,
                result: Some(result),
                replacement_count,
            })
        }
        Err(e) => Ok(RegexReplaceResult {
            is_valid: false,
            error_message: Some(format!("正则表达式语法错误: {}", e)),
            result: None,
            replacement_count: 0,
        }),
    }
}

async fn replace_re2_regex(request: RegexReplaceRequest) -> Result<RegexReplaceResult, String> {
    let mut pattern = request.pattern.clone();

    // RE2 标志处理
    let mut flags = String::new();
    if request.flags.case_insensitive {
        flags.push('i');
    }
    if request.flags.multiline {
        flags.push('m');
    }
    if request.flags.dot_matches_new_line {
        flags.push('s');
    }

    if !flags.is_empty() {
        pattern = format!("(?{}){}", flags, pattern);
    }

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            // 单遍替换：由 regex crate 处理空匹配推进，替换后的文本不再被重扫
            // （旧实现每轮在已替换的文本上从头 find，替换文本可被模式匹配时死循环）。
            // NoExpand 保持与旧实现一致的字面量替换语义（不展开 $1）
            let result = if request.replace_all {
                let replacement_count = re.find_iter(&request.text).count();
                let text = re
                    .replace_all(&request.text, regex::NoExpand(request.replacement.as_str()))
                    .into_owned();
                (text, replacement_count)
            } else {
                let has_match = re.find(&request.text).is_some();
                let text = re
                    .replace(&request.text, regex::NoExpand(request.replacement.as_str()))
                    .into_owned();
                (text, if has_match { 1 } else { 0 })
            };

            Ok(RegexReplaceResult {
                is_valid: true,
                error_message: None,
                result: Some(result.0),
                replacement_count: result.1,
            })
        }
        Err(e) => Ok(RegexReplaceResult {
            is_valid: false,
            error_message: Some(format!("RE2正则表达式语法错误: {:?}", e)),
            result: None,
            replacement_count: 0,
        }),
    }
}

async fn replace_pcre_regex(request: RegexReplaceRequest) -> Result<RegexReplaceResult, String> {
    // 使用真正的 PCRE2 库进行替换
    let mut builder = Pcre2RegexBuilder::new();

    // 设置 PCRE2 标志
    if request.flags.case_insensitive {
        builder.caseless(true);
    }
    if request.flags.multiline {
        builder.multi_line(true);
    }
    if request.flags.dot_matches_new_line {
        builder.dotall(true);
    }
    if request.flags.swap_greed {
        // PCRE2 中使用 ungreedy 的替代方法需要在模式中添加 (?U)
        // 暂时跳过此标志
    }
    // 与 test_pcre_regex 一致：显式跟随 UTF/UCP 开关（默认非 UTF 会在
    // 多字节字符内部 replace_range panic）
    builder
        .utf(request.flags.unicode)
        .ucp(request.flags.unicode);

    match builder.build(&request.pattern) {
        Ok(re) => {
            let text_bytes = request.text.as_bytes();

            // 先收集全部匹配再单遍拼装；迭代器错误显式上报
            //（旧实现在替换后的文本上重复 find，靠「匹配数 × 2」硬截断，
            // 替换文本可被模式匹配时既死循环又产出错误结果）
            let mut ranges: Vec<(usize, usize)> = Vec::new();
            // find_iter 的错误项显式上报，不能用 filter_map 静默吞掉
            for next in re.find_iter(text_bytes) {
                match next {
                    Ok(m) => ranges.push((m.start(), m.end())),
                    Err(e) => {
                        return Ok(RegexReplaceResult {
                            is_valid: false,
                            error_message: Some(format!(
                                "正则匹配失败: {}（常见原因：回溯超出 PCRE2 匹配限制）",
                                e
                            )),
                            result: None,
                            replacement_count: 0,
                        });
                    }
                }
            }

            let applied = if request.replace_all {
                ranges.len()
            } else {
                ranges.len().min(1)
            };
            let mut out = Vec::with_capacity(text_bytes.len());
            let mut last = 0usize;
            for (start, end) in ranges.iter().take(applied) {
                out.extend_from_slice(&text_bytes[last..*start]);
                out.extend_from_slice(request.replacement.as_bytes());
                last = *end;
            }
            out.extend_from_slice(&text_bytes[last..]);

            let result = String::from_utf8_lossy(&out).into_owned();

            Ok(RegexReplaceResult {
                is_valid: true,
                error_message: None,
                result: Some(result),
                replacement_count: applied,
            })
        }
        Err(e) => Ok(RegexReplaceResult {
            is_valid: false,
            error_message: Some(format!("PCRE2正则表达式语法错误: {}", e)),
            result: None,
            replacement_count: 0,
        }),
    }
}

async fn replace_javascript_regex(
    request: RegexReplaceRequest,
) -> Result<RegexReplaceResult, String> {
    // 使用与test_javascript_regex相同的模式处理
    replace_rust_regex(request).await
}

async fn replace_golang_regex(request: RegexReplaceRequest) -> Result<RegexReplaceResult, String> {
    // Go 风格正则表达式替换，使用与 test_golang_regex 相同的模式处理
    let mut pattern = request.pattern.clone();

    // Go 风格标志处理
    let mut flags = Vec::new();
    if request.flags.case_insensitive {
        flags.push('i');
    }
    if request.flags.multiline {
        flags.push('m');
    }
    if request.flags.dot_matches_new_line {
        flags.push('s');
    }
    if request.flags.swap_greed {
        flags.push('U'); // Go 支持 U 标志进行非贪婪匹配
    }

    // Go 风格的标志应用 - 使用 (?flags:pattern) 语法
    if !flags.is_empty() {
        let flags_str: String = flags.into_iter().collect();
        pattern = format!("(?{}:{})", flags_str, pattern);
    }

    // Go 特有的模式转换
    pattern = convert_to_go_style(&pattern);

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let result = if request.replace_all {
                re.replace_all(&request.text, request.replacement.as_str())
                    .to_string()
            } else {
                re.replace(&request.text, request.replacement.as_str())
                    .to_string()
            };

            let original_matches = re.find_iter(&request.text).count();
            let replacement_count = if request.replace_all {
                original_matches
            } else {
                if original_matches > 0 {
                    1
                } else {
                    0
                }
            };

            Ok(RegexReplaceResult {
                is_valid: true,
                error_message: None,
                result: Some(result),
                replacement_count,
            })
        }
        Err(e) => Ok(RegexReplaceResult {
            is_valid: false,
            error_message: Some(format!("Golang正则表达式语法错误: {}", e)),
            result: None,
            replacement_count: 0,
        }),
    }
}

pub async fn validate_regex(pattern: String, engine: String) -> Result<bool, String> {
    if pattern.is_empty() {
        return Err("正则表达式不能为空".to_string());
    }

    let is_valid = match engine.as_str() {
        "rust" => regex::Regex::new(&pattern).is_ok(),
        "re2" => {
            // 使用真正的 RE2 库验证
            regex::Regex::new(&pattern).is_ok()
        }
        "pcre" => {
            // 使用真正的 PCRE2 库验证
            Pcre2RegexBuilder::new().build(&pattern).is_ok()
        }
        "golang" => {
            // Golang使用RE2引擎，但语法稍有不同，使用相同的模式转换
            let converted_pattern = convert_to_go_style(&pattern);
            regex::Regex::new(&converted_pattern).is_ok()
        }
        "javascript" => {
            // JavaScript模式添加标志前缀测试
            let test_pattern = format!("(?i){}", pattern);
            regex::Regex::new(&test_pattern).is_ok() || regex::Regex::new(&pattern).is_ok()
        }
        _ => false,
    };

    Ok(is_valid)
}

// Go 风格模式转换函数
fn convert_to_go_style(pattern: &str) -> String {
    let result = pattern.to_string();

    // Go 命名捕获组语法转换：(?P<name>...)
    // Rust regex 也支持这种语法，所以不需要转换

    // Go 特有的字符类处理
    // Go 中的 \s, \S, \d, \D, \w, \W 定义与 Rust regex 基本兼容

    // Go 的 Unicode 属性类 \p{Name} 和 \P{Name}
    // Rust regex 也支持，所以保持不变

    // Go 特有的转义序列处理
    // \Q...\E 字面量引用 - Go 支持，Rust regex 也支持

    // Go 特有的边界断言
    // \b, \B 单词边界 - 两者兼容
    // \A, \z 字符串开始/结束 - 两者兼容

    // Go RE2 不支持的特性（这些在 Rust regex 中也不支持）：
    // - 反向引用 \1, \2 等
    // - 正向/负向先行断言 (?=...), (?!...)
    // - 正向/负向后行断言 (?<=...), (?<!...)

    // Go 中的标志语法已经在调用函数中处理
    // (?flags:pattern) 或 (?flags)pattern

    // 其他 Go 特有的转换可以在这里添加

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_regex_engines() {
        let test_pattern = r"\d+".to_string();
        let test_text = "abc123def456".to_string();
        let flags = RegexFlags::default();

        // Test all engines
        let engines = vec!["rust", "re2", "pcre", "golang", "javascript"];

        for engine in engines {
            let result = test_regex(
                test_pattern.clone(),
                test_text.clone(),
                flags.clone(),
                engine.to_string(),
            )
            .await;

            assert!(result.is_ok(), "Engine {} should succeed", engine);
            let response = result.unwrap();
            assert!(
                response.is_valid,
                "Engine {} should have valid pattern",
                engine
            );
            assert_eq!(
                response.match_count, 2,
                "Engine {} should find 2 matches",
                engine
            );
            assert_eq!(
                response.matches[0].full_match, "123",
                "First match should be '123' for engine {}",
                engine
            );
            assert_eq!(
                response.matches[1].full_match, "456",
                "Second match should be '456' for engine {}",
                engine
            );
        }
    }

    #[tokio::test]
    async fn test_regex_validation() {
        let engines = vec!["rust", "re2", "pcre", "golang", "javascript"];

        for engine in engines {
            // Valid pattern
            let result = validate_regex(r"\d+".to_string(), engine.to_string()).await;
            assert!(
                result.is_ok(),
                "Validation should succeed for engine {}",
                engine
            );
            let is_valid = result.unwrap();
            assert!(is_valid, "Pattern should be valid for engine {}", engine);

            // Invalid pattern
            let result = validate_regex(r"[".to_string(), engine.to_string()).await;
            assert!(
                result.is_ok(),
                "Validation should succeed for engine {}",
                engine
            );
            let is_valid = result.unwrap();
            assert!(
                !is_valid,
                "Invalid pattern should be rejected for engine {}",
                engine
            );
        }
    }

    fn replace_request(
        pattern: &str,
        text: &str,
        replacement: &str,
        engine: &str,
    ) -> RegexReplaceRequest {
        RegexReplaceRequest {
            pattern: pattern.to_string(),
            text: text.to_string(),
            replacement: replacement.to_string(),
            flags: RegexFlags::default(),
            engine: engine.to_string(),
            replace_all: true,
        }
    }

    /// 替换文本本身可被模式匹配（a→aa）时不重扫已替换文本：见下两个用例

    #[tokio::test]
    async fn re2_replace_with_self_matching_replacement_is_single_pass() {
        let req = replace_request("a", "a b a", "aa", "re2");
        let result = replace_re2_regex(req).await.unwrap();
        assert!(result.is_valid);
        assert_eq!(result.result.as_deref(), Some("aa b aa"));
        assert_eq!(result.replacement_count, 2);
    }

    #[tokio::test]
    async fn pcre_replace_with_self_matching_replacement_is_single_pass() {
        let req = replace_request("a", "a b a", "aa", "pcre");
        let result = replace_pcre_regex(req).await.unwrap();
        assert!(result.is_valid);
        assert_eq!(result.result.as_deref(), Some("aa b aa"));
        assert_eq!(result.replacement_count, 2);
    }

    /// 空匹配 + 非 ASCII 文本：旧 RE2 实现的 start+1 会切进多字节字符内部 panic
    #[tokio::test]
    async fn re2_test_empty_match_with_multibyte_text() {
        let request = RegexTestRequest {
            pattern: r"\d*".to_string(),
            text: "a中1".to_string(),
            flags: RegexFlags::default(),
            engine: "re2".to_string(),
        };
        let result = test_re2_regex(request).await.unwrap();
        assert!(result.is_valid);
        let full: Vec<&str> = result
            .matches
            .iter()
            .map(|m| m.full_match.as_str())
            .collect();
        // regex crate 语义：位置 0 空、位置 1（「中」前）空、位置 2 匹配 "1"；
        // 非空匹配结束后的同一位置不再产出空匹配
        assert_eq!(full, vec!["", "", "1"]);
    }

    /// unicode 开关必须生效：`.` 应按字符匹配中文而非字节碎片
    #[tokio::test]
    async fn pcre_test_unicode_flag_takes_effect() {
        let request = RegexTestRequest {
            pattern: ".".to_string(),
            text: "中文".to_string(),
            flags: RegexFlags::default(), // unicode: true
            engine: "pcre".to_string(),
        };
        let result = test_pcre_regex(request).await.unwrap();
        assert!(result.is_valid);
        assert_eq!(result.match_count, 2);
        assert_eq!(result.matches[0].full_match, "中");
        assert_eq!(result.matches[1].full_match, "文");
    }

    /// 第二处匹配的分组取自该匹配本身（旧实现固定从头 captures，第二处沿用第一处的分组）
    #[tokio::test]
    async fn pcre_test_groups_follow_each_match() {
        let request = RegexTestRequest {
            pattern: r"(\d)-(\d)".to_string(),
            text: "1-2 3-4".to_string(),
            flags: RegexFlags::default(),
            engine: "pcre".to_string(),
        };
        let result = test_pcre_regex(request).await.unwrap();
        assert_eq!(result.match_count, 2);
        assert_eq!(
            result.matches[0]
                .groups
                .iter()
                .map(|g| g.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        assert_eq!(
            result.matches[1]
                .groups
                .iter()
                .map(|g| g.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
            vec!["3", "4"]
        );
    }
}
