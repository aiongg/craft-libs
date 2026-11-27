//! The texture cache on the image store (docs/contract-rt.md section 10; Q10). A picture is an
//! [`ImageKey`] derived from the cache, the app's [`TextureKey`] and the picture's [`Revision`], so
//! a paint names it before it exists. The node that shows a picture reads its key in its build
//! ([`TextureCache::read`]): the read subscribes the node to the key's cell, so when a job delivers
//! the picture (or answers that there is none, or fails) only the nodes that read that key run
//! again, and nothing polls per frame. A job that makes a picture runs on the [`JobBoard`] at a
//! priority; the board wakes the loop once when it finishes.
//!
//! The cache keeps a byte and an entry budget over what it holds and removes the least recently
//! read pictures from the image store when one is exceeded, never one read in this frame or the
//! last (it is on screen). The per-frame upload budget is the image store's
//! ([`ImageStore::upload_budget`]).

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_behavior::PixelSize;
use craft_ui_rt::{BuildCx, ImageKey, ImageOptions, ImageStore, RgbaImage, SizeU, State, Store, Writes, hash_key};

use super::pending::{Cancel, Delivery, JobBoard, JobHandle, Priority};
use crate::primitives::textures::{CachedTexture, LoadFailure, MAX_TEXTURE, Revision, TextureBudget, TextureKey, TextureStatistics, fit_image};

/// What a key shows for a revision, from [`TextureCache::look`] and [`TextureCache::read`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TextureLook {
    /// Nothing is held for this key, or only an empty or failed entry of another revision.
    Missing,
    /// The picture for this revision is on its way.
    #[non_exhaustive]
    Loading {
        /// The key's picture of another revision, shown until the new one lands.
        stand_in: Option<CachedTexture>,
    },
    /// The picture for this revision.
    Ready(CachedTexture),
    /// This revision has no picture: nothing is drawn and nothing is asked again.
    Empty,
    /// Making this revision's picture failed; it is not asked again until the revision changes.
    Failed(LoadFailure),
}

enum Made {
    Loading {
        job: Option<(JobHandle, Priority)>,
        /// The frame time of the first read after the request.
        asked: Option<f64>,
    },
    Ready(CachedTexture),
    Empty,
    Failed(LoadFailure),
}

struct Entry {
    revision: Revision,
    made: Made,
    /// The last picture of an older revision, while a newer one loads.
    stand_in: Option<CachedTexture>,
    /// The last look: the least recent entries leave first.
    used: u64,
    /// The frame time of the last read.
    seen: Option<f64>,
    /// The cell the nodes that read this key subscribe to; written when a job answers.
    cell: Option<State<u64>>,
}

impl Entry {
    fn bytes(&self) -> usize {
        let ready = match &self.made {
            Made::Ready(texture) => byte_size(texture.size),
            _ => 0,
        };
        ready + self.stand_in.map_or(0, |t| byte_size(t.size))
    }

    fn cancel(&self) {
        if let Made::Loading { job: Some((job, _)), .. } = &self.made {
            job.cancel();
        }
    }
}

struct Entries<K> {
    map: HashMap<K, Entry>,
    clock: u64,
    bytes: usize,
    evicted: usize,
    /// The frame times of the last two frames that read from the cache.
    frames: [f64; 2],
}

/// Image store and cell work a cache change leaves for the caller, which holds the stores.
#[derive(Default)]
struct Effects {
    remove: Vec<ImageKey>,
    insert: Option<(ImageKey, Arc<RgbaImage>)>,
    /// Cells whose readers run again.
    touch: Vec<State<u64>>,
    /// Cells of entries that left: their readers run again, then the cells are freed.
    dead: Vec<State<u64>>,
}

impl Effects {
    fn images(&mut self, images: &mut ImageStore, options: ImageOptions) {
        for key in self.remove.drain(..) {
            images.remove(key);
        }
        if let Some((key, image)) = self.insert.take() {
            images.insert_with(key, image, options);
        }
    }

    fn cells(self, store: &mut Store) {
        for cell in self.touch {
            cell.update(store, |v| *v += 1);
        }
        for cell in self.dead {
            cell.update(store, |v| *v += 1);
            store.remove(cell);
        }
    }

    fn apply(mut self, cx: &mut BuildCx<'_>, options: ImageOptions) {
        self.images(cx.images(), options);
        self.cells(cx.store_mut());
    }

    fn deliver(mut self, d: &mut Delivery<'_>, options: ImageOptions) {
        self.images(d.images, options);
        self.cells(d.store);
    }
}

