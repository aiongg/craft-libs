//! [`CanvasGrid`]: major and minor grid lines over a document rect, in four line styles.

use egui::{AsIdSalt, Color32, Painter, Pos2, Rect as ScreenRect, Shape, Stroke, Ui, vec2};
use kurbo::{Point, Rect, Vec2};

use super::guides::dashes;
use super::{CanvasView, GuideLine, crisp_shape};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// Most lines the grid draws along one axis.
pub const MAX_LINES: usize = 10_000;
/// Most crossing dots the grid draws.
const MAX_CROSSINGS: usize = 200_000;

/// How the grid's lines are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridLine {
    #[default]
    Solid,
    /// Dashes of `guide.dash` with gaps of `guide.gap`.
    Dashed,
    /// Dots `guide.dot_gap` apart, of radius `guide.dot_radius`.
    Dots,
    /// A `grid.crossing` square at each crossing of two lines where either is major; no lines.
    Crossings,
}

impl From<GuideLine> for GridLine {
    fn from(line: GuideLine) -> Self {
        match line {
            GuideLine::Solid => GridLine::Solid,
            GuideLine::Dashed => GridLine::Dashed,
            GuideLine::Dots => GridLine::Dots,
        }
    }
}

/// The grid's look, from [`CanvasGrid::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GridStyle {
    /// Line width: `grid.width`.
    pub width: f32,
    /// Minor lines' share of the colour's alpha: `grid.minor_alpha`.
    pub minor_alpha: f32,
    /// Smallest step in screen points at which an axis draws: `grid.min_step`.
    pub min_step: f32,
    /// Side of a crossing dot: `grid.crossing`.
    pub crossing: f32,
    /// Smallest major spacing in screen points at which crossings draw: `grid.crossing_min`.
    pub crossing_min: f32,
    /// Dashes and gaps of the Dashed style: `guide.dash`, `guide.gap`.
    pub dash: f32,
    pub gap: f32,
    /// Dot spacing and radius of the Dots style: `guide.dot_gap`, `guide.dot_radius`.
    pub dot_gap: f32,
    pub dot_radius: f32,
    /// `overlay.grid`.
    pub color: Color32,
}

/// Which lines of one axis draw: positions in document units along the axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AxisLines {
    pub minor: Vec<f64>,
    pub major: Vec<f64>,
}

/// The lines a grid draws in a view: vertical lines (by x) and horizontal lines (by y), each
/// running across `extent`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct GridLines {
    pub x: AxisLines,
    pub y: AxisLines,
    pub extent: Option<Rect>,
    /// Whether the major spacing is wide enough for crossings on both axes.
    pub crossings: bool,
}

/// Major and minor grid lines over a document. Display only: it senses nothing and has no
/// accessibility node, so the host's menu item carries the state. Draw it before guides and
/// frames.
///
/// ```
/// use craft_ui::canvas::{CanvasGrid, CanvasView, GridLine, kurbo::{Point, Vec2}};
/// # egui::__run_test_ui(|ui| {
/// let view = CanvasView::new(ui.max_rect(), Point::new(200.0, 150.0), 1.0);
/// CanvasGrid::new("grid").spacing(Vec2::new(100.0, 100.0)).subdivisions(4).line(GridLine::Dashed).show(ui, &view);
/// # });
/// ```
#[derive(Clone, Debug)]
pub struct CanvasGrid {
    spacing: Vec2,
    subdivisions: u32,
    origin: Point,
    bounds: Option<Rect>,
    color: Option<Color32>,
    line: GridLine,
    min_zoom: Option<f64>,
}

impl CanvasGrid {
    /// A grid with a major line every 100 document units on both axes and no minor lines. `id`
    /// names the grid in the host's code; it senses nothing.
    pub fn new(_id: impl AsIdSalt) -> Self {
        Self {
            spacing: Vec2::new(100.0, 100.0),
            subdivisions: 1,
            origin: Point::ZERO,
            bounds: None,
            color: None,
            line: GridLine::Solid,
            min_zoom: None,
        }
    }

    /// The major step per axis in document units. A step that is not a positive number draws
    /// nothing on its axis.
    pub fn spacing(mut self, spacing: Vec2) -> Self {
        self.spacing = spacing;
        self
    }

    /// Minor lines per major step: 1 (or 0) is none.
    pub fn subdivisions(mut self, subdivisions: u32) -> Self {
        self.subdivisions = subdivisions;
        self
    }

    /// The document point the lines count from.
    pub fn origin(mut self, origin: Point) -> Self {
        self.origin = origin;
        self
    }

    /// Where the lines run, in document units. Default: the view's visible document rect.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// The host's colour, in place of `overlay.grid`.
    pub fn color(mut self, color: Option<Color32>) -> Self {
        self.color = color;
        self
    }

    pub fn line(mut self, line: GridLine) -> Self {
        self.line = line;
        self
    }

    /// Draws nothing while the view's zoom is below `min_zoom` (VectorCraft's pixel grid: 6).
    /// `min_zoom` is in points per unit, like `CanvasView.zoom`. A threshold in device pixels per
    /// unit (VectorCraft's 600 %) is divided by the context's `pixels_per_point` first.
    pub fn min_zoom(mut self, min_zoom: Option<f64>) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    pub fn style(tokens: &UiTokens) -> GridStyle {
        let (g, guide) = (&tokens.component.grid, &tokens.component.guide);
        GridStyle {
            width: g.width,
            minor_alpha: g.minor_alpha,
            min_step: g.min_step,
            crossing: g.crossing,
            crossing_min: g.crossing_min,
            dash: guide.dash,
            gap: guide.gap,
            dot_gap: guide.dot_gap,
            dot_radius: guide.dot_radius,
            color: tokens.color.overlay.grid.to_egui(),
        }
    }

