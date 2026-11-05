//! The Ruler tool: how far its ends reach and the size of their crosses. The same at every Size (the tool sits on the document). Its line is the `line_guide` family's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Ruler tool: the reach of its ends, the arms of their crosses and the gap after each readout.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MeasureTokens {
    /// A press within this many screen points of an end grabs it: `measure.hit`.
    pub hit: f32,
    /// Each arm of an end's cross, in screen points: `measure.handle_arm`.
    pub handle_arm: f32,
    /// The gap after each readout in the options bar: `measure.readout_gap`.
    pub readout_gap: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeasureField {}

impl Family for MeasureTokens {
    type Field = MeasureField;
    const FIELDS: &'static [MeasureField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { MeasureTokens;
        "hit" 0.0..=64.0 => hit;
        "handle_arm" 0.0..=64.0 => handle_arm;
        "readout_gap" 0.0..=64.0 => readout_gap;
    };

    fn defaults(_: &Semantic<'_>) -> MeasureTokens {
        MeasureTokens { hit: values::MEASURE_HIT, handle_arm: values::MEASURE_HANDLE_ARM, readout_gap: values::MEASURE_READOUT_GAP }
    }

    fn path(f: MeasureField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<MeasureField> {
        None
    }

    fn kind(f: MeasureField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: MeasureField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: MeasureField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
