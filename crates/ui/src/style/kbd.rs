//! Style of the `kbd` family.

use craft_theme::{Tokens, TypeRole};

/// Vertical padding of a keycap.
const PAD_Y: f32 = 1.0;
/// Corner radius a keycap does not exceed.
const RADIUS_MAX: f32 = 3.0;

/// Size and corner of a keycap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KbdStyle {
    /// Height: the caption line plus the vertical padding.
    pub height: f32,
    /// Corner radius: `shape.radius.control`, capped.
    pub radius: f32,
}

impl KbdStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        KbdStyle {
            height: tokens.typography.get(TypeRole::Caption).line_height + 2.0 * PAD_Y,
            radius: tokens.shape.radius.control.min(RADIUS_MAX),
        }
    }
}
