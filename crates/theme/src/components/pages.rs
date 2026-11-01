//! The page view: the spacing between pages, the margin above and below them and the side gutter, and what Hide White Space leaves. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Pages tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PagesTokens {
    /// Between pages, and between rows: [`values::PAGES_GAP`].
    pub gap: f32,
    /// Above the first and below the last row: [`values::PAGES_MARGIN`].
    pub margin: f32,
    /// Kept clear at each side of the pages: [`values::PAGES_SIDE`].
    pub side: f32,
    /// What Hide White Space leaves above and below each page's text area: [`values::PAGES_WHITE_SPACE_PAD`].
    pub white_space_pad: f32,
    /// The seam Hide White Space draws between pages: [`values::PAGES_SEAM`].
    pub seam: f32,
}

/// Pages tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PagesField {}

impl Family for PagesTokens {
    type Field = PagesField;
    const FIELDS: &'static [PagesField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PagesTokens;
        "gap" 0.0..=256.0 => gap;
        "margin" 0.0..=256.0 => margin;
        "side" 0.0..=512.0 => side;
        "white_space_pad" 0.0..=64.0 => white_space_pad;
        "seam" 0.0..=16.0 => seam;
    };

    fn defaults(_: &Semantic<'_>) -> PagesTokens {
        PagesTokens {
            gap: values::PAGES_GAP,
            margin: values::PAGES_MARGIN,
            side: values::PAGES_SIDE,
            white_space_pad: values::PAGES_WHITE_SPACE_PAD,
            seam: values::PAGES_SEAM,
        }
    }

    fn path(f: PagesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PagesField> {
        None
    }

    fn kind(f: PagesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PagesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PagesField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
