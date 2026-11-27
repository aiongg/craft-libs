//! `TextureStore<K>`: keyed pictures with revisions and a byte and entry budget, a thin wrapper over the runtime's
//! [`TextureCache`]. Uploads, atlases and the per-frame upload budget belong to the image store and the renderers.

use super::loader::LoadFailure;
use crate::contract::rt::{JobBoard, TextureCache, TextureLook, TextureReads};
use craft_behavior::PixelSize;
use craft_ui_rt::{BuildCx, ImageKey, ImageOptions, RgbaImage};
use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// What can key a stored texture: a layer id, a path, a `(document, page)` pair. Every type with these bounds is
/// one.
pub trait TextureKey: Clone + Eq + Hash + Send + Sync + 'static {}

impl<T: Clone + Eq + Hash + Send + Sync + 'static> TextureKey for T {}

/// The version of a key's picture, computed by the app: a stored fingerprint, an `Arc` address with a `Weak` guard,
/// a frame key. The library never reads pixels or compares images; a new revision means "this key's picture
/// changed", the same revision means "keep it".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, crate::contract::MemoProps)]
pub struct Revision(u64);

impl Revision {
    /// The revision `value` (a document's own change counter).
    pub const fn new(value: u64) -> Revision {
        Revision(value)
    }

    /// The number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The revision of anything hashable: `Revision::of(&(fingerprint, doc_width))`. Stable within one run of the
    /// app (std's `DefaultHasher` with its fixed keys), not across builds, so it is never stored.
    pub fn of(value: &impl Hash) -> Revision {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Revision(hasher.finish())
    }
}

/// How much a store may hold. Entries past `max_bytes` or `max_entries` are evicted, least recently read first,
/// never one read in this frame or the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextureBudget {
    /// The texture bytes held, 4 per texel.
    pub max_bytes: usize,
    /// The entries held, pictures, empty and failed ones alike.
    pub max_entries: usize,
}

const MIB: usize = 1024 * 1024;

impl TextureBudget {
    /// Holds `max_bytes` and `max_entries`.
    pub const fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self { max_bytes, max_entries }
    }
}

/// A store's settings, read when the store is first made (`TextureStore::of`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextureStoreOptions {
    /// The store's size limits.
    pub budget: TextureBudget,
    /// How every picture is sampled: linear by default; the clipping overlay and pixel-exact previews sample the
    /// nearest texel.
    pub filter: ImageOptions,
}

impl TextureStoreOptions {
    /// Row and grid thumbnails: 32 MiB and 512 entries.
    pub const fn thumbnails() -> Self {
        Self::with_budget(32 * MIB, 512)
    }

    /// Document pages: 128 MiB and 64 entries.
    pub const fn pages() -> Self {
        Self::with_budget(128 * MIB, 64)
    }

    /// Video frames and other named images: 64 MiB and 512 entries.
    pub const fn frames() -> Self {
        Self::with_budget(64 * MIB, 512)
    }

    const fn with_budget(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            budget: TextureBudget::new(max_bytes, max_entries),
            filter: ImageOptions {
                magnify: craft_ui_rt::Filter::Linear,
                minify: craft_ui_rt::Filter::Linear,
                mipmaps: false,
                wrap: craft_ui_rt::Wrap::Clamp,
            },
        }
    }

    /// How every picture is sampled.
    #[must_use]
    pub const fn filter(mut self, filter: ImageOptions) -> Self {
        self.filter = filter;
        self
    }

    /// Other limits.
    #[must_use]
    pub const fn budget(mut self, budget: TextureBudget) -> Self {
        self.budget = budget;
        self
    }
}

impl Default for TextureStoreOptions {
    /// `thumbnails`.
    fn default() -> Self {
        Self::thumbnails()
    }
}

/// A stored picture as a painter draws it: `draw.texture(rect, texture.key, uv)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CachedTexture {
    /// The picture in the image store.
    pub key: ImageKey,
    /// Its size in texels (reduced when the image was past the device limit).
    pub size: PixelSize,
}

