//! [`TextureLoader`]: a [`TextureCache`] fed by background jobs on a [`JobQueue`].

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use craft_host::Jobs;
use craft_host::queue::{Cancel, JobQueue, Priority};
use egui::{AsIdSalt, ColorImage, Context};

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
        // new: `TextureCacheOptions::thumbnails()`, `QueueOptions::default_lanes()`, `jobs: None`.
        todo!()
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

/// What a loader job returns: a [`LoadedPicture`], or a message for the failure tooltip.
pub(crate) type LoadResult = Result<LoadedPicture, String>;

/// A key's job in flight: the revision it makes and when it was last asked for.
struct Asked {
    revision: Revision,
    priority: Priority,
    /// The last pass a widget asked for the key.
    pass: u64,
}

struct Inner<K> {
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
///     move |cancel: &Cancel| pdf.render(page, cancel).map(LoadedPicture::from).map_err(|e| e.to_string())
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

impl<K: TextureKey> TextureLoader<K> {
    /// The loader stored in `ctx` under `id_salt`, made with `options` the first time; its cache is
    /// [`TextureCache::in_context`] under the same salt. The queue's wake is
    /// `ctx.request_repaint()`.
    pub fn in_context(ctx: &Context, id_salt: impl AsIdSalt, options: LoaderOptions) -> Self {
        // new: egui memory under `Id::new(("craft_ui::canvas::TextureLoader", id_salt))`; the queue is
        // `JobQueue::new(jobs, Wake::new(move || ctx.request_repaint()), QueueOptions { lanes, name: "craft-textures" })`.
        todo!()
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
        J: FnOnce(&Cancel) -> Result<LoadedPicture, String> + Send + 'static,
    {
        // new: `self.ask(..).unwrap_or_else(|| self.submit(.., job()))`.
        todo!()
    }

    /// What `key` shows for `revision` when that is known without a new job: `Some` of the cache's
    /// `Ready`, `Empty` or `Failed`, or `Some(Loading { stand_in })` while a job for this revision
    /// is queued or running; `None` when a job is needed. Marks the key asked for in this pass at
    /// `priority`, so its job is kept.
    pub fn ask(&self, ctx: &Context, key: &K, revision: Revision, priority: Priority) -> Option<TextureState> {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid/thumbs.rs:193 (state of a page: none, pending, ready, failed).
        todo!()
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
        job: impl FnOnce(&Cancel) -> Result<LoadedPicture, String> + Send + 'static,
    ) -> TextureState {
        // lightcraft: crates/preview/src/pool.rs:182 `submit` (slot replace) through `JobQueue::submit`.
        todo!()
    }
}
