//! [`turn_cursor`]: the rotate pointer outside a transform box's corners.

use egui::{Context, CursorIcon, Id, LayerId, Order, Pos2, Shape, Stroke, Vec2, vec2};

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
    let len = toward.length();
    if !len.is_finite() || len < 1e-6 {
        return;
    }
    ctx.set_cursor_icon(CursorIcon::None);
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
