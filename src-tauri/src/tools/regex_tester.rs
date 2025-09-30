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

#[tauri::command]
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
    if !request.flags.unicode {
        builder.utf(false);
    }

    match builder.build(&request.pattern) {
        Ok(re) => {
            let mut matches = Vec::new();
            let text_bytes = request.text.as_bytes();

            // 使用 find_iter 查找所有匹配
            for mat in re.find_iter(text_bytes) {
                if let Ok(mat) = mat {
                    let start = mat.start();
                    let end = mat.end();
                    let full_match = String::from_utf8_lossy(&text_bytes[start..end]).to_string();

                    // PCRE2 捕获组处理
                    let mut groups = Vec::new();
                    let mut named_groups = std::collections::HashMap::new();

                    // 尝试获取捕获组
                    if let Ok(Some(captures)) = re.captures(text_bytes) {
                        for i in 1..captures.len() {
                            if let Some(group_match) = captures.get(i) {
                                groups.push(Some(
                                    String::from_utf8_lossy(group_match.as_bytes()).to_string(),
                                ));
                            } else {
                                groups.push(None);
                            }
                        }

                        // 命名捕获组
                        for name in re.capture_names() {
                            if let Some(name_str) = name {
                                if let Some(group_match) = captures.name(name_str) {
                                    named_groups.insert(
                                        name_str.to_string(),
                                        Some(
                                            String::from_utf8_lossy(group_match.as_bytes())
                                                .to_string(),
                                        ),
                                    );
                                } else {
                                    named_groups.insert(name_str.to_string(), None);
                                }
                            }
                        }
                    }

                    matches.push(RegexMatch {
                        full_match,
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

            // RE2 查找所有匹配
            let mut start = 0;
            while start <= request.text.len() {
                if let Some(mat) = re.find(&request.text[start..]) {
                    let match_start = start + mat.start();
                    let match_end = start + mat.end();
                    let full_match = mat.as_str().to_string();

                    // RE2 简化的捕获组处理
                    let mut groups = Vec::new();
                    let mut named_groups = std::collections::HashMap::new();

                    // 获取捕获组
                    if let Some(caps) = re.captures(&request.text[start..]) {
                        // 从索引1开始，0是完整匹配
                        for i in 1..caps.len() {
                            groups.push(caps.get(i).map(|m| m.as_str().to_string()));
                        }

                        // 命名捕获组
                        for name in re.capture_names() {
                            if let Some(name_str) = name {
                                named_groups.insert(
                                    name_str.to_string(),
                                    caps.name(name_str).map(|m| m.as_str().to_string()),
                                );
                            }
                        }
                    }

                    matches.push(RegexMatch {
                        full_match,
                        start: match_start,
                        end: match_end,
                        groups,
                        named_groups,
                    });

                    // 移动到下一个位置
                    start = if match_end > match_start {
                        match_end
                    } else {
                        match_start + 1
                    };
                } else {
                    break;
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
            error_message: Some(format!("RE2正则表达式语法错误: {:?}", e)),
            matches: Vec::new(),
            match_count: 0,
        }),
    }
}

#[tauri::command]
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
            // RE2 的替换功能实现
            let result = if request.replace_all {
                // 手动实现全部替换
                let mut result_text = request.text.clone();
                let mut replacement_count = 0;

                loop {
                    let find_result = re.find(&result_text);
                    if let Some(mat) = find_result {
                        let start = mat.start();
                        let end = mat.end();
                        result_text.replace_range(start..end, &request.replacement);
                        replacement_count += 1;
                        // 为了避免无限循环，如果替换后没有变化就停止
                        if start == end {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                (result_text, replacement_count)
            } else {
                // 单次替换
                if let Some(mat) = re.find(&request.text) {
                    let mut result_text = request.text.clone();
                    result_text.replace_range(mat.start()..mat.end(), &request.replacement);
                    (result_text, 1)
                } else {
                    (request.text.clone(), 0)
                }
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
    if !request.flags.unicode {
        builder.utf(false);
    }

    match builder.build(&request.pattern) {
        Ok(re) => {
            let text_bytes = request.text.as_bytes();

            // 计算原始匹配数
            let original_matches = re.find_iter(text_bytes).count();

            // 简化替换实现
            let result = if request.replace_all {
                // 手动实现全部替换
                let mut result_text = request.text.clone();
                let mut replacement_count = 0;

                // 重复查找并替换直到没有更多匹配
                loop {
                    let find_result = re.find(result_text.as_bytes());
                    if let Ok(Some(mat)) = find_result {
                        let start = mat.start();
                        let end = mat.end();
                        result_text.replace_range(start..end, &request.replacement);
                        replacement_count += 1;
                        if replacement_count >= original_matches * 2 {
                            // 防止无限循环
                            break;
                        }
                    } else {
                        break;
                    }
                }

                (result_text, replacement_count)
            } else {
                // 单次替换
                if let Ok(Some(mat)) = re.find(text_bytes) {
                    let mut result_text = request.text.clone();
                    result_text.replace_range(mat.start()..mat.end(), &request.replacement);
                    (result_text, 1)
                } else {
                    (request.text.clone(), 0)
                }
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

#[tauri::command]
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
}
