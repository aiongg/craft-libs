//! [`Thumbnails`]: the one thumbnail contract. The app supplies pixels at once or as a job, keyed
//! by a key and a revision ([`ThumbSource`]); the library uploads, caches, evicts, asks for a
//! redraw and paints ([`Thumbnails::paint`]).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use craft_host::Jobs;
use craft_host::queue::{Cancel, Priority};
use egui::{AsIdSalt, Color32, ColorImage, Context, Painter, Rect, Vec2};

use super::cache::{Revision, TextureCache, TextureCacheOptions, TextureKey, TextureState};
use super::loader::{LoadedPicture, TextureLoader};
use super::style::{self, ThumbnailStyle};
use crate::theme::UiTokens;

/// What a thumbnail is asked at: the side in pixels and the screen's density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbRequest {
    /// The longest side of the picture, in pixels: a multiple of 16.
    pub side: u32,
    /// Pixels per point of the screen it shows on.
    pub pixels_per_point: f32,
}

impl ThumbRequest {
    /// The request for a cell `points` wide on a screen of `pixels_per_point`: the side in pixels
    /// rounded up to a multiple of 16, at least 16 and at most [`MAX_TEXTURE`](super::MAX_TEXTURE).
    /// 28 points at 2× is 64 pixels, PhotoCraft's side. Neighbouring cell sizes share one picture
    /// (PdfCraft's `SIZE_STEP`). [`Thumbnails::state`] lowers a side past its
    /// [`ThumbnailOptions::max_side`].
    pub fn for_cell(points: f32, pixels_per_point: f32) -> ThumbRequest {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:25 (`SIZE_STEP` 16).
        todo!()
    }
}

/// A background render: runs on a queue lane, checks the [`Cancel`] between steps, and returns the
/// picture ([`LoadedPicture::Image`]), [`LoadedPicture::Blank`] when the key turns out to have no
/// picture (state `Empty`, as [`ThumbRender::Blank`]), or a message for the failure tooltip. It
/// owns what it reads (an `Arc` of the node, a path), never a borrow of the document.
pub type ThumbJob = Box<dyn FnOnce(&Cancel) -> Result<LoadedPicture, String> + Send>;

/// What a [`ThumbSource`] renders for a key.
pub enum ThumbRender {
    /// The picture, made now on the UI thread (a light layer, a fill colour).
    Ready(ColorImage),
    /// A job that makes it on a queue lane (a heavy node, a PDF page, a decoded file).
    Later(ThumbJob),
    /// No picture: state `Empty`, and the row paints its own tile (a group's folder, an
    /// adjustment's glyph, a type layer's "T").
    Blank,
}

/// The app's side of the contract: how a key's picture is versioned and made.
///
/// - `revision` is cheap: a stored fingerprint or pointer identity with a `Weak` guard, never a
///   pixel walk. It covers the pixels only, so a step that changes only the selection keeps it and
///   every thumbnail stays (PhotoCraft's 0.7 s composite on a 20 MP document, #4518). The request's
///   side is added by [`Thumbnails`]; the revision need not include it.
/// - `render` is called only when the revision has no picture yet, within the pass's render
///   budget. The picture has the frame's aspect, longest side at most `request.side`, with no
///   letterbox bars ([`thumb_image`] builds one from a sampler); [`Thumbnails::paint`] fits it to
///   the cell.
/// - Pictures are display-space sRGBA, unmultiplied (`ColorImage::from_rgba_unmultiplied`);
///   colour management is the app's and happens before the picture is returned.
///
/// ```ignore
/// impl ThumbSource<NodeId> for Art<'_> {
///     fn revision(&self, key: &NodeId, _: &ThumbRequest) -> Revision { Revision::of(&self.doc.node_stamp(*key)) }
///     fn render(&self, key: &NodeId, request: &ThumbRequest) -> ThumbRender {
///         let (node, side) = (self.doc.node_arc(*key), request.side);
///         ThumbRender::Later(Box::new(move |cancel| draw_node(&node, side, cancel).map(LoadedPicture::from)))
///     }
/// }
/// ```
pub trait ThumbSource<K> {
    /// The version of `key`'s picture.
    fn revision(&self, key: &K, request: &ThumbRequest) -> Revision;
    /// Makes `key`'s picture, now or as a job.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender;
}

