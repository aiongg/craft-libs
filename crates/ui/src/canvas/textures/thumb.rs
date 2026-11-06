//! [`Thumbnails`]: the one thumbnail contract. The app supplies pixels at once or as a job, keyed
//! by a key and a revision ([`ThumbSource`]); the library uploads, caches, evicts, asks for a
//! redraw and paints ([`Thumbnails::paint`]).

use std::hash::{DefaultHasher, Hash, Hasher as _};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use craft_host::Jobs;
use craft_host::queue::{Cancel, Priority, QueueOptions};
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, ColorImage, Context, Id, Painter, Rect, Vec2, pos2, vec2};

use super::MAX_TEXTURE;
use super::cache::{CachedTexture, Revision, TextureCache, TextureCacheOptions, TextureKey, TextureState};
use super::loader::{LoadFailure, LoadedPicture, LoaderOptions, TextureLoader};
use super::style::{self, ThumbnailStyle};
use crate::icons::{self, Icon, IconSize};
use crate::theme::UiTokens;

/// What a thumbnail is asked at: the side in pixels, the screen's density and the part of the
/// frame wanted.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbRequest {
    /// The longest side of the picture, in pixels: a multiple of 16.
    pub side: u32,
    /// Pixels per point of the screen it shows on.
    pub pixels_per_point: f32,
    /// The part of the frame to render, in fractions of the frame (0 to 1 on both axes), or the
    /// whole frame. `side` is then the longest side of that part: a page larger than the texture
    /// side on screen asks for the part in view and shows its detail. The region is part of the
    /// picture's revision, so each region has its own texture.
    pub region: Option<Rect>,
}

/// Requested sides are multiples of this many pixels.
const SIZE_STEP: u32 = 16;

impl ThumbRequest {
    /// The request for a cell `points` wide on a screen of `pixels_per_point`: the side in pixels
    /// rounded up to a multiple of 16, at least 16 and at most [`MAX_TEXTURE`](super::MAX_TEXTURE).
    /// 28 points at 2× is 64 pixels, PhotoCraft's side. Neighbouring cell sizes share one picture
    /// (PdfCraft's `SIZE_STEP`). [`Thumbnails::state`] lowers a side past its
    /// [`ThumbnailOptions::max_side`].
    pub fn for_cell(points: f32, pixels_per_point: f32) -> ThumbRequest {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:25 (`SIZE_STEP` 16).
        let pixels = points * pixels_per_point;
        let steps = if pixels.is_finite() { (pixels / SIZE_STEP as f32).ceil().max(1.0) as u32 } else { 1 };
        let side = steps.saturating_mul(SIZE_STEP).min(MAX_TEXTURE / SIZE_STEP * SIZE_STEP);
        ThumbRequest { side, pixels_per_point, region: None }
    }

    /// The request for the part `region` of the frame (fractions of the frame, 0 to 1).
    pub fn region(self, region: Rect) -> ThumbRequest {
        ThumbRequest { region: Some(region), ..self }
    }
}

/// The request's region as hashable bits.
fn region_bits(region: Option<Rect>) -> Option<[u32; 4]> {
    region.map(|r| [r.min.x.to_bits(), r.min.y.to_bits(), r.max.x.to_bits(), r.max.y.to_bits()])
}

/// A background render: runs on a queue lane, checks the [`Cancel`] between steps, and returns the
/// picture ([`LoadedPicture::Image`]), [`LoadedPicture::Blank`] when the key turns out to have no
/// picture (state `Empty`, as [`ThumbRender::Blank`]), or the [`LoadFailure`] the tooltip words. It
/// owns what it reads (an `Arc` of the node, a path), never a borrow of the document.
pub type ThumbJob = Box<dyn FnOnce(&Cancel) -> Result<LoadedPicture, LoadFailure> + Send>;

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
        Self {
            max_side: 512,
            cache: TextureCacheOptions::thumbnails(),
            lanes: QueueOptions::default_lanes(),
            jobs: None,
            renders_per_pass: 16,
            render_time_per_pass: Duration::from_millis(8),
        }
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
    /// How the picture fits its box.
    pub fit: ThumbFit,
    /// The checkerboard behind the picture's transparency (default on). Off for opaque pictures
    /// such as photos and pages.
    pub checker: bool,
    /// Multiplies the picture (default white): a dimmed row's faint thumbnail.
    pub tint: Color32,
}

