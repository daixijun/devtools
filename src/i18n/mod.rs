//! 多语言支持（简体中文 / English）。
//!
//! 设计：中文原文即 key（调用点 `t("设置")`），英文经 EN 表查译，缺失时回退
//! 中文——增量补词条不阻塞功能。语言来源是共享配置的 `language` 字段
//! （`"system" | "zh-CN" | "en"`），`system` 按系统 locale 检测。
//!
//! EN 表分域组织（shell=壳层/菜单/设置、registry=工具名与描述、errors=错误
//! 消息、pages_*=工具页词条），合并时同 key 冲突会 panic（测试覆盖）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::LazyLock;

mod en_errors;
mod en_pages_cert;
mod en_pages_convert;
mod en_pages_hash;
mod en_pages_image;
mod en_pages_jwt;
mod en_pages_media;
mod en_pages_misc;
mod en_pages_network;
mod en_pages_ssl;
mod en_registry;
mod en_shell;

/// 语言选项（配置值为 `"system"` 时按系统 locale 解析）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    System,
    ZhCn,
    En,
}

impl Language {
    /// 解析配置字符串（未知值回退 system）。
    pub fn parse(value: &str) -> Self {
        match value {
            "zh-CN" => Language::ZhCn,
            "en" => Language::En,
            _ => Language::System,
        }
    }

    /// 写入配置的字符串形式。
    pub fn as_config_str(self) -> &'static str {
        match self {
            Language::System => "system",
            Language::ZhCn => "zh-CN",
            Language::En => "en",
        }
    }

    /// system 按系统 locale 检测（zh* → 中文，其余一律英文）。
    pub fn resolve(self) -> Self {
        match self {
            Language::System => detect_system(),
            other => other,
        }
    }
}

/// 按 locale 检测语言。
fn detect_system() -> Language {
    let lang = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    if lang.starts_with("zh") {
        Language::ZhCn
    } else {
        Language::En
    }
}

/// 解析后的运行时语言（0=中文 1=英文）。中文为 key 本身，无需查表。
static LOCALE: AtomicU8 = AtomicU8::new(0);

const ZH: u8 = 0;
const EN: u8 = 1;

/// 应用启动时按配置初始化语言（main.rs 在配置加载后调用）。
pub fn init(config_value: &str) {
    set_language(Language::parse(config_value));
}

/// 切换语言（resolve 后生效）。
pub fn set_language(language: Language) {
    let code = match language.resolve() {
        Language::En => EN,
        _ => ZH,
    };
    LOCALE.store(code, Ordering::SeqCst);
}

/// 当前生效语言（resolve 后）。
pub fn current_language() -> Language {
    if LOCALE.load(Ordering::SeqCst) == EN {
        Language::En
    } else {
        Language::ZhCn
    }
}

/// EN 合并表（分域词条扁平拼接；HashMap 合并为后写覆盖——译名冲突由
/// test_en_table_no_conflicting_keys 在 CI 兜底，新增词条须保证同 key 同值）。
static EN_TABLE: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        en_shell::EN_SHELL,
        en_registry::EN_REGISTRY,
        en_errors::EN_ERRORS,
        en_pages_ssl::EN_PAGES_SSL,
        en_pages_cert::EN_PAGES_CERT,
        en_pages_network::EN_PAGES_NETWORK,
        en_pages_image::EN_PAGES_IMAGE,
        en_pages_media::EN_PAGES_MEDIA,
        en_pages_hash::EN_PAGES_HASH,
        en_pages_jwt::EN_PAGES_JWT,
        en_pages_convert::EN_PAGES_CONVERT,
        en_pages_misc::EN_PAGES_MISC,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect()
});

/// 翻译：key 为中文原文；英文模式查表，缺失回退中文。
///
/// key 必须是 `'static`（调用点传字面量或 registry 元数据）。
///
/// 注意：key 是跨页面共享的全局词条。短语式 key（如「没有找到」）的 EN
/// 译法按所有拼接上下文取中性值，仅适用于特定拼接组合的词条应把完整
/// 语义并入 key（如「的图片文件」组合），避免后续页面复用时语义漂移。
pub fn t(key: &'static str) -> &'static str {
    if LOCALE.load(Ordering::SeqCst) == EN {
        EN_TABLE.get(key).copied().unwrap_or(key)
    } else {
        key
    }
}

/// 查 EN 译名（无论当前语言；供搜索索引等跨语言匹配场景）。
pub fn en_of(key: &str) -> Option<&'static str> {
    EN_TABLE.get(key).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_parse_and_config_roundtrip() {
        assert_eq!(Language::parse("zh-CN"), Language::ZhCn);
        assert_eq!(Language::parse("en"), Language::En);
        assert_eq!(Language::parse("system"), Language::System);
        assert_eq!(Language::parse("bogus"), Language::System);
        for lang in [Language::System, Language::ZhCn, Language::En] {
            assert_eq!(Language::parse(lang.as_config_str()), lang);
        }
    }

    #[test]
    fn test_t_falls_back_to_key_for_chinese() {
        set_language(Language::ZhCn);
        assert_eq!(t("设置"), "设置");
        assert_eq!(t("不存在的词条"), "不存在的词条");
    }

    #[test]
    fn test_t_translates_in_english() {
        set_language(Language::En);
        assert_eq!(t("设置"), "Settings");
        // 未收录词条回退中文，不阻塞功能
        assert_eq!(t("不存在的词条"), "不存在的词条");
        set_language(Language::ZhCn);
    }

    /// EN 表内同 key 不得出现两个不同译法（分域文件由多人/多批维护，防止
    /// 后写覆盖先写导致译名漂移；相同译法则无害）。
    #[test]
    fn test_en_table_no_conflicting_keys() {
        let mut seen: HashMap<&'static str, &'static str> = HashMap::new();
        let domains: [&[(&'static str, &'static str)]; 12] = [
            en_shell::EN_SHELL,
            en_registry::EN_REGISTRY,
            en_errors::EN_ERRORS,
            en_pages_ssl::EN_PAGES_SSL,
            en_pages_cert::EN_PAGES_CERT,
            en_pages_network::EN_PAGES_NETWORK,
            en_pages_image::EN_PAGES_IMAGE,
            en_pages_media::EN_PAGES_MEDIA,
            en_pages_hash::EN_PAGES_HASH,
            en_pages_jwt::EN_PAGES_JWT,
            en_pages_convert::EN_PAGES_CONVERT,
            en_pages_misc::EN_PAGES_MISC,
        ];
        for table in domains {
            for (key, value) in table.iter() {
                match seen.get(*key) {
                    Some(prev) if prev != value => {
                        panic!(
                            "EN 表 key 冲突: \"{}\" -> \"{}\" / \"{}\"",
                            key, prev, value
                        )
                    }
                    _ => {
                        seen.insert(*key, *value);
                    }
                }
            }
        }
        assert!(!seen.is_empty());
    }

    #[test]
    fn test_en_registry_covers_all_tools_and_categories() {
        // registry 全量工具名与描述、分类名都必须有英文译法
        let en: HashMap<&str, &str> = en_registry::EN_REGISTRY.iter().copied().collect();
        for category in crate::registry::CATEGORIES {
            assert!(
                en.contains_key(category.name),
                "分类缺英文词条: {}",
                category.name
            );
        }
        for tool in crate::registry::TOOLS {
            assert!(
                en.contains_key(tool.name),
                "工具名缺英文词条: {}",
                tool.name
            );
            assert!(
                en.contains_key(tool.description),
                "工具描述缺英文词条: {}",
                tool.description
            );
        }
    }
}