/// A [`Thumbnails`]' settings, read when it is created.
#[derive(Clone)]
pub struct ThumbnailOptions {
    /// The longest side any request is lowered to, rounded down to 16: 512 (LightCraft's top
    /// bucket); PdfCraft's page grid sets 1536.
    pub max_side: u32,
    /// Budgets of the texture cache: [`TextureCacheOptions::thumbnails`].
    pub cache: TextureCacheOptions,
    /// Lanes of the queue for `Later` renders; see
    /// [`LoaderOptions::lanes`](super::LoaderOptions::lanes).
    pub lanes: usize,
    /// Where the lanes run; see [`LoaderOptions::jobs`](super::LoaderOptions::jobs).
    pub jobs: Option<Arc<dyn Jobs>>,
    /// Renders on the UI thread per pass: 16 (VectorCraft's `THUMB_BUDGET_COUNT`).
    pub renders_per_pass: usize,
    /// Time for renders on the UI thread per pass: 8 ms (VectorCraft's `THUMB_BUDGET_MS`). Desktop
    /// only; on wasm32 only the count applies (no clock there).
    pub render_time_per_pass: Duration,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:76 (`THUMB_BUDGET_MS` 8), :78
        // (`THUMB_BUDGET_COUNT` 16); lightcraft: crates/engine/src/media/source.rs:7 (`THUMB_SIZES` up to 512);
        // `lanes: QueueOptions::default_lanes()`, `jobs: None`.
        todo!()
    }
}

/// How [`Thumbnails::paint`] fits a picture into its cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThumbFit {
    /// The whole picture, as large as fits, centred ([`fit_rect`]); the checkerboard is under the
    /// picture only, so the frame's empty sides stay the row's.
    #[default]
    Contain,
    /// The cell filled, the picture cut at its long sides around its centre (a square grid of
    /// photos).
    Cover,
}

/// How to paint one thumbnail.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbPaint {
    pub fit: ThumbFit,
    /// The checkerboard behind the picture's transparency (default on). Off for opaque pictures
    /// such as photos and pages.
    pub checker: bool,
    /// Multiplies the picture (default white): a dimmed row's faint thumbnail.
    pub tint: Color32,
}

impl Default for ThumbPaint {
    fn default() -> Self {
        Self { fit: ThumbFit::Contain, checker: true, tint: Color32::WHITE }
    }
}

impl ThumbPaint {
    pub fn fit(mut self, fit: ThumbFit) -> Self {
        self.fit = fit;
        self
    }

    pub fn checker(mut self, checker: bool) -> Self {
        self.checker = checker;
        self
    }

    pub fn tint(mut self, tint: Color32) -> Self {
        self.tint = tint;
        self
    }
}

/// The per-pass state of a [`Thumbnails`].
struct Inner {
    options: ThumbnailOptions,
    /// While held, shown pictures stay and no render starts.
    hold: bool,
    /// The pass the render counters are for, and the counters.
    pass: u64,
    renders: usize,
    #[cfg(not(target_arch = "wasm32"))]
    render_time: Duration,
}

/// Thumbnails of one kind (layers, channels, snapshots, a grid's files) over one cache: what
/// every list and grid in the library draws its pictures with.
///
/// ```ignore
/// let thumbs = Thumbnails::<LayerThumb<LayerId>>::in_context(ctx, "layers.thumbs", ThumbnailOptions::default());
/// let request = ThumbRequest::for_cell(side, ctx.pixels_per_point());
/// let state = thumbs.state(ctx, &doc_source, &key, &request, Priority::VISIBLE);
/// ListRow::new(layer.id, &layer.name).thumb(size, |painter, rect| { thumbs.paint(painter, rect, &state, &ThumbPaint::default()); });
/// ```
///
/// - **Asking.** A widget calls [`state`](Self::state) for each row whose rect is visible
///   (`ui.is_rect_visible`) at [`Priority::VISIBLE`], and for one row or one viewport beyond at
///   [`Priority::NEAR`], before the row's paint closure runs (the closure only gets a painter and
///   a rect). Rows asked for in neither this pass nor the previous one have their jobs dropped.
/// - **Making.** A revision without a picture is rendered by the source: on the UI thread within
///   [`renders_per_pass`](ThumbnailOptions::renders_per_pass) and
///   [`render_time_per_pass`](ThumbnailOptions::render_time_per_pass) (the rest wait with their
///   stand-ins and the pass asks for an immediate repaint), or as a `Later` job on the loader's
///   queue. A new revision keeps the old picture as the stand-in until the new one lands. A failure
///   is remembered for its revision, so nothing retries every pass.
/// - **Holding.** [`hold`](Self::hold)`(true)` while a drag or a stroke runs: shown pictures stay,
///   no render starts (VectorCraft's `busy`).
/// - **Painting.** [`paint`](Self::paint) draws the checkerboard, the picture fitted to its
///   aspect, the placeholder while nothing has arrived, or the failure icon. No widget draws its own
///   placeholder or failure glyph. Pictures never mirror in a right-to-left interface.
/// - **AccessKit.** None: the row names the thing.
///
/// `Thumbnails<K>` is `Clone + Send + Sync` and lives in the egui context.
// pc: crates/ui-egui/src/app/thumbs.rs:118-134 (synchronous render, in-place update);
// vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:66-78 (light nodes on the UI thread within
// 8 ms and 16 per pass, the rest on a worker; `busy` keeps shown thumbnails).
pub struct Thumbnails<K: TextureKey> {
    loader: TextureLoader<K>,
    inner: Arc<Mutex<Inner>>,
}

