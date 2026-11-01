//! Page layout and view (N3.06): where each page of a document sits in the scroll content of a
//! page view, at a zoom, and the zoom a fit mode gives.
//!
//! Two apps lay pages out today and they differ (fact sheet `S6b-pages-facts.md`): PdfCraft
//! shows one page across or a two-page spread (with Acrobat's cover-page rule), continuous or one
//! row at a time, with a 14 pt gap and a 70 pt side gutter; WordCraft flows as many pages across as
//! fit (Word's Print Layout zoomed out), with an 18 pt gap, a markup column beside each page and
//! Hide White Space. [`PageDisplay`] holds both, and [`LayoutOptions`] carries each app's
//! constants ([`LayoutOptions::ACROBAT`], [`LayoutOptions::WORD`]).
//!
//! ```ignore
//! let sizes: Vec<Size> = doc.pages.iter().map(|p| p.size()).collect();
//! let scale = fit_scale(&sizes, viewport, view.fit, &options, view.current).unwrap_or(view.zoom * SCREEN_POINTS_PER_POINT);
//! let laid = place_pages(&sizes, scale, viewport, &options);
//! let shown = if options.continuous { 0..sizes.len() } else { shown_row(sizes.len(), view.current, &options) };
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::{Range, RangeInclusive};

use kurbo::{Point, Rect, Size};

/// Screen points per page point at 100 %: 96 dpi, as browsers, Acrobat and Word show a page
/// (PdfCraft's `PT`, WordCraft's `PX_PER_PT`).
pub const SCREEN_POINTS_PER_POINT: f64 = 96.0 / 72.0;

/// The zooms a page view shows, 8 % to 6400 % (Acrobat's range; PdfCraft clamps to it).
pub const ZOOM_RANGE: RangeInclusive<f64> = 0.08..=64.0;

/// How pages are placed across the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageDisplay {
    /// One page across (Acrobat's Single Page, Word's One Page and Page Width).
    #[default]
    Single,
    /// Two-page spreads (Acrobat's Two Page); with [`LayoutOptions::cover`] the first page sits
    /// alone on the right.
    TwoUp,
    /// As many pages across as fit, at least [`LayoutOptions::min_columns`] (Word's Print Layout
    /// zoomed out, and Multiple Pages with a minimum of two).
    Columns,
}

impl PageDisplay {
    /// The order the View menu, a rail button and Preferences list them.
    pub const ORDER: [PageDisplay; 3] = [PageDisplay::Single, PageDisplay::TwoUp, PageDisplay::Columns];
}

/// The fit modes. A fit recomputes the zoom whenever the viewport or the pages change; a zoom the
/// user sets is [`Fit::None`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fit {
    /// Fit Width (Word's Page Width): the widest page, or the widest spread, fills the width.
    #[default]
    Width,
    /// Fit Page (Word's One Page; Multiple Pages with [`PageDisplay::Columns`]): a whole page or
    /// spread shows.
    Page,
    /// Fit Height.
    Height,
    /// A fixed zoom.
    None,
}

/// The constants and switches of a layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOptions {
    pub display: PageDisplay,
    /// [`PageDisplay::TwoUp`]: the first page alone, as a cover.
    pub cover: bool,
    /// Every page scrolls past; off shows one row (a page or a spread) at a time ([`shown_row`]).
    pub continuous: bool,
    /// Screen points between pages; 0 joins page edges (PdfCraft's Show page gaps off: pages are
    /// then sized in whole device pixels so no hairline seams show).
    pub gap: f64,
    /// Screen points above the first and below the last row.
    pub margin: f64,
    /// Screen points kept clear at each side (PdfCraft's gutter for the floating quick-action
    /// bar).
    pub side: f64,
    /// Page points beside each page for comment and revision balloons (WordCraft's markup area,
    /// 216 pt when shown), or 0.
    pub markup: f64,
    /// [`PageDisplay::Columns`]: at least this many pages across.
    pub min_columns: usize,
}

