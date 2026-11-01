//! [`range_bands`] and [`rect_around`]: the outline and fill of a range selection (GridCraft).

use egui::Rect;

/// The four sides of a `width` wide outline centred on `rect`, as filled rectangles cut to `clip`
/// (expanded by `2 × width`), all in screen points: left, right, top, bottom in that order, the
/// ones wholly outside the clip left out. A stroked rectangle that spans a 20-million-point column
/// loses its precision in the tessellator and its sides vanish; sides filled from rectangles that
/// stay near the viewport don't. Non-finite `rect` or `clip` gives none.
pub fn range_bands(rect: Rect, width: f32, clip: Rect) -> Vec<Rect> {
    // gridcraft: crates/ui-egui/src/grid/selection.rs:60-78 `outline_bands`.
    todo!()
}

/// `area` without `hole`, as up to four rectangles that never overlap (a translucent fill can't be
/// cleared by painting over it): the strip above the hole, the strip below, then what is left on
/// each side. A hole that misses the area, or isn't finite, leaves the area whole; pieces with no
/// size or with numbers that aren't finite are dropped. The fill of a range selection with its
/// active cell left clear.
pub fn rect_around(area: Rect, hole: Rect) -> Vec<Rect> {
    // gridcraft: crates/ui-egui/src/grid/selection.rs:33-53 `rect_around` (`intersect` takes the sane
    // side of a NaN, so a broken hole is dropped first).
    todo!()
}
