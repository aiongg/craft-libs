//! The painted pointers of the canvas: [`turn_cursor`] (rotate), [`distort_cursor`],
//! [`skew_cursor`] and [`selection_cursor`]. egui has no icons for them.

use egui::{Color32, Context, CursorIcon, Id, LayerId, Order, Painter, Pos2, Shape, Stroke, Vec2, vec2};

use super::CanvasView;
use crate::theme::{ToEgui, current};

/// Half the arc's sweep, in radians (PhotoCraft's).
const HALF_SWEEP: f32 = 0.75;
/// Segments of the drawn arc.
const ARC_SEGMENTS: usize = 16;

/// Paints PhotoCraft's rotate pointer at screen point `at` and hides the system pointer for this
/// pass: a short arc with an arrowhead at each end, bent round the corner that lies in screen
/// direction `toward` from the pointer (the corner's bisector into the box, as the view shows it).
/// egui has no rotate cursor.
///
/// The arc is `transform.turn_radius` across, an `overlay.stroke` line `transform.turn_line` wide
/// over an `overlay.halo` line `transform.turn_halo` wider on each side; the arrowheads are
/// `overlay.stroke` outlined in the halo. It draws on the tooltip layer, above the canvas and its
/// overlays. Nothing for a zero `toward`.
pub fn turn_cursor(ctx: &Context, at: Pos2, toward: Vec2) {
    turn_cursor_with(ctx, at, toward, true);
}

/// [`turn_cursor`] with the host's `native` switch: true hides the system pointer for this pass,
/// so the arc is the only pointer; false leaves the system pointer as the host has set it (a
/// marker at the hotspot) and paints the arc beside it.
pub fn turn_cursor_with(ctx: &Context, at: Pos2, toward: Vec2, native: bool) {
    let len = toward.length();
    if !len.is_finite() || len < 1e-6 {
        return;
    }
    if native {
        ctx.set_cursor_icon(CursorIcon::None);
    }
    let tokens = current(ctx);
    let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
    let (color, halo) = (o.stroke.to_egui(), o.halo.to_egui());
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::NULL.with("craft_ui::canvas::turn_cursor")));
    let u = toward / len;
    let r = t.turn_radius;
    let centre = at + u * r;
    let mid = (-u.y).atan2(-u.x);
    let on_arc = |a: f32| centre + vec2(a.cos(), a.sin()) * r;
    let points: Vec<Pos2> = (0..=ARC_SEGMENTS).map(|i| on_arc(mid - HALF_SWEEP + 2.0 * HALF_SWEEP * i as f32 / ARC_SEGMENTS as f32)).collect();
    painter.add(Shape::line(points.clone(), Stroke::new(t.turn_line + 2.0 * t.turn_halo, halo)));
    painter.add(Shape::line(points, Stroke::new(t.turn_line, color)));
    // Each arrowhead points along the arc, away from its middle; sized to the arc as PhotoCraft's.
    let head = r / 12.0;
    for (a, sign) in [(mid - HALF_SWEEP, -1.0f32), (mid + HALF_SWEEP, 1.0)] {
        let tangent = vec2(-a.sin(), a.cos()) * sign;
        let normal = vec2(a.cos(), a.sin());
        let tip = on_arc(a) + tangent * 2.0 * head;
        painter.add(Shape::convex_polygon(
            vec![tip + tangent * 3.5 * head, tip - tangent * 3.0 * head + normal * 3.5 * head, tip - tangent * 3.0 * head - normal * 3.5 * head],
            color,
            Stroke::new(t.turn_halo, halo),
        ));
    }
}

/// The ink of the cursors drawn here: `overlay.stroke` lines `handle.stroke` wide over an
/// `overlay.halo` line `handle.halo` wider on each side.
struct Ink {
    color: Color32,
    halo: Color32,
    line: f32,
    halo_line: f32,
    /// The halo stroke round a filled shape.
    outline: Stroke,
}

impl Ink {
    fn of(ctx: &Context) -> Self {
        let tokens = current(ctx);
        let (h, o) = (&tokens.component.handle, &tokens.color.overlay);
        let halo = o.halo.to_egui();
        let halo_line = h.stroke + 2.0 * h.halo;
        Ink { color: o.stroke.to_egui(), halo, line: h.stroke, halo_line, outline: Stroke::new(halo_line, halo) }
    }

    /// A segment: the halo line, then the ink line.
    fn segment(&self, painter: &Painter, a: Pos2, b: Pos2) {
        painter.line_segment([a, b], Stroke::new(self.halo_line, self.halo));
        painter.line_segment([a, b], Stroke::new(self.line, self.color));
    }

    /// A filled convex shape with the halo round it.
    fn fill(&self, painter: &Painter, points: Vec<Pos2>) {
        painter.add(Shape::convex_polygon(points, self.color, self.outline));
    }
}

fn tooltip_painter(ctx: &Context, name: &str) -> Painter {
    ctx.layer_painter(LayerId::new(Order::Tooltip, Id::NULL.with(name)))
}

/// The unit screen direction of document vector `v` in `view`, or `fallback` for a zero one.
fn screen_dir(view: &CanvasView, v: kurbo::Vec2, fallback: Vec2) -> Vec2 {
    let s = view.map_vec(v);
    let len = s.length();
    if len.is_finite() && len > 1e-6 { s / len } else { fallback }
}

