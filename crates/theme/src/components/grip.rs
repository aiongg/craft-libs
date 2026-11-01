//! Grips: the squares and midpoint bars of a selected object. The same at every Size (they sit on the document). Their fills are the semantic `overlay.grip`, `overlay.grip_warm`, `overlay.grip_hot` and `overlay.grip_stroke`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Grips: the squares and midpoint bars of an object's selection.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripTokens {
    /// A square grip's side, a midpoint's width: `grip.size`.
    pub size: f32,
    /// A midpoint grip's height: `grip.midpoint_height`.
    pub midpoint_height: f32,
    /// The outline width: `grip.stroke`.
    pub stroke: f32,
    /// The hit area beyond a grip's bounds: `grip.hit_pad`.
    pub hit_pad: f32,
    /// The warm area beyond a grip's bounds: `grip.warm_pad`.
    pub warm_pad: f32,
}

/// Grips have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GripField {}

impl Family for GripTokens {
    type Field = GripField;
    const FIELDS: &'static [GripField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { GripTokens;
        "size" 0.0..=64.0 => size;
        "midpoint_height" 0.0..=64.0 => midpoint_height;
        "stroke" 0.0..=16.0 => stroke;
        "hit_pad" 0.0..=64.0 => hit_pad;
        "warm_pad" 0.0..=64.0 => warm_pad;
    };

    fn defaults(_: &Semantic<'_>) -> GripTokens {
        GripTokens {
            size: values::GRIP_SIZE,
            midpoint_height: values::GRIP_MIDPOINT_HEIGHT,
            stroke: values::GRIP_STROKE,
            hit_pad: values::GRIP_HIT_PAD,
            warm_pad: values::GRIP_WARM_PAD,
        }
    }

    fn path(f: GripField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GripField> {
        None
    }

    fn kind(f: GripField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GripField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GripField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
