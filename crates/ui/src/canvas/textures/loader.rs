//! [`TextureLoader`]: a [`TextureCache`] fed by background jobs on a [`JobQueue`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

#[cfg(target_arch = "wasm32")]
use craft_host::InlineJobs;
#[cfg(not(target_arch = "wasm32"))]
use craft_host::ThreadJobs;
use craft_host::queue::{Cancel, JobQueue, Priority, QueueOptions};
use craft_host::{Host, Jobs, Wake};
use egui::{AsIdSalt, Color32, ColorImage, Context, Id};

use crate::localized::Labels;

use super::cache::{Revision, TextureCache, TextureCacheOptions, TextureKey, TextureState};

/// A loader's settings, read when it is created ([`TextureLoader::in_context`]).
#[derive(Clone)]
pub struct LoaderOptions {
    pub cache: TextureCacheOptions,
    /// Jobs running at once; 0 runs one job per pass on the UI thread (wasm32).
    /// Default: [`QueueOptions::default_lanes`](craft_host::queue::QueueOptions::default_lanes).
    pub lanes: usize,
    /// Where the lanes run. `None`: the installed host's [`Jobs`]
    /// ([`Host::jobs_shared`](craft_host::Host::jobs_shared)), or `ThreadJobs` (`InlineJobs` on
    /// wasm32) when no host is installed. Stories and tests pass a
    /// [`ManualJobs`](craft_host::ManualJobs) to finish one job at a time.
    pub jobs: Option<Arc<dyn Jobs>>,
}

impl Default for LoaderOptions {
    /// Thumbnail budgets, the machine's lanes, the host's jobs.
    fn default() -> Self {
        Self { cache: TextureCacheOptions::thumbnails(), lanes: QueueOptions::default_lanes(), jobs: None }
    }
}

impl std::fmt::Debug for LoaderOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoaderOptions").field("cache", &self.cache).field("lanes", &self.lanes).field("jobs", &self.jobs.is_some()).finish()
    }
}

/// What a background job makes when it succeeds: a picture, or the answer that the key has none.
#[derive(Debug)]
pub enum LoadedPicture {
    /// The picture to upload.
    Image(ColorImage),
    /// The key has no picture (a file the decoder doesn't read): stored with
    /// [`TextureCache::put_empty`], so its state is `Empty` and the row paints its own tile.
    Blank,
}

impl From<ColorImage> for LoadedPicture {
    fn from(image: ColorImage) -> Self {
        Self::Image(image)
    }
}

/// Why a job made no picture. The loader words it on the UI thread when it stores the failure
/// ([`TextureState::Failed`]'s message, the failure tooltip), so a job carries no English text.
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

crate::words! {
    /// The words of a picture that could not be made.
    pub(crate) struct LoadFailureLabels {
        missing: "textures.missing" = "The file is missing";
        unreadable: "textures.unreadable" = "The picture could not be read";
        panicked: "textures.panicked" = "The picture could not be made";
    }
}

impl LoadFailure {
    /// The failure tooltip.
    fn words(self, ctx: &Context) -> String {
        let labels = LoadFailureLabels::of_ctx(ctx);
        match self {
            Self::Missing => labels.missing.clone(),
            Self::Unreadable => labels.unreadable.clone(),
            Self::Panicked => labels.panicked.clone(),
            Self::Message(message) => message,
        }
    }
}

/// What a loader job returns: a [`LoadedPicture`], or why there is none.
pub(crate) type LoadResult = Result<LoadedPicture, LoadFailure>;

/// A key's job in flight: the revision it makes and when it was last asked for.
struct Asked {
    revision: Revision,
    priority: Priority,
    /// The last pass a widget asked for the key.
    pass: u64,
}

struct Inner<K> {
    lanes: usize,
    queue: JobQueue<K, (Revision, LoadResult)>,
    asked: HashMap<K, Asked>,
    /// The pass `poll` last ran in, so it runs once per pass.
    polled: u64,
    /// Finished results waiting for upload allowance, oldest first.
    waiting: Vec<(K, Revision, LoadResult)>,
}

