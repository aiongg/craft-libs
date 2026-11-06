//! Style of the `handle` family.

/// Canvas handles take every value from the semantic tokens (`size.handle`, `stroke.thin`,
/// `size.hit.target`); the style holds nothing of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandleStyle {}

impl HandleStyle {
    /// The handle style for `tokens`.
    pub fn new(_tokens: &craft_theme::Tokens) -> Self {
        Self {}
    }
}
