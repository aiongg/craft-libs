//! [`range_bands`] and [`rect_around`]: the outline and fill of a range selection (GridCraft).

use egui::{Rect, pos2};

/// The four sides of a `width` wide outline centred on `rect`, as filled rectangles cut to `clip`
/// (expanded by `2 × width`), all in screen points: left, right, top, bottom in that order, the
/// ones wholly outside the clip left out. A stroked rectangle that spans a 20-million-point column
/// loses its precision in the tessellator and its sides vanish; sides filled from rectangles that
/// stay near the viewport don't. Non-finite `rect` or `clip` gives none.
pub fn range_bands(rect: Rect, width: f32, clip: Rect) -> Vec<Rect> {
    // gridcraft: crates/ui-egui/src/grid/selection.rs:60-78 `outline_bands`.
    if !rect.is_finite() || !clip.is_finite() {
        return Vec::new();
    }
    let h = width / 2.0;
    let clip = clip.expand(width * 2.0);
    let sides = [
        Rect::from_min_max(pos2(rect.left() - h, rect.top() - h), pos2(rect.left() + h, rect.bottom() + h)),
        Rect::from_min_max(pos2(rect.right() - h, rect.top() - h), pos2(rect.right() + h, rect.bottom() + h)),
        Rect::from_min_max(pos2(rect.left() - h, rect.top() - h), pos2(rect.right() + h, rect.top() + h)),
        Rect::from_min_max(pos2(rect.left() - h, rect.bottom() - h), pos2(rect.right() + h, rect.bottom() + h)),
    ];
    sides.into_iter().map(|s| s.intersect(clip)).filter(|s| s.is_finite() && s.width() > 0.0 && s.height() > 0.0).collect()
}

/// `area` without `hole`, as up to four rectangles that never overlap (a translucent fill can't be
/// cleared by painting over it): the strip above the hole, the strip below, then what is left on
/// each side. A hole that misses the area, or isn't finite, leaves the area whole; pieces with no
/// size or with numbers that aren't finite are dropped. The fill of a range selection with its
/// active cell left clear.
pub fn rect_around(area: Rect, hole: Rect) -> Vec<Rect> {
    // gridcraft: crates/ui-egui/src/grid/selection.rs:33-53 `rect_around` (`intersect` takes the sane
    // side of a NaN, so a broken hole is dropped first).
    let hole = if hole.is_finite() { area.intersect(hole) } else { Rect::NOTHING };
    let parts = if hole.is_positive() {
        [
            Rect::from_min_max(area.min, pos2(area.max.x, hole.min.y)),
            Rect::from_min_max(pos2(area.min.x, hole.max.y), area.max),
            Rect::from_min_max(pos2(area.min.x, hole.min.y), pos2(hole.min.x, hole.max.y)),
            Rect::from_min_max(pos2(hole.max.x, hole.min.y), pos2(area.max.x, hole.max.y)),
        ]
    } else {
        [area, Rect::NOTHING, Rect::NOTHING, Rect::NOTHING]
    };
    parts.into_iter().filter(|r| r.is_positive() && r.is_finite()).collect()
}
