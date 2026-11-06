//! [`FilterGalleryStyle`]: the Filter Gallery's sizes, from tokens.

use craft_ui::theme::UiTokens;

/// A filter thumbnail's height, as a share of its width.
const THUMB_HEIGHT_SHARE: f32 = 0.7;
/// The effect-layer stack's height, in control heights.
const STACK_ROWS: f32 = 8.0;

/// The thumbnails and the effect-layer stack.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FilterGalleryStyle {
    /// A filter thumbnail's width: `size.thumb.lg`.
    pub thumb_width: f32,
    /// A filter thumbnail's height: 0.7 of its width.
    pub thumb_height: f32,
    /// The effect-layer stack's height: 8 control heights.
    pub stack_height: f32,
}

impl FilterGalleryStyle {
    /// The gallery's sizes from the theme `tokens`.
    pub fn of(tokens: &UiTokens) -> FilterGalleryStyle {
        let thumb_width = tokens.size.thumb.lg;
        FilterGalleryStyle {
            thumb_width,
            thumb_height: thumb_width * THUMB_HEIGHT_SHARE,
            stack_height: tokens.size.control.md * STACK_ROWS,
        }
    }
}
