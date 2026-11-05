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

use std::ops::Range;

use craft_ui::canvas::navigate::PageKey;
use craft_ui::canvas::textures::{Priority, TextureCacheOptions, ThumbPaint, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails, texture_side};
use craft_ui::canvas::{CanvasId, CanvasScrollbars, CanvasView, PageFrame, Pasteboard, ScrollbarNames, ZoomLadder};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens};
use egui::accesskit::Role;
use egui::{Key, Modifiers, Rect, Response, Sense, Ui, Vec2};
use kurbo::{Point, Size};

use super::style::PageViewStyle;
use crate::layout::{
    Fit, LayoutOptions, PageDisplay, PageSpacing, PagedLayout, SCREEN_POINTS_PER_POINT, ViewDefaults, ZOOM_RANGE, fit_scale, place_pages, shown_row,
};

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
        self.fit = Fit::None;
        self.zoom = ZoomLadder::step_in_range(self.zoom, if larger { 1 } else { -1 }, ZOOM_RANGE);
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
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.pages;
        PageViewStyle {
            spacing: PageSpacing {
                gap: f64::from(t.gap),
                margin: f64::from(t.margin),
                side: f64::from(t.side),
                white_space_pad: f64::from(t.white_space_pad),
                seam: f64::from(t.seam),
            },
            paper: t.paper.to_egui(),
        }
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
        let PageView { id_salt, source, state, options, name, bodies } = self;
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let spacing = Self::style(&tokens).spacing;
        let rect = ui.available_rect_before_wrap();
        ui.advance_cursor_after_rect(rect);
        let canvas_id = CanvasId::new(ui, id_salt);
        let response = canvas_id.interact(ui, rect, Sense::click_and_drag() | Sense::FOCUSABLE);
        ctx.accesskit_node_builder(response.id, |node| {
            node.set_role(Role::Document);
            if let Some(name) = &name {
                node.set_label(name.as_str());
            }
        });

        let count = source.page_count();
        let sizes: Vec<Size> = (0..count).map(|page| source.page_size(page)).collect();
        let viewport = Size::new(f64::from(rect.width()), f64::from(rect.height()));
        let build = Geometry { sizes: &sizes, viewport, options, spacing, bodies };
        let mut geo = build.of(state);
        state.zoom = geo.scale / SCREEN_POINTS_PER_POINT;
        let started_on = state.current;
        let remembered_id = canvas_id.with("offset");
        let remembered: Option<(f64, f64)> = ctx.data(|d| d.get_temp(remembered_id));
        let (vw, vh) = (viewport.width, viewport.height);

        let pointer = response.contains_pointer().then(|| ui.input(|i| i.pointer.hover_pos())).flatten();
        if let Some(pointer) = pointer {
            let delta = f64::from(ui.input(|i| i.zoom_delta()));
            if delta.is_finite() && delta > 0.0 && delta != 1.0 {
                geo = zoom_about(&build, state, geo, rect, pointer, delta);
            }
        }

        // Keys and the wheel move the wanted offset; the clamp below keeps it on the document.
        let mut want = state.offset;
        let mut turn: Option<bool> = None;
        let mut wheel_left = 0.0;
        if pointer.is_some() && ui.is_enabled() {
            let scroll = ui.input(|i| i.smooth_scroll_delta);
            if scroll != Vec2::ZERO {
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
                want.0 -= f64::from(scroll.x);
                want.1 -= f64::from(scroll.y);
                wheel_left = f64::from(scroll.y);
            }
        }
        let mut go_to = state.go_to.take();
        if response.has_focus() {
            let moves = read_keys(ui, state, &geo, rect);
            want.0 += moves.pan.0;
            want.1 = if moves.to_end { f64::INFINITY } else { want.1 + moves.pan.1 };
            turn = turn.or(moves.turn);
            go_to = moves.go_to.or(go_to);
        }

        // A page that fits turns with the wheel; one panned to its edge turns with the wheel past it.
        if !state.continuous && turn.is_none() && wheel_left != 0.0 {
            turn = wheel_turn(&ctx, canvas_id, &geo, want.1, vh);
        }
        if let Some(forward) = turn {
            let next = if forward {
                (geo.row.end < count).then_some(geo.row.end)
            } else {
                geo.row.start.checked_sub(1).map(|before| shown_row(count, before, &build.layout_options(state)).start)
            };
            if let Some(next) = next {
                state.current = next;
                geo = build.of(state);
                state.zoom = geo.scale / SCREEN_POINTS_PER_POINT;
                let (lo, hi) = offset_range(geo.doc.y0, geo.doc.y1, vh);
                want.1 = if forward { lo } else { hi };
            } else {
                want.1 = state.offset.1;
            }
        }
        let went = go_to.is_some();
        if let Some((page, fraction)) = go_to {
            state.current = page.min(count.saturating_sub(1));
            geo = build.of(state);
            state.zoom = geo.scale / SCREEN_POINTS_PER_POINT;
            want.1 = match geo.layout.rects.get(state.current) {
                Some(r) if state.continuous => {
                    let (top, bottom) = geo.layout.bands[state.current];
                    r.y0 + top + fraction * (bottom - top) - spacing.margin
                }
                _ => geo.doc.y0,
            };
        }

        let (xs, ys) = (offset_range(geo.doc.x0, geo.doc.x1, vw), offset_range(geo.doc.y0, geo.doc.y1, vh));
        let offset = (want.0.clamp(xs.0, xs.1), want.1.clamp(ys.0, ys.1));
        state.offset = offset;
        if state.continuous && count > 0 && !went && remembered.is_some_and(|last| last != offset) {
            state.current = page_on_line(&geo.layout, offset.1 + vh / 2.0);
        }
        ctx.data_mut(|d| d.insert_temp(remembered_id, offset));
        let current_changed = state.current != started_on;

        let view = CanvasView::new(rect, Point::new(offset.0 + vw / 2.0, offset.1 + vh / 2.0), 1.0);
        let in_view = kurbo::Rect::new(offset.0, offset.1, offset.0 + vw, offset.1 + vh);
        let shown: Vec<usize> = geo.row.clone().filter(|&p| overlaps(band_rect(&geo.layout, p), in_view)).collect();
        let visible = match (shown.first(), shown.last()) {
            (Some(&first), Some(&last)) => first..last + 1,
            _ => 0..0,
        };
        let to_screen = |r: &kurbo::Rect| Rect::from_min_max(view.to_screen(Point::new(r.x0, r.y0)), view.to_screen(Point::new(r.x1, r.y1)));
        let pages: Vec<PlacedPage> = shown
            .iter()
            .map(|&page| PlacedPage {
                page,
                rect: to_screen(&geo.layout.rects[page]),
                scale: geo.scale,
                band: (geo.layout.bands[page].0 as f32, geo.layout.bands[page].1 as f32),
            })
            .collect();

        let frames: Vec<PageFrame> = shown.iter().map(|&p| PageFrame::new(band_rect(&geo.layout, p))).collect();
        Pasteboard::new((id_salt, "pasteboard"), &frames).show(ui, &view);

        let thumbs = Thumbnails::<usize>::in_context(
            &ctx,
            (id_salt, "pictures"),
            ThumbnailOptions { max_side: texture_side(&ctx), cache: TextureCacheOptions::pages(), ..ThumbnailOptions::default() },
        );
        let ppp = ctx.pixels_per_point();
        let ask = |page: usize, size: Vec2, priority: Priority| {
            let request = ThumbRequest::for_cell(size.x.max(size.y), ppp);
            thumbs.state(&ctx, source, &page, &request, priority)
        };
        let painter = ui.painter_at(rect);
        for placed in &pages {
            let state = ask(placed.page, placed.rect.size(), Priority::VISIBLE);
            let (top, bottom) = placed.band;
            let band =
                Rect::from_min_max(egui::pos2(placed.rect.min.x, placed.rect.min.y + top), egui::pos2(placed.rect.max.x, placed.rect.min.y + bottom));
            thumbs.paint(&painter.with_clip_rect(rect.intersect(band)), placed.rect, &state, &ThumbPaint::default().checker(false));
        }
        for page in near_pages(&geo, state.continuous, &visible, count, &build.layout_options(state)) {
            let size = geo.layout.rects[page].size();
            ask(page, Vec2::new(size.width as f32, size.height as f32), Priority::NEAR);
        }

        let mut bars = view;
        let names = ScrollbarNames::of(ui);
        let moved = CanvasScrollbars::new((id_salt, "scrollbars"), names).show(ui, &mut bars, geo.doc);
        if moved.changed {
            state.offset = (bars.center.x - vw / 2.0, bars.center.y - vh / 2.0);
            ctx.request_repaint();
        }

        let toggle_white_space =
            response.double_clicked() && response.interact_pointer_pos().is_some_and(|at| geo.layout.on_page_gap(view.to_doc(at)));
        PageViewResponse { response, canvas: view, canvas_id, pages, layout: geo.layout, visible, current_changed, toggle_white_space }
    }
}