/// A keyed cache of pictures in the image store, with a byte and an entry budget. Clones share the
/// pictures.
pub struct TextureCache<K: TextureKey> {
    salt: u64,
    budget: TextureBudget,
    options: ImageOptions,
    board: JobBoard,
    entries: Arc<Mutex<Entries<K>>>,
}

impl<K: TextureKey> Clone for TextureCache<K> {
    fn clone(&self) -> Self {
        TextureCache { salt: self.salt, budget: self.budget, options: self.options, board: self.board.clone(), entries: self.entries.clone() }
    }
}

impl<K: TextureKey> TextureCache<K> {
    /// A cache named `salt` (two caches never share image keys) within `budget`, making pictures on
    /// `board`.
    pub fn new(salt: impl Hash, budget: TextureBudget, board: JobBoard) -> Self {
        let entries = Entries { map: HashMap::new(), clock: 0, bytes: 0, evicted: 0, frames: [f64::NEG_INFINITY; 2] };
        TextureCache { salt: hash_key(&salt), budget, options: ImageOptions::default(), board, entries: Arc::new(Mutex::new(entries)) }
    }

    /// How the renderers sample every picture of the cache (nearest for pixel-exact previews).
    #[must_use]
    pub fn options(mut self, options: ImageOptions) -> Self {
        self.options = options;
        self
    }

    fn lock(&self) -> MutexGuard<'_, Entries<K>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The image key of `key` at `revision`: the same for the same pair, so a paint can name a picture
    /// that is not there yet.
    pub fn image_key(&self, key: &K, revision: Revision) -> ImageKey {
        ImageKey(hash_key(&(self.salt, key, revision.get())))
    }

    /// The state of `key` at `revision`, marking it as just used. Records no read: a build that
    /// shows the picture uses [`TextureCache::read`].
    pub fn look(&self, key: &K, revision: Revision) -> TextureLook {
        let mut entries = self.lock();
        entries.clock += 1;
        let clock = entries.clock;
        let Some(entry) = entries.map.get_mut(key) else { return TextureLook::Missing };
        entry.used = clock;
        look_of(entry, revision)
    }

    /// [`TextureCache::look`] for the node building now, which runs again when a job answers for
    /// `key`: the one signal of a delivered picture. A key that was never asked for subscribes
    /// nothing; read it again after [`TextureCache::request`].
    pub fn read(&self, cx: &mut BuildCx<'_>, key: &K, revision: Revision) -> TextureLook {
        let now = cx.now();
        let (look, cell) = {
            let mut entries = self.lock();
            if entries.frames[1] != now {
                entries.frames = [entries.frames[1], now];
            }
            entries.clock += 1;
            let clock = entries.clock;
            let Some(entry) = entries.map.get_mut(key) else { return TextureLook::Missing };
            entry.used = clock;
            entry.seen = Some(now);
            if let Made::Loading { asked, .. } = &mut entry.made {
                asked.get_or_insert(now);
            }
            let cell = *entry.cell.get_or_insert_with(|| cx.store_mut().create(0u64));
            (look_of(entry, revision), cell)
        };
        cell.get(cx);
        look
    }

    /// Seconds since `key`'s picture was first read after its request, at frame time `now`; `None`
    /// when it is not loading.
    pub fn waited(&self, key: &K, now: f64) -> Option<f64> {
        match &self.lock().map.get(key)?.made {
            Made::Loading { asked: Some(asked), .. } => Some((now - asked).max(0.0)),
            _ => None,
        }
    }

    /// Whether `key` at `revision` has been asked for (loading or answered).
    pub fn requested(&self, key: &K, revision: Revision) -> bool {
        self.lock().map.get(key).is_some_and(|e| e.revision == revision)
    }

    /// Moves the waiting job of `key` at `revision` to `priority`.
    pub fn prioritize(&self, key: &K, revision: Revision, priority: Priority) {
        if let Some(entry) = self.lock().map.get_mut(key).filter(|e| e.revision == revision)
            && let Made::Loading { job: Some((job, asked)), .. } = &mut entry.made
            && *asked != priority
        {
            job.set_priority(priority);
            *asked = priority;
        }
    }

