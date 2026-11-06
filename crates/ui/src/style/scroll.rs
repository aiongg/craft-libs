//! Style of the `scroll` family.

use craft_theme::Edge;

const FLOATING_WIDTH: f32 = 4.0;
const HANDLE_OPACITY: f32 = 1.0;
const TRACK_OPACITY: f32 = 0.4;
const THUMB_MIN: f32 = 24.0;

/// Scroll bar values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollStyle {
    /// Panel scroll areas draw a solid bar that takes room beside the content, as a bevelled
    /// theme does, instead of a thin floating one.
    pub solid: bool,
    /// Width of a thin floating bar.
    pub floating_width: f32,
    /// Opacity of the handle while the pointer is on the bar.
    pub handle_opacity: f32,
    /// Opacity of the track while the pointer is on the bar.
    pub track_opacity: f32,
    /// Thickness of a canvas bar and side of the corner square.
    pub canvas_width: f32,
    /// Shortest canvas thumb.
    pub thumb_min: f32,
}

impl ScrollStyle {
    /// The scroll bar style for `tokens`.
    pub fn new(tokens: &craft_theme::Tokens) -> Self {
        Self {
            solid: matches!(tokens.shape.edge, Edge::Bevel { .. }),
            floating_width: FLOATING_WIDTH,
            handle_opacity: HANDLE_OPACITY,
            track_opacity: TRACK_OPACITY,
            canvas_width: tokens.size.icon.sm,
            thumb_min: THUMB_MIN,
        }
    }
}
