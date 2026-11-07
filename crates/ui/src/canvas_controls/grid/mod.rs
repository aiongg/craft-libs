//! `CanvasGrid`: major and minor grid lines over a document rect, in four line styles, a canvas control (20.6). It is
//! the document grid of Preferences > Guides, Grid & Slices. Display only: it senses nothing, so the host's menu item
//! carries the state. Show it before guides and frames.

#[cfg(feature = "testing")]
pub(super) mod stories;

use crate::canvas::{CanvasSpace, GuideLine, canvas_space};
use crate::contract::{Access, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role};
use crate::primitives::{ContentPainterCx, Rule};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, DataColor, RuleLook, StyleColor, StyleCx};
use craft_behavior::kurbo::{Point, Rect, Vec2};
use craft_behavior::{Input, Metrics};
use egui::{Color32, Pos2, Response, Shape};

/// Most lines the grid draws along one axis.
pub const MAX_LINES: usize = 10_000;

/// The major step of a new grid, in document units.
const DEFAULT_STEP: f64 = 100.0;

/// Most crossing squares the grid draws.
const MAX_CROSSINGS: usize = 200_000;

/// Minor lines' share of the colour's alpha.
const MINOR_ALPHA: f32 = 0.45;

/// The smallest step, in screen points, at which an axis draws.
const MIN_STEP: f64 = 6.0;

/// The smallest major step, in screen points, at which crossings draw.
const CROSSING_MIN: f64 = 8.0;

/// The radius of a dot, in screen points.
const DOT_RADIUS: f32 = 0.6;

/// How the grid's lines are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GridLine {
    /// An unbroken line.
    #[default]
    Solid,
    /// Dashes of `stroke.dash` with gaps of `stroke.dash_gap`.
    Dashed,
    /// Dots `stroke.dash_gap` apart.
    Dots,
    /// A `stroke.medium` square at each crossing of two lines where either is major; no lines.
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

/// The lines of one axis: positions in document units along it.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct GridAxis {
    /// The minor lines.
    pub minor: Vec<f64>,
    /// The major lines.
    pub major: Vec<f64>,
}

/// The lines a grid draws in a view: vertical lines (by x) and horizontal lines (by y), each running across `extent`.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct GridLines {
    /// The vertical lines.
    pub x: GridAxis,
    /// The horizontal lines.
    pub y: GridAxis,
    /// Where the lines run, in document units; `None` when the grid draws nothing.
    pub extent: Option<Rect>,
    /// Whether the major spacing is wide enough for crossings on both axes.
    pub crossings: bool,
}

/// The style of the canvas grid.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct GridStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The line colour when the host gives none.
    color: StyleColor,
    /// The line width.
    width: f32,
    /// The length of a dash and of the gap after it.
    dash: f32,
    gap: f32,
    /// The side of a crossing square.
    crossing: f32,
}

/// Major and minor grid lines over a document, counted from `origin`. A major step under six screen points draws
/// nothing on its axis; minor lines draw only where their step is at least that wide.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(canvas_control, content_painter)]
pub struct CanvasGrid {
    #[common]
    common: Common,
    /// The major step per axis in document units; a step that is not a positive number draws nothing on its axis.
    #[prop(quantity = "document units per major line")]
    spacing: Vec2,
    /// Minor lines per major step: 1 (or 0) is none.
    #[prop(quantity = "minor lines per major step")]
    subdivisions: u32,
    /// The document point the lines count from.
    #[prop]
    origin: Point,
    /// Where the lines run, in document units; the view's visible rect by default.
    #[prop]
    bounds: Option<Rect>,
    /// The host's colour, in place of the overlay grid colour.
    #[prop]
    color: Option<DataColor>,
    /// How the lines are drawn.
    #[prop]
    line: GridLine,
    /// Draws nothing while the view's device zoom (1 is 100 percent) is below this (VectorCraft's pixel grid: 6).
    #[prop(quantity = "device zoom")]
    min_zoom: Option<f64>,
}

impl CanvasGrid {
    /// A grid named `label` with a major line every 100 document units on both axes and no minor lines.
    pub fn new(label: impl Into<Label>) -> Self {
        CanvasGrid {
            common: Common::new(),
            spacing: Vec2 { x: DEFAULT_STEP, y: DEFAULT_STEP },
            subdivisions: 1,
            origin: Point::ZERO,
            bounds: None,
            color: None,
            line: GridLine::Solid,
            min_zoom: None,
        }
        .label(label)
    }

