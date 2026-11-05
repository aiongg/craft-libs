//! Thumbnails: what a picture thumbnail draws besides the picture, namely the checkerboard
//! behind its transparency, the cell while no picture has arrived and the failure icon. The frame
//! is the row's (`list.thumb.border`). No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Thumbnail tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbnailTokens {
    /// The checkerboard cell that holds the top left corner: `thumbnail.checker.a`. The same
    /// pinned neutral in every theme: a picture's backing is not interface chrome.
    pub checker_a: Rgba,
    /// The other checkerboard cell: `thumbnail.checker.b`.
    pub checker_b: Rgba,
    /// Side of a checkerboard cell: `values::THUMBNAIL_CHECKER_CELL`.
    pub checker_cell: f32,
    /// The cell while no picture has arrived: `surface.sunken`, the field's resting fill.
    pub placeholder: Rgba,
    /// The failure icon: `text.muted`.
    pub failed_icon: Rgba,
}

/// Thumbnails have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThumbnailField {}

impl Family for ThumbnailTokens {
    type Field = ThumbnailField;
    const FIELDS: &'static [ThumbnailField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ThumbnailTokens;
        "checker_cell" 1.0..=32.0 => checker_cell;
    };

    fn defaults(s: &Semantic<'_>) -> ThumbnailTokens {
        let (checker_a, checker_b) = values::SWATCH_CHECKER;
        ThumbnailTokens {
            checker_a,
            checker_b,
            checker_cell: values::THUMBNAIL_CHECKER_CELL,
            placeholder: s.color.surface.sunken,
            failed_icon: s.color.text.muted,
        }
    }

    fn path(f: ThumbnailField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ThumbnailField> {
        None
    }

    fn kind(f: ThumbnailField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ThumbnailField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ThumbnailField, _: OverrideValue) {
        match f {}
    }

    // The colours sit on a picture's backing, which no theme owns.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