/// A [`TextureCache`] whose pictures are made by background jobs: FilmCraft's frames, PdfCraft's
/// pages, and the `Later` renders of [`Thumbnails`](super::Thumbnails).
///
/// ```ignore
/// let loader = TextureLoader::<PageKey>::in_context(ctx, "pages", LoaderOptions::default());
/// let state = loader.request(ctx, &page, Revision::of(&doc.stamp()), Priority::VISIBLE, || {
///     let pdf = doc.clone_handle();
///     move |cancel: &Cancel| pdf.render(page, cancel).map(LoadedPicture::from).map_err(|e| LoadFailure::Message(e.to_string()))
/// });
/// ```
///
/// - **One pass.** The first call in a pass ([`request`](Self::request), [`ask`](Self::ask),
///   [`submit`](Self::submit)) takes finished results off the queue and uploads them in the order
///   they finished, while [`TextureCache::allow_upload`] allows (16 and 16 MiB per pass by
///   default). The rest wait for later passes and the loader calls `ctx.request_repaint()` for the
///   next one, an immediate repaint, never a timed one. A result for a revision other than the one
///   last asked for is discarded; a [`LoadedPicture::Blank`] answer is stored with
///   [`TextureCache::put_empty`] and a failure with [`TextureCache::put_failure`].
/// - **Dropping.** In that first call, keys nobody asked for in this pass or the previous one are
///   dropped from the queue ([`JobQueue::reprioritize`]): rows scrolled away, a panel closed. A
///   pass that does not draw the list asks for nothing, so its jobs are dropped at the next pass
///   that draws. Asked keys move to the priority they were last asked with.
/// - **Failures.** A job returns a [`LoadFailure`]; the loader words it when it stores the failure.
///   A job that panics is stored as [`LoadFailure::Panicked`].
/// - **Waking.** The queue wakes the app through `ctx.request_repaint()` when a job finishes; no
///   timed repaint.
/// - **Without lanes** (`lanes == 0`) the first call in a pass runs one job inline
///   ([`JobQueue::run_inline`]`(1)`).
///
/// `TextureLoader<K>` is `Clone + Send + Sync` and lives in the egui context like the cache.
// pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:203-215 (renders nobody wants any
// more are let go of each frame); lightcraft: crates/ui-egui/src/render.rs:847-860 `evict_variants`
// (jobs nobody asked for in the last frames dropped through `reprioritize`).
pub struct TextureLoader<K: TextureKey> {
    cache: TextureCache<K>,
    inner: Arc<Mutex<Inner<K>>>,
}

impl<K: TextureKey> Clone for TextureLoader<K> {
    fn clone(&self) -> Self {
        Self { cache: self.cache.clone(), inner: Arc::clone(&self.inner) }
    }
}

const _: () = {
    const fn send_sync<T: Send + Sync + Clone + 'static>() {}
    send_sync::<TextureLoader<u64>>();
};

#[cfg(not(target_arch = "wasm32"))]
fn default_jobs() -> Arc<dyn Jobs> {
    Arc::new(ThreadJobs)
}

#[cfg(target_arch = "wasm32")]
fn default_jobs() -> Arc<dyn Jobs> {
    Arc::new(InlineJobs)
}

impl<K: TextureKey> TextureLoader<K> {
    /// The loader stored in `ctx` under `id_salt`, made with `options` the first time; its cache is
    /// [`TextureCache::in_context`] under the same salt. The queue's wake is
    /// `ctx.request_repaint()`.
    pub fn in_context(ctx: &Context, id_salt: impl AsIdSalt, options: LoaderOptions) -> Self {
        let id = Id::unique("craft_ui::canvas::TextureLoader").with(&id_salt);
        if let Some(loader) = ctx.data(|data| data.get_temp::<Self>(id)) {
            return loader;
        }
        let cache = TextureCache::in_context(ctx, &id_salt, options.cache);
        let jobs = options.jobs.unwrap_or_else(|| match Host::try_of_ctx(ctx) {
            Some(host) => Arc::clone(host.jobs_shared()),
            None => default_jobs(),
        });
        let wake = {
            let ctx = ctx.clone();
            Wake::new(move || ctx.request_repaint())
        };
        let queue = JobQueue::new(jobs, wake, QueueOptions { lanes: options.lanes, name: "craft-textures" });
        let inner = Inner { lanes: options.lanes, queue, asked: HashMap::new(), polled: u64::MAX, waiting: Vec::new() };
        let loader = Self { cache, inner: Arc::new(Mutex::new(inner)) };
        ctx.data_mut(|data| data.get_temp_mut_or_insert_with(id, || loader).clone())
    }

    /// The loader's cache: [`TextureCache::state`], [`forget`](TextureCache::forget),
    /// [`retain`](TextureCache::retain) and [`statistics`](TextureCache::statistics) work on it as on
    /// any cache.
    pub fn cache(&self) -> &TextureCache<K> {
        &self.cache
    }

    /// What `key` shows for `revision`, queueing the job `job()` makes when the cache has nothing
    /// for this revision and no job for it is queued or running: [`ask`](Self::ask), then
    /// [`submit`](Self::submit). `job` is called only when a job is queued.
    pub fn request<J>(&self, ctx: &Context, key: &K, revision: Revision, priority: Priority, job: impl FnOnce() -> J) -> TextureState
    where
        J: FnOnce(&Cancel) -> Result<LoadedPicture, LoadFailure> + Send + 'static,
    {
        match self.ask(ctx, key, revision, priority) {
            Some(state) => state,
            None => self.submit(ctx, key.clone(), revision, priority, job()),
        }
    }

