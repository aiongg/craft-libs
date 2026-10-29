//! Rulers: the strips along the top and left of a canvas. They draw in `surface.chrome`,
//! `line.divider` and the text colours; the family holds their geometry, the same at every Size
//! and Text size (a 16 point strip has no room for larger labels).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Ruler tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RulerTokens {
    /// Thickness of a ruler strip and side of the corner square, in points: `ruler.size`.
    pub size: f32,
    /// Label font size: `ruler.label`.
    pub label: f32,
    /// Between the characters of the left ruler's stacked labels: `ruler.label_pitch`.
    pub label_pitch: f32,
    /// Between a top-ruler label and its tick: `ruler.label_pad`.
    pub label_pad: f32,
    /// Least distance between labelled ticks: `ruler.step_min`.
    pub step_min: f32,
    /// Length of the tick halfway between labelled ticks: `ruler.tick_mid`.
    pub tick_mid: f32,
    /// Length of the other minor ticks: `ruler.tick_minor`.
    pub tick_minor: f32,
}

/// Rulers have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RulerField {}

impl Family for RulerTokens {
    type Field = RulerField;
    const FIELDS: &'static [RulerField] = &[];

    fn defaults(_: &Semantic<'_>) -> RulerTokens {
        RulerTokens {
            size: values::RULER_SIZE,
            label: values::RULER_LABEL,
            label_pitch: values::RULER_LABEL_PITCH,
            label_pad: values::RULER_LABEL_PAD,
            step_min: values::RULER_STEP_MIN,
            tick_mid: values::RULER_TICK_MID,
            tick_minor: values::RULER_TICK_MINOR,
        }
    }

    fn path(f: RulerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<RulerField> {
        None
    }

    fn kind(f: RulerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: RulerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: RulerField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
