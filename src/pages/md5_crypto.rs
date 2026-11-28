//! MD5 加密工具。
//!
//! 三 Tab（文本/文件/批量）的状态与交互编排在 [`hash_common::HashPageCore`]；
//! 本文件只提供 MD5 算法派发与静态装配参数——单算法，无算法选择栏。
//! 文本哈希为同步纯函数（devtools::crypto::md5_hex）；文件哈希走 core 的
//! 流式 md5_file_hex（64KB 逐块读取，GB 级文件内存占用恒定）。

use devtools::crypto::{md5_file_hex, md5_hex};
use gpui::{prelude::*, App, Context, Entity, IntoElement, Render, Window};

use super::hash_common::{HashAlgoSpec, HashPageCore, HashPageIds, HashPageSpec};

/// 单算法派发。
static MD5_ALGORITHMS: &[HashAlgoSpec] = &[HashAlgoSpec {
    id: "md5",
    name: "MD5",
    text_hash: |text| Ok(md5_hex(text)),
    file_hash: |path| md5_file_hex(path).map_err(|err| err.to_string()),
}];

/// 页面静态装配参数。
static MD5_SPEC: HashPageSpec = HashPageSpec {
    page_id: "md5crypto",
    algorithms: MD5_ALGORITHMS,
    default_selected: 0,
    ids: HashPageIds {
        tabs: "md5-tabs",
        example: "md5-example",
        clear_text: "md5-clear-text",
        copy_text: "md5-copy-text",
        mono_text: "mono-md5-text",
        file_clear: "md5-file-clear",
        file_pick: "md5-file-pick",
        copy_file: "md5-copy-file",
        mono_file: "mono-md5-file",
        batch_clear: "md5-batch-clear",
        batch_pick: "md5-batch-pick",
        copy_batch: "md5-copy-batch",
        batch_export: "md5-batch-export",
    },
};

pub struct Md5Crypto {
    core: Entity<HashPageCore>,
}

impl Md5Crypto {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self {
            core: cx.new(|cx| HashPageCore::new(&MD5_SPEC, window, cx)),
        })
        .into()
    }
}

impl Render for Md5Crypto {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.core
            .update(cx, |core, cx| core.render(cx).into_any_element())
    }
}
