//! Painted pointers: the crosshair, the selection badge and the distort and skew arrows that
//! egui has no icon for. Their colours are the semantic `overlay.*` tokens; the family holds
//! only their geometry, which no theme overrides. The rotate pointer's geometry is
//! `transform.turn_*`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Painted-pointer tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CursorTokens {
    /// Length of each crosshair arm, from the hotspot: `cursor.crosshair_reach`.
    pub crosshair_reach: f32,
    /// Empty space round the hotspot where the crosshair arms begin: `cursor.crosshair_gap`.
    pub crosshair_gap: f32,
    /// Half the side of a selection badge's glyph: `cursor.badge_half`.
    pub badge_half: f32,
    /// Distance of the badge from the hotspot, right and down, beside a painted crosshair:
    /// `cursor.badge_offset`.
    pub badge_offset: f32,
    /// Distance of the badge right of the hotspot, beside a system pointer: `cursor.badge_beside_x`.
    pub badge_beside_x: f32,
    /// Distance of the badge below the hotspot, beside a system pointer: `cursor.badge_beside_y`.
    pub badge_beside_y: f32,
    /// Length of the distort arrowhead, tip to base: `cursor.dart_length`.
    pub dart_length: f32,
    /// Half the width of the distort arrowhead's base: `cursor.dart_half_width`.
    pub dart_half_width: f32,
    /// Width of the halo outline drawn over the distort arrowhead's edge: `cursor.dart_outline`.
    pub dart_outline: f32,
    /// Distance of each skew arrow from the pointer's axis: `cursor.skew_gap`.
    pub skew_gap: f32,
    /// Half the length of a skew arrow's shaft: `cursor.skew_half_length`.
    pub skew_half_length: f32,
    /// How far a skew arrowhead's tip passes the shaft's end, and its half width: `cursor.skew_head`.
    pub skew_head: f32,
    /// How far a skew arrowhead's base lies back from the shaft's end: `cursor.skew_back`.
    pub skew_back: f32,
    /// Width of the halo outline drawn over a skew arrowhead's edge: `cursor.skew_head_outline`.
    pub skew_head_outline: f32,
}

/// Painted pointers have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CursorField {}

impl Family for CursorTokens {
    type Field = CursorField;
    const FIELDS: &'static [CursorField] = &[];

    fn defaults(_: &Semantic<'_>) -> CursorTokens {
        CursorTokens {
            crosshair_reach: values::CURSOR_CROSSHAIR_REACH,
            crosshair_gap: values::CURSOR_CROSSHAIR_GAP,
            badge_half: values::CURSOR_BADGE_HALF,
            badge_offset: values::CURSOR_BADGE_OFFSET,
            badge_beside_x: values::CURSOR_BADGE_BESIDE.0,
            badge_beside_y: values::CURSOR_BADGE_BESIDE.1,
            dart_length: values::CURSOR_DART_LENGTH,
            dart_half_width: values::CURSOR_DART_HALF_WIDTH,
            dart_outline: values::CURSOR_DART_OUTLINE,
            skew_gap: values::CURSOR_SKEW_GAP,
            skew_half_length: values::CURSOR_SKEW_HALF_LENGTH,
            skew_head: values::CURSOR_SKEW_HEAD,
            skew_back: values::CURSOR_SKEW_BACK,
            skew_head_outline: values::CURSOR_SKEW_HEAD_OUTLINE,
        }
    }

    fn path(f: CursorField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CursorField> {
        None
    }

    fn kind(f: CursorField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CursorField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CursorField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: pointers draw in the semantic `overlay.*` tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
