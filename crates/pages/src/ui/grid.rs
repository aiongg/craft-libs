//! [`PageGrids`] (N3.26): InDesign's guides on a page: the margins (magenta), the column
//! boundaries (violet) and the baseline grid (light blue, from its zoom threshold), each a
//! hairline, drawn over the page under the selection.

use egui::epaint::PathShape;
use egui::{Color32, Painter, Shape};
use kurbo::{Point, Rect};

use craft_ui::canvas::CanvasView;
use craft_ui::theme::{StrokeExt as _, scale};

use super::view::PageViewState;

/// The share of its colour the baseline grid draws at (DesignCraft's `gamma_multiply(0.8)`): a
/// proportion, not a size.
const BASELINE_SHARE: f32 = 0.8;

/// The most baseline grid lines drawn on a page: one per point of the tallest page (216 in),
/// DesignCraft's `MAX_BASELINES`.
pub const MAX_BASELINES: usize = 216 * 72;

/// Baseline grid lines inside a page `height` tall, measured from its top: `start + n ×
/// increment` for n ≥ 0 below the top (the bottom edge is not a line); lines above the page are
/// skipped without walking them, however far up `start` is; an increment under 1 pt counts as 1.
// designcraft: crates/ui-egui/src/canvas/page_decor.rs:19 `baseline_grid_lines`.
pub fn baseline_lines(start: f64, increment: f64, height: f64) -> Vec<f64> {
    let step = increment.max(1.0);
    let first = if start < 0.0 { start.rem_euclid(step) } else { start };
    (0..MAX_BASELINES).map(|n| first + n as f64 * step).take_while(|y| *y < height).collect()
}

/// One page's grids, page points on the document.
#[derive(Clone, Debug, PartialEq)]
pub struct PageGrid {
    /// The page rectangle.
    pub page: Rect,
    /// The margin rectangle.
    pub margins: Rect,
    /// The column rectangles inside the margins; one column draws no column lines.
    pub columns: Vec<Rect>,
    /// The baseline grid: start (from the page top) and increment, if shown.
    pub baseline: Option<(f64, f64)>,
}

/// The colours, InDesign's defaults (Preferences › Guides & Pasteboard; Document Setup's Margins
/// and Columns colours).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageGridColors {
    /// Colour of the margin rectangle.
    pub margins: Color32,
    /// Colour of the column rectangles.
    pub columns: Color32,
    /// Drawn at 80 % (DesignCraft's `gamma_multiply(0.8)`).
    pub baseline: Color32,
}

/// The grids of the pages on screen, painted only.
///
/// - Margins as a hairline rectangle; columns as hairlines at each inner column edge; baseline
///   lines across the page when the page view's zoom ([`PageViewState::zoom`], 1.0 = 100 %; the
///   `view` maps page points to screen and carries a different scale) is at least [`baseline_threshold`](Self::baseline_threshold)
///   (InDesign's View Threshold, 75 % by default).
pub struct PageGrids<'a> {
    view: &'a CanvasView,
    pages: &'a [PageGrid],
    colors: PageGridColors,
    zoom: f64,
    threshold: f64,
}

impl<'a> PageGrids<'a> {
    /// The guide colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PageGridColors {
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.grid;
        PageGridColors { margins: t.page_margins.to_egui(), columns: t.page_columns.to_egui(), baseline: t.page_baseline.to_egui() }
    }

    /// The grids of `pages` through `view` (page points to screen), at the zoom of the page
    /// view `state` they are drawn over.
    pub fn new(view: &'a CanvasView, state: &PageViewState, pages: &'a [PageGrid], colors: PageGridColors) -> Self {
        PageGrids { view, pages, colors, zoom: state.zoom, threshold: 0.75 }
    }

    /// Sets the zoom below which the baseline grid is not drawn.
    pub fn baseline_threshold(mut self, zoom: f64) -> Self {
        self.threshold = zoom;
        self
    }

    // designcraft: crates/ui-egui/src/canvas/page_decor.rs:100 `draw_page_grids`.
    /// Draws the margins, columns and baseline grid of every page.
    pub fn paint(self, painter: &Painter) {
        let line = |color| scale::Stroke::Hairline.egui(painter.ctx(), color);
        let segment = |a: Point, b: Point, color| painter.line_segment([self.view.to_screen(a), self.view.to_screen(b)], line(color));
        for grid in self.pages {
            if let Some((start, increment)) = grid.baseline
                && self.zoom >= self.threshold
            {
                let color = self.colors.baseline.gamma_multiply(BASELINE_SHARE);
                for y in baseline_lines(start, increment, grid.page.height()) {
                    let y = grid.page.y0 + y;
                    segment(Point::new(grid.page.x0, y), Point::new(grid.page.x1, y), color);
                }
            }
            painter.add(Shape::Path(PathShape::closed_line(self.view.quad(grid.margins).to_vec(), line(self.colors.margins))));
            if grid.columns.len() > 1 {
                let last = grid.columns.len() - 1;
                for (i, column) in grid.columns.iter().enumerate() {
                    if i > 0 {
                        segment(Point::new(column.x0, column.y0), Point::new(column.x0, column.y1), self.colors.columns);
                    }
                    if i < last {
                        segment(Point::new(column.x1, column.y0), Point::new(column.x1, column.y1), self.colors.columns);
                    }
                }
            }
        }
    }
}
