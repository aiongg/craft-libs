//! A chart drawn from a primitive list (N6.05).

use craft_ui::theme::current;
use egui::epaint::{PathShape, PathStroke, TextShape};
use egui::{Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

use super::style::SheetStyle;

/// A line's width never shrinks below this share of its chart width (a zoom floor, not a size).
const MIN_STROKE_ZOOM: f32 = 0.5;
/// One wedge step spans this many radians.
const WEDGE_STEP: f32 = 0.05;
/// A wedge has between these many steps.
const WEDGE_STEPS: (usize, usize) = (2, 400);
/// Cross products smaller than this are collinear when testing a polygon for convexity.
const COLLINEAR: f32 = 1e-3;

/// Horizontal text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

/// A drawing primitive in chart coordinates (origin top-left, y down, points). Colours are
/// straight RGBA. The app's chart engine (GridCraft's `gridcraft-chart`) produces the list; the
/// library paints it.
#[derive(Clone, Debug, PartialEq)]
pub enum ChartPrimitive {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Option<[u8; 4]>,
        stroke: Option<([u8; 4], f32)>,
        radius: f32,
    },
    Line {
        points: Vec<[f32; 2]>,
        color: [u8; 4],
        width: f32,
        dash: bool,
    },
    Polygon {
        points: Vec<[f32; 2]>,
        fill: [u8; 4],
        stroke: Option<([u8; 4], f32)>,
    },
    /// A pie or doughnut sector: centre, radii, angles in radians (0 at 12 o'clock, clockwise).
    Wedge {
        cx: f32,
        cy: f32,
        r_outer: f32,
        r_inner: f32,
        a0: f32,
        a1: f32,
        fill: [u8; 4],
        stroke: Option<([u8; 4], f32)>,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
        fill: Option<[u8; 4]>,
        stroke: Option<([u8; 4], f32)>,
    },
    /// Text anchored at `(x, y)` by `align` / `valign`, rotated by `rotation` radians about the
    /// anchor; `size` in points.
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: [u8; 4],
        bold: bool,
        align: HAlign,
        valign: VAlign,
        rotation: f32,
    },
}

