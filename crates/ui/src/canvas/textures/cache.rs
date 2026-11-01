//! [`TextureCache`]: keyed textures with revisions, budgets and eviction, living in the egui
//! context.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use egui::{AsIdSalt, ColorImage, Context, Id, TextureHandle, TextureId, TextureOptions, Vec2};

use super::limits::{fit_image, reduce_image, texture_factor, texture_side};

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
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Revision(hasher.finish())
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
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner<K>> {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The cache stored in `ctx` under `id_salt`, made with `options` the first time. Later calls
    /// with the same salt get the same cache and ignore `options`.
    pub fn in_context(ctx: &Context, id_salt: impl AsIdSalt, options: TextureCacheOptions) -> Self {
        let id = Id::unique("craft_ui::canvas::TextureCache").with(&id_salt);
        let name = format!("{id_salt:?}");
        ctx.memory_mut(|memory| {
            memory
                .data
                .get_temp_mut_or_insert_with(id, || Self {
                    inner: Arc::new(Mutex::new(Inner {
                        options,
                        entries: HashMap::new(),
                        bytes: 0,
                        upload_pass: 0,
                        uploads: 0,
                        upload_bytes: 0,
                        evicted: 0,
                        name,
                        next_name: 0,
                    })),
                })
                .clone()
        })
    }

    /// What `key` shows for `revision` (see [`TextureState`]), and marks the entry seen this pass.
    /// - The entry has `revision`: `Ready`, `Empty` or `Failed`.
    /// - The entry has a picture of another revision: `Loading { stand_in: Some(..) }`.
    /// - No entry, or an empty or failed one of another revision: `Missing`.
    pub fn state(&self, ctx: &Context, key: &K, revision: Revision) -> TextureState {
        let pass = ctx.cumulative_pass_nr();
        let mut inner = self.lock();
        let Some(entry) = inner.entries.get_mut(key) else {
            return TextureState::Missing;
        };
        entry.seen = pass;
        let same = entry.revision == revision;
        match &entry.picture {
            Picture::Texture { handle, source, factor } if same => TextureState::Ready(cached(handle, *source, *factor)),
            Picture::Texture { handle, source, factor } => TextureState::Loading { stand_in: Some(cached(handle, *source, *factor)) },
            Picture::Empty if same => TextureState::Empty,
            Picture::Failed(message) if same => TextureState::Failed { message: message.clone() },
            Picture::Empty | Picture::Failed(_) => TextureState::Missing,
        }
    }

    /// Stores `image` as `key`'s picture at `revision` and returns it: in place when the key has a
    /// texture (`TextureHandle::set`, same id), else a new texture with the cache's filter. Reduced
    /// first when past [`texture_side`](super::texture_side). Counts as an upload this pass, marks
    /// the entry seen, then evicts (see the type's doc). The image is sRGBA as egui's `ColorImage`
    /// holds it (built from unmultiplied values with `ColorImage::from_rgba_unmultiplied`).
    pub fn put(&self, ctx: &Context, key: K, revision: Revision, image: ColorImage) -> CachedTexture {
        let pass = ctx.cumulative_pass_nr();
        let side = texture_side(ctx);
        let source = image.size;
        let factor = texture_factor(texel_size(source), side);
        let image = fit_image(image, side);
        let size = image.size;
        let mut inner = self.lock();
        inner.roll_uploads(pass);
        let filter = inner.options.filter;
        let existing = inner.entries.get_mut(&key).and_then(|entry| match &mut entry.picture {
            Picture::Texture { handle, .. } => Some(handle),
            _ => None,
        });
        let handle = match existing {
            Some(handle) => {
                handle.set(image, filter);
                handle.clone()
            }
            None => {
                let name = inner.next_texture_name();
                ctx.load_texture(name, image, filter)
            }
        };
        let texture = CachedTexture { id: handle.id(), size };
        let bytes = size[0] * size[1] * 4;
        inner.uploads += 1;
        inner.upload_bytes += bytes;
        inner.store(key, Entry { revision, picture: Picture::Texture { handle, source, factor }, seen: pass });
        inner.evict(pass);
        texture
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
        let pass = ctx.cumulative_pass_nr();
        let mut inner = self.lock();
        inner.roll_uploads(pass);
        let filter = inner.options.filter;
        let entry = inner.entries.get_mut(key)?;
        let Picture::Texture { handle, source, factor } = &mut entry.picture else {
            return None;
        };
        let [x, y] = origin;
        let [w, h] = image.size;
        if x.checked_add(w)? > source[0] || y.checked_add(h)? > source[1] {
            return None;
        }
        let f = *factor as usize;
        let on_grid = |start: usize, len: usize, whole: usize| start.is_multiple_of(f) && ((start + len).is_multiple_of(f) || start + len == whole);
        if f > 1 && !(on_grid(x, w, source[0]) && on_grid(y, h, source[1])) {
            return None;
        }
        let sent = if f > 1 { reduce_image(&image, *factor) } else { image };
        let uploaded = sent.size[0] * sent.size[1] * 4;
        if uploaded > 0 {
            handle.set_partial([x / f, y / f], sent, filter);
        }
        entry.revision = revision;
        entry.seen = pass;
        let texture = cached(handle, *source, *factor);
        inner.uploads += 1;
        inner.upload_bytes += uploaded;
        Some(texture)
    }

    /// `key`'s picture at `revision`, calling `make` and putting its image only when the entry
    /// has another revision or none (PhotoCraft's `cached_texture`, a preset's computed
    /// thumbnail, WordCraft's `Previews::get_or`). An `Empty` or `Failed` entry at `revision` is
    /// made again.
    pub fn get_or_put(&self, ctx: &Context, key: K, revision: Revision, make: impl FnOnce() -> ColorImage) -> CachedTexture {
        let pass = ctx.cumulative_pass_nr();
        {
            let mut inner = self.lock();
            if let Some(entry) = inner.entries.get_mut(&key)
                && entry.revision == revision
                && let Picture::Texture { handle, source, factor } = &entry.picture
            {
                entry.seen = pass;
                return cached(handle, *source, *factor);
            }
        }
        self.put(ctx, key, revision, make())
    }

    /// Records that `key` has no picture at `revision` (state `Empty`); frees its texture.
    pub fn put_empty(&self, ctx: &Context, key: K, revision: Revision) {
        let pass = ctx.cumulative_pass_nr();
        let mut inner = self.lock();
        inner.store(key, Entry { revision, picture: Picture::Empty, seen: pass });
        inner.evict(pass);
    }

    /// Records that making `key`'s picture at `revision` failed (state `Failed`), so nothing asks
    /// for it again until the revision changes; frees its texture.
    pub fn put_failure(&self, ctx: &Context, key: K, revision: Revision, message: impl Into<String>) {
        let pass = ctx.cumulative_pass_nr();
        let mut inner = self.lock();
        inner.store(key, Entry { revision, picture: Picture::Failed(message.into()), seen: pass });
        inner.evict(pass);
    }

    /// Drops `key`'s entry and frees its texture.
    pub fn forget(&self, key: &K) {
        let mut inner = self.lock();
        if let Some(entry) = inner.entries.remove(key) {
            inner.bytes -= picture_bytes(&entry.picture);
        }
    }

    /// Keeps only the entries whose key `keep` accepts (PhotoCraft's `prune_thumbs` when the open
    /// documents change).
    pub fn retain(&self, keep: impl FnMut(&K) -> bool) {
        let mut keep = keep;
        let mut inner = self.lock();
        let Inner { entries, bytes, .. } = &mut *inner;
        entries.retain(|key, entry| {
            let kept = keep(key);
            if !kept {
                *bytes -= picture_bytes(&entry.picture);
            }
            kept
        });
    }

    /// Drops every entry (another library opened).
    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.entries.clear();
        inner.bytes = 0;
    }

    /// Whether this pass's upload allowance has room for one more upload of `bytes`: fewer than
    /// `uploads_per_pass` uploads and at most `upload_bytes_per_pass` bytes so far, counting
    /// `bytes`. The first upload of a pass is always allowed, so an image larger than the byte
    /// allowance still lands. A loader asks before each put and leaves the rest for later passes;
    /// [`put`](Self::put) itself always uploads.
    pub fn allow_upload(&self, ctx: &Context, bytes: usize) -> bool {
        let mut inner = self.lock();
        inner.roll_uploads(ctx.cumulative_pass_nr());
        let budget = inner.options.budget;
        inner.uploads == 0 || (inner.uploads < budget.uploads_per_pass && inner.upload_bytes.saturating_add(bytes) <= budget.upload_bytes_per_pass)
    }

    /// What the cache holds and uploaded this pass.
    pub fn statistics(&self) -> TextureStatistics {
        let inner = self.lock();
        TextureStatistics {
            entries: inner.entries.len(),
            bytes: inner.bytes,
            uploads: inner.uploads,
            upload_bytes: inner.upload_bytes,
            evicted: inner.evicted,
        }
    }

    /// The options the cache was made with.
    pub fn options(&self) -> TextureCacheOptions {
        self.lock().options
    }
}

