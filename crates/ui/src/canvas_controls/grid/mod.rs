//! `CanvasGrid`: major and minor grid lines over a document rect, in four line styles, a canvas control (20.6). It is
//! the document grid of Preferences > Guides, Grid & Slices. Display only: it senses nothing, so the host's menu item
//! carries the state. Show it before guides and frames.

#[cfg(feature = "testing")]
pub(super) mod stories;

use crate::canvas::{CanvasSpace, GuideLine, canvas_space};
use crate::contract::{Access, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role};
use crate::primitives::{ContentPainterCx, GridInk, GridStroke};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, DataColor, StyleColor, StyleCx};
use craft_behavior::kurbo::{Point, Rect, Vec2};
use craft_behavior::{Input, Metrics};
use egui::{Color32, Response};

pub use crate::primitives::{GridAxis, GridLines, MAX_LINES};

/// The major step of a new grid, in document units.
const DEFAULT_STEP: f64 = 100.0;

/// Most crossing squares the grid draws.
const MAX_CROSSINGS: usize = 200_000;

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

/// The style of the canvas grid.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct GridStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The line colour when the host gives none.
    color: StyleColor,
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
            spacing: (DEFAULT_STEP, DEFAULT_STEP).into(),
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
        GridLines::document(view, extent, self.spacing, self.origin, self.subdivisions)
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
        GridStyle { metrics: Metrics::new(), color: cx.color(cx.tokens.color.overlay.grid), crossing: stroke.medium }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::ZERO
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let lines = self.lines(&space);
        if lines.extent.is_none() {
            return;
        }
        let style = &cx.style;
        let painter = ui.content_painter(cx);
        let base = self.color.map_or(style.color.egui(), |color| color.0);
        let stroke = match self.line {
            GridLine::Solid => GridStroke::Solid,
            GridLine::Dashed => GridStroke::Dashed,
            GridLine::Dots => GridStroke::Dots,
            GridLine::Crossings => {
                if lines.crossings {
                    paint_crossings(&painter, &space, &lines, base, style);
                }
                return;
            }
        };
        painter.grid_lines(&lines, &GridInk::new().major(DataColor::new(base)).stroke(stroke));
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
