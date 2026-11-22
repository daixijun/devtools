//! AES 加密/解密。
//!
//! 单工具双功能：加密 / 解密两个 Tab；两侧各自持有配置与输入输出状态，
//! 切换不丢失已输入内容。

use devtools::i18n::t;
use gpui::Entity;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, IntoElement, ParentElement as _, Render,
    Window,
};

use crate::components::ui;

use super::{aes_decrypt::AesDecrypt, aes_encrypt::AesEncrypt};

const TABS: &[&str] = &["加密", "解密"];

pub struct Aes {
    active_tab: usize,
    encrypt: Entity<AesEncrypt>,
    decrypt: Entity<AesDecrypt>,
}

impl Aes {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            active_tab: 0,
            encrypt: cx.new(|cx| AesEncrypt::new(window, cx)),
            decrypt: cx.new(|cx| AesDecrypt::new(window, cx)),
        }
    }
}

impl Render for Aes {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        ui::tool_page("aes")
            .child(ui::tab_bar(
                "aes-tabs",
                &[t(TABS[0]), t(TABS[1])],
                self.active_tab,
                cx.listener(|this, ix: &usize, _window, cx| {
                    if this.active_tab != *ix {
                        this.active_tab = *ix;
                        cx.notify();
                    }
                }),
            ))
            .when(self.active_tab == 0, |el| el.child(self.encrypt.clone()))
            .when(self.active_tab == 1, |el| el.child(self.decrypt.clone()))
    }
}
