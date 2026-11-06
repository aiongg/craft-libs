//! [`WideAngleStyle`]: Adaptive Wide Angle's marks, from tokens.

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// How near a right-click must pass a curve to open its menu.
const HIT_REACH: f32 = 10.0;
/// A drag shorter than this many screen points adds no constraint.
const MIN_DRAG: f32 = 6.0;

/// The constraints' look (the marks sit on the image, so they do not follow the Size preset).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WideAngleStyle {
    /// The lines' width: `stroke.thick`.
    pub line: f32,
    /// The end dots' radius: `size.handle.sm / 2 + stroke.thin`.
    pub end: f32,
    /// How near a right-click must pass a curve to open its menu: 10.
    pub hit: f32,
    /// A drag shorter than this many screen points adds no constraint.
    pub min_drag: f32,
    /// Free: `accent`; Horizontal: `status.warn.fg`; Vertical: `status.bad.fg`.
    pub free: Color32,
    #[allow(missing_docs)]
    pub horizontal: Color32,
    #[allow(missing_docs)]
    pub vertical: Color32,
    /// The halo under each line (the cue's contrast against the image): `surface.panel`, a `stroke.thin`
    /// wider than the line on each side (as Vanishing Point's).
    pub halo: Color32,
    #[allow(missing_docs)]
    pub halo_width: f32,
    /// The constraint being drawn: `accent.soft`.
    pub drawing: Color32,
}

impl WideAngleStyle {
    pub(crate) fn of(tokens: &UiTokens) -> WideAngleStyle {
        let stroke = &tokens.stroke;
        WideAngleStyle {
            line: stroke.thick,
            end: tokens.size.handle.sm / 2.0 + stroke.thin,
            hit: HIT_REACH,
            min_drag: MIN_DRAG,
            free: tokens.color.accent.solid.to_egui(),
            horizontal: tokens.color.status.warn.fg.to_egui(),
            vertical: tokens.color.status.bad.fg.to_egui(),
            halo: tokens.color.surface.panel.to_egui(),
            halo_width: stroke.thick + 2.0 * stroke.thin,
            drawing: tokens.color.accent.soft.to_egui(),
        }
    }
}
