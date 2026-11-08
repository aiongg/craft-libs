//! A chart drawn from a primitive list.

use craft_theme::TypeRole;
use craft_ui::primitives::{Draw, PaintShape, TextRun};
use craft_ui::style::{DataColor, StyleColor};
use egui::{CornerRadius, Pos2, Rect, pos2, vec2};

#[cfg(feature = "screen-testing")]
mod stories;

/// One wedge step spans this many radians.
const WEDGE_STEP: f32 = 0.05;
/// A wedge has between these many steps.
const WEDGE_STEPS: (usize, usize) = (2, 400);
/// Cross products smaller than this are collinear when testing a polygon for convexity.
const COLLINEAR: f32 = 1e-3;

/// Horizontal text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HAlign {
    /// The text starts at the anchor.
    #[default]
    Left,
    /// The text is centred on the anchor.
    Center,
    /// The text ends at the anchor.
    Right,
}

/// Vertical text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VAlign {
    /// The text hangs from the anchor.
    #[default]
    Top,
    /// The text is centred on the anchor.
    Center,
    /// The text stands on the anchor.
    Bottom,
}

/// A drawing primitive in chart coordinates (origin top-left, y down, points). The app's chart engine produces the
/// list; the library paints it.
///
/// Lines and outlines (`Line`, and every `stroke`) need a data-colour stroke at a data width, which `Draw` does not
/// offer; they paint nothing. Text takes its size from the body and strong body roles and is not rotated: a text
/// with a `rotation` other than zero paints nothing.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ChartPrimitive {
    /// A rectangle.
    Rect {
        /// Left edge.
        x: f32,
        /// Top edge.
        y: f32,
        /// Width.
        width: f32,
        /// Height.
        height: f32,
        /// Fill colour; `None` for no fill.
        fill: Option<DataColor>,
        /// Stroke colour and width; `None` for no stroke.
        stroke: Option<(DataColor, f32)>,
        /// Corner radius.
        radius: f32,
    },
    /// A polyline.
    Line {
        /// The vertices, in order.
        points: Vec<[f32; 2]>,
        /// Stroke colour.
        color: DataColor,
        /// Stroke width.
        width: f32,
        /// Draw dashed.
        dash: bool,
    },
    /// A filled polygon.
    Polygon {
        /// The vertices, in order.
        points: Vec<[f32; 2]>,
        /// Fill colour.
        fill: DataColor,
        /// Stroke colour and width; `None` for no stroke.
        stroke: Option<(DataColor, f32)>,
    },
    /// A pie or doughnut sector: centre, radii, angles in radians (0 at 12 o'clock, clockwise).
    Wedge {
        /// Centre x.
        center_x: f32,
        /// Centre y.
        center_y: f32,
        /// Outer radius.
        outer_radius: f32,
        /// Inner radius; 0 for a pie sector, above 0 for a doughnut.
        inner_radius: f32,
        /// Start angle.
        start_angle: f32,
        /// End angle.
        end_angle: f32,
        /// Fill colour.
        fill: DataColor,
        /// Stroke colour and width; `None` for no stroke.
        stroke: Option<(DataColor, f32)>,
    },
    /// A circle.
    Circle {
        /// Centre x.
        center_x: f32,
        /// Centre y.
        center_y: f32,
        /// Radius.
        radius: f32,
        /// Fill colour; `None` for no fill.
        fill: Option<DataColor>,
        /// Stroke colour and width; `None` for no stroke.
        stroke: Option<(DataColor, f32)>,
    },
    /// Text anchored at `(x, y)` by `align` / `valign`, rotated by `rotation` radians about the anchor.
    Text {
        /// Anchor x.
        x: f32,
        /// Anchor y.
        y: f32,
        /// The text to draw.
        text: String,
        /// The text colour; `None` takes the surface's ink.
        color: Option<StyleColor>,
        /// Draw in bold.
        bold: bool,
        /// Horizontal anchor.
        align: HAlign,
        /// Vertical anchor.
        valign: VAlign,
        /// Rotation in radians about the anchor.
        rotation: f32,
    },
}

