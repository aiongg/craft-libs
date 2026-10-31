//! [`PageGrids`] (N3.26): InDesign's guides on a page: the margins (magenta), the column
//! boundaries (violet) and the baseline grid (light blue, from its zoom threshold), each a
//! hairline, drawn over the page under the selection.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Painter};
use kurbo::Rect;

use craft_ui::canvas::CanvasView;

/// The most baseline grid lines drawn on a page: one per point of the tallest page (216 in),
/// DesignCraft's `MAX_BASELINES`.
pub const MAX_BASELINES: usize = 216 * 72;

/// Baseline grid lines inside a page `height` tall, measured from its top: `start + n ×
/// increment` for n ≥ 0 below the top (the bottom edge is not a line); lines above the page are
/// skipped without walking them, however far up `start` is; an increment under 1 pt counts as 1.
// designcraft: crates/ui-egui/src/canvas/page_decor.rs:19 `baseline_grid_lines`.
pub fn baseline_lines(start: f64, increment: f64, height: f64) -> Vec<f64> {
    todo!()
}

/// One page's grids, page points on the document.
#[derive(Clone, Debug, PartialEq)]
pub struct PageGrid {
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
    pub margins: Color32,
    pub columns: Color32,
    /// Drawn at 80 % (DesignCraft's `gamma_multiply(0.8)`).
    pub baseline: Color32,
}

/// The grids of the pages on screen, painted only.
///
/// - Margins as a hairline rectangle; columns as hairlines at each inner column edge; baseline
///   lines across the page when the view's zoom is at least [`baseline_threshold`](Self::baseline_threshold)
///   (InDesign's View Threshold, 75 % by default).
pub struct PageGrids<'a> {
    view: &'a CanvasView,
    pages: &'a [PageGrid],
    colors: PageGridColors,
    threshold: f64,
}

impl<'a> PageGrids<'a> {
    pub fn new(view: &'a CanvasView, pages: &'a [PageGrid], colors: PageGridColors) -> Self {
        PageGrids { view, pages, colors, threshold: 0.75 }
    }

    pub fn baseline_threshold(mut self, zoom: f64) -> Self {
        self.threshold = zoom;
        self
    }

    // designcraft: crates/ui-egui/src/canvas/page_decor.rs:100 `draw_page_grids`.
    pub fn paint(self, painter: &Painter) {
        todo!()
    }
}