    /// The lines to draw in `view`: positions only, no painting.
    pub(crate) fn lines(&self, view: &CanvasView, style: &GridStyle) -> GridLines {
        if self.min_zoom.is_some_and(|min| view.zoom < min || view.zoom.is_nan()) {
            return GridLines::default();
        }
        let visible = view.visible();
        let extent = match self.bounds {
            None => visible,
            Some(b) => {
                let b = b.abs();
                // Disjoint rects give nothing; `Rect::intersect` would clamp them to a point.
                if !(b.x0 < visible.x1 && visible.x0 < b.x1 && b.y0 < visible.y1 && visible.y0 < b.y1) {
                    return GridLines::default();
                }
                b.intersect(visible)
            }
        };
        if !(extent.x0.is_finite() && extent.y0.is_finite() && extent.x1.is_finite() && extent.y1.is_finite()) {
            return GridLines::default();
        }
        let subdivisions = self.subdivisions.max(1);
        let x = axis_lines(self.spacing.x, subdivisions, self.origin.x, (extent.x0, extent.x1), view.zoom_x(), style.min_step);
        let y = axis_lines(self.spacing.y, subdivisions, self.origin.y, (extent.y0, extent.y1), view.zoom, style.min_step);
        let crossings =
            self.spacing.x * view.zoom_x() >= f64::from(style.crossing_min) && self.spacing.y * view.zoom >= f64::from(style.crossing_min);
        GridLines { x, y, extent: Some(extent), crossings }
    }

    /// Draws the grid over `view.rect`.
    pub fn show(self, ui: &Ui, view: &CanvasView) {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let lines = self.lines(view, &style);
        let Some(extent) = lines.extent else { return };
        let painter = ui.painter_at(view.rect);
        let base = self.color.unwrap_or(style.color);
        let minor = base.gamma_multiply(style.minor_alpha);
        if self.line == GridLine::Crossings {
            if lines.crossings {
                paint_crossings(&painter, view, &lines, base, &style);
            }
            return;
        }
        for (color, x, y) in [(minor, &lines.x.minor, &lines.y.minor), (base, &lines.x.major, &lines.y.major)] {
            for &px in x {
                let (a, b) = (view.to_screen(Point::new(px, extent.y0)), view.to_screen(Point::new(px, extent.y1)));
                paint_line(&painter, a, b, color, self.line, &style);
            }
            for &py in y {
                let (a, b) = (view.to_screen(Point::new(extent.x0, py)), view.to_screen(Point::new(extent.x1, py)));
                paint_line(&painter, a, b, color, self.line, &style);
            }
        }
    }
}

/// The lines of one axis: majors every `step` from `origin`, minors between them when the minor
/// step is at least `min_step` screen points. An axis whose major step is under `min_step` draws
/// nothing.
fn axis_lines(step: f64, subdivisions: u32, origin: f64, (lo, hi): (f64, f64), zoom: f64, min_step: f32) -> AxisLines {
    let min_step = f64::from(min_step);
    if !(step.is_finite() && step > 0.0 && zoom.is_finite() && zoom > 0.0) || step * zoom < min_step {
        return AxisLines::default();
    }
    let subdivided = subdivisions > 1 && step / f64::from(subdivisions) * zoom >= min_step;
    let pitch = if subdivided { step / f64::from(subdivisions) } else { step };
    let every = if subdivided { i64::from(subdivisions) } else { 1 };
    let first = ((lo - origin) / pitch).ceil();
    let last = ((hi - origin) / pitch).floor();
    let mut lines = AxisLines::default();
    if !(first.is_finite() && last.is_finite()) || last < first {
        return lines;
    }
    let (first, last) = (first as i64, last as i64);
    for i in (first..=last).take(MAX_LINES) {
        let at = origin + i as f64 * pitch;
        if i.rem_euclid(every) == 0 { lines.major.push(at) } else { lines.minor.push(at) }
    }
    lines
}

fn paint_line(painter: &Painter, a: Pos2, b: Pos2, color: Color32, line: GridLine, style: &GridStyle) {
    let stroke = Stroke::new(style.width, color);
    let ppp = painter.pixels_per_point();
    match line {
        GridLine::Solid => {
            painter.add(crisp_shape(a, b, stroke, ppp));
        }
        GridLine::Dashed => painter.extend(dashes(a, b, style.dash, style.gap).map(|(s, e)| crisp_shape(s, e, stroke, ppp))),
        GridLine::Dots => painter.extend(Shape::dotted_line(&[a, b], color, style.dot_gap, style.dot_radius)),
        GridLine::Crossings => {}
    }
}

/// A square at each crossing where either line is major.
fn paint_crossings(painter: &Painter, view: &CanvasView, lines: &GridLines, color: Color32, style: &GridStyle) {
    let mut count = 0;
    let mut dot = |x: f64, y: f64| {
        if count < MAX_CROSSINGS {
            count += 1;
            let at = view.to_screen(Point::new(x, y));
            painter.rect_filled(ScreenRect::from_center_size(at, vec2(style.crossing, style.crossing)), 0.0, color);
        }
    };
    for &x in &lines.x.major {
        for &y in lines.y.major.iter().chain(&lines.y.minor) {
            dot(x, y);
        }
    }
    for &x in &lines.x.minor {
        for &y in &lines.y.major {
            dot(x, y);
        }
    }
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
