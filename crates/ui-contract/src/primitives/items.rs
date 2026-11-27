//! Display items in window coordinates: what the painters build and [`Canvas`](super::Canvas) records, moved into the
//! node's coordinates. Each constructor paints what the egui shape of the same name painted, so a list rendered by
//! craft-ui-render-epaint matches the egui build pixel for pixel.

use craft_theme::Rgba;
use craft_ui_rt::{CornerRadii, Dash, Fill};
/// The display-list types painters build, for crates that paint through the contract.
pub use craft_ui_rt::{Item, Mesh, Stroke, StrokeAlign, Vertex};
use craft_ui_rt::{Pos2, Rect, Vec2};
use egui::{Color32, CornerRadius};
use std::sync::Arc;

/// A colour from the style as a display-list colour (straight alpha).
pub fn rgba(color: Color32) -> Rgba {
    Rgba(color.to_srgba_unmultiplied())
}

/// The style's whole-point corner radii as display-list radii.
pub fn radii(r: CornerRadius) -> CornerRadii {
    CornerRadii { top_left: f32::from(r.nw), top_right: f32::from(r.ne), bottom_right: f32::from(r.se), bottom_left: f32::from(r.sw) }
}

/// A stroke of `width` points in `color`, at `align` against a closed shape's edge; `None` when it paints nothing.
pub fn stroke(width: f32, color: Color32, align: StrokeAlign) -> Option<Stroke> {
    (width > 0.0 && color != Color32::TRANSPARENT).then(|| Stroke::new(width, rgba(color)).aligned(align))
}

/// A rect with `fill` and `stroke`.
pub fn rect(rect: Rect, radius: CornerRadius, fill: Color32, stroke: Option<Stroke>) -> Item {
    Item::Rect { rect, radii: radii(radius), fill: rgba(fill), stroke }
}

/// A filled rect.
pub fn rect_filled(r: Rect, radius: CornerRadius, fill: Color32) -> Item {
    rect(r, radius, fill, None)
}

/// A rect's outline `width` wide at `align`.
pub fn rect_stroke(r: Rect, radius: CornerRadius, width: f32, color: Color32, align: StrokeAlign) -> Item {
    rect(r, radius, Color32::TRANSPARENT, stroke(width, color, align))
}

/// A filled circle.
pub fn circle_filled(center: Pos2, radius: f32, fill: Color32) -> Item {
    Item::Circle { center, radius, fill: rgba(fill), stroke: None }
}

/// A circle's outline `width` wide, outside the radius as egui strokes a circle.
pub fn circle_stroke(center: Pos2, radius: f32, width: f32, color: Color32) -> Item {
    Item::Circle { center, radius, fill: Rgba([0, 0, 0, 0]), stroke: stroke(width, color, StrokeAlign::Outside) }
}

/// A line segment, snapped to the pixel grid by the renderer.
pub fn segment(from: Pos2, to: Pos2, width: f32, color: Color32) -> Option<Item> {
    Some(Item::Line { from, to, stroke: stroke(width, color, StrokeAlign::Middle)? })
}

/// The path through `points` as kurbo has it.
fn bez(points: &[Pos2], closed: bool) -> craft_behavior::kurbo::BezPath {
    let at = |q: Pos2| craft_behavior::kurbo::Point::new(f64::from(q.x), f64::from(q.y));
    let mut path = craft_behavior::kurbo::BezPath::new();
    path.move_to(at(points[0]));
    for &q in &points[1..] {
        path.line_to(at(q));
    }
    if closed {
        path.close_path();
    }
    path
}

/// The open or closed line through `points`, stroked centred on it; `None` for fewer than two points.
pub fn polyline(points: &[Pos2], closed: bool, line: Option<Stroke>) -> Option<Item> {
    let line = line?;
    (points.len() >= 2).then(|| Item::Path { path: Arc::new(bez(points, closed)), fill: None, stroke: Some(line) })
}

/// The convex polygon through `points`, filled and stroked; `None` when it paints nothing.
pub fn convex_polygon(points: &[Pos2], fill: Color32, line: Option<Stroke>) -> Option<Item> {
    let fill = (fill != Color32::TRANSPARENT).then(|| Fill::convex(rgba(fill)));
    (points.len() >= 2 && (fill.is_some() || line.is_some())).then(|| Item::Path { path: Arc::new(bez(points, true)), fill, stroke: line })
}