    /// Makes the picture on the board at `priority` unless `key` at `revision` was asked for; a
    /// repeated request moves a waiting job to the new priority. `make` returns the pixels,
    /// `Ok(None)` for "there is none" or a failure, and reads the cancel flag between steps; a panic
    /// is [`LoadFailure::Panicked`]. A newer revision replaces the old picture when it arrives, and
    /// the old one stands in until then.
    pub fn request(
        &self,
        key: K,
        revision: Revision,
        priority: Priority,
        make: impl FnOnce(&Cancel) -> Result<Option<RgbaImage>, LoadFailure> + Send + 'static,
    ) {
        {
            let mut guard = self.lock();
            if guard.map.get(&key).is_some_and(|e| e.revision == revision) {
                drop(guard);
                self.prioritize(&key, revision, priority);
                return;
            }
            let entries = &mut *guard;
            let clock = entries.clock;
            match entries.map.get_mut(&key) {
                Some(entry) => {
                    entry.cancel();
                    // A settled entry holds no stand-in: its picture becomes the stand-in, and a
                    // loading one keeps its own.
                    if let Made::Ready(texture) = std::mem::replace(&mut entry.made, Made::Loading { job: None, asked: None }) {
                        entry.stand_in = Some(texture);
                    }
                    entry.revision = revision;
                    entry.used = clock;
                }
                None => {
                    entries.map.insert(
                        key.clone(),
                        Entry { revision, made: Made::Loading { job: None, asked: None }, stand_in: None, used: clock, seen: None, cell: None },
                    );
                }
            }
        }
        let cache = self.clone();
        let job_key = key.clone();
        let job = self.board.spawn_with(
            priority,
            move |cancel| make(cancel),
            move |result, d| {
                let result = result.unwrap_or(Err(LoadFailure::Panicked));
                cache.answer(d, &job_key, revision, result);
            },
        );
        if let Some(entry) = self.lock().map.get_mut(&key).filter(|e| e.revision == revision)
            && let Made::Loading { job: slot, .. } = &mut entry.made
            && slot.is_none()
        {
            *slot = Some((job, priority));
        }
    }

    /// A job's answer for `key` at `revision`, on the UI thread: the readers of the key run again.
    /// An answer for a revision the entry no longer wants, or for an entry that left, is dropped.
    fn answer(&self, d: &mut Delivery<'_>, key: &K, revision: Revision, result: Result<Option<RgbaImage>, LoadFailure>) {
        if !self.requested(key, revision) {
            return;
        }
        let mut fx = match result {
            Ok(Some(image)) => self.store(key, revision, image),
            Ok(None) => self.settle(key, revision, Made::Empty),
            Err(failure) => self.settle(key, revision, Made::Failed(failure)),
        };
        if let Some(cell) = self.lock().map.get(key).and_then(|e| e.cell) {
            fx.touch.push(cell);
        }
        fx.deliver(d, self.options);
    }

    /// Stores `image` for `key` at `revision` now (a picture made on the UI thread) and returns it.
    /// It replaces the key's picture of any revision. The building node reads the key as with
    /// [`TextureCache::read`]; other readers are not told (they read the picture when they build).
    pub fn put(&self, cx: &mut BuildCx<'_>, key: K, revision: Revision, image: RgbaImage) -> CachedTexture {
        let fx = self.store(&key, revision, image);
        let texture = fx.insert.as_ref().map(|(k, image)| CachedTexture::new(*k, pixel_size(image.size)));
        self.settled(cx, fx, &key, revision);
        texture.expect("a stored picture")
    }

    /// Records that `key` at `revision` has no picture.
    pub fn put_empty(&self, cx: &mut BuildCx<'_>, key: K, revision: Revision) {
        let fx = self.settle(&key, revision, Made::Empty);
        self.settled(cx, fx, &key, revision);
    }

    /// Records that making `key`'s picture at `revision` failed.
    pub fn put_failure(&self, cx: &mut BuildCx<'_>, key: K, revision: Revision, failure: LoadFailure) {
        let fx = self.settle(&key, revision, Made::Failed(failure));
        self.settled(cx, fx, &key, revision);
    }

    /// Applies a UI-thread change and subscribes the building node to the key, which it shows.
    fn settled(&self, cx: &mut BuildCx<'_>, fx: Effects, key: &K, revision: Revision) {
        fx.apply(cx, self.options);
        self.read(cx, key, revision);
    }

    fn store(&self, key: &K, revision: Revision, image: RgbaImage) -> Effects {
        let image = fit_image(image, MAX_TEXTURE);
        let texture = CachedTexture::new(self.image_key(key, revision), pixel_size(image.size));
        let mut fx = self.settle(key, revision, Made::Ready(texture));
        fx.insert = Some((texture.key, Arc::new(image)));
        fx
    }

