//! [`FilterGalleryStyle`]: the Filter Gallery's sizes, from tokens.

use craft_ui::theme::UiTokens;

/// The thumbnails and the effect-layer stack: the `filter_gallery` family, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FilterGalleryStyle {
    /// A filter thumbnail's width: `filter_gallery.thumb_width` (80; PhotoCraft's).
    pub thumb_width: f32,
    /// A filter thumbnail's height: `filter_gallery.thumb_height` (56).
    pub thumb_height: f32,
    /// The effect-layer stack's height: `filter_gallery.stack_height` (190).
    pub stack_height: f32,
}

impl FilterGalleryStyle {
    pub fn of(tokens: &UiTokens) -> FilterGalleryStyle {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:702-754 (thumbnails 80 x 56), :832 (list 190).
        let f = &tokens.component.filter_gallery;
        FilterGalleryStyle { thumb_width: f.thumb_width, thumb_height: f.thumb_height, stack_height: f.stack_height }
    }
}
