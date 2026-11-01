//! Text frame marks: ports, the overset mark and the hidden-character mark sizes. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// FrameMarks tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FrameMarksTokens {
    /// A port's side: [`values::FRAME_MARKS_PORT`].
    pub port: f32,
    /// A port's distance from the frame's corner: [`values::FRAME_MARKS_PORT_INSET`].
    pub port_inset: f32,
    /// The overset mark's arm length: [`values::FRAME_MARKS_OVERSET_ARM`].
    pub overset_arm: f32,
    /// The overset mark's stroke: [`values::FRAME_MARKS_OVERSET_STROKE`].
    pub overset_stroke: f32,
    /// The least hidden-character mark size: [`values::FRAME_MARKS_MARK_MIN`].
    pub mark_min: f32,
    /// The largest hidden-character mark size: [`values::FRAME_MARKS_MARK_MAX`].
    pub mark_max: f32,
    /// The overset mark's colour: `frame_marks.overset`, `status.bad.solid`.
    pub overset: Rgba,
}

/// FrameMarks tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FrameMarksField {}

impl Family for FrameMarksTokens {
    type Field = FrameMarksField;
    const FIELDS: &'static [FrameMarksField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FrameMarksTokens;
        "port" 2.0..=64.0 => port;
        "port_inset" 0.0..=64.0 => port_inset;
        "overset_arm" 1.0..=64.0 => overset_arm;
        "overset_stroke" 0.5..=8.0 => overset_stroke;
        "mark_min" 1.0..=64.0 => mark_min;
        "mark_max" 4.0..=256.0 => mark_max;
    };

    fn defaults(s: &Semantic<'_>) -> FrameMarksTokens {
        let c = s.color;
        FrameMarksTokens {
            port: values::FRAME_MARKS_PORT,
            port_inset: values::FRAME_MARKS_PORT_INSET,
            overset_arm: values::FRAME_MARKS_OVERSET_ARM,
            overset_stroke: values::FRAME_MARKS_OVERSET_STROKE,
            mark_min: values::FRAME_MARKS_MARK_MIN,
            mark_max: values::FRAME_MARKS_MARK_MAX,
            overset: c.status.bad.solid,
        }
    }

    fn path(f: FrameMarksField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FrameMarksField> {
        None
    }

    fn kind(f: FrameMarksField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FrameMarksField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FrameMarksField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
