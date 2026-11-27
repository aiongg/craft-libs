//! `Surface`: the pure display items of a surface kind.

use super::items;
use crate::style::{SurfaceEdge, SurfaceStyle};
use craft_i18n::Direction;
use craft_ui_rt::{Item, Mesh, StrokeAlign};
use egui::emath::GuiRounding as _;
use egui::{Color32, CornerRadius, Pos2, Rect, Vec2, vec2};

use crate::style::Corners;

/// The surface painter: shadow, fill, then edge.
pub struct Surface;

impl Surface {
    /// The items of `style` over `rect`, snapped to the pixel grid: shadow, fill, edge. `fill` replaces the
    /// kind's fill (the state fill), so washes never stack. A bevel is whole physical pixels wide.
    pub fn items(
        style: &SurfaceStyle,
        fill: Option<Color32>,
        rect: Rect,
        corners: Corners,
        direction: Direction,
        pixels_per_point: f32,
    ) -> Vec<Item> {
        let rect = rect.round_to_pixels(pixels_per_point);
        let radius = corners.apply(style.radius, direction);
        let mut out = Vec::with_capacity(3);
        if let Some(shadow) = style.shadow {
            out.push(Item::Shadow {
                rect,
                radii: items::radii(radius),
                offset: Vec2::new(f32::from(shadow.offset[0]), f32::from(shadow.offset[1])),
                blur: f32::from(shadow.blur),
                spread: f32::from(shadow.spread),
                color: items::rgba(shadow.color),
            });
        }
        let fill = fill.unwrap_or(style.fill);
        if fill != Color32::TRANSPARENT {
            out.push(items::rect_filled(rect, radius, fill));
        }
        match style.edge {
            SurfaceEdge::Border(stroke) => out.push(items::rect_stroke(rect, radius, stroke.width, stroke.color, StrokeAlign::Inside)),
            SurfaceEdge::Bevel { top_left, bottom_right, width } => {
                let width = (width * pixels_per_point).round().max(1.0) / pixels_per_point;
                out.extend(items::mesh(bevel(rect, radius, width, items::rgba(top_left), items::rgba(bottom_right))));
            }
            _ => {}
        }
        out
    }
}

/// The bevel inside `rect`: two strips between the outline and the outline inset by `width`, split at 45 degrees on
/// the top-right and bottom-left corners. Both outlines are sampled at the same angles, so square corners get
/// mitred joins and round ones follow the radius.
fn bevel(rect: Rect, radius: CornerRadius, width: f32, top_left: craft_theme::Rgba, bottom_right: craft_theme::Rgba) -> Mesh {
    let width = width.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let inner = rect.shrink(width);
    let max_r = rect.width().min(rect.height()) / 2.0;
    let corner = |point: Pos2, inner_point: Pos2, into: Vec2, radius: u8, start: f32| {
        let outer_r = f32::from(radius).min(max_r);
        let inner_r = (outer_r - width).max(0.0);
        (point + into * outer_r, outer_r, inner_point + into * inner_r, inner_r, start)
    };
    let corners = [
        corner(rect.left_top(), inner.left_top(), Vec2::RIGHT + Vec2::DOWN, radius.nw, 180.0),
        corner(rect.right_top(), inner.right_top(), Vec2::LEFT + Vec2::DOWN, radius.ne, 270.0),
        corner(rect.right_bottom(), inner.right_bottom(), Vec2::LEFT + Vec2::UP, radius.se, 0.0),
        corner(rect.left_bottom(), inner.left_bottom(), Vec2::RIGHT + Vec2::UP, radius.sw, 90.0),
    ];
    let at = |centre: Pos2, r: f32, degrees: f32| centre + r * vec2(degrees.to_radians().cos(), degrees.to_radians().sin());
    let arc = |i: usize, from: f32, to: f32, out: &mut Vec<(Pos2, Pos2)>| {
        let (oc, or, ic, ir, start) = corners[i];
        let steps = ((or * (to - from) / 90.0 / 2.0).ceil() as usize).clamp(1, 16);
        for s in 0..=steps {
            let a = start + from + (to - from) * s as f32 / steps as f32;
            out.push((at(oc, or, a), at(ic, ir, a)));
        }
    };
    let mut lit = Vec::new();
    arc(3, 45.0, 90.0, &mut lit);
    arc(0, 0.0, 90.0, &mut lit);
    arc(1, 0.0, 45.0, &mut lit);
    let mut shaded = Vec::new();
    arc(1, 45.0, 90.0, &mut shaded);
    arc(2, 0.0, 90.0, &mut shaded);
    arc(3, 0.0, 45.0, &mut shaded);
    let mut mesh = Mesh::default();
    for (strip, color) in [(lit, top_left), (shaded, bottom_right)] {
        let base = mesh.vertices.len() as u32;
        for (outer, inner) in &strip {
            mesh.colored_vertex(*outer, color);
            mesh.colored_vertex(*inner, color);
        }
        for i in 0..strip.len().saturating_sub(1) as u32 {
            let (o0, i0, o1, i1) = (base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3);
            mesh.add_triangle(o0, i0, o1);
            mesh.add_triangle(i0, i1, o1);
        }
    }
    mesh
}