impl<K: TextureKey> Clone for Thumbnails<K> {
    fn clone(&self) -> Self {
        Self { loader: self.loader.clone(), inner: Arc::clone(&self.inner) }
    }
}

const _: () = {
    const fn send_sync<T: Send + Sync + Clone + 'static>() {}
    send_sync::<Thumbnails<u64>>();
};

impl<K: TextureKey> Thumbnails<K> {
    /// The thumbnails stored in `ctx` under `id_salt`, made with `options` the first time; the
    /// loader and its cache are stored under the same salt.
    pub fn in_context(ctx: &Context, id_salt: impl AsIdSalt, options: ThumbnailOptions) -> Self {
        // new: egui memory under `Id::new(("craft_ui::canvas::Thumbnails", id_salt))`, with
        // `TextureLoader::in_context(ctx, id_salt, LoaderOptions { cache, lanes, jobs })`.
        todo!()
    }

    /// What `key` shows now; renders or queues it when its revision has no picture yet.
    ///
    /// In order: the request's side is lowered to `max_side` (rounded down to 16); the revision is
    /// `source.revision(key, request)` combined with the side; the loader is
    /// [asked](TextureLoader::ask) and its answer returned when it has one. Otherwise, while
    /// [held](Self::hold) or past the pass's render budget, the key's stand-in is returned as
    /// `Loading` (past the budget with `ctx.request_repaint()`). Otherwise `source.render`:
    /// `Ready` is put in the cache (counted against the budget), `Blank` is stored as `Empty`, and
    /// `Later` is [submitted](TextureLoader::submit) at `priority`.
    pub fn state(&self, ctx: &Context, source: &dyn ThumbSource<K>, key: &K, request: &ThumbRequest, priority: Priority) -> TextureState {
        // pc: crates/ui-egui/src/panels/layer_row.rs:529-535 (render inside the row, only when visible);
        // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:69-78 (UI-thread budget, heavy nodes on a worker).
        todo!()
    }

    /// Paints `state` into `rect` and returns whether it drew anything:
    /// - `Ready`, or `Loading` with a stand-in: the picture fitted by `paint.fit` (see [`ThumbFit`]),
    ///   over the [style](Self::style)'s checkerboard when `paint.checker`, tinted by `paint.tint`.
    /// - `Missing`, or `Loading` without a stand-in: the placeholder fill over `rect`.
    /// - `Failed`: the placeholder with `Icon::Warning` centred in the failed-icon colour, at
    ///   `size.icon.sm` or the cell's side less 4 points if smaller. The message is the host's
    ///   tooltip (`ListRow::tooltip`).
    /// - `Empty`: nothing; returns false, and the row paints its own tile.
    ///
    /// The style comes from the painter's context tokens. Rects are rounded to whole pixels.
    pub fn paint(&self, painter: &Painter, rect: Rect, state: &TextureState, paint: &ThumbPaint) -> bool {
        // pc: crates/ui-egui/src/panels/layer_row.rs:533-535 (`checker(p, fitted, 5.0)` then the image);
        // pc: crates/ui-egui/src/widgets/color.rs:29 `checker` (250 and 214 grey).
        todo!()
    }

    /// Freezes the shown pictures while `hold`: no render starts and no job is queued; `state`
    /// returns the current picture of each key (as `Loading` when its revision moved on). Results
    /// of jobs already running still land.
    pub fn hold(&self, hold: bool) {
        // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:15, :170 (`busy` during an interaction).
        todo!()
    }

    /// Keeps only the pictures whose key `keep` accepts (the open documents changed).
    pub fn retain(&self, keep: impl FnMut(&K) -> bool) {
        // pc: crates/ui-egui/src/app/thumbs.rs:4-37 `prune_thumbs`.
        todo!()
    }