impl LayoutOptions {
    /// PdfCraft's: single page, continuous, 14 pt gap, 28 pt margin, 70 pt side.
    pub const ACROBAT: LayoutOptions = LayoutOptions {
        display: PageDisplay::Single,
        cover: false,
        continuous: true,
        gap: 14.0,
        margin: 28.0,
        side: 70.0,
        markup: 0.0,
        min_columns: 1,
    };
    /// WordCraft's: as many across as fit, 18 pt gap and margin, no side.
    pub const WORD: LayoutOptions = LayoutOptions {
        display: PageDisplay::Columns,
        cover: false,
        continuous: true,
        gap: 18.0,
        margin: 18.0,
        side: 0.0,
        markup: 0.0,
        min_columns: 1,
    };
}

/// Pages placed in the scroll content of a view, origin at the content's top-left.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PagedLayout {
    /// Each page's rectangle in screen points.
    pub rects: Vec<Rect>,
    /// The content's size: at least the viewport's.
    pub size: Size,
    /// Pages per row.
    pub columns: usize,
    /// Per page, the part of its height that shows (screen points from its top): the whole page,
    /// except with Hide White Space ([`PagedLayout::hide_white_space`]).
    pub bands: Vec<(f64, f64)>,
}

/// Place pages of `sizes` (page points) at `scale` screen points per page point in a viewport
/// `viewport` wide and tall.
///
/// - [`PageDisplay::Single`]: each page centred, at least [`side`](LayoutOptions::side) from the
///   left, `gap` below the one before, from `margin` at the top.
/// - [`PageDisplay::TwoUp`]: pages in pairs, each pair centred and a shorter page centred vertically
///   on the taller one; with a cover the first
///   page sits alone right of the centre line.
/// - [`PageDisplay::Columns`]: rows of as many slots as fit (`(width - gap) / (slot + gap)`,
///   at least `min_columns`, at most the page count), each slot as wide as the widest page plus
///   the markup area; a narrower page is centred in its slot, keeping its markup area beside it.
///
/// Sizes that aren't finite or are below 1 pt count as 72 pt; scale outside 0.01–10 is clamped
/// (WordCraft's `sane_dim` and scale guard).
// pdfcraft: crates/ui-egui/src/canvas/zoom.rs:306 `layout`; wordcraft:
// crates/ui-egui/src/canvas/geometry.rs:126 `columns_for`, :139 `place_pages`.
pub fn place_pages(sizes: &[Size], scale: f64, viewport: Size, options: &LayoutOptions) -> PagedLayout {
    todo!()
}

impl PagedLayout {
    /// Word's Hide White Space: single-column pages stacked so that only each page's text area
    /// (`bodies`: the top and bottom of its body, page points) plus 6 pt shows, a 1 pt seam
    /// between pages. The first page keeps its top and the last its bottom. Rects keep the full
    /// page size, so page coordinates still map as `(p - rect.min) / scale`; neighbouring rects
    /// overlap in the hidden margins. Only for one column and more than one page; otherwise the
    /// layout is unchanged.
    // wordcraft: crates/ui-egui/src/canvas/geometry.rs:28 `collapse_white_space` (WHITE_SPACE_PAD
    // 6, SEAM 1).
    pub fn hide_white_space(&mut self, bodies: &[(f64, f64)], scale: f64, viewport_height: f64) {
        todo!()
    }

    /// Whether `p` (content points) is on the gap between two pages, or on the seam with white
    /// space hidden: a double-click there toggles Hide White Space, as in Word.
    // wordcraft: crates/ui-egui/src/canvas/geometry.rs:57 `on_page_gap`.
    pub fn on_page_gap(&self, p: Point) -> bool {
        todo!()
    }
}

/// The page whose shown part contains `p` (content points), if any; the canvas around and between
/// pages, and margins Hide White Space cuts, are no page.
// wordcraft: crates/ui-egui/src/canvas/geometry.rs:72 `on_visible_page`.
pub fn page_at(layout: &PagedLayout, p: Point) -> Option<usize> {
    todo!()
}