    /// Settles `key` at `revision` as `made`, replacing the key's picture of any revision and its
    /// stand-in and cancelling a job for another revision, and evicts past the budget.
    fn settle(&self, key: &K, revision: Revision, made: Made) -> Effects {
        let mut fx = Effects::default();
        let mut guard = self.lock();
        let entries = &mut *guard;
        entries.clock += 1;
        let clock = entries.clock;
        let entry = entries.map.entry(key.clone()).or_insert(Entry {
            revision,
            made: Made::Loading { job: None, asked: None },
            stand_in: None,
            used: clock,
            seen: None,
            cell: None,
        });
        // A job still making this key (for this revision or another) is no longer wanted.
        entry.cancel();
        entry.revision = revision;
        let before = entry.bytes();
        if let Made::Ready(old) = &entry.made {
            fx.remove.push(old.key);
        }
        fx.remove.extend(entry.stand_in.take().map(|t| t.key));
        entry.made = made;
        entry.used = clock;
        if let Made::Ready(texture) = &entry.made {
            fx.remove.retain(|k| *k != texture.key);
        }
        let after = entry.bytes();
        entries.bytes = entries.bytes + after - before;
        self.evict(entries, key, &mut fx);
        fx
    }

    fn evict(&self, entries: &mut Entries<K>, keep: &K, fx: &mut Effects) {
        let recent = entries.frames[0];
        while entries.bytes > self.budget.max_bytes || entries.map.len() > self.budget.max_entries {
            let oldest = entries
                .map
                .iter()
                .filter(|(k, e)| *k != keep && e.seen.is_none_or(|seen| seen < recent))
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| k.clone());
            let Some(k) = oldest else { break };
            if let Some(entry) = entries.map.remove(&k) {
                self.drop_entry(entries, entry, fx);
                entries.evicted += 1;
            }
        }
    }

    fn drop_entry(&self, entries: &mut Entries<K>, entry: Entry, fx: &mut Effects) {
        entries.bytes -= entry.bytes();
        entry.cancel();
        if let Made::Ready(texture) = &entry.made {
            fx.remove.push(texture.key);
        }
        if let Some(stand_in) = entry.stand_in {
            fx.remove.push(stand_in.key);
        }
        fx.dead.extend(entry.cell);
    }

    /// Drops `key`'s pictures and cancels its job; its readers run again.
    pub fn forget(&self, cx: &mut BuildCx<'_>, key: &K) {
        self.retain(cx, |k| k != key);
    }

    /// Keeps only the keys `keep` accepts.
    pub fn retain(&self, cx: &mut BuildCx<'_>, mut keep: impl FnMut(&K) -> bool) {
        let mut fx = Effects::default();
        {
            let mut guard = self.lock();
            let entries = &mut *guard;
            let gone: Vec<K> = entries.map.keys().filter(|k| !keep(k)).cloned().collect();
            for key in gone {
                if let Some(entry) = entries.map.remove(&key) {
                    self.drop_entry(entries, entry, &mut fx);
                }
            }
        }
        fx.apply(cx, self.options);
    }

    /// Drops every picture.
    pub fn clear(&self, cx: &mut BuildCx<'_>) {
        self.retain(cx, |_| false);
    }

    /// The board the cache makes its pictures on.
    pub fn board(&self) -> &JobBoard {
        &self.board
    }

    /// The bytes of pixels the cache holds.
    pub fn bytes(&self) -> usize {
        self.lock().bytes
    }

    /// What the cache holds and evicted, for automation, stories and tests.
    pub fn statistics(&self) -> TextureStatistics {
        let entries = self.lock();
        TextureStatistics { entries: entries.map.len(), bytes: entries.bytes, evicted: entries.evicted }
    }

    /// The cache's budget.
    pub fn budget(&self) -> TextureBudget {
        self.budget
    }
}

fn look_of(entry: &Entry, revision: Revision) -> TextureLook {
    let shown = match &entry.made {
        Made::Ready(texture) => Some(*texture),
        _ => entry.stand_in,
    };
    if entry.revision != revision {
        return match shown {
            Some(texture) => TextureLook::Loading { stand_in: Some(texture) },
            None => TextureLook::Missing,
        };
    }
    match &entry.made {
        Made::Loading { .. } => TextureLook::Loading { stand_in: entry.stand_in },
        Made::Ready(texture) => TextureLook::Ready(*texture),
        Made::Empty => TextureLook::Empty,
        Made::Failed(failure) => TextureLook::Failed(failure.clone()),
    }
}

fn pixel_size(size: SizeU) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn byte_size(size: PixelSize) -> usize {
    [size.width, size.height].into_iter().fold(4usize, |bytes, side| bytes.saturating_mul(side as usize))
}
