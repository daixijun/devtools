//! 工具页面注册表。
//!
//! 每个工具一个模块文件，`create_tool_view` 按 registry 的 tool id 分发。
//! 工具视图需实现 `Render`，构造签名统一为 `fn new(window, cx) -> Self`。

use devtools::registry::ToolDefinition;
use gpui::{App, Window};

/// 按工具 id 创建页面视图（未注册 id 返回占位页）。
pub fn create_tool_view(
    tool: &'static ToolDefinition,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyView {
    match tool.id {
        "base64" => base64::Base64::create_view(window, cx),
        "url" => url::Url::create_view(window, cx),
        "aes" => aes::Aes::create_view(window, cx),
        "md5crypto" => md5_crypto::Md5Crypto::create_view(window, cx),
        "shacrypto" => sha_crypto::ShaCrypto::create_view(window, cx),
        "jwt" => jwt::Jwt::create_view(window, cx),
        "passwordgenerator" => password_generator::PasswordGenerator::create_view(window, cx),
        "passwordhasher" => password_hasher::PasswordHasher::create_view(window, cx),
        "jsonformatter" => json_formatter::JsonFormatter::create_view(window, cx),
        "timestamp" => timestamp_converter::TimestampConverter::create_view(window, cx),
        "subnetcalculator" => subnet_calculator::SubnetCalculator::create_view(window, cx),
        "formatconverter" => format_converter::FormatConverter::create_view(window, cx),
        "certificate" => certificate_viewer::CertificateViewer::create_view(window, cx),
        "csrviewer" => csr_viewer::CsrViewer::create_view(window, cx),
        "csrgenerator" => csr_generator::CsrGenerator::create_view(window, cx),
        "pemtopfx" => pem_to_pfx::PemToPfxConverter::create_view(window, cx),
        "pfxtopem" => pfx_to_pem::PfxToPemConverter::create_view(window, cx),
        "rsakeygenerator" => rsa_key_generator::RsaKeyGenerator::create_view(window, cx),
        "regextester" => regex_tester::RegexTester::create_view(window, cx),
        "ipinfo" => ip_info::IpInfo::create_view(window, cx),
        "whois" => whois_lookup::WhoisLookup::create_view(window, cx),
        "jsontogo" => json_to_go::JsonToGo::create_view(window, cx),
        "sslchecker" => ssl_checker::SslChecker::create_view(window, cx),
        "imageconverter" => image_converter::ImageConverter::create_view(window, cx),
        "imagecompressor" => image_compressor::ImageCompressor::create_view(window, cx),
        "imagepreview" => image_preview::ImagePreview::create_view(window, cx),
        "videoconverter" => video_converter::VideoConverter::create_view(window, cx),
        "sqltogo" => sql_to_go::SqlToGo::create_view(window, cx),
        "sqltoent" => sql_to_ent::SqlToEnt::create_view(window, cx),
        "dnsresolver" => dns_resolver::DnsResolver::create_view(window, cx),
        "settings" => settings::Settings::create_view(window, cx),
        _ => placeholder::Placeholder::create_view(window, cx),
    }
}

pub mod placeholder;

// 纯文本工具（base64/url/aes/jwt 为 Tab 容器，子视图见对应 *_encode/*_decode）
pub mod aes;
pub mod aes_common;
pub mod aes_decrypt;
pub mod aes_encrypt;
pub mod base64;
pub mod base64_decode;
pub mod base64_encode;
pub mod format_converter;
pub mod hash_common;
pub mod json_formatter;
pub mod jwt;
pub mod jwt_decode;
pub mod jwt_encode;
pub mod md5_crypto;
pub mod password_generator;
pub mod password_hasher;
pub mod sha_crypto;
pub mod subnet_calculator;
pub mod timestamp_converter;
pub mod url;
pub mod url_decode;
pub mod url_encode;

// 证书/网络/正则工具
pub mod certificate_viewer;
pub mod csr_generator;
pub mod csr_viewer;
pub mod ip_info;
pub mod json_to_go;
pub mod pem_to_pfx;
pub mod pfx_to_pem;
pub mod regex_tester;
pub mod rsa_key_generator;
pub mod whois_lookup;

// 复杂工具
pub mod dns_resolver;
pub mod image_compressor;
pub mod image_converter;
pub mod image_preview;
pub mod settings;
pub mod sql_to_ent;
pub mod sql_to_go;
pub mod ssl_checker;
pub mod video_converter;
