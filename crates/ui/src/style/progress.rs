//! Style of the `progress` family.

use craft_theme::Tokens;

/// Default width of a progress bar in control heights.
const WIDTH_IN_CONTROLS: f32 = 5.0;

/// Size of a progress bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressStyle {
    /// Default width.
    pub width: f32,
    /// Thickness of the track: two thick strokes and a thin one.
    pub track_height: f32,
}

impl ProgressStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        ProgressStyle {
            width: tokens.size.control.md * WIDTH_IN_CONTROLS,
            track_height: 2.0 * tokens.stroke.thick + tokens.stroke.thin,
        }
    }
}
