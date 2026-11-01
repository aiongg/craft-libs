//! [`WideAngleStyle`]: Adaptive Wide Angle's marks, from tokens (task TK4).

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// The constraints' look: the `wide_angle` family, the same at every Size (the marks sit on the
/// image).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WideAngleStyle {
    /// `wide_angle.line` (2).
    pub line: f32,
    /// The end dots' radius: `wide_angle.end` (3.5).
    pub end: f32,
    /// How near a right-click must pass a curve to open its menu: `wide_angle.hit` (10).
    pub hit: f32,
    /// Free: `accent`; Horizontal: `status.warn.fg`; Vertical: `status.bad.fg` (PhotoCraft
    /// `t.accent`, `t.warning`, `t.danger`).
    pub free: Color32,
    pub horizontal: Color32,
    pub vertical: Color32,
    /// The halo under each line (the cue's contrast against the image): `wide_angle.halo`, black
    /// at 160 alpha, 2.5 wider than the line (as Vanishing Point's).
    pub halo: Color32,
    pub halo_width: f32,
}

impl WideAngleStyle {
    pub fn of(tokens: &UiTokens) -> WideAngleStyle {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:331-356 (2 pt lines, 3.5 pt dots, the
        // three colours).
        let w = &tokens.component.wide_angle;
        WideAngleStyle {
            line: w.line,
            end: w.end,
            hit: w.hit,
            free: tokens.color.accent.solid.to_egui(),
            horizontal: tokens.color.status.warn.fg.to_egui(),
            vertical: tokens.color.status.bad.fg.to_egui(),
            halo: w.halo.to_egui(),
            halo_width: w.halo_width,
        }
    }
}
