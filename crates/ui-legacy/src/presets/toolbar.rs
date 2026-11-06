//! The toolbar with the search field and the view buttons (DS-44 task 4).

use egui::{Align, Layout, Rect, Ui, UiBuilder, vec2};

use super::labels::PresetLabels;
use super::state::{PresetState, PresetView};
use super::style::PresetStyle;
use crate::controls::{IconButton, SearchField, TextField};
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

/// Shows the search field, and at the reading end the grid and list buttons unless the view is
/// locked; they write `state.filter` and `state.view`.
pub(super) fn show<K>(ui: &mut Ui, state: &mut PresetState<K>, labels: &PresetLabels, view_buttons: bool) {
    let tokens = Tokens::of(ui);
    let style = PresetStyle::from_tokens(&tokens);
    let side = IconButton::style(&tokens, ControlSize::Md).side;
    let height = TextField::style(&tokens, ControlSize::Md).height.max(side);
    let (_, bar) = ui.allocate_space(vec2(ui.available_width(), height));
    let rtl = direction::is_rtl(ui);
    let buttons = if view_buttons { 2.0 * side + style.toolbar_gap } else { 0.0 };
    let reserved = if view_buttons { buttons + style.toolbar_gap } else { 0.0 };
    let field =
        if rtl { Rect::from_min_max(bar.min + vec2(reserved, 0.0), bar.max) } else { Rect::from_min_max(bar.min, bar.max - vec2(reserved, 0.0)) };
    let mut child = ui.new_child(UiBuilder::new().id_salt("preset-toolbar-search").max_rect(field).layout(Layout::top_down_justified(Align::Min)));
    let edit =
        SearchField::new("search", labels.search.as_str(), &state.filter, labels.clear.as_str()).placeholder(labels.search.as_str()).show(&mut child);
    if let Some(
        EditEvent::Begin { value: Some(value), .. }
        | EditEvent::Update { value, .. }
        | EditEvent::Set { value, .. }
        | EditEvent::Commit { value, .. },
    ) = edit.event
    {
        state.filter = value;
    }
    if view_buttons {
        let x =
            |i: f32| if rtl { bar.right() - side - i * (side + style.toolbar_gap) } else { bar.right() - buttons + i * (side + style.toolbar_gap) };
        let top = bar.center().y - side / 2.0;
        for (i, (icon, label, view)) in
            [(Icon::LayoutGrid, &labels.grid_view, PresetView::Grid), (Icon::List, &labels.list_view, PresetView::List)].into_iter().enumerate()
        {
            let rect = Rect::from_min_size(egui::pos2(x(i as f32), top), vec2(side, side));
            if IconButton::new(icon, label.as_str()).selected(state.view == view).id_salt(("preset-view", i)).show_at(ui, rect).clicked() {
                state.view = view;
            }
        }
    }
}
