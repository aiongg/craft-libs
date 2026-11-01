//! Ratings: a row of stars. The star colour is overridable per theme (it defaults to the accent);
//! the star and cell sizes come from the code, as every size does.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Rating tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RatingTokens {
    /// A filled star while the rating is enabled: `rating.star`, the accent.
    pub star: Rgba,
    /// Side of a medium star: `rating.star_md`.
    pub star_md: f32,
    /// Side of a medium star's cell: `rating.cell_md`.
    pub cell_md: f32,
    /// Side of a small star: `rating.star_sm`.
    pub star_sm: f32,
    /// Side of a small star's cell: `rating.cell_sm`.
    pub cell_sm: f32,
}

/// An overridable rating field: the star's colour. Path: `star`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RatingField {
    /// `rating.star`
    Star,
}

impl Family for RatingTokens {
    type Field = RatingField;
    const FIELDS: &'static [RatingField] = &[RatingField::Star];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { RatingTokens;
        "star_md" 0.0..=256.0 => star_md;
        "cell_md" 0.0..=256.0 => cell_md;
        "star_sm" 0.0..=256.0 => star_sm;
        "cell_sm" 0.0..=256.0 => cell_sm;
    };

    fn defaults(s: &Semantic<'_>) -> RatingTokens {
        RatingTokens {
            star: s.color.accent.solid,
            star_md: values::RATING_STAR_MD,
            cell_md: values::RATING_CELL_MD,
            star_sm: values::RATING_STAR_SM,
            cell_sm: values::RATING_CELL_SM,
        }
    }

    fn path(_: RatingField) -> String {
        "star".to_owned()
    }

    fn from_path(path: &str) -> Option<RatingField> {
        (path == "star").then_some(RatingField::Star)
    }

    fn kind(_: RatingField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: RatingField) -> OverrideValue {
        OverrideValue::Color(self.star)
    }

    fn set(&mut self, _: RatingField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            self.star = c;
        }
    }

    /// A star is a glyph beside its neighbours, not text: no pair.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