/// Paints PhotoCraft's Free Distort pointer at screen point `at` and hides the system pointer
/// for this pass: an arrowhead with its tip at `at`, pointing along document direction `doc_dir`
/// as `view` shows it (from the pivot out through the corner under the pointer). A zero `doc_dir`
/// points up and left. Ink as [`turn_cursor`]'s, at the handle widths.
pub fn distort_cursor(ctx: &Context, at: Pos2, view: &CanvasView, doc_dir: kurbo::Vec2) {
    const LENGTH: f32 = 16.0;
    const HALF_WIDTH: f32 = 7.0;
    ctx.set_cursor_icon(CursorIcon::None);
    let (ink, painter) = (Ink::of(ctx), tooltip_painter(ctx, "craft_ui::canvas::distort_cursor"));
    let u = screen_dir(view, doc_dir, vec2(-1.0, -1.0).normalized());
    let perp = vec2(-u.y, u.x);
    let base = at - u * LENGTH;
    let (left, right, notch) = (base + perp * HALF_WIDTH, base - perp * HALF_WIDTH, at - u * (LENGTH * 0.65));
    // Two triangles meeting along the tip and the notch: the dart is not convex.
    painter.add(Shape::closed_line(vec![at, left, notch, right], ink.outline));
    ink.fill(&painter, vec![at, left, notch]);
    ink.fill(&painter, vec![at, notch, right]);
}

/// Paints PhotoCraft's Skew pointer at screen point `at` and hides the system pointer for this
/// pass: two opposed arrows beside each other, along document direction `doc_dir` (the edge's
/// tangent) as `view` shows it. A zero `doc_dir` lies along x. Ink as [`distort_cursor`]'s.
pub fn skew_cursor(ctx: &Context, at: Pos2, view: &CanvasView, doc_dir: kurbo::Vec2) {
    ctx.set_cursor_icon(CursorIcon::None);
    let (ink, painter) = (Ink::of(ctx), tooltip_painter(ctx, "craft_ui::canvas::skew_cursor"));
    let u = screen_dir(view, doc_dir, vec2(1.0, 0.0));
    let perp = vec2(-u.y, u.x);
    for (offset, dir) in [(4.0f32, 1.0f32), (-4.0, -1.0)] {
        let centre = at + perp * offset;
        let end = centre + u * (dir * 9.0);
        ink.segment(&painter, centre - u * (dir * 9.0), end);
        ink.fill(&painter, vec![end + u * (dir * 3.5), end - u * (dir * 2.5) + perp * 3.5, end - u * (dir * 2.5) - perp * 3.5]);
    }
}

/// What a selection tool's next gesture does to the selection, as its cursor shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionBadge {
    /// A plus.
    Add,
    /// A minus.
    Subtract,
    /// A cross.
    Intersect,
    /// Replaces the selection: no badge.
    New,
}

impl SelectionBadge {
    /// The badge glyph as segments round `c`, half a side `h`.
    fn glyph(self, c: Pos2, h: f32) -> Vec<[Pos2; 2]> {
        match self {
            SelectionBadge::Add => vec![[c - vec2(h, 0.0), c + vec2(h, 0.0)], [c - vec2(0.0, h), c + vec2(0.0, h)]],
            SelectionBadge::Subtract => vec![[c - vec2(h, 0.0), c + vec2(h, 0.0)]],
            SelectionBadge::Intersect => {
                let d = h * 0.8;
                vec![[c + vec2(-d, -d), c + vec2(d, d)], [c + vec2(-d, d), c + vec2(d, -d)]]
            }
            SelectionBadge::New => Vec::new(),
        }
    }
}

/// Paints a selection tool's pointer at screen point `at`: the crosshair with the `badge` beside
/// it. `native` is the host's switch: true sets the system crosshair icon and paints the badge
/// only (an icon cannot carry it); false hides the system pointer and paints the crosshair too
/// (PhotoCraft's `tool_cursor`, where the system cannot show the host's cursor image).
pub fn selection_cursor(ctx: &Context, at: Pos2, badge: SelectionBadge, native: bool) {
    const REACH: f32 = 8.0;
    const GAP: f32 = 2.0;
    let (ink, painter) = (Ink::of(ctx), tooltip_painter(ctx, "craft_ui::canvas::selection_cursor"));
    if native {
        ctx.set_cursor_icon(CursorIcon::Crosshair);
    } else {
        ctx.set_cursor_icon(CursorIcon::None);
        for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
            ink.segment(&painter, at + d * GAP, at + d * REACH);
        }
    }
    paint_badge(&ink, &painter, at + vec2(12.0, 12.0), badge);
}

/// [`selection_cursor`] for a brush-circle tool, whose circle the host draws: the `badge` at
/// `at`, the circle's centre. The system pointer is the host's.
pub fn selection_badge_at(ctx: &Context, at: Pos2, badge: SelectionBadge) {
    paint_badge(&Ink::of(ctx), &tooltip_painter(ctx, "craft_ui::canvas::selection_cursor"), at, badge);
}

fn paint_badge(ink: &Ink, painter: &Painter, at: Pos2, badge: SelectionBadge) {
    for [a, b] in badge.glyph(at, 3.5) {
        ink.segment(painter, a, b);
    }
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;