    /// The texture cache under the thumbnails, for [`forget`](TextureCache::forget),
    /// [`clear`](TextureCache::clear) and [`statistics`](TextureCache::statistics).
    pub fn cache(&self) -> &TextureCache<K> {
        self.loader.cache()
    }

    /// The thumbnail look of `tokens`: checkerboard, placeholder and failure icon.
    pub fn style(tokens: &UiTokens) -> ThumbnailStyle {
        style::style(tokens)
    }
}

/// The rect inside `cell` that shows a picture of `picture`'s aspect whole, as large as fits and
/// centred; `cell` itself for a zero-sized picture.
pub fn fit_rect(cell: Rect, picture: Vec2) -> Rect {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:557 `fit_thumb` (without its letterbox UVs).
    todo!()
}

/// A picture of the document area `frame_size` (pixels) with that exact aspect, its longest side
/// `longest_side`, each side at least 1: texel `(tx, ty)` is `sample(x, y)` at the document pixel
/// under the texel's centre, `x` and `y` counted from the frame's top left (the caller adds the
/// frame's origin). `sample` returns sRGBA, unmultiplied. A frame smaller than the side is
/// sampled nearest, larger ones point-sampled at texel centres, as PhotoCraft does.
pub fn thumb_image(frame_size: [u32; 2], longest_side: u32, sample: impl FnMut(u32, u32) -> [u8; 4]) -> ColorImage {
    // pc: crates/ui-egui/src/app/thumbs.rs:162 `thumb_image` (letterboxed square there; exact aspect here).
    todo!()
}

/// A [`ThumbSource`] for stories and tests in every crate: pictures made from the key and a
/// revision the story sets, so every state shows without a document.
///
/// - The picture: a diagonal gradient between two colours whose hues come from the key's hash
///   (start hue `hash % 360`, end hue 120° on), fully opaque except a transparent top-left quarter
///   (so the checkerboard shows), with `revision % 8` white bars of one texel along the bottom edge;
///   [`aspect`](Self::aspect) shapes the frame.
/// - [`later`](Self::later)`(true)` renders as `Later` jobs, so with a
///   [`ManualJobs`](craft_host::ManualJobs) in [`ThumbnailOptions::jobs`] pictures arrive one per
///   `run_next`.
/// - Keys given to [`failing`](Self::failing) fail with the message "Fake failure" (always as a
///   `Later` job, since a synchronous render cannot fail); keys given to [`blank`](Self::blank)
///   render `Blank`.
/// - [`set_revision`](Self::set_revision) and [`bump`](Self::bump) change every key's revision, so
///   stand-ins show.
pub struct FakeThumbSource<K> {
    revision: u64,
    later: bool,
    aspect: [u32; 2],
    failing: Vec<K>,
    blank: Vec<K>,
}

impl<K: Hash + PartialEq + Clone + Send + 'static> FakeThumbSource<K> {
    /// Synchronous square pictures at revision 0.
    pub fn new() -> Self {
        Self { revision: 0, later: false, aspect: [1, 1], failing: Vec::new(), blank: Vec::new() }
    }

    /// Renders as `Later` jobs.
    pub fn later(mut self, later: bool) -> Self {
        self.later = later;
        self
    }

    /// The frame's width to height (4:3 landscape, 3:4 portrait).
    pub fn aspect(mut self, width: u32, height: u32) -> Self {
        self.aspect = [width.max(1), height.max(1)];
        self
    }

    /// These keys fail.
    pub fn failing(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        self.failing.extend(keys);
        self
    }

    /// These keys have no picture.
    pub fn blank(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        self.blank.extend(keys);
        self
    }

    /// Every key's revision.
    pub fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    /// Moves every key's revision on by one.
    pub fn bump(&mut self) {
        self.revision += 1;
    }
}

impl<K: Hash + PartialEq + Clone + Send + 'static> Default for FakeThumbSource<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + PartialEq + Clone + Send + 'static> ThumbSource<K> for FakeThumbSource<K> {
    /// `Revision::of(&(key, revision))`.
    fn revision(&self, key: &K, request: &ThumbRequest) -> Revision {
        // new: see `FakeThumbSource`.
        todo!()
    }

    /// The picture described on [`FakeThumbSource`], at `thumb_image(aspect scaled, request.side, ..)`.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender {
        // new: see `FakeThumbSource`.
        todo!()
    }
}
