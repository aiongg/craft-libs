//! [`PageView`] (N3.06): a scrolling view of a document's pages, laid out by
//! [`crate::layout`], drawn through craft-ui's [`Pasteboard`](craft_ui::canvas::Pasteboard) page
//! frames on a [`CanvasView`], with the page pictures the app renders through craft-ui's texture
//! contract ([`ThumbSource`]).
//!
//! The view is a canvas: the app runs its [`ToolHost`](craft_ui::canvas::ToolHost) over
//! [`PageViewResponse::canvas`] with the page tools (Select Text,
//! [`TextSelectTool`](super::TextSelectTool); craft-ui's `HandTool` and `ZoomTool`), as
//! PhotoCraft runs its tools over its canvas.
//!
//! ```ignore
//! let r = PageView::new("doc-7", &doc_pages, &mut view_state).options(LayoutOptions::ACROBAT).show(ui);
//! let mut tool = TextSelectTool::new(&r.pages, &doc_texts, &mut tab.text_selection).marks(&find_marks);
//! let tools = ToolHost::new(r.canvas_id).show(ui, &r.canvas, &r.response, &mut tool);
//! if r.current_changed { thumbnails_follow = true; }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use craft_ui::canvas::textures::ThumbSource;
use craft_ui::canvas::{CanvasId, CanvasView};
use craft_ui::theme::UiTokens;
use egui::{Rect, Response, Ui};
use kurbo::{Point, Size};

use super::style::PageViewStyle;
use crate::layout::{Fit, LayoutOptions, PageDisplay, PagedLayout, ViewDefaults};

/// The document's pages as a page view, thumbnails and previews read them. The app implements it
/// over its document; the view never owns pages.
///
/// Pictures go through craft-ui's one texture contract, [`ThumbSource<usize>`] keyed by page
/// index (`revision` a cheap stamp of the page's content, `render` the page at
/// `request.side` pixels on its longest side, usually as a `ThumbRender::Later` job on the
/// loader's queue). The library uploads, caches (`TextureCacheOptions::pages()` for the view,
/// `thumbnails()` for the strip), evicts, keeps a stale picture as the stand-in until the new
/// one lands and asks for the redraw; the app holds no texture and polls nothing.
pub trait PageSource: ThumbSource<usize> {
    fn page_count(&self) -> usize;

    /// Page `page`'s size in points as shown (its rotation applied).
    fn page_size(&self, page: usize) -> Size;

    /// The page's label ("iv", "A-3"; PDF page labels, section numbering): what the page box,
    /// thumbnails and the accessibility tree call it. 1-based number by default.
    fn page_label(&self, page: usize) -> String {
        (page + 1).to_string()
    }
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
    /// A page the next [`PageView::show`] brings to the top of the view, with a fraction of its
    /// height to put at the top; that `show` clears it.
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

    /// Ask the next [`PageView::show`] to bring `page` to the top of the view (page box,
    /// outline, thumbnails, find), and make it current now.
    pub fn go_to(&mut self, page: usize) {
        self.go_to = Some((page, 0.0));
        self.current = page;
    }

    /// Zoom In / Zoom Out: the next rung of craft-ui's [`ZoomLadder`](craft_ui::canvas::ZoomLadder)
    /// above (or below) the shown zoom, within [`ZOOM_RANGE`](crate::layout::ZOOM_RANGE)
    /// (`ZoomLadder::step_in_range`; Photoshop's rungs, the suite's one zoom ladder); a fit ends.
    /// The zoom a fit showed is the start (WordCraft issue #67).
    // wordcraft: crates/ui-egui/src/canvas/geometry.rs:234 `sync_fit_zoom` (start from the shown
    // zoom); the steps are craft-ui's.
    pub fn zoom_step(&mut self, larger: bool) {
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
    /// The canvas this pass: content coordinates of [`layout`](Self::layout) (screen points at
    /// the zoom, origin at the content's top-left) as the document, for the app's `ToolHost`,
    /// `CanvasScrollbars` and `Rulers`.
    pub canvas: CanvasView,
    /// The view's canvas id (from its id salt), for the `ToolHost` and craft-ui's canvas state.
    pub canvas_id: CanvasId,
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
///   ([`fit_scale`](crate::layout::fit_scale)), with the [`PageSpacing`](crate::layout::PageSpacing)
///   of [`PageView::style`], recomputed when the viewport or the pages change; the zoom is written
///   back so it equals what shows. Pages draw as craft-ui
///   [`PageFrame`](craft_ui::canvas::PageFrame)s on a [`Pasteboard`](craft_ui::canvas::Pasteboard)
///   (its page fill, border and shadow), the page picture over each through
///   `Thumbnails::state`/`paint` at `Priority::VISIBLE`, one row either side at `Priority::NEAR`;
///   a zoom past `MAX_TEXTURE` adds a detail picture of the part in view (`DetailPlan`).
/// - Scrolling: [`CanvasScrollbars`](craft_ui::canvas::CanvasScrollbars) and craft-ui's canvas
///   navigation (`canvas::navigate`: wheel, space-drag, the Hand and Zoom tools in the app's
///   `ToolHost`); the view keeps its offset in the state. Not
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
    source: &'a dyn PageSource,
    state: &'a mut PageViewState,
    options: LayoutOptions,
    name: Option<String>,
    bodies: Option<&'a [(f64, f64)]>,
}

impl<'a> PageView<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, state: &'a mut PageViewState) -> Self {
        PageView { id_salt: egui::IdSalt::new(id_salt), source, state, options: LayoutOptions::ACROBAT, name: None, bodies: None }
    }

    /// The view's spacing (`pages.gap`, `pages.margin`, `pages.side`, Hide White Space's pad and
    /// seam), from the tokens.
    pub fn style(tokens: &UiTokens) -> PageViewStyle {
        PageViewStyle::of(tokens)
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
