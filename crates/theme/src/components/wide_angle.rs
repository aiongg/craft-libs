//! Adaptive Wide Angle's constraint marks. The same at every Size (they sit on the image). The line colours are the semantic `accent`, `status.warn.fg` and `status.bad.fg`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Adaptive Wide Angle's constraint marks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WideAngleTokens {
    /// A constraint line's width: `wide_angle.line`.
    pub line: f32,
    /// The end dots' radius: `wide_angle.end`.
    pub end: f32,
    /// How near a right-click must pass a curve to open its menu: `wide_angle.hit`.
    pub hit: f32,
    /// The extra width of the halo under each line: `wide_angle.halo_width`.
    pub halo_width: f32,
    /// The halo under each line: `wide_angle.halo`. Black at alpha 160 in every theme: it must darken whatever the image shows.
    pub halo: Rgba,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WideAngleField {}

impl Family for WideAngleTokens {
    type Field = WideAngleField;
    const FIELDS: &'static [WideAngleField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { WideAngleTokens;
        "line" 0.0..=16.0 => line;
        "end" 0.5..=32.0 => end;
        "hit" 0.0..=64.0 => hit;
        "halo_width" 0.0..=16.0 => halo_width;
    };

    fn defaults(_: &Semantic<'_>) -> WideAngleTokens {
        WideAngleTokens {
            line: values::WIDE_ANGLE_LINE,
            end: values::WIDE_ANGLE_END,
            hit: values::WIDE_ANGLE_HIT,
            halo_width: values::WIDE_ANGLE_HALO_WIDTH,
            halo: values::WIDE_ANGLE_HALO,
        }
    }

    fn path(f: WideAngleField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<WideAngleField> {
        None
    }

    fn kind(f: WideAngleField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: WideAngleField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: WideAngleField, _: OverrideValue) {
        match f {}
    }

    // The halo is pinned black, not a theme colour; the line colours are semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