/// The layout of this pass: what [`place_pages`] and the state's fit give, the row shown and the
/// scrollable document.
struct Placed {
    layout: PagedLayout,
    /// Screen points per page point.
    scale: f64,
    /// The pages that show: all when continuous, else the current page or spread.
    row: Range<usize>,
    /// The scroll area in content coordinates: the whole layout, or the shown row with its margins.
    doc: kurbo::Rect,
}

/// What a layout is made from.
struct Geometry<'a> {
    sizes: &'a [Size],
    viewport: Size,
    options: LayoutOptions,
    spacing: PageSpacing,
    bodies: Option<&'a [(f64, f64)]>,
}

impl Geometry<'_> {
    fn layout_options(&self, state: &PageViewState) -> LayoutOptions {
        LayoutOptions { display: state.display, cover: state.cover, continuous: state.continuous, page_gaps: state.page_gaps, ..self.options }
    }

    fn of(&self, state: &PageViewState) -> Placed {
        let options = self.layout_options(state);
        let scale = fit_scale(self.sizes, self.viewport, state.fit, &options, &self.spacing, Some(state.current))
            .unwrap_or(state.zoom * SCREEN_POINTS_PER_POINT)
            .clamp(ZOOM_RANGE.start() * SCREEN_POINTS_PER_POINT, ZOOM_RANGE.end() * SCREEN_POINTS_PER_POINT);
        let mut layout = place_pages(self.sizes, scale, self.viewport, &options, &self.spacing);
        if let Some(bodies) = self.bodies.filter(|_| state.hide_white_space) {
            layout.hide_white_space(bodies, scale, self.viewport.height, &self.spacing);
        }
        let count = self.sizes.len();
        let row = if state.continuous { 0..count } else { shown_row(count, state.current, &options) };
        let whole = kurbo::Rect::new(0.0, 0.0, layout.size.width, layout.size.height);
        let doc = if state.continuous || row.is_empty() {
            whole
        } else {
            let shown = row.clone().map(|p| band_rect(&layout, p)).reduce(|a, b| a.union(b)).unwrap_or(whole);
            kurbo::Rect::new(0.0, shown.y0 - self.spacing.margin, layout.size.width, shown.y1 + self.spacing.margin)
        };
        Placed { layout, scale, row, doc }
    }
}