impl Default for ThumbPaint {
    fn default() -> Self {
        Self { fit: ThumbFit::Contain, checker: true, tint: Color32::WHITE } // design: allow(identity tint)
    }
}

impl ThumbPaint {
    /// Sets how the picture fits its box.
    pub fn fit(mut self, fit: ThumbFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets whether the checkerboard shows behind transparency.
    pub fn checker(mut self, checker: bool) -> Self {
        self.checker = checker;
        self
    }

    /// Sets the tint that multiplies the picture.
    pub fn tint(mut self, tint: Color32) -> Self {
        self.tint = tint;
        self
    }
}

impl Inner {
    /// Whether this pass has used its count or time for renders on the UI thread.
    fn over_budget(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.render_time >= self.options.render_time_per_pass {
            return true;
        }
        self.renders >= self.options.renders_per_pass
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
        let id = Id::unique("craft_ui::canvas::Thumbnails").with(&id_salt);
        if let Some(thumbnails) = ctx.data(|data| data.get_temp::<Self>(id)) {
            return thumbnails;
        }
        let loader_options = LoaderOptions { cache: options.cache, lanes: options.lanes, jobs: options.jobs.clone() };
        let loader = TextureLoader::in_context(ctx, &id_salt, loader_options);
        let inner = Inner {
            options,
            hold: false,
            pass: u64::MAX,
            renders: 0,
            #[cfg(not(target_arch = "wasm32"))]
            render_time: Duration::ZERO,
        };
        let thumbnails = Self { loader, inner: Arc::new(Mutex::new(inner)) };
        ctx.data_mut(|data| data.get_temp_mut_or_insert_with(id, || thumbnails).clone())
    }

    /// What `key` shows now; renders or queues it when its revision has no picture yet.
    ///
    /// In order: the request's side is lowered to `max_side` (rounded down to 16); the revision is
    /// `source.revision(key, request)` combined with the side and the region; the loader is
    /// [asked](TextureLoader::ask) and its answer returned when it has one. Otherwise, while
    /// [held](Self::hold) or past the pass's render budget, the key's stand-in is returned as
    /// `Loading` (past the budget with `ctx.request_repaint()`). Otherwise `source.render`:
    /// `Ready` is put in the cache (counted against the budget), `Blank` is stored as `Empty`, and
    /// `Later` is [submitted](TextureLoader::submit) at `priority`.
    pub fn state(&self, ctx: &Context, source: &dyn ThumbSource<K>, key: &K, request: &ThumbRequest, priority: Priority) -> TextureState {
        // pc: crates/ui-egui/src/panels/layer_row.rs:529-535 (render inside the row, only when visible);
        // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:69-78 (UI-thread budget, heavy nodes on a worker).
        let max_side = self.lock().options.max_side;
        let side = request.side.min((max_side / SIZE_STEP * SIZE_STEP).max(SIZE_STEP));
        let request = ThumbRequest { side, ..*request };
        let revision = Revision::of(&(source.revision(key, &request), side, region_bits(request.region)));
        if let Some(state) = self.loader.ask(ctx, key, revision, priority) {
            return state;
        }
        let stand_in = || TextureState::Loading { stand_in: self.loader.cache().state(ctx, key, revision).shown() };
        {
            let mut inner = self.lock();
            let pass = ctx.cumulative_pass_nr();
            if inner.pass != pass {
                inner.pass = pass;
                inner.renders = 0;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    inner.render_time = Duration::ZERO;
                }
            }
            if inner.hold {
                return stand_in();
            }
            if inner.over_budget() {
                ctx.request_repaint();
                return stand_in();
            }
        }
        match source.render(key, &request) {
            ThumbRender::Ready(image) => {
                #[cfg(not(target_arch = "wasm32"))]
                let started = Instant::now();
                let texture = self.loader.cache().put(ctx, key.clone(), revision, image);
                let mut inner = self.lock();
                inner.renders += 1;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    inner.render_time += started.elapsed();
                }
                TextureState::Ready(texture)
            }
            ThumbRender::Blank => {
                self.loader.cache().put_empty(ctx, key.clone(), revision);
                TextureState::Empty
            }
            ThumbRender::Later(job) => self.loader.submit(ctx, key.clone(), revision, priority, job),
        }
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
        let ppp = painter.ctx().pixels_per_point();
        let rect = rect.round_to_pixels(ppp);
        let style = Self::style(&crate::theme::current(painter.ctx()));
        match state {
            TextureState::Empty => false,
            TextureState::Ready(texture) | TextureState::Loading { stand_in: Some(texture) } => {
                paint_picture(painter, rect, texture, paint, &style);
                true
            }
            TextureState::Missing | TextureState::Loading { stand_in: None } => {
                Self::paint_placeholder(painter, rect);
                true
            }
            TextureState::Failed { .. } => {
                Self::paint_placeholder(painter, rect);
                let tokens = crate::theme::current(painter.ctx());
                let points = IconSize::Sm.points(&tokens).min(rect.width().min(rect.height()) - 4.0).max(0.0);
                let set = icons::active(painter.ctx());
                icons::paint_on(painter, rect, Icon::Warning, set, points, IconSize::Sm.stroke(), style.failed_icon);
                true
            }
        }
    }

    /// Fills `rect` with the placeholder look, for widgets that need it without a [`TextureState`]
    /// (a row with no cache). [`paint`](Self::paint) draws it for `Missing`, `Loading` and `Failed`.
    pub fn paint_placeholder(painter: &Painter, rect: Rect) {
        let rect = rect.round_to_pixels(painter.ctx().pixels_per_point());
        painter.rect_filled(rect, 0.0, Self::style(&crate::theme::current(painter.ctx())).placeholder);
    }

    /// Stops the queue of `Later` renders and waits up to `timeout` for the running ones to end
    /// ([`TextureLoader::shutdown`]); returns whether they have. For an app about to exit the
    /// process.
    pub fn shutdown(&self, timeout: Duration) -> bool {
        self.loader.shutdown(timeout)
    }

    /// Freezes the shown pictures while `hold`: no render starts and no job is queued; `state`
    /// returns the current picture of each key (as `Loading` when its revision moved on). Results
    /// of jobs already running still land.
    pub fn hold(&self, hold: bool) {
        // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:15, :170 (`busy` during an interaction).
        self.lock().hold = hold;
    }

    /// Keeps only the pictures whose key `keep` accepts (the open documents changed).
    pub fn retain(&self, keep: impl FnMut(&K) -> bool) {
        // pc: crates/ui-egui/src/app/thumbs.rs:4-37 `prune_thumbs`.
        self.loader.cache().retain(keep);
    }

    /// The texture cache under the thumbnails, for [`forget`](TextureCache::forget),
    /// [`clear`](TextureCache::clear) and [`statistics`](TextureCache::statistics).
    pub fn cache(&self) -> &TextureCache<K> {
        self.loader.cache()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
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
    if !(picture.x > 0.0 && picture.y > 0.0) {
        return cell;
    }
    let scale = (cell.width() / picture.x).min(cell.height() / picture.y);
    Rect::from_center_size(cell.center(), picture * scale)
}

