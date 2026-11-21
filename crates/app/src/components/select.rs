//! 简单下拉选择：为 gpui-component Select 提供 value/label 平铺数据源。
//!
//! 工具页用法：
//! ```ignore
//! let state = crate::components::select_state(window, cx, &[(  "CBC", "CBC 模式"), ("ECB", "ECB 模式")], 0);
//! cx.subscribe(&state, |this, _, event: &SelectEvent<SimpleSelectDelegate>, cx| match event {
//!     SelectEvent::Confirm(Some(value)) => { /* value 为选项 value */ }
//!     _ => {}
//! }).detach();
//! crate::components::ui::field("模式", Select::new(&state))
//! ```

use gpui::{AppContext as _, Entity, SharedString, Window};
use gpui_component::{
    select::SelectState,
    searchable_list::{SearchableListDelegate, SearchableListItem},
    IndexPath,
};

/// 选项：value 为逻辑值（写入状态），label 为展示文案。
#[derive(Clone)]
pub struct SelectItem {
    pub value: SharedString,
    pub label: SharedString,
}

impl SearchableListItem for SelectItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

/// 平铺列表 delegate（单 section）。
pub struct SimpleSelectDelegate {
    items: Vec<SelectItem>,
}

impl SearchableListDelegate for SimpleSelectDelegate {
    type Item = SelectItem;

    fn items_count(&self, _section: usize) -> usize {
        self.items.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.items
            .iter()
            .position(|item| item.value() == value)
            .map(|row| IndexPath { section: 0, row, column: 0 })
    }
}

/// 创建下拉状态。`selected_row` 为初始选中行号。
pub fn select_state(
    window: &mut Window,
    cx: &mut gpui::App,
    options: &[(&str, &str)],
    selected_row: usize,
) -> Entity<SelectState<SimpleSelectDelegate>> {
    let items: Vec<SelectItem> = options
        .iter()
        .map(|(value, label)| SelectItem {
            value: SharedString::from(*value),
            label: SharedString::from(*label),
        })
        .collect();
    let delegate = SimpleSelectDelegate { items };
    let selected = (selected_row < 100).then_some(IndexPath {
        section: 0,
        row: selected_row,
        column: 0,
    });
    cx.new(|cx| SelectState::new(delegate, selected, window, cx))
}
