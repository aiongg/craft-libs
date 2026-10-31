//! [`PageView`] (N3.06): a scrolling view of a document's pages, laid out by
//! [`crate::layout`], drawn through craft-ui's [`Pasteboard`](craft_ui::canvas::Pasteboard) page
//! frames, with the page rasters the app renders.
//!
//! ```ignore
//! let r = PageView::new("doc-7", &mut doc_pages, &mut view_state).options(LayoutOptions::ACROBAT).show(ui);
//! for placed in &r.pages {
//!     selection_layer.show(ui, placed, &page_text(placed.page));   // overlays draw on placed pages
//! }
//! if r.current_changed { thumbnails_follow = true; }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use egui::{Rect, Response, TextureId, Ui};
use kurbo::{Point, Size};

use crate::layout::{Fit, LayoutOptions, PageDisplay, PagedLayout, ViewDefaults};

/// The document's pages as a page view, thumbnails and previews read them. The app implements it
/// over its document and its render cache; the view never owns pages.
pub trait PageSource {
    fn page_count(&self) -> usize;

    /// Page `page`'s size in points as shown (its rotation applied).
    fn page_size(&self, page: usize) -> Size;

    /// The page's label ("iv", "A-3"; PDF page labels, section numbering): what the page box,
    /// thumbnails and the accessibility tree call it. 1-based number by default.
    fn page_label(&self, page: usize) -> String {
        (page + 1).to_string()
    }

    /// The page's raster for drawing `pixels` device pixels wide, if the app has one; the app
    /// starts rendering when it hasn't and asks for a repaint when it is ready. `visible` is false
    /// for a page just outside the view, rendered ahead. A stale raster of another size is fine
    /// to return: the view draws it stretched until the new one arrives (PdfCraft).
    fn texture(&mut self, page: usize, pixels: u32, visible: bool) -> Option<TextureId>;
}

/// The view's state, kept by the app per document view (one per tab).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PageViewState {
    /// The current page: the one the page box shows and paging moves from.
    pub current: usize,
    pub display: PageDisplay,
    pub continuous: bool,
    /// Two-page spreads start with the first page alone.
    pub cover: bool,
    pub page_gaps: bool,
    pub fit: Fit,
    /// The zoom (1.0 = 100 %), kept equal to what a fit shows so zoom steps start from it (Word,
    /// WordCraft issue #67).
    pub zoom: f64,
    /// Word's Hide White Space.
    pub hide_white_space: bool,
    /// The scroll offset (screen points).
    pub offset: (f64, f64),
    /// A page to bring into view next pass, with a fraction of its height to put at the top.
    #[serde(skip)]
    pub go_to: Option<(usize, f64)>,
}

impl Default for PageViewState {
    fn default() -> Self {
        PageViewState::from_defaults(&ViewDefaults::default())
    }
}

impl PageViewState {
    /// A new view as the user's view defaults say.
    pub fn from_defaults(defaults: &ViewDefaults) -> Self {
        PageViewState {
            current: 0,
            display: defaults.display,
            continuous: defaults.continuous,
            cover: false,
            page_gaps: defaults.page_gaps,
            fit: defaults.fit,
            zoom: defaults.zoom,
            hide_white_space: false,
            offset: (0.0, 0.0),
            go_to: None,
        }
    }

    /// Bring `page` to the top of the view next pass (page box, outline, thumbnails, find).
    pub fn go_to(&mut self, page: usize) {
        self.go_to = Some((page, 0.0));
        self.current = page;
    }

    /// Zoom In / Zoom Out: the next of `steps` above (or below) the shown zoom; a fit ends.
    // pdfcraft: crates/ui-egui/src/canvas/zoom.rs (zoom steps); wordcraft:
    // crates/ui-egui/src/canvas/geometry.rs:234 `sync_fit_zoom`.
    pub fn zoom_step(&mut self, steps: &[f64], larger: bool) {
        todo!()
    }
}

/// One page on screen this pass, for overlays: its index, screen rectangle and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedPage {
    pub page: usize,
    /// The whole page on screen (part of it may be clipped or, with Hide White Space, hidden).
    pub rect: Rect,
    /// Screen points per page point.
    pub scale: f64,
    /// The part of the page's height that shows (screen points from its top).
    pub band: (f32, f32),
}

