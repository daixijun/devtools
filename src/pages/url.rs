//! URL 编解码。
//!
//! 单工具双功能：编码 / 解码两个 Tab；两侧各自持有输入输出状态，
//! 切换不丢失已输入内容。

use gpui::Entity;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, IntoElement, ParentElement as _, Render,
    Window,
};

use crate::components::ui;

use super::{url_decode::UrlDecode, url_encode::UrlEncode};

const TABS: &[&str] = &["编码", "解码"];

pub struct Url {
    active_tab: usize,
    encode: Entity<UrlEncode>,
    decode: Entity<UrlDecode>,
}

impl Url {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            active_tab: 0,
            encode: cx.new(|cx| UrlEncode::new(window, cx)),
            decode: cx.new(|cx| UrlDecode::new(window, cx)),
        }
    }
}

impl Render for Url {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        ui::tool_page("url")
            .child(ui::tab_bar(
                "url-tabs",
                TABS,
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
