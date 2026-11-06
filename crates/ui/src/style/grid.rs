//! Style of the `grid` family.

/// Grid values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridStyle {
    /// Minor lines' share of the colour.
    pub minor_alpha: f32,
    /// Smallest step in screen points at which an axis draws.
    pub min_step: f32,
    /// Smallest major spacing at which Crossings draw.
    pub crossing_min: f32,
}

impl GridStyle {
    /// The grid style for `tokens`.
    pub fn new(_tokens: &craft_theme::Tokens) -> Self {
        Self { minor_alpha: 0.45, min_step: 6.0, crossing_min: 8.0 }
    }
}
