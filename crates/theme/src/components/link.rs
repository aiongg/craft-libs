//! Link Media: the sizes of the Links panel's status badge and of the Link Media dialog's table,
//! candidate list and preview. Colours are the status and text tokens'. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Link tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LinkTokens {
    /// Side of a status badge: [`values::LINK_BADGE`].
    pub badge: f32,
    /// Width and height of the preview box: [`values::LINK_PREVIEW`].
    pub preview: [f32; 2],
    /// Height of the candidate list: [`values::LINK_CANDIDATES_HEIGHT`].
    pub candidates_height: f32,
    /// Least height of the links table: [`values::LINK_TABLE_MIN`].
    pub table_min: f32,
    /// Greatest height of the links table: [`values::LINK_TABLE_MAX`].
    pub table_max: f32,
    /// The table's share of the window's height before the clamp: [`values::LINK_TABLE_SHARE`].
    pub table_share: f32,
    /// Greatest width of the dialog: [`values::LINK_DIALOG_MAX_WIDTH`].
    pub dialog_max_width: f32,
}

/// Link has no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkField {}

impl Family for LinkTokens {
    type Field = LinkField;
    const FIELDS: &'static [LinkField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { LinkTokens;
        "badge" 8.0..=48.0 => badge;
        "preview.width" 32.0..=640.0 => preview[0];
        "preview.height" 32.0..=640.0 => preview[1];
        "candidates_height" 32.0..=480.0 => candidates_height;
        "table_min" 32.0..=480.0 => table_min;
        "table_max" 32.0..=1024.0 => table_max;
        "table_share" 0.05..=0.9 => table_share;
        "dialog_max_width" 320.0..=1600.0 => dialog_max_width;
    };

    fn defaults(_: &Semantic<'_>) -> LinkTokens {
        LinkTokens {
            badge: values::LINK_BADGE,
            preview: values::LINK_PREVIEW,
            candidates_height: values::LINK_CANDIDATES_HEIGHT,
            table_min: values::LINK_TABLE_MIN,
            table_max: values::LINK_TABLE_MAX,
            table_share: values::LINK_TABLE_SHARE,
            dialog_max_width: values::LINK_DIALOG_MAX_WIDTH,
        }
    }

    fn path(f: LinkField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<LinkField> {
        None
    }

    fn kind(f: LinkField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: LinkField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: LinkField, _: OverrideValue) {
        match f {}
    }

    // Sizes only.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
