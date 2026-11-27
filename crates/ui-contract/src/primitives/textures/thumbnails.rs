//! `Thumbnails`: the one thumbnail contract. The app supplies pixels at once or as a job, keyed by a key and a
//! revision (`ThumbSource`); the library uploads, stores, evicts, asks for a redraw and paints (`Thumbnails::paint`).

use super::limits::MAX_TEXTURE;
use super::loader::{Cancel, LoadFailure, LoadedPicture, LoaderOptions, Priority, TextureLoader};
use super::store::{CachedTexture, Revision, TextureKey, TextureState, TextureStoreOptions};
use super::words::TextureWords;
use crate::contract::rt::{JobBoard, TextureReads};
use crate::primitives::{ContentPainterCx, HAlign, IconSpec, Surface, TextRun, Wrap};
use crate::style::{Corners, DataColor, SurfaceKind, TypeRole, surface_style};
use craft_behavior::{IconId, PixelSize};
use craft_ui_rt::{BuildCx, RgbaImage, SizeU};
use craft_ui_rt::{Rect, Vec2, pos2, vec2};
use egui::emath::GuiRounding as _;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What a thumbnail is asked at: the side in pixels, the screen's density and the part of the frame wanted.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbRequest {
    /// The longest side of the picture, in pixels: a multiple of 16.
    pub side: u32,
    /// Pixels per point of the screen it shows on.
    pub pixels_per_point: f32,
    /// The part of the frame to render, in fractions of the frame (0 to 1 on both axes), or the whole frame. `side`
    /// is then the longest side of that part: a page larger than the texture side on screen asks for the part in
    /// view and shows its detail. The region is part of the picture's revision, so each region has its own
    /// texture.
    pub region: Option<Rect>,
}

/// Requested sides are multiples of this many pixels.
const SIZE_STEP: u32 = 16;

/// Seconds of waiting after which a `Captioned` thumbnail says it is still rendering.
const STILL_RENDERING: f64 = 6.0;

impl ThumbRequest {
    /// The request for a cell `points` wide on a screen of `pixels_per_point`: the side in pixels rounded up to a
    /// multiple of 16, at least 16 and at most `MAX_TEXTURE`. 28 points at 2x is 64 pixels. Neighbouring cell sizes
    /// share one picture. [`Thumbnails::state`] lowers a side past its [`ThumbnailOptions::max_side`].
    pub fn for_cell(points: f32, pixels_per_point: f32) -> ThumbRequest {
        let pixels = points * pixels_per_point;
        let steps = if pixels.is_finite() { (pixels / SIZE_STEP as f32).ceil().max(1.0) as u32 } else { 1 };
        let side = steps.saturating_mul(SIZE_STEP).min(MAX_TEXTURE / SIZE_STEP * SIZE_STEP);
        ThumbRequest { side, pixels_per_point, region: None }
    }

    /// The request for the part `region` of the frame (fractions of the frame, 0 to 1).
    #[must_use]
    pub fn region(self, region: Rect) -> ThumbRequest {
        ThumbRequest { region: Some(region), ..self }
    }
}

/// The request's region as hashable bits.
fn region_bits(region: Option<Rect>) -> Option<[u32; 4]> {
    region.map(|r| [r.min.x.to_bits(), r.min.y.to_bits(), r.max.x.to_bits(), r.max.y.to_bits()])
}

/// A background render: runs on a board lane, checks the [`Cancel`] between steps, and returns the picture, the
/// answer that the key has none (`LoadedPicture::Blank`, state `Empty`), or the `LoadFailure` the tooltip words. It
/// owns what it reads (an `Arc` of the node, a path), never a borrow of the document.
pub type ThumbJob = Box<dyn FnOnce(&Cancel) -> Result<LoadedPicture, LoadFailure> + Send>;

