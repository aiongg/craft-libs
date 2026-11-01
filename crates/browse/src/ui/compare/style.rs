//! `CompareStyle` from the `tile` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The compare tile's look. Until task TK3 the values come from the tokens they will derive
/// from and the spec's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompareStyle {
    /// `tile.compare_caption` (26).
    pub caption: f32,
    /// `tile.compare_margin` (18).
    pub margin: f32,
    /// `tile.compare_gap` (16).
    pub gap: f32,
    /// `tile.compare_name_room` (110).
    pub name_room: f32,
    /// The divider (`line.divider`).
    pub divider: Color32,
    /// The active ring (`tile.ring.selected`).
    pub ring: Color32,
    /// Its halo (`tile.ring.halo`).
    pub ring_halo: Color32,
    /// The veil (`tile.veil`).
    pub veil: Color32,
    /// The Rendering pill (`surface.scrim`) and its text (`text.on_strong`).
    pub pending_fill: Color32,
    /// The pill text.
    pub pill_text: Color32,
    /// The Render failed pill (`status.bad.solid`) and its text (`text.on_status`).
    pub failed_fill: Color32,
    /// The failed pill's text.
    pub failed_text: Color32,
}

impl CompareStyle {
    /// The style of `tokens`.
    pub fn of(tokens: &UiTokens) -> CompareStyle {
        let c = &tokens.color;
        CompareStyle {
            caption: 26.0,
            margin: 18.0,
            gap: 16.0,
            name_room: 110.0,
            divider: c.line.divider.to_egui(),
            ring: c.accent.solid.to_egui(),
            ring_halo: c.overlay.halo.to_egui(),
            veil: c.surface.scrim.to_egui().gamma_multiply(0.45),
            pending_fill: c.surface.scrim.to_egui(),
            pill_text: c.text.on_strong.to_egui(),
            failed_fill: c.status.bad.solid.to_egui(),
            failed_text: c.text.on_status.to_egui(),
        }
    }
}
