//! Style of the `bezier` family.

use craft_theme::Tokens;

/// Largest distance between a curve and the lines it is drawn with.
const FLATTEN_TOLERANCE: f32 = 0.3;
/// Width of the close cue's ring.
const CLOSE_RING_WIDTH: f32 = 3.5;

/// Path geometry that the semantic tokens don't name directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierStyle {
    /// Largest distance between a curve and the lines it is drawn with.
    pub flatten: f32,
    /// Width of the close cue's ring beside the pointer.
    pub close_ring: f32,
    /// Distance across and down from the pointer to the close cue's ring.
    pub close_offset: f32,
}

impl BezierStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        Self { flatten: FLATTEN_TOLERANCE, close_ring: CLOSE_RING_WIDTH, close_offset: tokens.size.handle.md + tokens.space.gap.inline }
    }
}
