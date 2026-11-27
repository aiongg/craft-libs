//! The texture store on the image store (docs/contract-rt.md section 10; B2's hand-over). A picture
//! (a thumbnail, a cached texture) is an [`ImageKey`] derived from the app's key and the picture's
//! revision, so a paint names it before it exists: the node draws `Item::Image` with that key and
//! ui-rt damages exactly the nodes that show the key when the pixels arrive
//! ([`ImageStore::insert`]). A job that makes the picture runs on the [`JobBoard`], which wakes the
//! loop when it finishes; nothing re-runs, and only the nodes drawing the picture repaint.
//!
//! The cache keeps a byte budget over the pictures it holds and removes the least recently looked-at
//! ones from the image store when the budget is exceeded.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_ui_rt::{ImageKey, ImageStore, RgbaImage, hash_key};

use super::pending::JobBoard;

/// What a cache key is: anything hashable the app names a picture by.
pub trait TextureKey: Hash + Eq + Clone + Send + 'static {}
impl<K: Hash + Eq + Clone + Send + 'static> TextureKey for K {}

/// What a picture's state says to a paint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureLook {
    /// Not made yet: paint the placeholder; the picture's image key is drawn too, so the pixels show when
    /// they arrive.
    Loading,
    /// In the image store under this key.
    Ready(ImageKey),
    /// The job answered that there is none.
    Empty,
    /// The job failed.
    Failed,
}

enum State {
    Loading,
    Ready { bytes: usize },
    Empty,
    Failed,
}

struct Entry {
    revision: u64,
    state: State,
    /// The last look: the least recent entries leave first.
    used: u64,
}

struct Entries<K> {
    map: HashMap<K, Entry>,
    clock: u64,
    bytes: usize,
}

/// A keyed cache of pictures in the image store, with a byte budget.
pub struct TextureCache<K: TextureKey> {
    salt: u64,
    budget: usize,
    board: JobBoard,
    entries: Arc<Mutex<Entries<K>>>,
}

impl<K: TextureKey> Clone for TextureCache<K> {
    fn clone(&self) -> Self {
        TextureCache { salt: self.salt, budget: self.budget, board: self.board.clone(), entries: self.entries.clone() }
    }
}

impl<K: TextureKey> TextureCache<K> {
    /// A cache named `salt` (two caches never share image keys), holding at most `budget` bytes of pixels,
    /// making pictures on `board`.
    pub fn new(salt: impl Hash, budget: usize, board: JobBoard) -> Self {
        TextureCache { salt: hash_key(&salt), budget, board, entries: Arc::new(Mutex::new(Entries { map: HashMap::new(), clock: 0, bytes: 0 })) }
    }

    fn lock(&self) -> MutexGuard<'_, Entries<K>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The image key of `key` at `revision`: the same for the same pair, so a paint can name a picture
    /// that is not there yet.
    pub fn image_key(&self, key: &K, revision: u64) -> ImageKey {
        ImageKey(hash_key(&(self.salt, key, revision)))
    }

    /// The state of `key` at `revision`, marking it as just used.
    pub fn look(&self, key: &K, revision: u64) -> TextureLook {
        let mut entries = self.lock();
        entries.clock += 1;
        let clock = entries.clock;
        let Some(entry) = entries.map.get_mut(key).filter(|e| e.revision == revision) else { return TextureLook::Loading };
        entry.used = clock;
        match entry.state {
            State::Loading => TextureLook::Loading,
            State::Ready { .. } => TextureLook::Ready(self.image_key(key, revision)),
            State::Empty => TextureLook::Empty,
            State::Failed => TextureLook::Failed,
        }
    }

    /// Whether `key` at `revision` has been asked for (loading or answered).
    pub fn requested(&self, key: &K, revision: u64) -> bool {
        self.lock().map.get(key).is_some_and(|e| e.revision == revision)
    }

    /// Makes the picture on the board unless `key` at `revision` was asked for. `make` returns the pixels,
    /// or `None` for "there is none"; a panic is a failure. A newer revision replaces the old picture when
    /// it arrives.
    pub fn request(&self, key: K, revision: u64, make: impl FnOnce() -> Option<RgbaImage> + Send + 'static) {
        {
            let mut entries = self.lock();
            let clock = entries.clock;
            if entries.map.get(&key).is_some_and(|e| e.revision == revision) {
                return;
            }
            let old = entries.map.insert(key.clone(), Entry { revision, state: State::Loading, used: clock });
            if let Some(Entry { state: State::Ready { bytes }, .. }) = old {
                entries.bytes = entries.bytes.saturating_sub(bytes);
            }
        }
        let cache = self.clone();
        self.board.spawn("craft-texture", make, move |result, delivery| {
            let (state, image) = match result {
                Ok(Some(image)) => (None, Some(image)),
                Ok(None) => (Some(State::Empty), None),
                Err(()) => (Some(State::Failed), None),
            };
            match image {
                Some(image) => cache.put(delivery.images, &key, revision, image),
                None => {
                    let mut entries = cache.lock();
                    if let Some(entry) = entries.map.get_mut(&key).filter(|e| e.revision == revision) {
                        entry.state = state.unwrap_or(State::Failed);
                    }
                }
            }
        });
    }

    /// Stores `image` for `key` at `revision` now (a picture made on the UI thread). A picture for a
    /// revision that is no longer the entry's is dropped.
    pub fn put(&self, images: &mut ImageStore, key: &K, revision: u64, image: RgbaImage) {
        let bytes = image.pixels.len();
        let image_key = self.image_key(key, revision);
        let mut gone = Vec::new();
        {
            let mut entries = self.lock();
            entries.clock += 1;
            let clock = entries.clock;
            if entries.map.get(key).is_some_and(|e| e.revision != revision) {
                return;
            }
            entries.map.insert(key.clone(), Entry { revision, state: State::Ready { bytes }, used: clock });
            entries.bytes += bytes;
            while entries.bytes > self.budget {
                let oldest = entries
                    .map
                    .iter()
                    .filter(|(k, e)| matches!(e.state, State::Ready { .. }) && *k != key)
                    .min_by_key(|(_, e)| e.used)
                    .map(|(k, e)| (k.clone(), e.revision));
                let Some((k, rev)) = oldest else { break };
                if let Some(Entry { state: State::Ready { bytes }, .. }) = entries.map.remove(&k) {
                    entries.bytes = entries.bytes.saturating_sub(bytes);
                }
                gone.push(self.image_key(&k, rev));
            }
        }
        for old in gone {
            images.remove(old);
        }
        images.insert(image_key, Arc::new(image));
    }

    /// The board the cache makes its pictures on.
    pub fn board(&self) -> &JobBoard {
        &self.board
    }

    /// The bytes of pixels the cache holds.
    pub fn bytes(&self) -> usize {
        self.lock().bytes
    }
}
