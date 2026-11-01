//! Textures for every picture an app supplies: the device limit ([`MAX_TEXTURE`],
//! [`fit_image`]), a keyed cache with budgets ([`TextureCache`]), background loading
//! ([`TextureLoader`]) and the thumbnail contract ([`Thumbnails`] over a [`ThumbSource`]).
//!
//! One rule for every consumer of pixels: the app supplies pixels at once or as a job, keyed by a
//! key and a [`Revision`]; the library uploads, caches, evicts and asks for a redraw. The app
//! never holds a `TextureHandle`, never counts bytes and never polls on a timer.
//!
//! | Need | Use |
//! |---|---|
//! | Rows and grids of thumbnails (layers, channels, snapshots, files) | [`Thumbnails`] with the app's [`ThumbSource`]; widgets take `.thumbnails(&thumbs, &source)` |
//! | A picture computed once per revision (a preset's pattern) | [`TextureCache::get_or_put`] inside the existing paint closure |
//! | Frames, pages, a navigator image | [`TextureCache::put`], [`TextureCache::put_region`] for damage |
//! | Pages or frames made in the background | [`TextureLoader::request`] |
//! | Stories and tests | [`FakeThumbSource`] with [`craft_host::ManualJobs`] in [`ThumbnailOptions::jobs`] |
//!
//! [`Priority`] is `craft_host::queue::Priority`, re-exported for crates that reach the host only
//! through craft-ui.

mod cache;
#[cfg(test)]
mod cache_tests;
pub(crate) mod limits;
mod loader;
#[cfg(test)]
mod loader_tests;
pub(crate) mod stories;
mod style;
mod thumb;
#[cfg(test)]
mod thumb_tests;

pub use cache::{CachedTexture, Revision, TextureBudget, TextureCache, TextureCacheOptions, TextureKey, TextureState, TextureStatistics};
pub use craft_host::queue::Priority;
pub use limits::{DetailPlan, MAX_TEXTURE, fit_image, texture_factor, texture_side};
pub use loader::{LoaderOptions, TextureLoader};
pub use style::ThumbnailStyle;
pub use thumb::{
    FakeThumbSource, ThumbFit, ThumbJob, ThumbPaint, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails, fit_rect, thumb_image,
};
