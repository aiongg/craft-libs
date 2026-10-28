//! Paths: outlines, anchors, direction handles and the path being drawn. Their colours are the
//! semantic `overlay.*` tokens or a host's layer colour; the family holds their geometry, the same
//! at every Size because paths sit on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Path tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BezierTokens {
    /// Width of a path's outline, in points: `bezier.line`.
    pub line: f32,
    /// Width of a selected segment: `bezier.selected_segment`.
    pub selected_segment: f32,
    /// Width of the path being drawn: `bezier.draft_line`.
    pub draft_line: f32,
    /// Side of an anchor square: `bezier.anchor`.
    pub anchor: f32,
    /// Diameter of a direction handle's end: `bezier.control`.
    pub control: f32,
    /// Width of the line from an anchor to a direction handle's end: `bezier.control_line`.
    pub control_line: f32,
    /// Distance from an anchor, handle end or segment within which the pointer takes it: `bezier.hit`.
    pub hit: f32,
    /// Farthest a press may move and still be a click: `bezier.click_slop`.
    pub click_slop: f32,
    /// Largest distance between a curve and the lines it is drawn with: `bezier.flatten`.
    pub flatten: f32,
    /// Dash length of the pen's rubber band: `bezier.rubber_dash`.
    pub rubber_dash: f32,
    /// Gap between the rubber band's dashes: `bezier.rubber_gap`.
    pub rubber_gap: f32,
    /// Side of the first anchor while a press there would close the path: `bezier.close_anchor`.
    pub close_anchor: f32,
    /// Radius of the close cue's ring beside the pointer: `bezier.close_ring`.
    pub close_ring: f32,
    /// Distance across and down from the pointer to the close cue's ring: `bezier.close_offset`.
    pub close_offset: f32,
}

/// Paths have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BezierField {}

impl Family for BezierTokens {
    type Field = BezierField;
    const FIELDS: &'static [BezierField] = &[];

    fn defaults(_: &Semantic<'_>) -> BezierTokens {
        BezierTokens {
            line: values::BEZIER_LINE,
            selected_segment: values::BEZIER_SELECTED_SEGMENT,
            draft_line: values::BEZIER_DRAFT_LINE,
            anchor: values::BEZIER_ANCHOR,
            control: values::BEZIER_CONTROL,
            control_line: values::BEZIER_CONTROL_LINE,
            hit: values::BEZIER_HIT,
            click_slop: values::BEZIER_CLICK_SLOP,
            flatten: values::BEZIER_FLATTEN,
            rubber_dash: values::BEZIER_RUBBER_DASH,
            rubber_gap: values::BEZIER_RUBBER_GAP,
            close_anchor: values::BEZIER_CLOSE_ANCHOR,
            close_ring: values::BEZIER_CLOSE_RING,
            close_offset: values::BEZIER_CLOSE_OFFSET,
        }
    }

    fn path(f: BezierField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<BezierField> {
        None
    }

    fn kind(f: BezierField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: BezierField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: BezierField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
