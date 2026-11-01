//! [`TextureCache`]: keyed textures with revisions, budgets and eviction, living in the egui
//! context.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use egui::{AsIdSalt, ColorImage, Context, TextureHandle, TextureId, TextureOptions, Vec2};

/// What can key a cached texture: a layer id, a path, a `(document, page)` pair. Every type with
/// these bounds is one.
pub trait TextureKey: Clone + Eq + Hash + Send + Sync + 'static {}

impl<T: Clone + Eq + Hash + Send + Sync + 'static> TextureKey for T {}

/// The version of a key's picture, computed by the app: a stored fingerprint, an `Arc` address
/// with a `Weak` guard, a frame key. The library never reads pixels or compares images; a new
/// revision means "this key's picture changed", the same revision means "keep it".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Revision(pub u64);

impl Revision {
    /// The revision of anything hashable: `Revision::of(&(fingerprint, doc_width))`. Stable within
    /// one run of the app (std's `DefaultHasher` with its fixed keys), not across builds, so it is
    /// never stored.
    pub fn of(value: &impl Hash) -> Revision {
        // pc: crates/ui-egui/src/app/thumbs.rs:138 `surface_fingerprint` (a hash of the content's identity).
        todo!()
    }
}

/// How much a cache may hold and upload. Entries past `max_bytes` or `max_entries` are evicted;
/// `uploads_per_pass` and `upload_bytes_per_pass` are the per-pass allowance a
/// [`TextureLoader`](super::TextureLoader) keeps to ([`TextureCache::allow_upload`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureBudget {
    /// The texture bytes held, 4 per texel.
    pub max_bytes: usize,
    /// The entries held, pictures, empty and failed ones alike.
    pub max_entries: usize,
    /// Uploads a loader makes per pass.
    pub uploads_per_pass: usize,
    /// Bytes a loader uploads per pass.
    pub upload_bytes_per_pass: usize,
}

const MIB: usize = 1024 * 1024;

/// A cache's settings, read when the cache is created ([`TextureCache::in_context`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureCacheOptions {
    pub budget: TextureBudget,
    /// The sampling of every texture: `TextureOptions::LINEAR` by default; the clipping overlay
    /// and pixel-exact previews use `TextureOptions::NEAREST`.
    pub filter: TextureOptions,
}

impl TextureCacheOptions {
    /// Row and grid thumbnails: 32 MiB and 512 entries; 16 uploads and 16 MiB per pass.
    // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:13 (`TEXTURE_BUDGET` 32 MB);
    // vectorcraft: crates/ui-egui/src/panels/layers/thumbs.rs:66 (`MAX_THUMBS` 512);
    // pdfcraft: crates/ui-egui/src/canvas/cache.rs:46 (`UPLOAD_BYTES_PER_FRAME` 16 MB).
    pub fn thumbnails() -> Self {
        Self::with_budget(32 * MIB, 512)
    }

    /// Document pages: 128 MiB and 64 entries; 16 uploads and 16 MiB per pass.
    // pdfcraft: crates/ui-egui/src/canvas/cache.rs:42 (`PAGE_BYTES` 128 MB).
    pub fn pages() -> Self {
        Self::with_budget(128 * MIB, 64)
    }

    /// Video frames and other named images: 64 MiB and 512 entries; 16 uploads and 16 MiB per
    /// pass.
    // filmcraft: crates/ui-egui/src/app/textures.rs:124-136 (`MAX_THUMB_TEXTURES` 512).
    pub fn frames() -> Self {
        Self::with_budget(64 * MIB, 512)
    }

    fn with_budget(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            budget: TextureBudget { max_bytes, max_entries, uploads_per_pass: 16, upload_bytes_per_pass: 16 * MIB },
            filter: TextureOptions::LINEAR,
        }
    }

    /// The sampling of every texture.
    pub fn filter(mut self, filter: TextureOptions) -> Self {
        self.filter = filter;
        self
    }

    /// Other limits.
    pub fn budget(mut self, budget: TextureBudget) -> Self {
        self.budget = budget;
        self
    }
}

impl Default for TextureCacheOptions {
    /// [`thumbnails`](TextureCacheOptions::thumbnails).
    fn default() -> Self {
        Self::thumbnails()
    }
}

/// A cached picture as a painter draws it: `painter.image(texture.id, rect, uv, tint)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedTexture {
    pub id: TextureId,
    /// The texture's size in texels (reduced when the image was past the device limit).
    pub size: [usize; 2],
}

impl CachedTexture {
    /// [`size`](Self::size) as a vector, for fitting: only the aspect matters to a cell.
    pub fn size_vec2(&self) -> Vec2 {
        egui::vec2(self.size[0] as f32, self.size[1] as f32)
    }
}

/// What a key shows for a revision, from [`TextureCache::state`].
#[derive(Clone, Debug, PartialEq)]
pub enum TextureState {
    /// Nothing is cached for this key, or only an empty or failed entry of another revision.
    Missing,
    /// The picture for this revision is on its way. `stand_in` is the key's picture of another
    /// revision, shown until the new one lands.
    Loading { stand_in: Option<CachedTexture> },
    /// The picture for this revision.
    Ready(CachedTexture),
    /// This revision has no picture (a group row): nothing is drawn and nothing is asked again.
    Empty,
    /// Making this revision's picture failed; it is not asked again until the revision changes.
    /// `message` is for the host's tooltip.
    Failed { message: String },
}

