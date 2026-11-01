//! The page thumbnails panel and strip: the card width and the panel's inset (rings, insert line and spacing are the tile grid's `tile.*`). No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Thumbnails tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbnailsTokens {
    /// The thumbnail width at zoom 1: [`values::THUMBNAILS_USUAL_WIDTH`].
    pub usual_width: f32,
    /// What the panel keeps free beside the thumbnails: [`values::THUMBNAILS_PANEL_INSET`].
    pub panel_inset: f32,
}

/// Thumbnails tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThumbnailsField {}

impl Family for ThumbnailsTokens {
    type Field = ThumbnailsField;
    const FIELDS: &'static [ThumbnailsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ThumbnailsTokens;
        "usual_width" 16.0..=1024.0 => usual_width;
        "panel_inset" 0.0..=256.0 => panel_inset;
    };

    fn defaults(_: &Semantic<'_>) -> ThumbnailsTokens {
        ThumbnailsTokens { usual_width: values::THUMBNAILS_USUAL_WIDTH, panel_inset: values::THUMBNAILS_PANEL_INSET }
    }

    fn path(f: ThumbnailsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ThumbnailsField> {
        None
    }

    fn kind(f: ThumbnailsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ThumbnailsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ThumbnailsField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
