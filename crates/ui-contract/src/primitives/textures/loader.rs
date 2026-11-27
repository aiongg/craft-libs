//! `TextureLoader<K>`: a `TextureStore` whose pictures are made by jobs on the runtime's `JobBoard`.

use super::store::{Revision, TextureKey, TextureState, TextureStore, TextureStoreOptions};
use super::words::TextureWords;
use crate::contract::rt::JobBoard;
use craft_ui_rt::{BuildCx, RgbaImage};
use std::hash::Hash;

pub use crate::contract::rt::pending::{Cancel, Priority};

/// A loader's settings, read when it is first made (`TextureLoader::of`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LoaderOptions {
    /// Settings of the loader's store.
    pub store: TextureStoreOptions,
}

impl LoaderOptions {
    /// Thumbnail budgets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Settings of the loader's store.
    #[must_use]
    pub fn store(mut self, store: TextureStoreOptions) -> Self {
        self.store = store;
        self
    }
}

/// What a background job makes when it succeeds: a picture, or the answer that the key has none.
#[derive(Debug)]
#[non_exhaustive]
pub enum LoadedPicture {
    /// The picture to upload.
    Image(RgbaImage),
    /// The key has no picture (a file the decoder doesn't read): its state is `Empty` and the row paints its own
    /// tile.
    Blank,
}

impl From<RgbaImage> for LoadedPicture {
    fn from(image: RgbaImage) -> Self {
        Self::Image(image)
    }
}

/// Why a job made no picture. A job carries no text: the state carries this value and the painter words it at run
/// time with `LoadFailure::text`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadFailure {
    /// The file or page the picture comes from is gone.
    Missing,
    /// It exists but could not be read or decoded.
    Unreadable,
    /// The job panicked. The loader makes this itself.
    Panicked,
    /// The source's own message, already in the user's language (a decoder's error text).
    Message(String),
}

impl LoadFailure {
    /// The failure tooltip in the words of the scope (`cx.words::<TextureWords>()`).
    pub fn text<'a>(&'a self, words: &'a TextureWords) -> &'a str {
        match self {
            Self::Missing => words.missing.as_str(),
            Self::Unreadable => words.unreadable.as_str(),
            Self::Panicked => words.panicked.as_str(),
            Self::Message(message) => message,
        }
    }
}

/// What a loader job returns: a `LoadedPicture`, or why there is none.
pub type LoadResult = Result<LoadedPicture, LoadFailure>;

/// A `TextureStore` whose pictures are made by background jobs: FilmCraft's frames, PdfCraft's pages and the
/// thumbnails of a list.
///
/// - **Jobs.** A job runs on the runtime's `JobBoard` at the priority it was asked with, best first; asking again
///   at another priority moves a waiting job. A job reads its `Cancel` between steps.
/// - **Delivery.** A finished job wakes the loop once. Its picture goes into the image store (the image store's
///   per-frame upload budget spreads a burst over frames) and only the nodes that read the key in their build run
///   again. A result for a revision other than the one last asked for is discarded; a `LoadedPicture::Blank`
///   answer is stored as `Empty` and a failure as `Failed`. A job that panics is stored as
///   `LoadFailure::Panicked`.
/// - **Dropping.** `TextureStore::forget` and `retain` drop keys in every state of their job: a waiting job never
///   runs and a running one is cancelled.
pub struct TextureLoader<K: TextureKey> {
    store: TextureStore<K>,
}

impl<K: TextureKey> Clone for TextureLoader<K> {
    fn clone(&self) -> Self {
        Self { store: self.store.clone() }
    }
}

impl<K: TextureKey> TextureLoader<K> {
    /// The loader of the building node's runtime under `salt`, made with `options` the first time; its store is
    /// `TextureStore::of` under the same salt.
    pub fn of(cx: &BuildCx<'_>, salt: impl Hash, options: LoaderOptions) -> Self {
        Self { store: TextureStore::of(cx, salt, options.store) }
    }

    /// The loader under `salt` on `board` (see [`TextureLoader::of`]).
    pub fn on(board: &JobBoard, salt: impl Hash, options: LoaderOptions) -> Self {
        Self { store: TextureStore::on(board, salt, options.store) }
    }

    /// The loader's store: `state`, `forget`, `retain` and `statistics` work on it as on any store.
    pub fn cache(&self) -> &TextureStore<K> {
        &self.store
    }

    /// What `key` shows for `revision`, queueing the job `job()` makes when the store has nothing for this revision
    /// and no job for it is queued or running: `ask`, then `submit`. `job` is called only when a job is queued.
    pub fn request<J>(&self, cx: &mut BuildCx<'_>, key: &K, revision: Revision, priority: Priority, job: impl FnOnce() -> J) -> TextureState
    where
        J: FnOnce(&Cancel) -> LoadResult + Send + 'static,
    {
        match self.ask(cx, key, revision, priority) {
            Some(state) => state,
            None => self.submit(cx, key.clone(), revision, priority, job()),
        }
    }

    /// What `key` shows for `revision` when that is known without a new job: `Some` of the store's `Ready`, `Empty`
    /// or `Failed`, or `Some(Loading { stand_in })` while a job for this revision is queued or running, moved to
    /// `priority`; `None` when a job is needed. The building node runs again when the job answers.
    pub fn ask(&self, cx: &mut BuildCx<'_>, key: &K, revision: Revision, priority: Priority) -> Option<TextureState> {
        let cache = self.store.cache();
        if !cache.requested(key, revision) {
            return None;
        }
        cache.prioritize(key, revision, priority);
        Some(self.store.state(cx, key, revision))
    }

    /// Queues `job` making `key`'s picture at `revision`, replacing a job of the key for another revision, and
    /// returns `Loading` with the key's old picture as the stand-in. The building node runs again when it answers.
    pub fn submit(
        &self,
        cx: &mut BuildCx<'_>,
        key: K,
        revision: Revision,
        priority: Priority,
        job: impl FnOnce(&Cancel) -> LoadResult + Send + 'static,
    ) -> TextureState {
        self.store.cache().request(key.clone(), revision, priority, move |cancel| match job(cancel)? {
            LoadedPicture::Image(image) => Ok(Some(image)),
            LoadedPicture::Blank => Ok(None),
        });
        self.store.state(cx, &key, revision)
    }

    /// Seconds `key` has waited for its picture, by the frame clock; `None` when it is not loading.
    pub fn waited(&self, cx: &BuildCx<'_>, key: &K) -> Option<f64> {
        self.store.cache().waited(key, cx.now())
    }
}
