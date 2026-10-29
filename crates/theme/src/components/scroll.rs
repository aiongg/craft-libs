//! Scroll bars: the thin floating bars of panel scroll areas and the always-shown bars along the
//! bottom and right of a canvas. The family holds their geometry, the same at every Size and Text
//! size. The colours are semantic tokens: the canvas bars' track is `surface.chrome` with a
//! `line.divider` edge, the thumb `text.faint` at rest and `text.muted` while hot.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::tokens::Edge;
use crate::values;

/// Scroll bar tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollTokens {
    /// Panel scroll areas draw a solid bar that takes room beside the content, as a bevelled
    /// theme does, instead of a thin floating one: `scroll.solid`.
    pub solid: bool,
    /// Width of a thin floating bar: `scroll.floating_width`.
    pub floating_width: f32,
    /// Opacity of the handle while the pointer is on the bar: `scroll.handle_opacity`.
    pub handle_opacity: f32,
    /// Opacity of the track while the pointer is on the bar: `scroll.track_opacity`.
    pub track_opacity: f32,
    /// Thickness of a canvas bar and side of the corner square: `scroll.canvas_width`.
    pub canvas_width: f32,
    /// Between a canvas bar's edge and its thumb: `scroll.thumb_inset`.
    pub thumb_inset: f32,
    /// Shortest canvas thumb: `scroll.thumb_min`.
    pub thumb_min: f32,
}

/// Scroll bars have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScrollField {}

impl Family for ScrollTokens {
    type Field = ScrollField;
    const FIELDS: &'static [ScrollField] = &[];

    fn defaults(s: &Semantic<'_>) -> ScrollTokens {
        ScrollTokens {
            solid: matches!(s.shape.edge, Edge::Bevel { .. }),
            floating_width: values::SCROLL_FLOATING_WIDTH,
            handle_opacity: values::SCROLL_HANDLE_OPACITY,
            track_opacity: values::SCROLL_TRACK_OPACITY,
            canvas_width: values::SCROLL_CANVAS_WIDTH,
            thumb_inset: values::SCROLL_THUMB_INSET,
            thumb_min: values::SCROLL_THUMB_MIN,
        }
    }

    fn path(f: ScrollField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ScrollField> {
        None
    }

    fn kind(f: ScrollField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ScrollField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ScrollField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the thumb against the track is a semantic rule (`values::RULES`).
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