/// The zoom a fit gives, as a scale (screen points per page point), or `None` for
/// [`Fit::None`]. `current` is the page shown when not continuous.
///
/// - The page fitted is the largest page (so the zoom holds still while pages of other sizes
///   scroll past), except a single page shown on its own, which fits that page.
/// - Width: `(width - 2 × side - gap × (n - 1)) / (page width × n)` with n the pages per row
///   (2 for spreads); Page: the smaller of that and `(height - 2 × margin) / page height`;
///   Height: the second alone. The width is at least 100.
/// - Clamped to [`ZOOM_RANGE`] × [`SCREEN_POINTS_PER_POINT`].
// pdfcraft: crates/ui-egui/src/canvas/zoom.rs:259 `fit_zoom`; wordcraft:
// crates/ui-egui/src/canvas/geometry.rs:178 (pageWidth, onePage, multiplePages).
pub fn fit_scale(sizes: &[Size], viewport: Size, fit: Fit, options: &LayoutOptions, current: Option<usize>) -> Option<f64> {
    todo!()
}

/// When not continuous, the pages of the row shown: the current page, or the current spread. With
/// a cover, page 0 is alone and spreads are (1, 2), (3, 4), …; without, (0, 1), (2, 3). Empty for
/// no pages.
// pdfcraft: crates/ui-egui/src/canvas.rs:2831 `shown_row`.
pub fn shown_row(count: usize, current: usize, options: &LayoutOptions) -> Range<usize> {
    todo!()
}

/// The first page of the spread page `page` is on (with or without a cover).
// pdfcraft: crates/ui-egui/src/canvas/zoom.rs:255 `spread_first`.
pub fn spread_first(page: usize, cover: bool) -> usize {
    todo!()
}

/// The one token settings and the control channel use for a display and continuous scrolling:
/// `single`, `continuous` (single, scrolling), `two-page`, `two-up` (two-page, scrolling) and
/// `columns`. PdfCraft's names, kept for its settings files.
// pdfcraft: crates/ui-egui/src/canvas.rs:159 `display_token`.
pub fn display_token(display: PageDisplay, continuous: bool) -> &'static str {
    todo!()
}

/// The display and continuous flag a [`display_token`] names, in any case; `two-up-single` is
/// `two-page`. `None` for anything else, so a typo is refused rather than switching silently.
// pdfcraft: crates/ui-egui/src/canvas.rs:169 `parse_display_token`.
pub fn parse_display_token(token: &str) -> Option<(PageDisplay, bool)> {
    todo!()
}

/// How a newly opened document is shown (Preferences › Documents and view). A file that asks for
/// its own layout or zoom gets that instead.
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViewDefaults {
    pub display: PageDisplay,
    pub continuous: bool,
    /// Separate pages with a gap and shadow; off joins adjacent page edges.
    pub page_gaps: bool,
    /// [`Fit::Width`] or [`Fit::Page`], or [`Fit::None`] for a fixed `zoom`.
    pub fit: Fit,
    /// The zoom with [`Fit::None`] (1.0 = 100 %).
    pub zoom: f64,
}

impl Default for ViewDefaults {
    /// Single page, continuous, page gaps, fit width (PdfCraft's).
    fn default() -> Self {
        ViewDefaults { display: PageDisplay::Single, continuous: true, page_gaps: true, fit: Fit::Width, zoom: 1.0 }
    }
}

impl ViewDefaults {
    /// The zoom as settings name it: `fit-width`, `fit-page` or a percentage (`125%`). Fit
    /// Height is saved as `fit-width`, as PdfCraft does.
    // pdfcraft: crates/ui-egui/src/canvas.rs:206 `ViewDefaults::zoom_name`.
    pub fn zoom_name(&self) -> String {
        todo!()
    }

    /// These defaults with the zoom a [`zoom_name`](Self::zoom_name) names, in any case. `None`
    /// for anything else, including a percentage outside 8–6400 %.
    // pdfcraft: crates/ui-egui/src/canvas.rs:216 `ViewDefaults::with_zoom`.
    pub fn with_zoom(self, name: &str) -> Option<Self> {
        todo!()
    }
}
