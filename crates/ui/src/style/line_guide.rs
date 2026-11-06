//! Style of the `line_guide` family.

use egui::Color32;

use crate::theme::{ToEgui as _, UiTokens};

/// The look of a guide line drawn on the document (the Shift-click line, the Ruler line and its
/// protractor arm): a wide halo under a thin line, from the semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineGuideStyle {
    /// Width of the line in points: `stroke.thin`.
    pub width: f32,
    /// Width of the halo under it: `stroke.medium + 2 * stroke.thin`.
    pub halo: f32,
    /// The line's colour: `overlay.stroke`.
    pub line: Color32,
    /// The halo's colour: `overlay.halo`.
    pub halo_color: Color32,
}

impl LineGuideStyle {
    /// The line guide style for `tokens`.
    pub fn new(tokens: &UiTokens) -> Self {
        let o = &tokens.color.overlay;
        Self {
            width: tokens.stroke.thin,
            halo: tokens.stroke.medium + 2.0 * tokens.stroke.thin,
            line: o.stroke.to_egui(),
            halo_color: o.halo.to_egui(),
        }
    }
}
