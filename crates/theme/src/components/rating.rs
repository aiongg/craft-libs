//! Ratings: a row of stars. The star colour is overridable per theme (it defaults to the text
//! colour, as LightCraft's near-text star); the star and cell sizes come from the code, as every
//! size does.

use super::{BySize, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Rating tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RatingTokens {
    /// A lit star while the rating is enabled: `rating.star`, the text colour.
    pub star: Rgba,
    /// Side of a star in a rating row: `rating.star.sm`, `rating.star.md`.
    pub star_size: BySize<f32>,
    /// Side of a star's cell in a rating row: `rating.cell.sm`, `rating.cell.md`.
    pub cell: BySize<f32>,
    /// Side of a star painted on a thumbnail or caption: `rating.badge_star.sm`, `.md`.
    pub badge_star: BySize<f32>,
    /// From one painted star to the next: `rating.badge_step.sm`, `.md`.
    pub badge_step: BySize<f32>,
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
        "star.sm" 0.0..=256.0 => star_size.sm;
        "star.md" 0.0..=256.0 => star_size.md;
        "cell.sm" 0.0..=256.0 => cell.sm;
        "cell.md" 0.0..=256.0 => cell.md;
        "badge_star.sm" 0.0..=256.0 => badge_star.sm;
        "badge_star.md" 0.0..=256.0 => badge_star.md;
        "badge_step.sm" 0.0..=256.0 => badge_step.sm;
        "badge_step.md" 0.0..=256.0 => badge_step.md;
    };

    fn defaults(s: &Semantic<'_>) -> RatingTokens {
        RatingTokens {
            star: s.color.text.normal,
            star_size: BySize { sm: values::RATING_STAR.0, md: values::RATING_STAR.1 },
            cell: BySize { sm: values::RATING_CELL.0, md: values::RATING_CELL.1 },
            badge_star: BySize { sm: values::RATING_BADGE_STAR.0, md: values::RATING_BADGE_STAR.1 },
            badge_step: BySize { sm: values::RATING_BADGE_STEP.0, md: values::RATING_BADGE_STEP.1 },
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
