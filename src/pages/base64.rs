//! Base64 编解码。
//!
//! 单工具双功能：编码 / 解码两个 Tab；两侧各自持有输入输出状态，
//! 切换不丢失已输入内容。

use devtools::i18n::t;
use gpui::Entity;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, IntoElement, ParentElement as _, Render,
    Window,
};

use crate::components::ui;

use super::{base64_decode::Base64Decode, base64_encode::Base64Encode};

const TABS: &[&str] = &["编码", "解码"];

pub struct Base64 {
    active_tab: usize,
    encode: Entity<Base64Encode>,
    decode: Entity<Base64Decode>,
}

impl Base64 {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            active_tab: 0,
            encode: cx.new(|cx| Base64Encode::new(window, cx)),
            decode: cx.new(|cx| Base64Decode::new(window, cx)),
        }
    }
}

impl Render for Base64 {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        ui::tool_page("base64")
            .child(ui::tab_bar(
                "base64-tabs",
                &[t(TABS[0]), t(TABS[1])],
                self.active_tab,
                cx.listener(|this, ix: &usize, _window, cx| {
                    if this.active_tab != *ix {
                        this.active_tab = *ix;
                        cx.notify();
                    }
                }),
            ))
            .when(self.active_tab == 0, |el| el.child(self.encode.clone()))
            .when(self.active_tab == 1, |el| el.child(self.decode.clone()))
    }
}
