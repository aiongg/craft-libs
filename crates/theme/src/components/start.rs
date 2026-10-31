//! Start screen: the recent-file card grid. The family holds sizes only; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Start tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StartTokens {
    /// Least width of a recent-file card: `start.card_min`.
    pub card_min: f32,
    /// Widest a recent-file card grows: `start.card_max`.
    pub card_max: f32,
}

/// Start tokens have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StartField {}

impl Family for StartTokens {
    type Field = StartField;
    const FIELDS: &'static [StartField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { StartTokens;
        "card_min" 0.0..=1024.0 => card_min;
        "card_max" 0.0..=1024.0 => card_max;
    };

    fn defaults(_: &Semantic<'_>) -> StartTokens {
        StartTokens { card_min: values::START_CARD_MIN, card_max: values::START_CARD_MAX }
    }

    fn path(f: StartField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<StartField> {
        None
    }

    fn kind(f: StartField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: StartField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: StartField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
