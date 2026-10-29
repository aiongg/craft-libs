//! Spinners: the ring of an indeterminate wait (egui's stock spinner, which PhotoCraft draws). Sizes only; the colour is a text role.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};

/// Panel footer tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SpinnerTokens {
    /// Between the box and the ring's centre line, so the ring's radius is half the box less this
    /// (egui's 2 at Compact): `spinner.inset`.
    pub inset: f32,
    /// Stroke width of the ring (egui's 3 at Compact): `spinner.stroke`.
    pub stroke: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpinnerField {}

impl Family for SpinnerTokens {
    type Field = SpinnerField;
    const FIELDS: &'static [SpinnerField] = &[];

    /// Half the inline gap (2 at Compact, 3 at Comfortable) and one more.
    fn defaults(s: &Semantic<'_>) -> SpinnerTokens {
        let inset = s.space.gap.inline / 2.0;
        SpinnerTokens { inset, stroke: inset + 1.0 }
    }

    fn path(f: SpinnerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SpinnerField> {
        None
    }

    fn kind(f: SpinnerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SpinnerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SpinnerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