/// What a [`ThumbSource`] renders for a key.
#[non_exhaustive]
pub enum ThumbRender {
    /// The picture, made now on the UI thread (cheap: a small proxy, a cached image).
    Ready(RgbaImage),
    /// A job that makes it in the background.
    Later(ThumbJob),
    /// The key has no picture (a group row): state `Empty`.
    Blank,
}

/// The app's side of the contract: how a key's picture is versioned and made.
///
/// - `revision` is cheap: a stored fingerprint or pointer identity with a `Weak` guard, never a pixel walk. It covers
///   the pixels only, so a step that changes only the selection keeps it and every thumbnail stays. The request's
///   side is added by [`Thumbnails`]; the revision need not include it.
/// - `render` is called only when the revision has no picture yet, and not in a frame whose image upload budget is
///   spent. The picture has the frame's aspect, longest side at most `request.side`, with no letterbox bars
///   ([`thumb_image`] builds one from a sampler); [`Thumbnails::paint`] fits it to the cell.
/// - Pictures are display-space sRGBA with straight alpha; colour management is the app's and happens before the
///   picture is returned.
pub trait ThumbSource<K> {
    /// The version of `key`'s pixels at every request: a kept view that shows `key` includes it in its version, so a
    /// pixel edit reaches the view without a request.
    fn key_revision(&self, key: &K) -> Revision;
    /// The version of `key`'s picture for `request`; `key_revision` unless the picture depends on the request beyond
    /// its side and region (a source that renders in stages per side).
    fn revision(&self, key: &K, _request: &ThumbRequest) -> Revision {
        self.key_revision(key)
    }
    /// Makes `key`'s picture for `request`.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender;
}

/// A [`Thumbnails`]' settings, read when it is first made.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbnailOptions {
    /// The longest side asked of a source, in pixels: requests past it are lowered to it.
    pub max_side: u32,
    /// Settings of the store.
    pub store: TextureStoreOptions,
}

impl ThumbnailOptions {
    /// Sides up to 512 pixels, thumbnail budgets.
    pub fn new() -> Self {
        Self { max_side: 512, store: TextureStoreOptions::thumbnails() }
    }

    /// The longest side asked of a source.
    #[must_use]
    pub fn max_side(mut self, max_side: u32) -> Self {
        self.max_side = max_side;
        self
    }

    /// Settings of the store.
    #[must_use]
    pub fn store(mut self, store: TextureStoreOptions) -> Self {
        self.store = store;
        self
    }
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// How [`Thumbnails::paint`] fits a picture into its cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ThumbFit {
    /// The whole picture, as large as fits, centred ([`fit_rect`]); the checkerboard is under the picture only, so
    /// the frame's empty sides stay the row's.
    #[default]
    Contain,
    /// The cell filled, the picture cut at its long sides around its centre (a square grid of photos).
    Cover,
}

/// How to paint one thumbnail.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbPaint {
    /// How the picture fits its box.
    pub fit: ThumbFit,
    /// The transparency grid behind the picture (default on). Off for opaque pictures such as photos and pages.
    /// Ignored while `paper` is set.
    pub checker: bool,
    /// The paper the picture sits on, when the document gives one: a fill under the picture's rect in place of the
    /// transparency grid (default none). It wins over `checker`; the two never paint together.
    pub paper: Option<DataColor>,
    /// Multiplies the picture (default untinted): a dimmed row's faint thumbnail.
    pub tint: DataColor,
    /// What a picture that is not ready, or failed, shows: a glyph or a caption (default `Glyph`).
    pub states: StateLook,
}

/// What a thumbnail that is not ready, or failed, shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum StateLook {
    /// The placeholder and the failure glyph.
    #[default]
    Glyph,
    /// "Rendering…", then "Still rendering…" once the wait passes 6 s, then the failure text.
    Captioned,
}

impl Default for ThumbPaint {
    fn default() -> Self {
        Self { fit: ThumbFit::Contain, checker: true, paper: None, tint: DataColor::new(super::UNTINTED), states: StateLook::Glyph }
    }
}