impl CachedTexture {
    /// The picture `key` of `size` texels.
    pub const fn new(key: ImageKey, size: PixelSize) -> Self {
        Self { key, size }
    }

    /// `size` as a vector, for fitting: only the aspect matters to a cell.
    pub fn size_vec2(&self) -> craft_ui_rt::Vec2 {
        craft_ui_rt::Vec2::new(self.size.width as f32, self.size.height as f32)
    }
}

/// What a key shows for a revision, from `TextureStore::state`: the cache's [`TextureLook`] with how long a loading
/// picture has waited.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TextureState {
    /// Nothing is stored for this key, or only an empty or failed entry of another revision.
    Missing,
    /// The picture for this revision is on its way.
    #[non_exhaustive]
    Loading {
        /// The key's picture of another revision, shown until the new one lands.
        stand_in: Option<CachedTexture>,
        /// Seconds since the key's picture was first read after its request, by the frame clock.
        waited: Option<f64>,
    },
    /// The picture for this revision.
    Ready(CachedTexture),
    /// This revision has no picture (a group row): nothing is drawn and nothing is asked again.
    Empty,
    /// Making this revision's picture failed; it is not asked again until the revision changes.
    #[non_exhaustive]
    Failed {
        /// Why; the painter words it with `LoadFailure::text` at run time.
        failure: LoadFailure,
    },
}

impl TextureState {
    /// The state of `look`, loading for `waited` seconds.
    pub fn of(look: TextureLook, waited: Option<f64>) -> TextureState {
        match look {
            TextureLook::Loading { stand_in } => TextureState::Loading { stand_in, waited },
            TextureLook::Ready(texture) => TextureState::Ready(texture),
            TextureLook::Empty => TextureState::Empty,
            TextureLook::Failed(failure) => TextureState::Failed { failure },
            _ => TextureState::Missing,
        }
    }

    /// The picture to draw now: the ready one, or the stand-in while loading.
    pub fn shown(&self) -> Option<CachedTexture> {
        match self {
            TextureState::Ready(texture) | TextureState::Loading { stand_in: Some(texture), .. } => Some(*texture),
            _ => None,
        }
    }
}

/// What a store holds and did, for automation, stories and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextureStatistics {
    /// Entries held: pictures, empty and failed ones.
    pub entries: usize,
    /// Texture bytes held.
    pub bytes: usize,
    /// Entries evicted since the store was made.
    pub evicted: usize,
}

impl std::ops::Add for TextureStatistics {
    type Output = TextureStatistics;

    /// The field-wise sum, for the statistics of several stores taken together.
    fn add(self, other: TextureStatistics) -> TextureStatistics {
        TextureStatistics { entries: self.entries + other.entries, bytes: self.bytes + other.bytes, evicted: self.evicted + other.evicted }
    }
}

/// The numbers that keep apart the textures one owner makes: a mask or preview made anew gets a number no earlier one
/// of the same `Serials` had, so an entry kept under its key is never taken for the new one's. The owner of what the
/// textures show (a tool's state, a model) holds the `Serials`. The numbers are unique per owner, not per store: a
/// texture store that several owners share keys each entry by the owner's component id as well as the serial. Plain
/// data, so it can sit in tracked or model state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Serials(u64);

