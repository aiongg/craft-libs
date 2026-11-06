//! Style of the `rail` family.

use craft_theme::Tokens;

/// Fraction of the tool button that the glyph fills.
const TOOL_ICON_SHARE: f32 = 2.0 / 3.0;
/// Fraction of the panel button that the glyph fills.
const PANEL_ICON_SHARE: f32 = 0.52;

/// Sizes of the rails that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailStyle {
    /// Width of the panel rail.
    pub panel_width: f32,
    /// Glyph of a tool-rail button.
    pub icon: f32,
    /// Glyph of a panel-rail button.
    pub panel_icon: f32,
}

impl RailStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let (space, size) = (&tokens.space, &tokens.size);
        Self {
            panel_width: size.control.md + 2.0 * space.gap.tight,
            icon: (size.rail.button * TOOL_ICON_SHARE).round(),
            panel_icon: (size.control.md * PANEL_ICON_SHARE).round(),
        }
    }
}
