//! Frosted backdrops: a bounded blurred composition of the layers under a surface, and its upload to the texture
//! store.

use super::{CachedTexture, TextureKey, TextureStore};
use egui::{Color32, ColorImage, Context, Rect};

/// The most pixels a source image may have for [`Thumbnail::from_image`].
pub const MAX_SOURCE_PIXELS: usize = 16_777_216;

/// The most texels a composed image may have.
pub const MAX_TEXELS: usize = 262_144;

/// The longest side a composed image may have.
pub const MAX_SIDE: usize = 1024;

/// A source image reduced to blocks of `block` pixels, with `paper` under its transparent pixels.
#[derive(Clone, Debug)]
pub struct Thumbnail {
    _reduced: (),
}

impl Thumbnail {
    /// The reduction of `image` to blocks of `block` pixels over `paper`; `None` for an image over
    /// [`MAX_SOURCE_PIXELS`] or a zero block.
    pub fn from_image(image: &ColorImage, block: usize, paper: Color32) -> Option<Thumbnail> {
        let _ = (image, block, paper);
        todo!("fx.F55")
    }
}

/// One layer of a backdrop: where it lies, its reduced picture and the colour that stands in for a missing one.
#[derive(Clone, Copy, Debug)]
pub struct BackdropLayer<'a> {
    /// Where the layer lies.
    pub rect: Rect,
    /// The layer's reduced picture.
    pub thumbnail: Option<&'a Thumbnail>,
    /// The colour of the layer without a thumbnail.
    pub fallback: Color32,
}

/// The blurred composition of `layers` over `background` for `area`; `None` when the image would pass the limits.
pub fn compose(area: Rect, layers: &[BackdropLayer<'_>], background: Color32) -> Option<ColorImage> {
    let _ = (area, layers, background);
    todo!("fx.F55")
}

/// Puts `image` in `store` under `key`: an equal revision keeps the cached texture, a new one replaces it in place.
/// `None` for an image over [`MAX_TEXELS`] or with a side over [`MAX_SIDE`], without touching the store.
pub fn upload<K: TextureKey>(store: &TextureStore<K>, ctx: &Context, key: K, image: ColorImage) -> Option<CachedTexture> {
    let _ = (store, ctx, key, image);
    todo!("fx.F55")
}