/// Paints `primitives` with their origin at `origin`, scaled by `zoom`: rects (stroke inside),
/// lines (stroke width × max(zoom, 0.5); dashed 4 / 3 points × zoom), polygons (a convex fill,
/// else a mesh of the closed path), wedges as meshes with one step per 0.05 rad (2…400 steps)
/// and their outline, circles, text at `size × zoom` in the UI face (bold when asked), rotated
/// about its anchor.
// gridcraft: crates/ui-egui/src/chartview.rs:45 `paint_prims`, `:152` `is_convex`, `:173`
// `area_mesh`; crates/chart/src/lib.rs:53 `Prim`.
pub fn paint_chart(painter: &Painter, origin: Pos2, primitives: &[ChartPrimitive], zoom: f32) {
    let tokens = current(painter.ctx());
    let style = SheetStyle::of(&tokens);
    let at = |x: f32, y: f32| pos2(origin.x + x * zoom, origin.y + y * zoom);
    let rgba = |c: [u8; 4]| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    let stroke_zoom = zoom.max(MIN_STROKE_ZOOM);
    for primitive in primitives {
        match primitive {
            ChartPrimitive::Rect { x, y, w, h, fill, stroke, radius } => {
                let rect = Rect::from_min_size(at(*x, *y), vec2(w * zoom, h * zoom));
                if let Some(fill) = fill {
                    painter.rect_filled(rect, radius * zoom, rgba(*fill));
                }
                if let Some((color, width)) = stroke {
                    painter.rect_stroke(rect, radius * zoom, Stroke::new(width * stroke_zoom, rgba(*color)), StrokeKind::Inside);
                }
            }
            ChartPrimitive::Line { points, color, width, dash } => {
                let points: Vec<Pos2> = points.iter().map(|q| at(q[0], q[1])).collect();
                if points.len() < 2 {
                    continue;
                }
                let stroke = Stroke::new(width * stroke_zoom, rgba(*color));
                if *dash {
                    painter.extend(Shape::dashed_line(&points, stroke, style.chart_dash * zoom, style.chart_dash_gap * zoom));
                } else {
                    painter.add(PathShape::line(points, PathStroke::new(stroke.width, stroke.color)));
                }
            }
            ChartPrimitive::Polygon { points, fill, stroke } => {
                let points: Vec<Pos2> = points.iter().map(|q| at(q[0], q[1])).collect();
                if points.len() < 3 {
                    continue;
                }
                if is_convex(&points) {
                    painter.add(PathShape { points: points.clone(), closed: true, fill: rgba(*fill), stroke: PathStroke::NONE });
                } else {
                    painter.add(Shape::mesh(area_mesh(&points, rgba(*fill))));
                }
                if let Some((color, width)) = stroke {
                    painter.add(PathShape::closed_line(points, PathStroke::new(width * stroke_zoom, rgba(*color))));
                }
            }
            ChartPrimitive::Wedge { cx, cy, r_outer, r_inner, a0, a1, fill, stroke } => {
                let centre = at(*cx, *cy);
                let steps = (((a1 - a0).abs() / WEDGE_STEP).ceil() as usize).clamp(WEDGE_STEPS.0, WEDGE_STEPS.1);
                let on = |angle: f32, radius: f32| pos2(centre.x + radius * zoom * angle.sin(), centre.y - radius * zoom * angle.cos());
                let angle = |i: usize| a0 + (a1 - a0) * i as f32 / steps as f32;
                let color = rgba(*fill);
                let mut mesh = Mesh::default();
                for i in 0..=steps {
                    mesh.colored_vertex(on(angle(i), *r_outer), color);
                    mesh.colored_vertex(if *r_inner > 0.0 { on(angle(i), *r_inner) } else { centre }, color);
                }
                for i in 0..steps as u32 {
                    let b = i * 2;
                    mesh.add_triangle(b, b + 1, b + 2);
                    mesh.add_triangle(b + 1, b + 3, b + 2);
                }
                painter.add(Shape::mesh(mesh));
                if let Some((color, width)) = stroke {
                    let mut outline: Vec<Pos2> = (0..=steps).map(|i| on(angle(i), *r_outer)).collect();
                    if *r_inner > 0.0 {
                        outline.extend((0..=steps).rev().map(|i| on(angle(i), *r_inner)));
                    } else {
                        outline.push(centre);
                    }
                    painter.add(PathShape::closed_line(outline, PathStroke::new(width * stroke_zoom, rgba(*color))));
                }
            }
            ChartPrimitive::Circle { cx, cy, r, fill, stroke } => {
                if let Some(fill) = fill {
                    painter.circle_filled(at(*cx, *cy), r * zoom, rgba(*fill));
                }
                if let Some((color, width)) = stroke {
                    painter.circle_stroke(at(*cx, *cy), r * zoom, Stroke::new(width * stroke_zoom, rgba(*color)));
                }
            }
            ChartPrimitive::Text { x, y, text, size, color, bold, align, valign, rotation } => {
                let role = if *bold { craft_theme::TypeRole::BodyStrong } else { craft_theme::TypeRole::Body };
                let family = tokens.font(role).family;
                let color = rgba(*color);
                let galley = painter.layout_no_wrap(text.clone(), FontId::new(size * zoom, family), color);
                let size = galley.size();
                let ax = match align {
                    HAlign::Left => 0.0,
                    HAlign::Center => 0.5,
                    HAlign::Right => 1.0,
                };
                let ay = match valign {
                    VAlign::Top => 0.0,
                    VAlign::Middle => 0.5,
                    VAlign::Bottom => 1.0,
                };
                let anchor = at(*x, *y);
                if *rotation == 0.0 {
                    painter.galley(pos2(anchor.x - size.x * ax, anchor.y - size.y * ay), galley, color);
                } else {
                    let (sin, cos) = rotation.sin_cos();
                    let off = vec2(-size.x * ax, -size.y * ay);
                    let turned = vec2(off.x * cos - off.y * sin, off.x * sin + off.y * cos);
                    let mut shape = TextShape::new(anchor + turned, galley, color);
                    shape.angle = *rotation;
                    painter.add(shape);
                }
            }
        }
    }
}

/// Whether the closed path turns one way throughout.
fn is_convex(points: &[Pos2]) -> bool {
    let n = points.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() > COLLINEAR {
            if sign == 0.0 {
                sign = cross.signum();
            } else if cross.signum() != sign {
                return false;
            }
        }
    }
    true
}

/// A mesh of vertical strips for an area chart's polygon: its top edge runs along the first half
/// of the points and its baseline back along the second half.
fn area_mesh(points: &[Pos2], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let n = points.len();
    if n < 3 {
        return mesh;
    }
    for i in 0..(n / 2).saturating_sub(1) {
        let base = mesh.vertices.len() as u32;
        for q in [points[i], points[i + 1], points[n - 2 - i], points[n - 1 - i]] {
            mesh.colored_vertex(q, color);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    mesh
}