    /// The lines to draw in `space`: positions only, no painting.
    pub fn lines(&self, space: &CanvasSpace) -> GridLines {
        let view = &space.view;
        if self.min_zoom.is_some_and(|min| space.zoom_device() < min || space.zoom_device().is_nan()) {
            return GridLines::default();
        }
        let visible = view.visible();
        let extent = match self.bounds {
            None => visible,
            Some(bounds) => {
                let bounds = bounds.abs();
                // Disjoint rects give nothing; `Rect::intersect` would clamp them to a point.
                if !(bounds.x0 < visible.x1 && visible.x0 < bounds.x1 && bounds.y0 < visible.y1 && visible.y0 < bounds.y1) {
                    return GridLines::default();
                }
                bounds.intersect(visible)
            }
        };
        if !(extent.x0.is_finite() && extent.y0.is_finite() && extent.x1.is_finite() && extent.y1.is_finite()) {
            return GridLines::default();
        }
        let subdivisions = self.subdivisions.max(1);
        let x = axis_lines(self.spacing.x, subdivisions, self.origin.x, (extent.x0, extent.x1), view.zoom_x());
        let y = axis_lines(self.spacing.y, subdivisions, self.origin.y, (extent.y0, extent.y1), view.zoom);
        let crossings = self.spacing.x * view.zoom_x() >= CROSSING_MIN && self.spacing.y * view.zoom >= CROSSING_MIN;
        GridLines { x, y, extent: Some(extent), crossings }
    }
}

/// The lines of one axis: majors every `step` from `origin`, minors between them when the minor step is at least
/// `MIN_STEP` screen points. An axis whose major step is under `MIN_STEP` draws nothing.
fn axis_lines(step: f64, subdivisions: u32, origin: f64, (lo, hi): (f64, f64), zoom: f64) -> GridAxis {
    if !(step.is_finite() && step > 0.0 && zoom.is_finite() && zoom > 0.0) || step * zoom < MIN_STEP {
        return GridAxis::default();
    }
    let subdivided = subdivisions > 1 && step / f64::from(subdivisions) * zoom >= MIN_STEP;
    let pitch = if subdivided { step / f64::from(subdivisions) } else { step };
    let every = if subdivided { i64::from(subdivisions) } else { 1 };
    let (first, last) = (((lo - origin) / pitch).ceil(), ((hi - origin) / pitch).floor());
    let mut lines = GridAxis::default();
    if !(first.is_finite() && last.is_finite()) || last < first {
        return lines;
    }
    for i in (first as i64..=last as i64).take(MAX_LINES) {
        let at = origin + i as f64 * pitch;
        if i.rem_euclid(every) == 0 { lines.major.push(at) } else { lines.minor.push(at) }
    }
    lines
}

fn paint_line(painter: &ContentPainterCx<'_>, a: Pos2, b: Pos2, color: Color32, line: GridLine, style: &GridStyle) {
    match line {
        GridLine::Solid => painter.painter.extend(Rule::shapes(&RuleLook { color, width: style.width }, a, b, painter.pixels_per_point())),
        GridLine::Dashed => painter.dashed(&[a, b], style.width, style.dash, style.gap, 0.0, color),
        GridLine::Dots => painter.painter.extend(Shape::dotted_line(&[a, b], color, style.gap, DOT_RADIUS)),
        GridLine::Crossings => {}
    }
}

impl Component for CanvasGrid {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    const INPUT: Input = Input::Display;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = GridStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: CanvasGridStyleKey) -> GridStyle {
        let stroke = &cx.tokens.stroke;
        GridStyle {
            metrics: Metrics::new(),
            color: cx.color(cx.tokens.color.overlay.grid),
            width: stroke.thin,
            dash: stroke.dash,
            gap: stroke.dash_gap,
            crossing: stroke.medium,
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::ZERO
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let lines = self.lines(&space);
        let Some(extent) = lines.extent else { return };
        let (view, style) = (&space.view, &cx.style);
        let painter = ui.content_painter(cx);
        let base = self.color.map_or(style.color.egui(), |color| color.0);
        if self.line == GridLine::Crossings {
            if lines.crossings {
                paint_crossings(&painter, &space, &lines, base, style);
            }
            return;
        }
        let minor = base.gamma_multiply(MINOR_ALPHA);
        for (color, x, y) in [(minor, &lines.x.minor, &lines.y.minor), (base, &lines.x.major, &lines.y.major)] {
            for &at in x {
                paint_line(&painter, view.to_screen(Point::new(at, extent.y0)), view.to_screen(Point::new(at, extent.y1)), color, self.line, style);
            }
            for &at in y {
                paint_line(&painter, view.to_screen(Point::new(extent.x0, at)), view.to_screen(Point::new(extent.x1, at)), color, self.line, style);
            }
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.role(Role::None);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A square at each crossing where either line is major.
fn paint_crossings(painter: &ContentPainterCx<'_>, space: &CanvasSpace, lines: &GridLines, color: Color32, style: &GridStyle) {
    let mut count = 0;
    let mut square = |x: f64, y: f64| {
        if count < MAX_CROSSINGS {
            count += 1;
            let at = space.view.to_screen(Point::new(x, y));
            painter.painter.rect_filled(egui::Rect::from_center_size(at, egui::Vec2::splat(style.crossing)), 0.0, color);
        }
    };
    for &x in &lines.x.major {
        for &y in lines.y.major.iter().chain(&lines.y.minor) {
            square(x, y);
        }
    }
    for &x in &lines.x.minor {
        for &y in &lines.y.major {
            square(x, y);
        }
    }
}