fn texel_size(size: [usize; 2]) -> [u32; 2] {
    size.map(|n| u32::try_from(n).unwrap_or(u32::MAX))
}

/// The texel size of a texture made from an image of size `source` reduced by `factor`. The
/// texture manager's own size lags an in-place `set` until the pass ends.
fn texture_size(source: [usize; 2], factor: u32) -> [usize; 2] {
    if factor <= 1 { source } else { source.map(|n| n.div_ceil(factor as usize).max(1)) }
}

fn cached(handle: &TextureHandle, source: [usize; 2], factor: u32) -> CachedTexture {
    CachedTexture { id: handle.id(), size: texture_size(source, factor) }
}

fn picture_bytes(picture: &Picture) -> usize {
    match picture {
        Picture::Texture { source, factor, .. } => {
            let [w, h] = texture_size(*source, *factor);
            w * h * 4
        }
        Picture::Empty | Picture::Failed(_) => 0,
    }
}

impl<K: TextureKey> Inner<K> {
    /// Starts the upload counters over when `pass` is a new pass.
    fn roll_uploads(&mut self, pass: u64) {
        if self.upload_pass != pass {
            self.upload_pass = pass;
            self.uploads = 0;
            self.upload_bytes = 0;
        }
    }

    fn next_texture_name(&mut self) -> String {
        self.next_name += 1;
        format!("{}/{}", self.name, self.next_name)
    }

    /// Replaces `key`'s entry, keeping the byte count.
    fn store(&mut self, key: K, entry: Entry) {
        self.bytes += picture_bytes(&entry.picture);
        if let Some(old) = self.entries.insert(key, entry) {
            self.bytes -= picture_bytes(&old.picture);
        }
    }

    /// Drops the least recently seen entries until the budget holds, never one seen in `pass` or
    /// the pass before.
    fn evict(&mut self, pass: u64) {
        let budget = self.options.budget;
        if self.bytes <= budget.max_bytes && self.entries.len() <= budget.max_entries {
            return;
        }
        let protected = pass.saturating_sub(1);
        let mut candidates: Vec<(u64, K)> =
            self.entries.iter().filter(|(_, entry)| entry.seen < protected).map(|(key, entry)| (entry.seen, key.clone())).collect();
        candidates.sort_by_key(|(seen, _)| *seen);
        for (_, key) in candidates {
            if self.bytes <= budget.max_bytes && self.entries.len() <= budget.max_entries {
                break;
            }
            if let Some(entry) = self.entries.remove(&key) {
                self.bytes -= picture_bytes(&entry.picture);
                self.evicted += 1;
            }
        }
    }
}
