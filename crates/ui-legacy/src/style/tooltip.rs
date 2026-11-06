//! Style of the `tooltip` family.

/// Widest a tooltip grows before its text wraps.
pub const MAX_WIDTH: f32 = 280.0;

/// Size limit of a tooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipStyle {
    /// Widest the tooltip grows.
    pub max_width: f32,
}

impl TooltipStyle {
    /// The style for `tokens`.
    pub fn new(_tokens: &craft_theme::Tokens) -> Self {
        TooltipStyle { max_width: MAX_WIDTH }
    }
}