impl ThumbPaint {
    /// Sets how the picture fits its box.
    #[must_use]
    pub fn fit(mut self, fit: ThumbFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets whether the transparency grid shows behind the picture.
    #[must_use]
    pub fn checker(mut self, checker: bool) -> Self {
        self.checker = checker;
        self
    }

    /// Sets the paper under the picture: a fill in place of the transparency grid, whatever `checker` says (a page
    /// thumbnail's paper is document data).
    #[must_use]
    pub fn paper(mut self, paper: DataColor) -> Self {
        self.paper = Some(paper);
        self
    }

    /// Sets the tint that multiplies the picture.
    #[must_use]
    pub fn tint(mut self, tint: DataColor) -> Self {
        self.tint = tint;
        self
    }

    /// Sets what a picture that is not ready, or failed, shows.
    #[must_use]
    pub fn states(mut self, states: StateLook) -> Self {
        self.states = states;
        self
    }
}

/// The thumbnails of one salt in a runtime: a [`TextureLoader`] fed by a [`ThumbSource`].
///
/// - **Reading.** [`Thumbnails::state`] runs in the build of the node that shows the picture. It returns what is
///   there and asks the source only when the revision has no picture and no job; the node runs again when a job
///   answers, and nothing else does.
/// - **Budget.** A source renders on the UI thread only in a frame whose image upload budget has room
///   ([`craft_ui_rt::ImageStore::has_room`]); otherwise the node builds again in the next frame and asks then.
/// - **Holding.** While held ([`Thumbnails::hold`]: a drag, a live resize) nothing is rendered or queued and the
///   stand-ins show.
pub struct Thumbnails<K: TextureKey> {
    loader: TextureLoader<K>,
    options: ThumbnailOptions,
    hold: Arc<AtomicBool>,
}

impl<K: TextureKey> Clone for Thumbnails<K> {
    fn clone(&self) -> Self {
        Self { loader: self.loader.clone(), options: self.options, hold: Arc::clone(&self.hold) }
    }
}

const _: () = {
    const fn send_sync<T: Send + Sync + Clone + 'static>() {}
    send_sync::<Thumbnails<u64>>();
};

impl<K: TextureKey> Thumbnails<K> {
    /// The thumbnails of the building node's runtime under `salt`, made with `options` the first time; later calls
    /// with the same salt get the same thumbnails and ignore `options`.
    pub fn of(cx: &BuildCx<'_>, salt: impl Hash, options: ThumbnailOptions) -> Self {
        let board = cx.provided::<JobBoard>().unwrap_or_else(JobBoard::inline);
        Self::on(&board, salt, options)
    }

    /// The thumbnails of `ctx`'s window under `salt` (see [`Thumbnails::of`]), for app code that holds a `Ui` and no
    /// build: on the job board the host provided above the window root.
    pub fn in_context(ctx: &crate::contract::Context, salt: impl Hash, options: ThumbnailOptions) -> Self {
        Self::on(&ctx.jobs(), salt, options)
    }

    /// The thumbnails under `salt` on `board` (see [`Thumbnails::of`]).
    pub fn on(board: &JobBoard, salt: impl Hash, options: ThumbnailOptions) -> Self {
        let name = craft_ui_rt::hash_key(&(std::any::TypeId::of::<K>(), &salt));
        board.shared(name, || Self {
            loader: TextureLoader::on(board, &salt, LoaderOptions::new().store(options.store)),
            options,
            hold: Arc::new(AtomicBool::new(false)),
        })
    }