/// The part of page `page` that shows, content coordinates.
fn band_rect(layout: &PagedLayout, page: usize) -> kurbo::Rect {
    let r = layout.rects[page];
    let (top, bottom) = layout.bands[page];
    kurbo::Rect::new(r.x0, r.y0 + top, r.x1, r.y0 + bottom)
}

fn overlaps(a: kurbo::Rect, b: kurbo::Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// The offsets a view `len` long can take over `lo..hi`; a document shorter than the view sits
/// centred in it.
fn offset_range(lo: f64, hi: f64, len: f64) -> (f64, f64) {
    let spare = hi - lo - len;
    if spare <= 0.0 {
        let middle = lo + spare / 2.0;
        (middle, middle)
    } else {
        (lo, lo + spare)
    }
}

/// The page that owns the horizontal line `y` (content coordinates); a tie at a shared edge goes
/// half a point to the earlier page.
fn page_on_line(layout: &PagedLayout, y: f64) -> usize {
    let last = layout.rects.len().saturating_sub(1);
    layout.rects.iter().zip(&layout.bands).position(|(r, &(_, bottom))| r.y0 + bottom + 0.5 > y).unwrap_or(last)
}

/// Zoom by `delta` keeping the page point under `pointer` where it is.
fn zoom_about(build: &Geometry, state: &mut PageViewState, old: Placed, rect: Rect, pointer: egui::Pos2, delta: f64) -> Placed {
    let inside = (f64::from(pointer.x - rect.min.x), f64::from(pointer.y - rect.min.y));
    let at = Point::new(state.offset.0 + inside.0, state.offset.1 + inside.1);
    state.fit = Fit::None;
    state.zoom = (state.zoom * delta).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
    let new = build.of(state);
    let (x, y) = match crate::layout::page_at(&old.layout, at) {
        Some(page) => {
            let (before, after) = (old.layout.rects[page], new.layout.rects[page]);
            (after.x0 + (at.x - before.x0) / before.width() * after.width(), after.y0 + (at.y - before.y0) / before.height() * after.height())
        }
        None => {
            let ratio = new.scale / old.scale;
            (at.x * ratio, at.y * ratio)
        }
    };
    state.offset = (x - inside.0, y - inside.1);
    new
}

/// The pages one row either side of what shows: the neighbouring rows, or the neighbouring
/// row of a single shown row.
fn near_pages(placed: &Placed, continuous: bool, visible: &Range<usize>, count: usize, options: &LayoutOptions) -> Vec<usize> {
    if continuous {
        if visible.is_empty() {
            return Vec::new();
        }
        let columns = placed.layout.columns.max(1);
        let before = visible.start.saturating_sub(columns)..visible.start;
        let after = visible.end..(visible.end + columns).min(count);
        before.chain(after).collect()
    } else {
        let before = placed.row.start.checked_sub(1).map(|p| shown_row(count, p, options));
        let after = (placed.row.end < count).then(|| shown_row(count, placed.row.end, options));
        before.into_iter().chain(after).flatten().collect()
    }
}

/// What this pass's keys ask of the view.
#[derive(Default)]
struct KeyMoves {
    /// Screen points to move the offset.
    pan: (f64, f64),
    /// End in a continuous view: the bottom.
    to_end: bool,
    /// Turn to the next (`true`) or previous row when not continuous.
    turn: Option<bool>,
    go_to: Option<(usize, f64)>,
}

/// Page Up/Down a screen (a row when not continuous), Home/End the first and last page, ↑/↓ a
/// line, ←/→ a page when the page fits the width, else a line sideways.
fn read_keys(ui: &Ui, state: &PageViewState, geo: &Placed, rect: Rect) -> KeyMoves {
    let key = |k: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, k));
    let (last, line) = (geo.layout.rects.len().saturating_sub(1), f64::from(PageKey::SMALL_STEP));
    let fits_width = geo.doc.width() <= f64::from(rect.width());
    let mut moves = KeyMoves::default();
    for k in [Key::PageDown, Key::PageUp] {
        if !key(k) {
            continue;
        }
        if state.continuous {
            if let Some(PageKey::Scroll(d)) = PageKey::of(k, Modifiers::NONE, rect.size()) {
                moves.pan.1 += f64::from(d.y);
            }
        } else {
            moves.turn = Some(k == Key::PageDown);
        }
    }
    if key(Key::Home) {
        moves.go_to = Some((0, 0.0));
    }
    if key(Key::End) {
        if state.continuous {
            moves.to_end = true;
        } else {
            moves.go_to = Some((last, 0.0));
        }
    }
    if key(Key::ArrowDown) {
        moves.pan.1 += line;
    }
    if key(Key::ArrowUp) {
        moves.pan.1 -= line;
    }
    for (k, forward) in [(Key::ArrowRight, true), (Key::ArrowLeft, false)] {
        if !key(k) {
            continue;
        }
        if !fits_width {
            moves.pan.0 += if forward { line } else { -line };
        } else if state.continuous {
            let next = if forward { state.current + 1 } else { state.current.saturating_sub(1) };
            moves.go_to = Some((next.min(last), 0.0));
        } else {
            moves.turn = Some(forward);
        }
    }
    moves
}

/// The wheel past the edge of the shown row gathers over passes; a notch's worth turns the page.
fn wheel_turn(ctx: &egui::Context, canvas: CanvasId, geo: &Placed, wanted_y: f64, view_height: f64) -> Option<bool> {
    let (lo, hi) = offset_range(geo.doc.y0, geo.doc.y1, view_height);
    let beyond = if wanted_y > hi {
        wanted_y - hi
    } else if wanted_y < lo {
        wanted_y - lo
    } else {
        0.0
    };
    let notch = f64::from(ctx.options(|o| o.input_options.line_scroll_speed));
    let id = canvas.with("turn");
    let gathered = ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(0.0);
    let gathered = if beyond == 0.0 || gathered * beyond < 0.0 { beyond } else { gathered + beyond };
    if gathered.abs() >= notch {
        ctx.data_mut(|d| d.remove::<f64>(id));
        Some(gathered > 0.0)
    } else {
        ctx.data_mut(|d| d.insert_temp(id, gathered));
        None
    }
}
