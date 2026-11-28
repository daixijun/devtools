//! SHA 哈希加密工具。
//!
//! 三 Tab（文本/文件/批量）的状态与交互编排在 [`hash_common::HashPageCore`]；
//! 本文件提供 6 个算法（sha1/sha224/sha256/sha384/sha512/sha3）的派发与
//! 静态装配参数——多算法，顶部全局算法 select 由共享编排渲染，切换立即
//! 重算文本页与文件页；批量以任务开始时的算法为准。
//! sha3 = Keccak-512（devtools::crypto::sha3_bytes_hex，历史 "SHA3" 语义）。

use devtools::crypto::{
    sha1_bytes_hex, sha1_file_hex, sha224_bytes_hex, sha224_file_hex, sha256_bytes_hex,
    sha256_file_hex, sha384_bytes_hex, sha384_file_hex, sha3_bytes_hex, sha3_file_hex,
    sha512_bytes_hex, sha512_file_hex,
};
use gpui::{prelude::*, App, Context, Entity, IntoElement, Render, Window};

use super::hash_common::{HashAlgoSpec, HashPageCore, HashPageIds, HashPageSpec};

/// 按算法流式计算文件哈希（core 统一实现，IO 错误转字符串）。
fn file_hash_by_id(path: &std::path::Path, id: &str) -> Result<String, String> {
    let result = match id {
        "sha1" => sha1_file_hex(path),
        "sha224" => sha224_file_hex(path),
        "sha384" => sha384_file_hex(path),
        "sha512" => sha512_file_hex(path),
        "sha3" => sha3_file_hex(path),
        _ => sha256_file_hex(path),
    };
    result.map_err(|err| err.to_string())
}

macro_rules! sha_algo {
    ($id:expr, $name:expr, $bytes_fn:expr, $file_id:expr) => {
        HashAlgoSpec {
            id: $id,
            name: $name,
            text_hash: |text| Ok($bytes_fn(text.as_bytes())),
            file_hash: |path| file_hash_by_id(path, $file_id),
        }
    };
}

/// 6 算法派发。
static SHA_ALGORITHMS: &[HashAlgoSpec] = &[
    sha_algo!("sha1", "SHA-1", sha1_bytes_hex, "sha1"),
    sha_algo!("sha224", "SHA-224", sha224_bytes_hex, "sha224"),
    sha_algo!("sha256", "SHA-256", sha256_bytes_hex, "sha256"),
    sha_algo!("sha384", "SHA-384", sha384_bytes_hex, "sha384"),
    sha_algo!("sha512", "SHA-512", sha512_bytes_hex, "sha512"),
    sha_algo!("sha3", "SHA-3", sha3_bytes_hex, "sha3"),
];

/// 页面静态装配参数（默认 sha256，第 3 项）。
static SHA_SPEC: HashPageSpec = HashPageSpec {
    page_id: "shacrypto",
    algorithms: SHA_ALGORITHMS,
    default_selected: 2,
    ids: HashPageIds {
        tabs: "sha-tabs",
        example: "sha-example",
        clear_text: "sha-clear-text",
        copy_text: "sha-copy-text",
        mono_text: "mono-sha-text",
        file_clear: "sha-file-clear",
        file_pick: "sha-file-pick",
        copy_file: "sha-copy-file",
        mono_file: "mono-sha-file",
        batch_clear: "sha-batch-clear",
        batch_pick: "sha-batch-pick",
        copy_batch: "sha-copy-batch",
        batch_export: "sha-batch-export",
    },
};

pub struct ShaCrypto {
    core: Entity<HashPageCore>,
}

impl ShaCrypto {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self {
            core: cx.new(|cx| HashPageCore::new(&SHA_SPEC, window, cx)),
        })
        .into()
    }
}

impl Render for ShaCrypto {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.core
            .update(cx, |core, cx| core.render(cx).into_any_element())
    }
}