impl PlacedPage {
    /// Page point (origin top-left, y down) to screen.
    pub fn to_screen(&self, p: Point) -> egui::Pos2 {
        egui::pos2(self.rect.min.x + (p.x * self.scale) as f32, self.rect.min.y + (p.y * self.scale) as f32)
    }

    /// Screen to page point.
    pub fn to_page(&self, p: egui::Pos2) -> Point {
        Point::new(f64::from(p.x - self.rect.min.x) / self.scale, f64::from(p.y - self.rect.min.y) / self.scale)
    }
}

/// What the view did this pass.
#[derive(Debug)]
pub struct PageViewResponse {
    /// The view's area; it has the keyboard focus for paging keys.
    pub response: Response,
    /// The pages drawn, in order.
    pub pages: Vec<PlacedPage>,
    /// The whole layout, content coordinates.
    pub layout: PagedLayout,
    /// The pages at least partly in view.
    pub visible: Range<usize>,
    /// The current page changed (scrolling, paging keys, the wheel turning a page).
    pub current_changed: bool,
    /// A double-click on the gap between pages: the app toggles Hide White Space (Word).
    pub toggle_white_space: bool,
}

/// The page view, built per pass over the app's [`PageSource`] and [`PageViewState`].
///
/// - Layout: [`place_pages`](crate::layout::place_pages) at the state's zoom, or the fit's
///   ([`fit_scale`](crate::layout::fit_scale)), recomputed when the viewport or the pages
///   change; the zoom is written back so it equals what shows. Pages draw as craft-ui
///   [`PageFrame`](craft_ui::canvas::PageFrame)s (white, the pasteboard's border and shadow), the
///   raster over them, a white page until it arrives; pages outside the view plus one row either
///   side ask for rasters (`visible` false for those outside).
/// - Scrolling: wheel, scroll bars, space-drag and the Hand tool are the app's canvas
///   navigation (craft-ui `canvas::navigate`); the view keeps its offset in the state. Not
///   continuous, a page that fits turns with the wheel instead (PdfCraft #1924), and one zoomed in
///   far enough to pan pans and turns the page at its edge.
/// - Keys (with the view focused, one Tab stop): Page Up/Down a screen (a page or spread when not
///   continuous), Home/End the first and last page, ↑/↓ a line, ←/→ a page when the page fits the
///   width. ⌘/Ctrl + wheel and pinch zoom about the pointer, keeping the point under it still.
/// - Current page: the page whose rectangle holds the view's middle line (PdfCraft breaks ties
///   at half a point toward the earlier page).
/// - AccessKit: the area is a Document node named by the app ([`PageView::name`]); the
///   accessibility bridge ([`super::access`]) describes the text under it.
pub struct PageView<'a> {
    id_salt: egui::IdSalt,
    source: &'a mut dyn PageSource,
    state: &'a mut PageViewState,
    options: LayoutOptions,
    name: Option<String>,
    bodies: Option<&'a [(f64, f64)]>,
}

impl<'a> PageView<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a mut dyn PageSource, state: &'a mut PageViewState) -> Self {
        PageView { id_salt: egui::IdSalt::new(id_salt), source, state, options: LayoutOptions::ACROBAT, name: None, bodies: None }
    }

    /// The layout constants; the state's display, continuous, cover and page gaps override
    /// theirs.
    pub fn options(mut self, options: LayoutOptions) -> Self {
        self.options = options;
        self
    }

    /// The document's name for the AccessKit Document node.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Each page's body (top and bottom of its text area, points), for Hide White Space.
    pub fn bodies(mut self, bodies: &'a [(f64, f64)]) -> Self {
        self.bodies = Some(bodies);
        self
    }

    // pdfcraft: crates/ui-egui/src/canvas.rs:3060 (the page area pass: fit, layout, shown row,
    // paging, scroll area), :948 `paint_page_surface`; wordcraft: crates/ui-egui/src/canvas/geometry.rs:166
    // `geometry`.
    pub fn show(self, ui: &mut Ui) -> PageViewResponse {
        todo!()
    }
}
