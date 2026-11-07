//! A size in whole pixels.

/// The size of an image or document in whole pixels; `PixelSize::new` builds one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PixelSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelSize {
    /// A size of `width` by `height` pixels.
    pub const fn new(width: u32, height: u32) -> Self {
        PixelSize { width, height }
    }
}