impl Serials {
    /// A number this `Serials` never gave before.
    pub fn mint(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

/// Keyed pictures with revisions and a budget: the runtime's [`TextureCache`] named by a salt, so every part of the
/// app that names the same salt and key type shares the pictures.
pub struct TextureStore<K: TextureKey> {
    cache: TextureCache<K>,
    options: TextureStoreOptions,
}

impl<K: TextureKey> Clone for TextureStore<K> {
    fn clone(&self) -> Self {
        Self { cache: self.cache.clone(), options: self.options }
    }
}

impl<K: TextureKey> TextureStore<K> {
    /// The store named `salt` of the runtime the building node is in, made with `options` the first time; later
    /// calls with the same salt and key type get the same store and ignore their options.
    pub fn of(cx: &BuildCx<'_>, salt: impl Hash, options: TextureStoreOptions) -> Self {
        let board = cx.provided::<JobBoard>().unwrap_or_else(JobBoard::inline);
        Self::on(&board, salt, options)
    }

    /// The store named `salt` of `ctx`'s window (see [`TextureStore::of`]), for code that holds a `Ui`, a `Cx` or a
    /// `MeasureCx` and no build: on the job board the host provided above the window root.
    pub fn in_context(ctx: &crate::contract::Context, salt: impl Hash, options: TextureStoreOptions) -> Self {
        Self::on(&ctx.jobs(), salt, options)
    }

    /// The store named `salt` on `board` (see [`TextureStore::of`]).
    pub fn on(board: &JobBoard, salt: impl Hash, options: TextureStoreOptions) -> Self {
        let salt = craft_ui_rt::hash_key(&(TypeId::of::<K>(), salt));
        board.shared(salt, || Self { cache: TextureCache::new(salt, options.budget, board.clone()).options(options.filter), options })
    }

    /// The cache the store wraps.
    pub fn cache(&self) -> &TextureCache<K> {
        &self.cache
    }

    /// What `key` shows for `revision` (see `TextureState`), for the node building now, which runs again when a job
    /// answers for `key`.
    /// - The entry has `revision`: `Ready`, `Empty`, `Failed` or `Loading`.
    /// - The entry has a picture of another revision: `Loading { stand_in: Some(..) }`.
    /// - No entry, or an empty or failed one of another revision: `Missing`.
    pub fn state(&self, cx: &mut impl TextureReads, key: &K, revision: Revision) -> TextureState {
        let look = self.cache.read(cx, key, revision);
        TextureState::of(look, self.cache.waited(key, cx.now()))
    }

    /// Stores `image` for `key` at `revision`, replacing the key's picture.
    pub fn put(&self, cx: &mut impl TextureReads, key: K, revision: Revision, image: RgbaImage) -> CachedTexture {
        self.cache.put(cx, key, revision, image)
    }

    /// `key`'s picture at `revision`, calling `make` and putting its image only when the entry has another revision
    /// or none.
    pub fn get_or_put(&self, cx: &mut impl TextureReads, key: K, revision: Revision, make: impl FnOnce() -> RgbaImage) -> CachedTexture {
        match self.cache.read(cx, &key, revision) {
            TextureLook::Ready(texture) => texture,
            _ => self.cache.put(cx, key, revision, make()),
        }
    }

    /// Records that `key` has no picture at `revision` (state `Empty`); frees its picture.
    pub fn put_empty(&self, cx: &mut impl TextureReads, key: K, revision: Revision) {
        self.cache.put_empty(cx, key, revision);
    }

    /// Records that making `key`'s picture at `revision` failed (state `Failed`).
    pub fn put_failure(&self, cx: &mut impl TextureReads, key: K, revision: Revision, failure: LoadFailure) {
        self.cache.put_failure(cx, key, revision, failure);
    }

    /// Drops `key`'s entry and frees its picture.
    pub fn forget(&self, cx: &mut BuildCx<'_>, key: &K) {
        self.cache.forget(cx, key);
    }

    /// Keeps only the entries whose key `keep` accepts (PhotoCraft's `prune_thumbs` when the open documents change).
    pub fn retain(&self, cx: &mut BuildCx<'_>, keep: impl FnMut(&K) -> bool) {
        self.cache.retain(cx, keep);
    }

    /// Drops every entry.
    pub fn clear(&self, cx: &mut BuildCx<'_>) {
        self.cache.clear(cx);
    }

    /// What the store holds and evicted.
    pub fn statistics(&self) -> TextureStatistics {
        self.cache.statistics()
    }

    /// The options the store was made with.
    pub fn options(&self) -> TextureStoreOptions {
        self.options
    }
}
