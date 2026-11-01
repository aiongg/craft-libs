//! The Filter Gallery's sizes: the filter thumbnails and the effect-layer stack. Panel sizes, the same at both Sizes (the thumbnails show a 80 by 56 render).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Filter Gallery's sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FilterGalleryTokens {
    /// A filter thumbnail's width: `filter_gallery.thumb_width`.
    pub thumb_width: f32,
    /// A filter thumbnail's height: `filter_gallery.thumb_height`.
    pub thumb_height: f32,
    /// The effect-layer stack's height: `filter_gallery.stack_height`.
    pub stack_height: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FilterGalleryField {}

impl Family for FilterGalleryTokens {
    type Field = FilterGalleryField;
    const FIELDS: &'static [FilterGalleryField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FilterGalleryTokens;
        "thumb_width" 16.0..=512.0 => thumb_width;
        "thumb_height" 16.0..=512.0 => thumb_height;
        "stack_height" 24.0..=1024.0 => stack_height;
    };

    fn defaults(_: &Semantic<'_>) -> FilterGalleryTokens {
        FilterGalleryTokens {
            thumb_width: values::FILTER_GALLERY_THUMB_WIDTH,
            thumb_height: values::FILTER_GALLERY_THUMB_HEIGHT,
            stack_height: values::FILTER_GALLERY_STACK_HEIGHT,
        }
    }

    fn path(f: FilterGalleryField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FilterGalleryField> {
        None
    }

    fn kind(f: FilterGalleryField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FilterGalleryField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FilterGalleryField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
