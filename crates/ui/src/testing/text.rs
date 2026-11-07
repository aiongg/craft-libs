//! Reading text parts in trials: the widget id, the caret and the text a frame painted.

use crate::primitives::text_adapter;
use craft_behavior::PartKey;
use egui::{Context, Id, Shape};

pub use crate::runtime::egui::fixture::ImeSeen;

/// The egui widget id of the text part `key` of component `id`.
pub fn widget_id(id: Id, key: PartKey) -> Id {
    text_adapter::text_id(id, key)
}

/// The caret of a text widget as a character index.
pub fn caret(ctx: &Context, widget: Id) -> Option<usize> {
    crate::runtime::egui::text_edit::selection(ctx, widget).map(|selection| selection.0)
}

/// The texts of the text shapes among `shapes`.
pub fn painted(shapes: &[Shape]) -> Vec<String> {
    shapes.iter().filter_map(|shape| if let Shape::Text(text) = shape { Some(text.galley.text().to_owned()) } else { None }).collect()
}
