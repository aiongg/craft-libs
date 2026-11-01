//! `FileThumbSource`: a `ThumbSource` over files, decoded by the app's `ThumbDecoder` in jobs.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_ui::canvas::textures::{Revision, ThumbRender, ThumbRequest, ThumbSource};
use egui::ColorImage;

use super::{DecodedThumb, FileThumbKey, ThumbDecoder, ThumbStore};

/// How a [`FileThumbSource`] uses the decoder and the store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileThumbOptions {
    /// Ask [`ThumbDecoder::quick`] first and show its picture until `decode` lands. Default on.
    pub quick_previews: bool,
    /// Save what `decode` makes to the store. Default on.
    pub save_to_store: bool,
}

impl Default for FileThumbOptions {
    fn default() -> Self {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:693 (`request_thumb`: quick preview, then the bucket render).
        todo!()
    }
}

/// Where a key's pictures are: before the first job, after a quick preview, or done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Fresh,
    NeedsFull,
    Done,
}

/// File thumbnails as a [`ThumbSource`]: keyed by [`FileThumbKey`] directly, or by the widget's
/// own key through [`keyed`](Self::keyed). App state, one per library; cloning shares it.
///
/// - `revision`: `Revision::of(&(generation, full))`, where `generation` counts the
///   [`invalidate`](Self::invalidate) calls for the path and [`clear`](Self::clear) calls, and
///   `full` is 1 once a quick preview has landed and the full picture is wanted (0 before, and
///   when the first job already made the full picture). So the first picture is never asked
///   twice, and after a quick preview the cache keeps it as the stand-in while the full job runs.
/// - `render` always returns `ThumbRender::Later`: statting (`craft_platform::watch::FileStamp`),
///   store reads, decoding and saving happen in the job, never on the UI thread. The first job
///   stats the file (missing: `Err` with the "missing" message), loads the store at
///   `StoreKey { len, modified, side }` (a hit is final), else with `quick_previews` asks
///   `quick` (a picture: stage `NeedsFull`, and the job wakes the host so the next pass asks the
///   new revision), else decodes, saves, and is final. The second job (stage `NeedsFull`)
///   decodes and saves.
/// - `ThumbError::Unsupported` ends the job with `Ok(LoadedPicture::Blank)`, which the cache
///   stores as `Empty` (the tile shows the host's own icon); `Failed(message)` is the failure
///   tooltip; `Cancelled` is reported as the queue's
///   cancel. A failure is remembered by the cache for its revision, so it is not retried until
///   the file changes (`invalidate`) (risk 9: the widget asks `Thumbnails::state` before it
///   paints, the source never draws).
#[derive(Clone)]
pub struct FileThumbSource {
    decoder: Arc<dyn ThumbDecoder>,
    store: Option<Arc<dyn ThumbStore>>,
    options: FileThumbOptions,
    generations: Arc<Mutex<HashMap<PathBuf, u64>>>,
    stages: Arc<Mutex<HashMap<FileThumbKey, Stage>>>,
    epoch: Arc<Mutex<u64>>,
}

impl FileThumbSource {
    /// A source over `decoder`, with the app's `store` if it has one.
    pub fn new(decoder: Arc<dyn ThumbDecoder>, store: Option<Arc<dyn ThumbStore>>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:693 (`request_thumb`).
        todo!()
    }

    /// Replaces the options.
    #[must_use]
    pub fn options(self, options: FileThumbOptions) -> Self {
        // new: builder.
        todo!()
    }

    /// The source seen through the widget's key: `locate` turns a tile key into the file's key
    /// (`None`: no file, the tile shows no picture). Built per pass; borrows `self`.
    pub fn keyed<K, F: Fn(&K) -> Option<FileThumbKey>>(&self, locate: F) -> KeyedFileThumbs<'_, F> {
        // new: per-pass adapter (thumbs spec, call site 3).
        todo!()
    }

    /// `path` changed on disk: every variant's revision moves on, so the next pass decodes again
    /// and shows the old picture until the new one lands.
    pub fn invalidate(&self, path: &Path) {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:759 (`skips_the_real_render`: a fresh render replaces the embedded one).
        todo!()
    }

    /// Another library opened: every revision moves on and the stages are forgotten.
    pub fn clear(&self) {
        // new: one call for a library switch.
        todo!()
    }
}

/// Turns a finished [`DecodedThumb`] into the picture the cache uploads
/// (`LoadedPicture::Image`; an unsupported file is `LoadedPicture::Blank`, never a picture).
fn to_image(thumb: DecodedThumb) -> ColorImage {
    // new: `ColorImage::from_rgba_unmultiplied`.
    todo!()
}

impl ThumbSource<FileThumbKey> for FileThumbSource {
    fn revision(&self, key: &FileThumbKey, request: &ThumbRequest) -> Revision {
        // new: as the type's doc says.
        todo!()
    }

    fn render(&self, key: &FileThumbKey, request: &ThumbRequest) -> ThumbRender {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:693 (`request_thumb`, now always a job).
        todo!()
    }
}

/// [`FileThumbSource::keyed`]: a [`ThumbSource`] over the widget's key.
pub struct KeyedFileThumbs<'a, F> {
    source: &'a FileThumbSource,
    locate: F,
}

impl<K, F: Fn(&K) -> Option<FileThumbKey>> ThumbSource<K> for KeyedFileThumbs<'_, F> {
    /// The located key's revision; `Revision(0)` when `locate` gives `None`.
    fn revision(&self, key: &K, request: &ThumbRequest) -> Revision {
        // new: delegates.
        todo!()
    }

    /// The located key's render; `ThumbRender::Blank` when `locate` gives `None`.
    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender {
        // new: delegates.
        todo!()
    }
}