impl TextureState {
    /// The picture to draw now: the ready one, or the stand-in while loading.
    pub fn shown(&self) -> Option<CachedTexture> {
        match self {
            TextureState::Ready(texture) | TextureState::Loading { stand_in: Some(texture) } => Some(*texture),
            _ => None,
        }
    }
}

/// What a cache holds and did, for automation, stories and tests (PdfCraft's `RasterMemory`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextureStatistics {
    /// Entries held: pictures, empty and failed ones.
    pub entries: usize,
    /// Texture bytes held.
    pub bytes: usize,
    /// Uploads this pass, and their bytes.
    pub uploads: usize,
    pub upload_bytes: usize,
    /// Entries evicted since the cache was made.
    pub evicted: usize,
}

/// One key's entry.
enum Picture {
    Texture {
        handle: TextureHandle,
        /// The size of the image put, before any reduction.
        source: [usize; 2],
        /// The whole factor `fit_image` reduced it by: 1 when it fitted.
        factor: u32,
    },
    Empty,
    Failed(String),
}

struct Entry {
    revision: Revision,
    picture: Picture,
    /// The last pass `state`, `put` or a paint saw it (`Context::cumulative_pass_nr`).
    seen: u64,
}

struct Inner<K> {
    options: TextureCacheOptions,
    entries: HashMap<K, Entry>,
    bytes: usize,
    /// The pass the upload counters are for, and the counters.
    upload_pass: u64,
    uploads: usize,
    upload_bytes: usize,
    evicted: usize,
    /// Debug names of the egui textures (`"<id_salt>/<n>"`).
    name: String,
    next_name: u64,
}

/// Textures by key and revision, with byte and entry budgets: the one place an app's pictures
/// become egui textures. The app never holds a `TextureHandle`.
///
/// ```ignore
/// let cache = TextureCache::<String>::in_context(ctx, "frames", TextureCacheOptions::frames());
/// let shown = cache.put(ctx, name.clone(), Revision::of(&frame_key), image);          // upload or replace
/// let pattern = cache.get_or_put(ctx, id, Revision::of(&stamp), || render_pattern(id)); // made once per revision
/// painter.image(shown.id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
/// ```
///
/// - **Where it lives.** In the egui context under `id_salt` ([`in_context`](Self::in_context));
///   the value is a cheap handle that clones share. Textures belong to a context, so a new context
///   starts empty and a reset of its memory frees every texture (VectorCraft's epoch workaround is
///   not needed).
/// - **Revisions.** [`state`](Self::state) answers for a key and the revision the app computes
///   now. A new revision keeps the old picture as the stand-in until the new one is put.
/// - **Upload.** [`put`](Self::put) replaces an existing texture in place
///   (`TextureHandle::set`, same `TextureId`, so painters holding the id last pass keep working)
///   and uploads a new one otherwise. An image past the device limit
///   ([`texture_side`](super::texture_side)) is reduced first ([`fit_image`](super::fit_image)).
/// - **Eviction**, after every put: least recently seen first, until both the bytes and the entry
///   count are within the budget, but never an entry seen in this pass or the previous one; so
///   what is in view stays even past the budget.
/// - **One texture per entry**, no atlas. [`CachedTexture`] is opaque enough that small pictures
///   could share an atlas page later without changing callers (WordCraft's DirectX 12 problem with
///   hundreds of freed small textures, #1002; in-place `set` avoids most frees).
///
/// `TextureCache<K>` is `Clone + Send + Sync`, as egui memory requires.
// pc: crates/ui-egui/src/app/thumbs.rs:118 `store_thumb` (in-place `set` on a revision change);
// filmcraft: crates/ui-egui/src/app/textures.rs:124-136 (LRU by pass, never one drawn this pass);
// deckcraft: crates/ui-egui/src/textures.rs:400-403 (count cap 600, trim the 200 least recently stamped).
pub struct TextureCache<K: TextureKey> {
    inner: Arc<Mutex<Inner<K>>>,
}

impl<K: TextureKey> Clone for TextureCache<K> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

const _: () = {
    const fn send_sync<T: Send + Sync + Clone + 'static>() {}
    send_sync::<TextureCache<u64>>();
};

impl<K: TextureKey> TextureCache<K> {
    /// The cache stored in `ctx` under `id_salt`, made with `options` the first time. Later calls
    /// with the same salt get the same cache and ignore `options`.
    pub fn in_context(ctx: &Context, id_salt: impl AsIdSalt, options: TextureCacheOptions) -> Self {
        // new: egui memory under `Id::new(("craft_ui::canvas::TextureCache", id_salt))`, via
        // `get_temp_mut_or_insert_with`; the debug name of each texture is the salt and a counter.
        todo!()
    }

