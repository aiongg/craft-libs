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
    };

    fn defaults(_: &Semantic<'_>) -> BlurPinsTokens {
        BlurPinsTokens {
            dot: values::BLUR_PINS_DOT,
            grab: values::BLUR_PINS_GRAB,
            ring: values::BLUR_PINS_RING,
            ring_band: values::BLUR_PINS_RING_BAND,
            handle: values::BLUR_PINS_HANDLE,
            line_grab: values::BLUR_PINS_LINE_GRAB,
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
