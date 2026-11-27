//! `StateLayer`: the colour the resolver chose, over a shape.

use super::items;
use crate::style::StyleColor;
use craft_ui_rt::Rect;
use craft_ui_rt::{Item, StrokeAlign};
use egui::CornerRadius;

/// How a state layer paints its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StateLayerKind {
    /// A fill.
    #[default]
    Fill,
    /// An outline inside the rect (thumbnails, tiles).
    Outline,
}

/// The state layer painter.
pub struct StateLayer;

impl StateLayer {
    /// The shape of `color` over `rect`; an outline is `width` wide.
    pub fn items(kind: StateLayerKind, color: StyleColor, rect: Rect, radius: CornerRadius, width: f32) -> Vec<Item> {
        let color = color.egui();
        match kind {
            StateLayerKind::Fill => vec![items::rect_filled(rect, radius, color)],
            StateLayerKind::Outline => vec![items::rect_stroke(rect, radius, width, color, StrokeAlign::Inside)],
        }
    }
}