    /// What `key` shows for `request`, asking `source` when this revision has no picture and no job; for the node
    /// building now, which runs again when the picture's job answers. A ready render is stored at once.
    pub fn state(
        &self,
        cx: &mut impl TextureReads,
        source: &dyn ThumbSource<K>,
        key: &K,
        request: &ThumbRequest,
        priority: Priority,
    ) -> TextureState {
        let side = request.side.min((self.options.max_side / SIZE_STEP * SIZE_STEP).max(SIZE_STEP));
        let request = ThumbRequest { side, ..*request };
        let revision = Revision::of(&(source.revision(key, &request), side, region_bits(request.region)));
        if let Some(state) = self.loader.ask(cx, key, revision, priority) {
            return caption_wake(cx, state);
        }
        if self.hold.load(Ordering::Relaxed) {
            return self.cache().state(cx, key, revision);
        }
        if !cx.image_room() {
            let now = cx.now();
            cx.with_build(move |cx| cx.request_build_at(Some(now)));
            return self.cache().state(cx, key, revision);
        }
        match source.render(key, &request) {
            ThumbRender::Ready(image) => TextureState::Ready(self.cache().put(cx, key.clone(), revision, image)),
            ThumbRender::Blank => {
                self.cache().put_empty(cx, key.clone(), revision);
                TextureState::Empty
            }
            ThumbRender::Later(job) => {
                let state = self.loader.submit(cx, key.clone(), revision, priority, job);
                caption_wake(cx, state)
            }
        }
    }

    /// Paints `state` into `rect`: the picture fitted per `paint`, the placeholder while nothing shows, the failure
    /// glyph or caption. Returns false for `Empty`, which paints nothing.
    pub fn paint(&self, cx: &ContentPainterCx<'_>, rect: Rect, state: &TextureState, paint: &ThumbPaint) -> bool {
        let rect = rect.round_to_pixels(cx.pixels_per_point());
        match state {
            TextureState::Empty => false,
            TextureState::Ready(texture) | TextureState::Loading { stand_in: Some(texture), .. } => {
                paint_picture(cx, rect, texture, paint);
                true
            }
            TextureState::Missing | TextureState::Loading { stand_in: None, .. } => {
                paint_placeholder(cx, rect, paint.paper);
                if paint.states == StateLook::Captioned {
                    let waited = if let TextureState::Loading { waited, .. } = state { *waited } else { None };
                    paint_waiting(cx, rect, waited);
                }
                true
            }
            TextureState::Failed { failure } => {
                paint_placeholder(cx, rect, paint.paper);
                match paint.states {
                    StateLook::Glyph => paint_failure(cx, rect),
                    StateLook::Captioned => paint_caption(cx, rect, failure.text(&cx.words::<TextureWords>())),
                }
                true
            }
        }
    }

    /// Holds rendering: while `hold` is true nothing is rendered or queued and the stand-ins show. Releasing it
    /// does not run anything: the next build that reads a thumbnail asks again.
    pub fn hold(&self, hold: bool) {
        self.hold.store(hold, Ordering::Relaxed);
    }

    /// Keeps only the keys `keep` accepts; dropped keys' jobs are cancelled.
    pub fn retain(&self, cx: &mut BuildCx<'_>, keep: impl FnMut(&K) -> bool) {
        self.cache().retain(cx, keep);
    }

