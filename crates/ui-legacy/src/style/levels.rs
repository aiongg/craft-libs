//! Style of the `levels` family.

use craft_theme::Tokens;

/// Histogram height in control heights.
const HISTOGRAM_CONTROLS: f32 = 4.6;
/// Output gradient bar thickness in control heights.
const BAR_CONTROLS: f32 = 0.42;

/// Sizes of a levels editor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelsStyle {
    /// Least width.
    pub min_width: f32,
    /// Greatest width.
    pub max_width: f32,
    /// Width of a triangle thumb.
    pub thumb_width: f32,
    /// Height of the histogram.
    pub histogram: f32,
    /// Thickness of the output gradient bar.
    pub bar: f32,
}

impl LevelsStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let size = &tokens.size;
        LevelsStyle {
            min_width: size.control.md * 5.0,
            max_width: size.control.md * 12.5,
            thumb_width: size.handle.md + size.handle.sm / 2.0,
            histogram: (size.control.md * HISTOGRAM_CONTROLS).round(),
            bar: (size.control.md * BAR_CONTROLS).round(),
        }
    }
}