/// The open line through `points` as dashes of `pattern` (`on`, `off`, starting `offset` into it).
pub fn dashed(points: &[Pos2], dash: Dash, width: f32, color: Color32) -> Option<Item> {
    polyline(points, false, stroke(width, color, StrokeAlign::Middle).map(|line| line.dashed(dash)))
}

/// A rect shading from `from` at the edge `direction` starts at to `to` at the other, as egui's gradient rect (the
/// same vertices and triangles).
pub fn gradient_rect(rect: Rect, direction: egui::Direction, [from, to]: [Color32; 2]) -> Item {
    let (from, to) = (rgba(from), rgba(to));
    let (left_top, right_top, left_bottom, right_bottom) = match direction {
        egui::Direction::TopDown => (from, from, to, to),
        egui::Direction::BottomUp => (to, to, from, from),
        egui::Direction::LeftToRight => (from, to, from, to),
        egui::Direction::RightToLeft => (to, from, to, from),
    };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), left_top);
    mesh.colored_vertex(rect.right_top(), right_top);
    mesh.colored_vertex(rect.left_bottom(), left_bottom);
    mesh.colored_vertex(rect.right_bottom(), right_bottom);
    mesh.indices = vec![0, 1, 2, 2, 1, 3];
    Item::Mesh(Arc::new(mesh))
}

/// `mesh` as an item; `None` when it has no triangles.
pub fn mesh(mesh: Mesh) -> Option<Item> {
    (!mesh.is_empty()).then(|| Item::Mesh(Arc::new(mesh)))
}

/// `item` moved by `by`.
pub fn translate(item: &mut Item, by: Vec2) {
    if by == Vec2::ZERO {
        return;
    }
    match item {
        Item::Rect { rect, .. } | Item::Shadow { rect, .. } | Item::Image { rect, .. } | Item::ImageMask { rect, .. } => *rect = rect.translate(by),
        Item::VGradient { rect, .. } | Item::Surface { rect, .. } => *rect = rect.translate(by),
        Item::Circle { center, .. } => *center += by,
        Item::Line { from, to, .. } => {
            *from += by;
            *to += by;
        }
        Item::Path { path, .. } => {
            Arc::make_mut(path).apply_affine(craft_behavior::kurbo::Affine::translate((f64::from(by.x), f64::from(by.y))));
        }
        Item::Mesh(mesh) => Arc::make_mut(mesh).vertices.iter_mut().for_each(|v| v.pos += by),
        Item::Text { origin, .. } => *origin += by,
        Item::PushClip(clip) => clip.rect = clip.rect.translate(by),
        Item::PushOpacity { bounds, .. } => *bounds = bounds.translate(by),
        Item::Slot(_) | Item::PopClip | Item::PopOpacity => {}
    }
}

/// Every colour `item` paints passed through `map`; a text without a colour of its own keeps the layout's.
pub(crate) fn map_colors(item: &mut Item, map: impl Fn(Rgba) -> Rgba) {
    let line = |s: &mut Option<Stroke>| {
        if let Some(s) = s {
            s.color = map(s.color);
        }
    };
    match item {
        Item::Rect { fill, stroke, .. } | Item::Circle { fill, stroke, .. } => {
            *fill = map(*fill);
            line(stroke);
        }
        Item::Line { stroke, .. } => stroke.color = map(stroke.color),
        Item::Path { fill, stroke, .. } => {
            if let Some(fill) = fill {
                fill.color = map(fill.color);
            }
            line(stroke);
        }
        Item::Mesh(mesh) => Arc::make_mut(mesh).vertices.iter_mut().for_each(|v| v.color = map(v.color)),
        Item::Shadow { color, .. } | Item::ImageMask { color, .. } => *color = map(*color),
        Item::Image { tint, .. } => *tint = map(*tint),
        Item::VGradient { top, bottom, .. } => {
            *top = map(*top);
            *bottom = map(*bottom);
        }
        Item::Text { color, highlight, .. } => {
            if let Some(color) = color {
                *color = map(*color);
            }
            if let Some(highlight) = highlight {
                highlight.fill = map(highlight.fill);
            }
        }
        Item::Surface { .. } | Item::Slot(_) | Item::PushClip(_) | Item::PopClip | Item::PushOpacity { .. } | Item::PopOpacity => {}
    }
}

/// `color` at `share` of its opacity, rounded as egui's painter fades a colour (premultiplied, per channel).
pub(crate) fn fade(color: Rgba, share: f32) -> Rgba {
    let [r, g, b, a] = color.0;
    rgba(Color32::from_rgba_unmultiplied(r, g, b, a).gamma_multiply(share))
}