    /// The store the thumbnails live in.
    pub fn cache(&self) -> &super::TextureStore<K> {
        self.loader.cache()
    }
}

/// The rect inside `cell` that shows a picture of `picture`'s aspect whole, as large as fits and centred; `cell`
/// itself for a zero-sized picture.
pub fn fit_rect(cell: Rect, picture: Vec2) -> Rect {
    if !(picture.x > 0.0 && picture.y > 0.0) {
        return cell;
    }
    let scale = (cell.width() / picture.x).min(cell.height() / picture.y);
    Rect::from_center_size(cell.center(), picture * scale)
}

/// A picture of the document area `frame` (pixels) with that exact aspect, its longest side `longest_side`, each
/// side at least 1: texel `(tx, ty)` is `sample(x, y)` at the document pixel under the texel's centre, `x` and `y`
/// counted from the frame's top left (the caller adds the frame's origin). `sample` returns sRGBA, unmultiplied. A
/// frame smaller than the side is sampled nearest, larger ones point-sampled at texel centres.
pub fn thumb_image(frame: PixelSize, longest_side: u32, mut sample: impl FnMut(u32, u32) -> [u8; 4]) -> RgbaImage {
    let PixelSize { width, height, .. } = thumb_size(frame, longest_side);
    let (frame_width, frame_height) = (u64::from(frame.width.max(1)), u64::from(frame.height.max(1)));
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for ty in 0..height {
        let y = ((2 * u64::from(ty) + 1) * frame_height / (2 * u64::from(height))).min(frame_height - 1) as u32;
        for tx in 0..width {
            let x = ((2 * u64::from(tx) + 1) * frame_width / (2 * u64::from(width))).min(frame_width - 1) as u32;
            pixels.extend_from_slice(&sample(x, y));
        }
    }
    RgbaImage { size: SizeU::new(width, height), pixels }
}

/// The pixel size of a picture of `frame`'s aspect with its longest side `longest_side`.
fn thumb_size(frame: PixelSize, longest_side: u32) -> PixelSize {
    let (frame_width, frame_height) = (f64::from(frame.width.max(1)), f64::from(frame.height.max(1)));
    let scale = f64::from(longest_side.max(1)) / frame_width.max(frame_height);
    PixelSize::new((frame_width * scale).round().max(1.0) as u32, (frame_height * scale).round().max(1.0) as u32)
}

/// Draws the transparency grid and the picture for `paint.fit`.
fn paint_picture(cx: &ContentPainterCx<'_>, cell: Rect, texture: &CachedTexture, paint: &ThumbPaint) {
    let size = texture.size_vec2();
    let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let (target, fit) = match paint.fit {
        ThumbFit::Contain => (fit_rect(cell, size).round_to_pixels(cx.pixels_per_point()), full),
        ThumbFit::Cover => (cell, cover_uv(cell, size)),
    };
    if let Some(paper) = paint.paper {
        cx.painter.rect_filled(target, egui::CornerRadius::ZERO, paper.0);
    } else if paint.checker {
        cx.checker(target);
    }
    cx.painter.image(texture.key, target, fit, paint.tint.0);
}

/// The part of a picture a `Cover` fit shows in `cell`: centred, cut at the long sides.
pub(crate) fn cover_uv(cell: Rect, picture: Vec2) -> Rect {
    if !(picture.x > 0.0 && picture.y > 0.0 && cell.width() > 0.0 && cell.height() > 0.0) {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let scale = (cell.width() / picture.x).max(cell.height() / picture.y);
    let shown = vec2(cell.width() / (picture.x * scale), cell.height() / (picture.y * scale));
    Rect::from_center_size(pos2(0.5, 0.5), shown)
}

/// The cell while no picture has arrived, and behind the failure icon: a sunken surface, in the caller's `paper`
/// when it gives one.
fn paint_placeholder(cx: &ContentPainterCx<'_>, rect: Rect, paper: Option<DataColor>) {
    let mut style = surface_style(cx.tokens, SurfaceKind::Sunken);
    if let Some(paper) = paper {
        style.fill = paper.0;
    }
    cx.painter.extend(Surface::items(&style, None, rect, Corners::None, cx.direction, cx.pixels_per_point()));
}

/// A picture with no stand-in that is still loading captions "Still rendering…" once it has waited [`STILL_RENDERING`]
/// seconds: its node builds again then, so no frame runs just to find out.
fn caption_wake(cx: &mut impl TextureReads, state: TextureState) -> TextureState {
    if let TextureState::Loading { stand_in: None, waited: Some(waited), .. } = &state
        && *waited < STILL_RENDERING
    {
        let at = cx.now() + (STILL_RENDERING - waited);
        cx.with_build(move |cx| cx.request_build_at(Some(at)));
    }
    state
}

/// The caption of a picture that has not arrived: "Rendering…", then "Still rendering…" once it has waited
/// [`STILL_RENDERING`] seconds, read when the node paints.
fn paint_waiting(cx: &ContentPainterCx<'_>, rect: Rect, waited: Option<f64>) {
    let words = cx.words::<TextureWords>();
    let text = match waited {
        Some(waited) if waited >= STILL_RENDERING => &words.still_rendering,
        _ => &words.rendering,
    };
    paint_caption(cx, rect, text.as_str());
}

/// `text` in the body role, centred below `rect`'s centre and wrapped to `rect` less the roomy padding on each side.
fn paint_caption(cx: &ContentPainterCx<'_>, rect: Rect, text: &str) {
    let room = rect.shrink(cx.tokens.space.pad.roomy);
    let below = Rect::from_min_max(pos2(room.min.x, rect.center().y), room.max);
    if !below.is_positive() {
        return;
    }
    let run = TextRun::new(text).role(TypeRole::Body).wrap(Wrap::Wrap).align(HAlign::Center);
    let _ = cx.with_draw(|draw| draw.text(below, &run));
}

/// The warning icon centred in `rect`, no larger than the small icon size.
fn paint_failure(cx: &ContentPainterCx<'_>, rect: Rect) {
    let side = cx.tokens.size.icon.sm.min(rect.width().min(rect.height()));
    let icon = Rect::from_center_size(rect.center(), vec2(side, side));
    crate::primitives::icon::paint(&cx.painter, cx.tokens, cx.direction, icon, &IconSpec::svg(IconId::new("warning")), false, false);
}

/// A [`ThumbSource`] for stories and tests in every crate: pictures made from the key and a revision the test sets,
/// so every state shows without a document.
///
/// - The picture: a diagonal gradient between two colours whose hues come from the key's hash (start hue
///   `hash % 360`, end hue 120 degrees on), fully opaque except a transparent top-left quarter (so the checkerboard
///   shows), with `revision % 8` white bars of one texel along the bottom edge; [`aspect`](Self::aspect) shapes the
///   frame and [`opaque`](Self::opaque) fills the quarter, as a story's sample photos are.
/// - [`later`](Self::later)`(true)` renders as `Later` jobs, so with a `ManualJobs` in [`ThumbnailOptions::jobs`]
///   pictures arrive one per `run_next`.
/// - Keys given to [`failing`](Self::failing) fail with the message "Fake failure" (always as a `Later` job, since a
///   synchronous render cannot fail); keys given to [`blank`](Self::blank) render `Blank`.
/// - [`set_revision`](Self::set_revision) and [`bump`](Self::bump) change every key's revision, so stand-ins show.
#[cfg(feature = "testing")]
pub struct FakeThumbSource<K> {
    revision: u64,
    later: bool,
    opaque: bool,
    aspect: PixelSize,
    failing: Vec<K>,
    blank: Vec<K>,
}

#[cfg(feature = "testing")]
impl<K: std::hash::Hash + PartialEq + Clone + Send + 'static> FakeThumbSource<K> {
    /// Synchronous square pictures at revision 0.
    pub fn new() -> Self {
        Self { revision: 0, later: false, opaque: false, aspect: PixelSize::new(1, 1), failing: Vec::new(), blank: Vec::new() }
    }

    /// Renders as `Later` jobs.
    #[must_use]
    pub fn later(mut self, later: bool) -> Self {
        self.later = later;
        self
    }

    /// The picture has no transparent quarter.
    #[must_use]
    pub fn opaque(mut self, opaque: bool) -> Self {
        self.opaque = opaque;
        self
    }

    /// The frame's width to height (4:3 landscape, 3:4 portrait).
    #[must_use]
    pub fn aspect(mut self, width: u32, height: u32) -> Self {
        self.aspect = PixelSize::new(width.max(1), height.max(1));
        self
    }

    /// These keys fail.
    #[must_use]
    pub fn failing(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        self.failing.extend(keys);
        self
    }

    /// These keys have no picture.
    #[must_use]
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

#[cfg(feature = "testing")]
impl<K: std::hash::Hash + PartialEq + Clone + Send + 'static> Default for FakeThumbSource<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "testing")]
impl<K: std::hash::Hash + PartialEq + Clone + Send + 'static> ThumbSource<K> for FakeThumbSource<K> {
    /// `Revision::of(&(key, revision))`.
    fn key_revision(&self, key: &K) -> Revision {
        Revision::of(&(key, self.revision))
    }

    /// The picture described on [`FakeThumbSource`], at `thumb_image(aspect scaled, request.side, ..)`.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender {
        if self.blank.contains(key) {
            return ThumbRender::Blank;
        }
        if self.failing.contains(key) {
            return ThumbRender::Later(Box::new(|_| Err(LoadFailure::Message("Fake failure".to_owned()))));
        }
        let frame = thumb_size(self.aspect, request.side);
        let image = fake_picture(frame, fake_hue(key), self.revision, self.opaque);
        if self.later { ThumbRender::Later(Box::new(move |_| Ok(LoadedPicture::Image(image)))) } else { ThumbRender::Ready(image) }
    }
}