    /// What `key` shows for `revision` (see [`TextureState`]), and marks the entry seen this pass.
    /// - The entry has `revision`: `Ready`, `Empty` or `Failed`.
    /// - The entry has a picture of another revision: `Loading { stand_in: Some(..) }`.
    /// - No entry, or an empty or failed one of another revision: `Missing`.
    pub fn state(&self, ctx: &Context, key: &K, revision: Revision) -> TextureState {
        // pc: crates/ui-egui/src/app/thumbs.rs:118-134 (a hit at the same revision is reused).
        todo!()
    }

    /// Stores `image` as `key`'s picture at `revision` and returns it: in place when the key has a
    /// texture (`TextureHandle::set`, same id), else a new texture with the cache's filter. Reduced
    /// first when past [`texture_side`](super::texture_side). Counts as an upload this pass, marks
    /// the entry seen, then evicts (see the type's doc). The image is sRGBA as egui's `ColorImage`
    /// holds it (built from unmultiplied values with `ColorImage::from_rgba_unmultiplied`).
    pub fn put(&self, ctx: &Context, key: K, revision: Revision, image: ColorImage) -> CachedTexture {
        // pc: crates/ui-egui/src/app/thumbs.rs:118 `store_thumb`; deckcraft: crates/ui-egui/src/textures.rs:174 `shrink_to`.
        todo!()
    }

    /// Updates part of `key`'s picture in place and moves the entry to `revision`: `image` goes at
    /// `origin`, both in the pixels of the image last [`put`](Self::put) (the navigator's damage
    /// updates). Uses `TextureHandle::set_partial`, so the id stays. Returns `None` and changes
    /// nothing when the key has no picture, or when the region does not fit the stored image.
    ///
    /// An image that was reduced by the whole factor `f` on put is updated reduced the same way:
    /// the region's origin must be a multiple of `f`, and its far edges multiples of `f` or the
    /// image's own edges; it is box-averaged by `f` and written at `origin / f`. A region off that
    /// grid returns `None`, and the app puts the whole image.
    pub fn put_region(&self, ctx: &Context, key: &K, revision: Revision, origin: [usize; 2], image: ColorImage) -> Option<CachedTexture> {
        // new: `TextureHandle::set_partial` (epaint/src/texture_handle.rs:78) with the reduction of `fit_image`.
        todo!()
    }

    /// `key`'s picture at `revision`, calling `make` and putting its image only when the entry
    /// has another revision or none (PhotoCraft's `cached_texture`, a preset's computed
    /// thumbnail, WordCraft's `Previews::get_or`). An `Empty` or `Failed` entry at `revision` is
    /// made again.
    pub fn get_or_put(&self, ctx: &Context, key: K, revision: Revision, make: impl FnOnce() -> ColorImage) -> CachedTexture {
        // wordcraft: crates/ui-egui/src/previews.rs:276 `Previews::get_or`.
        todo!()
    }

    /// Records that `key` has no picture at `revision` (state `Empty`); frees its texture.
    pub fn put_empty(&self, ctx: &Context, key: K, revision: Revision) {
        // new: group, adjustment and type rows (`ThumbRender::Blank`).
        todo!()
    }

    /// Records that making `key`'s picture at `revision` failed (state `Failed`), so nothing asks
    /// for it again until the revision changes; frees its texture.
    pub fn put_failure(&self, ctx: &Context, key: K, revision: Revision, message: impl Into<String>) {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:193 (`ThumbState::Failed`, not retried per frame).
        todo!()
    }

    /// Drops `key`'s entry and frees its texture.
    pub fn forget(&self, key: &K) {
        // new: a closed document's pages.
        todo!()
    }

    /// Keeps only the entries whose key `keep` accepts (PhotoCraft's `prune_thumbs` when the open
    /// documents change).
    pub fn retain(&self, keep: impl FnMut(&K) -> bool) {
        // pc: crates/ui-egui/src/app/thumbs.rs:4-37 `prune_thumbs`.
        todo!()
    }

    /// Drops every entry (another library opened).
    pub fn clear(&self) {
        // lightcraft: crates/ui-egui/src/render.rs:705 `forget_all`.
        todo!()
    }

    /// Whether this pass's upload allowance has room for one more upload of `bytes`: fewer than
    /// `uploads_per_pass` uploads and at most `upload_bytes_per_pass` bytes so far, counting
    /// `bytes`. The first upload of a pass is always allowed, so an image larger than the byte
    /// allowance still lands. A loader asks before each put and leaves the rest for later passes;
    /// [`put`](Self::put) itself always uploads.
    pub fn allow_upload(&self, ctx: &Context, bytes: usize) -> bool {
        // pdfcraft: crates/ui-egui/src/canvas/cache.rs:46 (`UPLOAD_BYTES_PER_FRAME`), :48 (`RESULTS_PER_FRAME`).
        todo!()
    }

    /// What the cache holds and uploaded this pass.
    pub fn statistics(&self) -> TextureStatistics {
        // pdfcraft: crates/ui-egui/src/canvas/cache.rs:86 (`RasterMemory`).
        todo!()
    }

    /// The options the cache was made with.
    pub fn options(&self) -> TextureCacheOptions {
        // new: the loader reads the upload allowance.
        todo!()
    }
}
