//! `ComparePair` and `survey_columns`: the two-pane and survey arrangements.

use egui::{Rect, pos2, vec2};

use super::CompareStyle;

/// The two panes of Compare: equal rects in the area inset `tile.compare_margin` (18), split by
/// `tile.compare_gap` (16) with a 1 pt `line.divider` in the middle. In a right-to-left
/// interface the Select pane is the right one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComparePair {
    rtl: bool,
}

impl ComparePair {
    /// Left to right.
    pub fn new() -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:499 (`show_compare`).
        Self { rtl: false }
    }

    /// Mirrors the panes.
    #[must_use]
    pub fn rtl(self, rtl: bool) -> Self {
        // new: the Select pane is at the reading start.
        Self { rtl }
    }

    /// The Select pane and the Candidate pane, in that order: each `(area − 2 × margin − gap) / 2`
    /// wide.
    pub fn rects(&self, area: Rect, style: &CompareStyle) -> [Rect; 2] {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:499 (two panes of (area − 16)/2 in an area inset 18).
        let inner = area.shrink(style.margin);
        let side = ((inner.width() - style.gap) / 2.0).max(0.0);
        let left = Rect::from_min_size(inner.min, vec2(side, inner.height()));
        let right = Rect::from_min_size(pos2(inner.max.x - side, inner.min.y), vec2(side, inner.height()));
        if self.rtl { [right, left] } else { [left, right] }
    }

    /// Where the divider goes: a vertical segment between the panes.
    pub fn divider(&self, area: Rect, style: &CompareStyle) -> Rect {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:499 (the 1 pt line between the panes).
        let inner = area.shrink(style.margin);
        // A zero-width rect: the caller strokes its vertical edge with `line.divider`.
        Rect::from_min_max(pos2(inner.center().x, inner.min.y), pos2(inner.center().x, inner.max.y))
    }
}

/// How many columns show `n` survey tiles largest in `area`, for 3:2 pictures: the column count
/// in `1..=n` whose tile `min(width / columns, height / rows × 1.5)` is largest (the first on a
/// tie).
pub fn survey_columns(n: usize, area: Rect) -> usize {
    // lightcraft: crates/ui-egui/src/panels/compare.rs:539 (`survey_columns`).
    let mut best = (1, 0.0f32);
    for columns in 1..=n.max(1) {
        let rows = n.div_ceil(columns);
        let size = (area.width() / columns as f32).min(area.height() / rows as f32 * 1.5);
        if size > best.1 {
            best = (columns, size);
        }
    }
    best.0
}