/// Hue in degrees from the key's hash.
#[cfg(feature = "testing")]
fn fake_hue(key: &impl std::hash::Hash) -> f32 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() % 360) as f32
}

/// The opaque picture of a [`FakeThumbSource`] for `key` at revision 0, `frame` pixels: a sample picture for a
/// component that takes an image rather than a source.
#[cfg(feature = "testing")]
pub(crate) fn fake_thumb_image(key: &impl std::hash::Hash, frame: PixelSize) -> RgbaImage {
    fake_picture(frame, fake_hue(key), 0, true)
}

/// The fake source's picture: a gradient from `hue` to `hue + 120` degrees, a transparent top left quarter unless
/// `opaque`, and `revision % 8` white texels along the bottom row. Every test context renders its pictures anew, so
/// each picture is made once per process.
#[cfg(feature = "testing")]
fn fake_picture(frame: PixelSize, hue: f32, revision: u64, opaque: bool) -> RgbaImage {
    use std::collections::HashMap;
    use std::sync::{Mutex, PoisonError};
    type Key = (u32, u32, u32, u32, bool);
    static MADE: Mutex<Option<HashMap<Key, RgbaImage>>> = Mutex::new(None);
    let bars = (revision % 8) as u32;
    let key = (frame.width, frame.height, hue.to_bits(), bars, opaque);
    let mut made = MADE.lock().unwrap_or_else(PoisonError::into_inner);
    made.get_or_insert_default().entry(key).or_insert_with(|| make_fake_picture(frame, hue, bars, opaque)).clone()
}

#[cfg(feature = "testing")]
fn make_fake_picture(frame: PixelSize, hue: f32, bars: u32, opaque: bool) -> RgbaImage {
    let PixelSize { width, height, .. } = frame;
    let end = |degrees: f32| egui::ecolor::Hsva::new((degrees % 360.0) / 360.0, 0.6, 0.85, 1.0).to_srgba_unmultiplied();
    let (from, to) = (end(hue), end(hue + 120.0));
    thumb_image(frame, width.max(height), |x, y| {
        if y + 1 == height && x % 2 == 0 && x / 2 < bars {
            return [255, 255, 255, 255];
        }
        if !opaque && x < width / 2 && y < height / 2 {
            return [0, 0, 0, 0];
        }
        let t = ((x as f32 + 0.5) / width as f32 + (y as f32 + 0.5) / height as f32) / 2.0;
        let mix = |i: usize| (f32::from(from[i]) + (f32::from(to[i]) - f32::from(from[i])) * t).round() as u8;
        [mix(0), mix(1), mix(2), 255]
    })
}
