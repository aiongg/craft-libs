//! Transform boxes and warp meshes: the box line, the reference point, the rotate cursor and the
//! warp grid. Their colours are the semantic `overlay.*` tokens; the family holds their geometry,
//! the same at every Size because they sit on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Transform tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TransformTokens {
    /// Width of the box line and of a warp mesh's patch boundaries, in points: `transform.line`.
    pub line: f32,
    /// Distance from a handle or the reference point within which the pointer takes it: `transform.hit`.
    pub hit: f32,
    /// Distance from a corner, outside the box, within which the rotate cursor shows: `transform.rotate_reach`.
    pub rotate_reach: f32,
    /// Inner edge of the rotate band outside a corner, from the corner: `transform.band_inner`.
    pub band_inner: f32,
    /// Outer edge of the rotate band outside a corner, from the corner: `transform.band_outer`.
    pub band_outer: f32,
    /// Farthest a press outside the box may move and still be a click: `transform.click_slop`.
    pub click_slop: f32,
    /// Radius of the reference point's circle: `transform.pivot_radius`.
    pub pivot_radius: f32,
    /// Length of each arm of the reference point's cross, from its centre: `transform.pivot_arm`.
    pub pivot_arm: f32,
    /// Radius of the rotate cursor's arc: `transform.turn_radius`.
    pub turn_radius: f32,
    /// Width of the rotate cursor's arc: `transform.turn_line`.
    pub turn_line: f32,
    /// Width of the halo on each side of the rotate cursor's arc: `transform.turn_halo`.
    pub turn_halo: f32,
    /// Width of the rule-of-thirds lines inside a one-patch warp mesh: `transform.warp_minor`.
    pub warp_minor: f32,
    /// Side of a warp mesh's anchor squares: `transform.warp_anchor`.
    pub warp_anchor: f32,
    /// Diameter of a warp mesh's handle ends: `transform.warp_control`.
    pub warp_control: f32,
    /// Distance from the pointer within which a warp handle and its arm show: `transform.warp_reveal`.
    pub warp_reveal: f32,
}

/// Transform boxes have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransformField {}

impl Family for TransformTokens {
    type Field = TransformField;
    const FIELDS: &'static [TransformField] = &[];

    fn defaults(_: &Semantic<'_>) -> TransformTokens {
        TransformTokens {
            line: values::TRANSFORM_LINE,
            hit: values::TRANSFORM_HIT,
            rotate_reach: values::TRANSFORM_ROTATE_REACH,
            band_inner: values::TRANSFORM_BAND_INNER,
            band_outer: values::TRANSFORM_BAND_OUTER,
            click_slop: values::TRANSFORM_CLICK_SLOP,
            pivot_radius: values::TRANSFORM_PIVOT_RADIUS,
            pivot_arm: values::TRANSFORM_PIVOT_ARM,
            turn_radius: values::TRANSFORM_TURN_RADIUS,
            turn_line: values::TRANSFORM_TURN_LINE,
            turn_halo: values::TRANSFORM_TURN_HALO,
            warp_minor: values::TRANSFORM_WARP_MINOR,
            warp_anchor: values::TRANSFORM_WARP_ANCHOR,
            warp_control: values::TRANSFORM_WARP_CONTROL,
            warp_reveal: values::TRANSFORM_WARP_REVEAL,
        }
    }

    fn path(f: TransformField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TransformField> {
        None
    }

    fn kind(f: TransformField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TransformField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TransformField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
