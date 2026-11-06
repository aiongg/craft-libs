//! Style of the `popup` family.

/// Popups take every value from the semantic tokens (`surface.panel`, `line.border`,
/// `elevation.popup`); the style holds nothing of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupStyle {}

impl PopupStyle {
    /// The popup style for `tokens`.
    pub fn new(_tokens: &craft_theme::Tokens) -> Self {
        Self {}
    }
}
