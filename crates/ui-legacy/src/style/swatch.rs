//! Style of the `swatch` family.

use craft_theme::{BySize, Tokens};

/// Corner radius a swatch does not exceed.
const RADIUS_MAX: f32 = 4.0;

/// Size and corner of a colour swatch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwatchStyle {
    /// Side of the square.
    pub size: BySize<f32>,
    /// Corner radius: `shape.radius.control`, capped.
    pub radius: f32,
}

impl SwatchStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        SwatchStyle { size: BySize { sm: tokens.size.control.sm, md: tokens.size.control.md }, radius: tokens.shape.radius.control.min(RADIUS_MAX) }
    }
}