    /// What `key` shows for `revision` when that is known without a new job: `Some` of the cache's
    /// `Ready`, `Empty` or `Failed`, or `Some(Loading { stand_in })` while a job for this revision
    /// is queued or running; `None` when a job is needed. Marks the key asked for in this pass at
    /// `priority`, so its job is kept.
    pub fn ask(&self, ctx: &Context, key: &K, revision: Revision, priority: Priority) -> Option<TextureState> {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:193 (state of a page: none, pending, ready, failed).
        let mut inner = self.begin_pass(ctx);
        let state = match self.cache.state(ctx, key, revision) {
            settled @ (TextureState::Ready(_) | TextureState::Empty | TextureState::Failed { .. }) => return Some(settled),
            other => other,
        };
        let asked = inner.asked.get_mut(key).filter(|asked| asked.revision == revision)?;
        asked.priority = priority;
        asked.pass = ctx.cumulative_pass_nr();
        Some(TextureState::Loading { stand_in: state.shown() })
    }

    /// Queues `job` making `key`'s picture at `revision`, replacing a queued job of the key, and
    /// returns `Loading` with the key's old picture as the stand-in. Marks the key asked for in this
    /// pass.
    pub fn submit(
        &self,
        ctx: &Context,
        key: K,
        revision: Revision,
        priority: Priority,
        job: impl FnOnce(&Cancel) -> Result<LoadedPicture, LoadFailure> + Send + 'static,
    ) -> TextureState {
        // lightcraft: crates/preview/src/pool.rs:182 `submit` (slot replace) through `JobQueue::submit`.
        let mut inner = self.begin_pass(ctx);
        let stand_in = self.cache.state(ctx, &key, revision).shown();
        inner.asked.insert(key.clone(), Asked { revision, priority, pass: ctx.cumulative_pass_nr() });
        // A panicking job is a failure, not a key that stays Loading for good.
        inner.queue.submit(key, priority, move |cancel| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(cancel))).unwrap_or(Err(LoadFailure::Panicked));
            (revision, result)
        });
        if inner.lanes == 0 {
            // The job runs in the next pass's `begin_pass`; the web repaints only on input.
            ctx.request_repaint();
        }
        TextureState::Loading { stand_in }
    }
}

impl<K: TextureKey> TextureLoader<K> {
    /// Stops the loader's queue: waiting jobs are dropped, running ones are cancelled, and the
    /// call waits up to `timeout` for them to end ([`JobQueue::shutdown`]). Returns whether every
    /// job has. For an app about to exit the process.
    pub fn shutdown(&self, timeout: std::time::Duration) -> bool {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner).queue.shutdown(timeout)
    }

    /// Locks the loader's state and, in the first call of a pass, runs the pass's work: an inline
    /// job when there are no lanes, dropping jobs nobody asked for, then storing finished results
    /// while the cache's upload allowance lasts. A result for another revision than the one last
    /// asked for is discarded.
    fn begin_pass(&self, ctx: &Context) -> MutexGuard<'_, Inner<K>> {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let inner = &mut *guard;
        let pass = ctx.cumulative_pass_nr();
        if inner.polled == pass {
            return guard;
        }
        inner.polled = pass;
        if inner.lanes == 0 {
            inner.queue.run_inline(1);
        }
        let recent = pass.saturating_sub(1);
        let asked = &mut inner.asked;
        for key in inner.queue.reprioritize(|key, _| asked.get(key).filter(|asked| asked.pass >= recent).map(|asked| asked.priority)) {
            asked.remove(&key);
        }
        while let Some(finished) = inner.queue.try_recv() {
            let (revision, result) = finished.result;
            inner.waiting.push((finished.key, revision, result));
        }
        let mut waiting = std::mem::take(&mut inner.waiting).into_iter();
        while let Some((key, revision, result)) = waiting.next() {
            if inner.asked.get(&key).is_none_or(|asked| asked.revision != revision) {
                continue;
            }
            match result {
                Ok(LoadedPicture::Image(image)) => {
                    if !self.cache.allow_upload(ctx, image.pixels.len() * size_of::<Color32>()) {
                        inner.waiting.push((key, revision, Ok(LoadedPicture::Image(image))));
                        inner.waiting.extend(waiting);
                        ctx.request_repaint();
                        break;
                    }
                    self.cache.put(ctx, key.clone(), revision, image);
                }
                Ok(LoadedPicture::Blank) => self.cache.put_empty(ctx, key.clone(), revision),
                Err(failure) => self.cache.put_failure(ctx, key.clone(), revision, failure.words(ctx)),
            }
            inner.asked.remove(&key);
        }
        guard
    }
}
