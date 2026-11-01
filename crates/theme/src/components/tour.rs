//! Tours: the coach marks and the card of a guided tour. The family holds sizes only; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Tour tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TourTokens {
    /// Width of the card's contents: `tour.card_width`.
    pub card_width: f32,
    /// Between the highlighted target (at its widest pulse) and the card: `tour.target_gap`.
    pub target_gap: f32,
    /// How far the highlight stands off the target at rest: `tour.highlight_offset`.
    pub highlight_offset: f32,
    /// How far the pulse adds to the highlight offset: `tour.highlight_pulse`.
    pub highlight_pulse: f32,
}

/// Tour tokens have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TourField {}

impl Family for TourTokens {
    type Field = TourField;
    const FIELDS: &'static [TourField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TourTokens;
        "card_width" 0.0..=1024.0 => card_width;
        "target_gap" 0.0..=1024.0 => target_gap;
        "highlight_offset" 0.0..=1024.0 => highlight_offset;
        "highlight_pulse" 0.0..=1024.0 => highlight_pulse;
    };

    fn defaults(_: &Semantic<'_>) -> TourTokens {
        TourTokens {
            card_width: values::TOUR_CARD_WIDTH,
            target_gap: values::TOUR_TARGET_GAP,
            highlight_offset: values::TOUR_HIGHLIGHT_OFFSET,
            highlight_pulse: values::TOUR_HIGHLIGHT_PULSE,
        }
    }

    fn path(f: TourField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TourField> {
        None
    }

    fn kind(f: TourField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TourField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TourField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