/// Paints `primitives` with their origin at `origin`, scaled by `zoom`: rects, polygons (a convex one whole, else
/// as the vertical strips of an area chart's polygon: its top edge along the first half of the points and its
/// baseline back along the second half, exact for an area chart's "mountain" shape), wedges as one convex shape per
/// step of 0.05 rad (2…400 steps), circles, and text anchored by its alignment.
pub fn paint_chart(draw: &Draw<'_>, origin: Pos2, primitives: &[ChartPrimitive], zoom: f32) {
    let at = |x: f32, y: f32| pos2(origin.x + x * zoom, origin.y + y * zoom);
    for primitive in primitives {
        match primitive {
            ChartPrimitive::Rect { x, y, width, height, fill, radius, .. } => {
                let rect = Rect::from_min_size(at(*x, *y), vec2(width * zoom, height * zoom));
                if let Some(fill) = fill {
                    let corners = CornerRadius::same((radius * zoom).round().clamp(0.0, 255.0) as u8);
                    draw.data_fill(&PaintShape::Rect(rect, corners), *fill);
                }
            }
            ChartPrimitive::Line { .. } => {}
            ChartPrimitive::Polygon { points, fill, .. } => {
                let points: Vec<Pos2> = points.iter().map(|q| at(q[0], q[1])).collect();
                if points.len() < 3 {
                    continue;
                }
                if is_convex(&points) {
                    draw.data_fill(&PaintShape::Polygon(points), *fill);
                } else {
                    for strip in area_strips(&points) {
                        draw.data_fill(&PaintShape::Polygon(strip.to_vec()), *fill);
                    }
                }
            }
            ChartPrimitive::Wedge { center_x, center_y, outer_radius, inner_radius, start_angle, end_angle, fill, .. } => {
                let centre = at(*center_x, *center_y);
                let steps = (((end_angle - start_angle).abs() / WEDGE_STEP).ceil() as usize).clamp(WEDGE_STEPS.0, WEDGE_STEPS.1);
                let on = |angle: f32, radius: f32| pos2(centre.x + radius * zoom * angle.sin(), centre.y - radius * zoom * angle.cos());
                let angle = |i: usize| start_angle + (end_angle - start_angle) * i as f32 / steps as f32;
                for i in 0..steps {
                    let (a, b) = (angle(i), angle(i + 1));
                    let step = if *inner_radius > 0.0 {
                        vec![on(a, *outer_radius), on(b, *outer_radius), on(b, *inner_radius), on(a, *inner_radius)]
                    } else {
                        vec![on(a, *outer_radius), on(b, *outer_radius), centre]
                    };
                    draw.data_fill(&PaintShape::Polygon(step), *fill);
                }
            }
            ChartPrimitive::Circle { center_x, center_y, radius, fill, .. } => {
                if let Some(fill) = fill {
                    draw.data_fill(&PaintShape::Circle(at(*center_x, *center_y), radius * zoom), *fill);
                }
            }
            ChartPrimitive::Text { x, y, text, color, bold, align, valign, rotation } => {
                if *rotation != 0.0 {
                    continue;
                }
                let mut run = TextRun::new(text.as_str()).role(if *bold { TypeRole::BodyStrong } else { TypeRole::Body });
                if let Some(color) = color {
                    run = run.color(*color);
                }
                let layout = draw.layout(&run, f32::INFINITY);
                let size = layout.size();
                let ax = match align {
                    HAlign::Left => 0.0,
                    HAlign::Center => 0.5,
                    HAlign::Right => 1.0,
                };
                let ay = match valign {
                    VAlign::Top => 0.0,
                    VAlign::Center => 0.5,
                    VAlign::Bottom => 1.0,
                };
                let anchor = at(*x, *y);
                draw.galley(pos2(anchor.x - size.x * ax, anchor.y - size.y * ay), &layout);
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

/// The vertical strips of an area chart's polygon, each a convex quad: the top edge runs along the first half of
/// the points and the baseline back along the second half.
fn area_strips(points: &[Pos2]) -> Vec<[Pos2; 4]> {
    let n = points.len();
    (0..(n / 2).saturating_sub(1)).map(|i| [points[i], points[i + 1], points[n - 2 - i], points[n - 1 - i]]).collect()
}
