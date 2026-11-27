//! Pictures by key on the runtime's image store: the [`TextureCache`], the keyed `TextureStore` over it, the
//! `TextureLoader` whose pictures come from jobs, and `Thumbnails`.

mod limits;
mod loader;
mod store;
mod words;

pub use crate::contract::rt::textures::{TextureCache, TextureLook};
pub use limits::{MAX_TEXTURE, fit_image, reduce_image, texture_factor};
pub use loader::{Cancel, LoadFailure, LoadResult, LoadedPicture, LoaderOptions, Priority, TextureLoader};
pub use store::{CachedTexture, Revision, Serials, TextureBudget, TextureKey, TextureState, TextureStatistics, TextureStore, TextureStoreOptions};
pub use words::TextureWords;

/// The tint that leaves a texture's own colours as they are: every texel is multiplied by white.
#[doc(hidden)]
pub const UNTINTED: egui::Color32 = egui::Color32::from_gray(u8::MAX);

mod thumbnails;
#[cfg(feature = "testing")]
pub use thumbnails::FakeThumbSource;
pub(crate) use thumbnails::cover_uv;
#[cfg(feature = "testing")]
pub(crate) use thumbnails::fake_thumb_image;
pub use thumbnails::{
    StateLook, ThumbFit, ThumbJob, ThumbPaint, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails, fit_rect, thumb_image,
};
