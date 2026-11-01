//! Reset dots: the small dot beside a changed setting that resets it. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ResetDot tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResetDotTokens {
    /// Side of the dot's square: `reset_dot.size`.
    pub size: f32,
    /// Radius of the dot: `reset_dot.radius`.
    pub radius: f32,
    /// Corner radius of the square's hover fill: `reset_dot.corner`.
    pub corner: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResetDotField {}

impl Family for ResetDotTokens {
    type Field = ResetDotField;
    const FIELDS: &'static [ResetDotField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ResetDotTokens;
        "size" 0.0..=256.0 => size;
        "radius" 0.0..=64.0 => radius;
        "corner" 0.0..=64.0 => corner;
    };

    fn defaults(_: &Semantic<'_>) -> ResetDotTokens {
        ResetDotTokens { size: values::RESET_DOT_SIZE, radius: values::RESET_DOT_RADIUS, corner: values::RESET_DOT_CORNER }
    }

    fn path(f: ResetDotField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ResetDotField> {
        None
    }

    fn kind(f: ResetDotField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ResetDotField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ResetDotField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
