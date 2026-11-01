//! The blur pins' marks: dots, the amount ring, shape handles and the grab reach. The same at every Size (they sit on the image). The colours are the semantic `overlay.handle_fill`, `overlay.halo` and `accent`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The blur pins' marks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlurPinsTokens {
    /// A pin's dot radius: `blur_pins.dot`.
    pub dot: f32,
    /// How near a press grabs a pin: `blur_pins.grab`.
    pub grab: f32,
    /// The ring's radius: `blur_pins.ring`.
    pub ring: f32,
    /// The ring's band width: `blur_pins.ring_band`.
    pub ring_band: f32,
    /// A shape handle's radius: `blur_pins.handle`.
    pub handle: f32,
    /// How near a press grabs a Tilt-Shift line: `blur_pins.line_grab`.
    pub line_grab: f32,
    /// A mark's line width: `blur_pins.line_width`.
    pub line_width: f32,
    /// The halo under a mark's line: `blur_pins.line_halo`.
    pub line_halo: f32,
    /// The amount arc's width: `blur_pins.arc`.
    pub arc: f32,
    /// The halo under the amount arc: `blur_pins.arc_halo`.
    pub arc_halo: f32,
    /// A dash of a Tilt-Shift transition line: `blur_pins.dash`.
    pub dash: f32,
    /// The gap between dashes: `blur_pins.dash_gap`.
    pub dash_gap: f32,
    /// A handle's outline width: `blur_pins.handle_outline`.
    pub handle_outline: f32,
    /// How much larger than a handle a feather dot is: `blur_pins.feather_extra`.
    pub feather_extra: f32,
    /// How much larger than a handle a Tilt-Shift turn dot is: `blur_pins.turn_extra`.
    pub turn_extra: f32,
    /// The rounding square's corner radius: `blur_pins.round_corner`.
    pub round_corner: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlurPinsField {}

impl Family for BlurPinsTokens {
    type Field = BlurPinsField;
    const FIELDS: &'static [BlurPinsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BlurPinsTokens;
        "dot" 0.5..=32.0 => dot;
        "grab" 0.0..=64.0 => grab;
        "ring" 1.0..=128.0 => ring;
        "ring_band" 0.0..=64.0 => ring_band;
        "handle" 0.5..=32.0 => handle;
        "line_grab" 0.0..=64.0 => line_grab;
        "line_width" 0.0..=64.0 => line_width;
        "line_halo" 0.0..=64.0 => line_halo;
        "arc" 0.0..=64.0 => arc;
        "arc_halo" 0.0..=64.0 => arc_halo;
        "dash" 0.0..=64.0 => dash;
        "dash_gap" 0.0..=64.0 => dash_gap;
        "handle_outline" 0.0..=64.0 => handle_outline;
        "feather_extra" 0.0..=64.0 => feather_extra;
        "turn_extra" 0.0..=64.0 => turn_extra;
        "round_corner" 0.0..=64.0 => round_corner;
    };

    fn defaults(_: &Semantic<'_>) -> BlurPinsTokens {
        BlurPinsTokens {
            dot: values::BLUR_PINS_DOT,
            grab: values::BLUR_PINS_GRAB,
            ring: values::BLUR_PINS_RING,
            ring_band: values::BLUR_PINS_RING_BAND,
            handle: values::BLUR_PINS_HANDLE,
            line_grab: values::BLUR_PINS_LINE_GRAB,
            line_width: values::BLUR_PINS_LINE_WIDTH,
            line_halo: values::BLUR_PINS_LINE_HALO,
            arc: values::BLUR_PINS_ARC,
            arc_halo: values::BLUR_PINS_ARC_HALO,
            dash: values::BLUR_PINS_DASH,
            dash_gap: values::BLUR_PINS_DASH_GAP,
            handle_outline: values::BLUR_PINS_HANDLE_OUTLINE,
            feather_extra: values::BLUR_PINS_FEATHER_EXTRA,
            turn_extra: values::BLUR_PINS_TURN_EXTRA,
            round_corner: values::BLUR_PINS_ROUND_CORNER,
        }
    }

    fn path(f: BlurPinsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<BlurPinsField> {
        None
    }

    fn kind(f: BlurPinsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: BlurPinsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: BlurPinsField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
