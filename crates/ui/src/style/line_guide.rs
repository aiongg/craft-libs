//! Style of the `line_guide` family.

/// Line guide values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineGuideStyle {
    /// The halo's width, under the line.
    pub halo: f32,
}

impl LineGuideStyle {
    /// The line guide style for `tokens`.
    pub fn new(tokens: &craft_theme::Tokens) -> Self {
        Self { halo: tokens.stroke.medium + 2.0 * tokens.stroke.thin }
    }
}