/// A picture of the document area `frame_size` (pixels) with that exact aspect, its longest side
/// `longest_side`, each side at least 1: texel `(tx, ty)` is `sample(x, y)` at the document pixel
/// under the texel's centre, `x` and `y` counted from the frame's top left (the caller adds the
/// frame's origin). `sample` returns sRGBA, unmultiplied. A frame smaller than the side is
/// sampled nearest, larger ones point-sampled at texel centres, as PhotoCraft does.
pub fn thumb_image(frame_size: [u32; 2], longest_side: u32, sample: impl FnMut(u32, u32) -> [u8; 4]) -> ColorImage {
    // pc: crates/ui-egui/src/app/thumbs.rs:162 `thumb_image` (letterboxed square there; exact aspect here).
    let [width, height] = thumb_size(frame_size, longest_side);
    let [frame_width, frame_height] = frame_size.map(|side| u64::from(side.max(1)));
    let mut sample = sample;
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for ty in 0..height {
        let y = ((2 * u64::from(ty) + 1) * frame_height / (2 * u64::from(height))).min(frame_height - 1) as u32;
        for tx in 0..width {
            let x = ((2 * u64::from(tx) + 1) * frame_width / (2 * u64::from(width))).min(frame_width - 1) as u32;
            let [r, g, b, a] = sample(x, y);
            pixels.push(Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    ColorImage::new([width as usize, height as usize], pixels)
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
    fn revision(&self, key: &K, _request: &ThumbRequest) -> Revision {
        // new: see `FakeThumbSource`.
        Revision::of(&(key, self.revision))
    }

    /// The picture described on [`FakeThumbSource`], at `thumb_image(aspect scaled, request.side, ..)`.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender {
        // new: see `FakeThumbSource`.
        if self.blank.contains(key) {
            return ThumbRender::Blank;
        }
        if self.failing.contains(key) {
            return ThumbRender::Later(Box::new(|_| Err(LoadFailure::Message("Fake failure".to_owned()))));
        }
        let [width, height] = self.aspect;
        let frame = thumb_size([width, height], request.side);
        let (start, revision) = (key_hue(key), self.revision);
        let image = fake_picture(frame, start, revision);
        if self.later { ThumbRender::Later(Box::new(move |_| Ok(LoadedPicture::Image(image)))) } else { ThumbRender::Ready(image) }
    }
}

/// The pixel size of a picture of `frame_size`'s aspect with its longest side `longest_side`.
fn thumb_size(frame_size: [u32; 2], longest_side: u32) -> [u32; 2] {
    let [frame_width, frame_height] = frame_size.map(|side| f64::from(side.max(1)));
    let longest = f64::from(longest_side.max(1));
    let scale = longest / frame_width.max(frame_height);
    [(frame_width * scale).round().max(1.0) as u32, (frame_height * scale).round().max(1.0) as u32]
}

/// Hue in degrees from the key's hash.
fn key_hue(key: &impl Hash) -> f32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() % 360) as f32
}

