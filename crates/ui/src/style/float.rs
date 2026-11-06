//! Style of the `float` family.

use craft_theme::Tokens;

/// Width of a new floating window.
const WIDTH: f32 = 260.0;
/// Height of a new floating window.
const HEIGHT: f32 = 320.0;
/// Least height of a floating window that fits its content.
const MIN_FITTED_HEIGHT: f32 = 120.0;
/// Least width of a floating window.
const MIN_WIDTH: f32 = 40.0;
/// Least height of a floating window's body.
const MIN_BODY_HEIGHT: f32 = 20.0;

/// Sizes of floating windows that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatStyle {
    /// Width of a new floating window.
    pub width: f32,
    /// Height of a new floating window.
    pub height: f32,
    /// Least height of a floating window that fits its content.
    pub min_fitted_height: f32,
    /// Least width of a floating window.
    pub min_width: f32,
    /// Least height of a floating window's body.
    pub min_body_height: f32,
}

impl FloatStyle {
    /// The style for `tokens`.
    pub fn new(_tokens: &Tokens) -> Self {
        Self { width: WIDTH, height: HEIGHT, min_fitted_height: MIN_FITTED_HEIGHT, min_width: MIN_WIDTH, min_body_height: MIN_BODY_HEIGHT }
    }
}
