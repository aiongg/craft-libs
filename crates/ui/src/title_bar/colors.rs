//! The caption buttons' colours.

use egui::Color32;

/// What the caption buttons paint in the app's colours. The app fills it from its theme. The OS
/// reds of the Windows and Breeze close buttons are fixed and not part of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptionColors {
    /// The glyphs in the active window.
    pub glyph: Color32,
    /// The glyphs while the window is inactive, as Windows dims its captions.
    pub glyph_inactive: Color32,
    /// The fill under the pointer: minimize and maximize, and close too on Adwaita and Generic.
    pub hover: Color32,
    /// The fill while a button is held down.
    pub pressed: Color32,
    /// The keyboard focus ring.
    pub focus: Color32,
}
