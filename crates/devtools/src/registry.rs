//! 工具注册表（32 个工具 + 8 个分类）。
//!
//! 工具元数据的权威来源（id、中文名称/描述、keywords、分类、窗口尺寸）。

/// 工具窗口尺寸（逻辑像素）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
}

/// 工具定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub keywords: &'static [&'static str],
    pub category: &'static str,
    pub window_size: WindowSize,
}

/// 工具分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolCategory {
    pub id: &'static str,
    pub name: &'static str,
}

pub const CATEGORIES: &[ToolCategory] = &[
    ToolCategory {
        id: "encoding",
        name: "编码/解码",
    },
    ToolCategory {
        id: "certificate",
        name: "证书工具",
    },
    ToolCategory {
        id: "network",
        name: "网络工具",
    },
    ToolCategory {
        id: "dataformat",
        name: "数据格式转换",
    },
    ToolCategory {
        id: "media",
        name: "媒体格式转换",
    },
    ToolCategory {
        id: "developer",
        name: "开发工具",
    },
    ToolCategory {
        id: "time",
        name: "时间工具",
    },
    ToolCategory {
        id: "settings",
        name: "设置",
    },
];

pub const TOOLS: &[ToolDefinition] = &[
    // ---------- encoding 编码/解码 ----------
    ToolDefinition {
        id: "base64",
        name: "Base64 编解码",
        description: "Base64 编码和解码工具，支持文本的 Base64 转换与格式校验",
        keywords: &["base64", "编码", "解码", "encode", "decode", "转换", "文本"],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "url",
        name: "URL 编解码",
        description: "URL 编码和解码工具，对 URL 中的特殊字符进行百分号编码转换",
        keywords: &[
            "url",
            "uri",
            "编码",
            "解码",
            "encode",
            "decode",
            "percent",
            "百分号",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "aes",
        name: "AES 加密/解密",
        description:
            "AES 对称加密和解密工具，支持 CBC/ECB/CFB/OFB/CTR 模式与多种填充，结果为 Base64",
        keywords: &[
            "aes",
            "加密",
            "解密",
            "encrypt",
            "decrypt",
            "对称加密",
            "symmetric",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "md5crypto",
        name: "MD5 加密",
        description: "MD5 哈希生成工具，计算文本或文件的 MD5 摘要值",
        keywords: &["md5", "哈希", "hash", "摘要", "digest", "散列", "加密"],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "shacrypto",
        name: "SHA 哈希加密",
        description: "SHA 系列哈希生成工具，支持 SHA-1/SHA-256/SHA-512 等算法",
        keywords: &[
            "sha", "sha1", "sha256", "sha512", "哈希", "hash", "加密", "散列",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "jwt",
        name: "JWT 工具",
        description: "JWT Token 生成与解码工具，创建、解析和验证 JSON Web Token",
        keywords: &[
            "jwt",
            "token",
            "生成",
            "解码",
            "generate",
            "encode",
            "decode",
            "verify",
            "令牌",
            "json web token",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 1000,
            height: 750,
        },
    },
    ToolDefinition {
        id: "passwordgenerator",
        name: "密码生成器",
        description: "安全密码生成工具，可自定义长度、字符类型等选项生成强密码",
        keywords: &[
            "密码",
            "password",
            "生成",
            "generate",
            "随机",
            "random",
            "强密码",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "passwordhasher",
        name: "密码加密验证",
        description: "密码哈希和验证工具，支持 bcrypt 等算法对密码进行安全加密和比对",
        keywords: &[
            "密码", "password", "哈希", "hash", "加密", "verify", "验证", "bcrypt",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "rsakeygenerator",
        name: "RSA 密钥对生成",
        description:
            "RSA 公私钥对生成工具，私钥与公钥统一格式选择：PKCS#8（通用）/ PKCS#1（传统）/ OpenSSH",
        keywords: &[
            "rsa",
            "ssh",
            "密钥",
            "公钥",
            "私钥",
            "key pair",
            "generate",
            "pkcs8",
            "pkcs1",
            "pem",
            "authorized_keys",
        ],
        category: "encoding",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    // ---------- certificate 证书工具 ----------
    ToolDefinition {
        id: "certificate",
        name: "证书查看器",
        description: "PEM 证书解析工具，查看证书的详细信息、有效期和公钥等",
        keywords: &[
            "证书",
            "certificate",
            "pem",
            "x509",
            "查看",
            "viewer",
            "ssl",
            "tls",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    ToolDefinition {
        id: "csrviewer",
        name: "CSR 查看",
        description:
            "PKCS#10 证书签名请求（CSR）解析工具，查看主题、公钥、签名算法与请求的扩展信息",
        keywords: &[
            "csr",
            "pkcs10",
            "certificate request",
            "证书请求",
            "签名请求",
            "csr查看",
            "csr 查看",
            "subject",
            "公钥",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 950,
            height: 750,
        },
    },
    ToolDefinition {
        id: "csrgenerator",
        name: "CSR 生成",
        description: "生成 PKCS#10 证书签名请求（CSR）与配套私钥，支持 RSA/ECC 密钥和 SAN 扩展",
        keywords: &[
            "csr",
            "csr生成",
            "csr 生成",
            "generate",
            "generate csr",
            "生成证书请求",
            "证书请求",
            "签名请求",
            "pkcs10",
            "san",
            "私钥",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 1000,
            height: 800,
        },
    },
    ToolDefinition {
        id: "pemtopfx",
        name: "PEM 转 PFX",
        description: "将 PEM 格式证书转换为 PFX/PKCS#12 格式，方便在 Windows 等环境中使用",
        keywords: &[
            "pem",
            "pfx",
            "pkcs12",
            "转换",
            "convert",
            "证书",
            "certificate",
            "导入",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 950,
            height: 750,
        },
    },
    ToolDefinition {
        id: "pfxtopem",
        name: "PFX 转 PEM",
        description: "将 PFX/PKCS#12 格式证书转换为 PEM 格式，提取证书和私钥",
        keywords: &[
            "pfx",
            "pem",
            "pkcs12",
            "转换",
            "convert",
            "证书",
            "certificate",
            "提取",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 950,
            height: 750,
        },
    },
    ToolDefinition {
        id: "sslchecker",
        name: "在线 SSL 检测",
        description: "在线 SSL 证书检测工具，检查网站的 SSL 证书信息和到期时间",
        keywords: &[
            "ssl",
            "tls",
            "检测",
            "checker",
            "证书",
            "certificate",
            "https",
            "在线",
        ],
        category: "certificate",
        window_size: WindowSize {
            width: 950,
            height: 750,
        },
    },
    // ---------- network 网络工具 ----------
    ToolDefinition {
        id: "subnetcalculator",
        name: "子网掩码计算器",
        description: "子网掩码计算工具，支持 IP 地址与子网掩码的网络规划计算",
        keywords: &[
            "子网",
            "subnet",
            "掩码",
            "mask",
            "计算器",
            "calculator",
            "ip",
            "cidr",
            "网络",
        ],
        category: "network",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "ipinfo",
        name: "IP 地址信息查询",
        description: "IP 地址信息查询工具，获取 IP 地址的地理位置、运营商等详细信息",
        keywords: &[
            "ip",
            "地址",
            "address",
            "查询",
            "lookup",
            "地理位置",
            "geoip",
            "信息",
        ],
        category: "network",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "dnsresolver",
        name: "DNS 解析工具",
        description: "DNS 记录查询工具，解析域名对应的 A/AAAA/CNAME/MX/TXT 等记录",
        keywords: &[
            "dns", "解析", "resolve", "域名", "domain", "记录", "record", "a记录", "cname",
        ],
        category: "network",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "whois",
        name: "域名 Whois 查询",
        description: "域名 Whois 信息查询工具，查看域名的注册信息、到期时间和注册商",
        keywords: &[
            "whois",
            "域名",
            "domain",
            "注册",
            "查询",
            "lookup",
            "注册商",
            "registrar",
        ],
        category: "network",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    // ---------- dataformat 数据格式转换 ----------
    ToolDefinition {
        id: "jsonformatter",
        name: "JSON 格式化",
        description: "JSON 格式化工具，支持美化、压缩、验证 JSON 数据并高亮显示",
        keywords: &[
            "json",
            "格式化",
            "format",
            "美化",
            "pretty",
            "验证",
            "validate",
            "压缩",
        ],
        category: "dataformat",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    ToolDefinition {
        id: "formatconverter",
        name: "格式转换器",
        description: "多格式转换工具，支持 JSON、YAML、TOML 等数据格式之间的互相转换",
        keywords: &[
            "格式", "format", "转换", "convert", "json", "yaml", "toml", "互转",
        ],
        category: "dataformat",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    ToolDefinition {
        id: "jsontogo",
        name: "JSON 转 Go 结构体",
        description: "将 JSON 数据自动生成 Go 语言结构体定义，支持嵌套类型",
        keywords: &[
            "json",
            "go",
            "golang",
            "结构体",
            "struct",
            "转换",
            "convert",
            "生成",
        ],
        category: "dataformat",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    ToolDefinition {
        id: "sqltogo",
        name: "SQL 转 Go 结构体",
        description: "将 SQL CREATE TABLE 语句转换为 Go 语言结构体定义，支持多表解析",
        keywords: &[
            "sql",
            "go",
            "golang",
            "结构体",
            "struct",
            "转换",
            "convert",
            "建表",
            "create table",
        ],
        category: "dataformat",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    ToolDefinition {
        id: "sqltoent",
        name: "SQL 转 Go Ent ORM",
        description: "将 SQL DDL 语句转换为 Go Ent ORM Schema 代码，支持多表和复杂类型映射",
        keywords: &[
            "sql",
            "go",
            "ent",
            "orm",
            "schema",
            "转换",
            "convert",
            "数据库",
            "database",
        ],
        category: "dataformat",
        window_size: WindowSize {
            width: 1100,
            height: 800,
        },
    },
    // ---------- media 媒体格式转换 ----------
    ToolDefinition {
        id: "imageconverter",
        name: "图片格式转换",
        description: "图片格式转换工具，支持 PNG、JPEG、WebP、BMP 等常见图片格式互转",
        keywords: &[
            "图片", "image", "格式", "format", "转换", "convert", "png", "jpeg", "webp", "bmp",
        ],
        category: "media",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "imagepreview",
        name: "图片预览器",
        description: "图片预览和信息查看工具，支持拖拽上传、缩放浏览和图片元数据查看",
        keywords: &[
            "图片",
            "image",
            "预览",
            "preview",
            "查看",
            "viewer",
            "元数据",
            "metadata",
            "exif",
        ],
        category: "media",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "videoconverter",
        name: "视频格式转换",
        description: "视频格式转换工具，支持 MP4、WebM、AVI 等常见视频格式互转",
        keywords: &[
            "视频", "video", "格式", "format", "转换", "convert", "mp4", "webm", "avi", "ffmpeg",
        ],
        category: "media",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    ToolDefinition {
        id: "imagecompressor",
        name: "图片压缩工具",
        description: "PNG/JPG 无损优化与质量压缩，支持等比缩放、批量处理、压缩前后滑块对比预览",
        keywords: &[
            "图片", "压缩", "image", "compress", "无损", "lossless", "缩放", "resize", "优化",
            "optimize", "png", "jpg", "jpeg", "webp", "瘦身",
        ],
        category: "media",
        window_size: WindowSize {
            width: 1100,
            height: 820,
        },
    },
    // ---------- developer 开发工具 ----------
    ToolDefinition {
        id: "regextester",
        name: "正则表达式测试器",
        description: "正则表达式在线测试工具，实时匹配高亮并显示捕获组结果",
        keywords: &[
            "正则",
            "regex",
            "regexp",
            "表达式",
            "测试",
            "test",
            "匹配",
            "match",
            "pattern",
        ],
        category: "developer",
        window_size: WindowSize {
            width: 1000,
            height: 750,
        },
    },
    // ---------- time 时间工具 ----------
    ToolDefinition {
        id: "timestamp",
        name: "时间戳转换",
        description: "Unix 时间戳转换工具，支持时间戳与可读日期的双向转换，支持秒级和毫秒级",
        keywords: &[
            "时间戳",
            "timestamp",
            "unix",
            "转换",
            "convert",
            "日期",
            "date",
            "时间",
            "time",
            "epoch",
        ],
        category: "time",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
    // ---------- settings 设置 ----------
    ToolDefinition {
        id: "settings",
        name: "设置",
        description: "应用配置和偏好设置，全局快捷键、主题等",
        keywords: &[
            "设置",
            "settings",
            "配置",
            "config",
            "偏好",
            "preference",
            "快捷键",
            "shortcut",
        ],
        category: "settings",
        window_size: WindowSize {
            width: 900,
            height: 700,
        },
    },
];

pub fn get_tool_by_id(id: &str) -> Option<&'static ToolDefinition> {
    TOOLS.iter().find(|t| t.id == id)
}

pub fn tools_in_category(
    category_id: &'static str,
) -> impl Iterator<Item = &'static ToolDefinition> {
    TOOLS.iter().filter(move |t| t.category == category_id)
}

pub fn category_name(category_id: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|c| c.id == category_id)
        .map(|c| c.name)
        .unwrap_or("")
}

/// 搜索工具：按 name+description+分类名+keywords 匹配
/// 拼接后做小写子串匹配；空查询返回全部工具。
pub fn search_tools(query: &str) -> Vec<&'static ToolDefinition> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return TOOLS.iter().collect();
    }

    let q = trimmed.to_lowercase();
    TOOLS
        .iter()
        .filter(|tool| {
            let mut haystack = String::with_capacity(256);
            haystack.push_str(tool.name);
            haystack.push(' ');
            haystack.push_str(tool.description);
            haystack.push(' ');
            haystack.push_str(category_name(tool.category));
            for keyword in tool.keywords {
                haystack.push(' ');
                haystack.push_str(keyword);
            }
            haystack.to_lowercase().contains(&q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_counts() {
        assert_eq!(TOOLS.len(), 31);
        assert_eq!(CATEGORIES.len(), 8);
        let per_category: Vec<(usize, &str)> = CATEGORIES
            .iter()
            .map(|c| (tools_in_category(c.id).count(), c.id))
            .collect();
        assert_eq!(
            per_category,
            vec![
                (9, "encoding"),
                (6, "certificate"),
                (4, "network"),
                (5, "dataformat"),
                (4, "media"),
                (1, "developer"),
                (1, "time"),
                (1, "settings"),
            ]
        );
    }

    #[test]
    fn test_search() {
        assert_eq!(search_tools("").len(), 31);
        assert_eq!(search_tools("   ").len(), 31);
        assert!(search_tools("json").iter().any(|t| t.id == "jsonformatter"));
        assert!(search_tools("JWT").iter().any(|t| t.id == "jwt"));
        assert!(search_tools("子网")
            .iter()
            .any(|t| t.id == "subnetcalculator"));
        assert!(search_tools("不存在的工具xyz").is_empty());
    }
}
