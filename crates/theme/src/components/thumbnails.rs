//! The page thumbnails panel and strip: cell sizes and the outlines of the current and picked pages. No theme overrides it.

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
    /// A cell's padding around its thumbnail: [`values::THUMBNAILS_CELL_PAD`].
    pub cell_pad: f32,
    /// Between rows of cells: [`values::THUMBNAILS_ROW_GAP`].
    pub row_gap: f32,
    /// The current page's border: [`values::THUMBNAILS_CURRENT_BORDER`].
    pub current_border: f32,
    /// The border of the other pages: [`values::THUMBNAILS_BORDER`].
    pub border: f32,
    /// A picked page's outline: [`values::THUMBNAILS_PICKED_OUTLINE`].
    pub picked_outline: f32,
    /// The drop bar's width: [`values::THUMBNAILS_DROP_BAR`].
    pub drop_bar: f32,
    /// The outline round the current and picked slides of a strip: [`values::THUMBNAILS_STRIP_OUTLINE`].
    pub strip_outline: f32,
    /// How far outward the strip outline sits: [`values::THUMBNAILS_STRIP_OUTLINE_OFFSET`].
    pub strip_outline_offset: f32,
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
        "cell_pad" 0.0..=64.0 => cell_pad;
        "row_gap" 0.0..=64.0 => row_gap;
        "current_border" 0.0..=16.0 => current_border;
        "border" 0.0..=16.0 => border;
        "picked_outline" 0.0..=16.0 => picked_outline;
        "drop_bar" 0.0..=16.0 => drop_bar;
        "strip_outline" 0.0..=16.0 => strip_outline;
        "strip_outline_offset" 0.0..=16.0 => strip_outline_offset;
    };

    fn defaults(_: &Semantic<'_>) -> ThumbnailsTokens {
        ThumbnailsTokens {
            usual_width: values::THUMBNAILS_USUAL_WIDTH,
            panel_inset: values::THUMBNAILS_PANEL_INSET,
            cell_pad: values::THUMBNAILS_CELL_PAD,
            row_gap: values::THUMBNAILS_ROW_GAP,
            current_border: values::THUMBNAILS_CURRENT_BORDER,
            border: values::THUMBNAILS_BORDER,
            picked_outline: values::THUMBNAILS_PICKED_OUTLINE,
            drop_bar: values::THUMBNAILS_DROP_BAR,
            strip_outline: values::THUMBNAILS_STRIP_OUTLINE,
            strip_outline_offset: values::THUMBNAILS_STRIP_OUTLINE_OFFSET,
        }
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
