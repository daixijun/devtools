//! JWT 工具。
//!
//! 单工具双功能：生成 / 解码两个 Tab；两侧各自持有输入输出状态，
//! 切换不丢失已输入内容。

use devtools::i18n::t;
use gpui::Entity;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, IntoElement, ParentElement as _, Render,
    Window,
};

use crate::components::ui;

use super::{jwt_decode::JwtDecode, jwt_encode::JwtEncode};

const TABS: &[&str] = &["生成", "解码"];

pub struct Jwt {
    active_tab: usize,
    encode: Entity<JwtEncode>,
    decode: Entity<JwtDecode>,
}

impl Jwt {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            active_tab: 0,
            encode: cx.new(|cx| JwtEncode::new(window, cx)),
            decode: cx.new(|cx| JwtDecode::new(window, cx)),
        }
    }
}

impl Render for Jwt {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        ui::tool_page("jwt")
            .child(ui::tab_bar(
                "jwt-tabs",
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