/// [`FakeThumbSource`]'s picture: a gradient from `hue` to `hue + 120` degrees, a transparent top
/// left quarter and `revision % 8` white texels along the bottom row.
fn fake_picture([width, height]: [u32; 2], hue: f32, revision: u64) -> ColorImage {
    let bars = (revision % 8) as u32;
    thumb_image([width, height], width.max(height), |x, y| {
        if y + 1 == height && x % 2 == 0 && x / 2 < bars {
            return [255, 255, 255, 255];
        }
        if x < width / 2 && y < height / 2 {
            return [0, 0, 0, 0];
        }
        let t = ((x as f32 + 0.5) / width as f32 + (y as f32 + 0.5) / height as f32) / 2.0;
        let end = |degrees: f32| egui::ecolor::Hsva::new((degrees % 360.0) / 360.0, 0.6, 0.85, 1.0).to_srgba_unmultiplied();
        let (from, to) = (end(hue), end(hue + 120.0));
        let mix = |i: usize| (f32::from(from[i]) + (f32::from(to[i]) - f32::from(from[i])) * t).round() as u8;
        [mix(0), mix(1), mix(2), 255]
    })
}

/// Draws the checkerboard and the picture for `paint.fit`.
fn paint_picture(painter: &Painter, cell: Rect, texture: &CachedTexture, paint: &ThumbPaint, style: &ThumbnailStyle) {
    let ppp = painter.ctx().pixels_per_point();
    let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let (target, uv) = match paint.fit {
        ThumbFit::Contain => (fit_rect(cell, texture.size_vec2()).round_to_pixels(ppp), full),
        ThumbFit::Cover => (cell, cover_uv(cell, texture.size_vec2())),
    };
    if paint.checker {
        paint_checker(painter, target, style);
    }
    painter.image(texture.id, target, uv, paint.tint);
}

/// The part of a picture a `Cover` fit shows in `cell`: centred, cut at the long sides.
fn cover_uv(cell: Rect, picture: Vec2) -> Rect {
    if !(picture.x > 0.0 && picture.y > 0.0 && cell.width() > 0.0 && cell.height() > 0.0) {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let scale = (cell.width() / picture.x).max(cell.height() / picture.y);
    let shown = vec2(cell.width() / (picture.x * scale), cell.height() / (picture.y * scale));
    Rect::from_center_size(pos2(0.5, 0.5), shown)
}

/// The checkerboard over `rect`: cell `a` at the top left, `b` beside it, cut at the rect's edges.
fn paint_checker(painter: &Painter, rect: Rect, style: &ThumbnailStyle) {
    painter.rect_filled(rect, 0.0, style.checker[0]);
    let cell = style.checker_cell.max(1.0);
    let (columns, rows) = ((rect.width() / cell).ceil() as usize, (rect.height() / cell).ceil() as usize);
    for row in 0..rows {
        for column in 0..columns {
            if (row + column) % 2 == 0 {
                continue;
            }
            let min = rect.min + vec2(column as f32 * cell, row as f32 * cell);
            painter.rect_filled(Rect::from_min_size(min, vec2(cell, cell)).intersect(rect), 0.0, style.checker[1]);
        }
    }
}
