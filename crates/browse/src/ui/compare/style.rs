//! `CompareStyle` from the `tile` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The compare tile's look. The `tile` family.
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
        let t = &tokens.component.tile;
        CompareStyle {
            caption: t.compare_caption,
            margin: t.compare_margin,
            gap: t.compare_gap,
            name_room: t.compare_name_room,
            divider: c.line.divider.to_egui(),
            ring: t.ring_selected.to_egui(),
            ring_halo: t.ring_halo.to_egui(),
            veil: t.veil.to_egui(),
            pending_fill: t.scrim.to_egui(),
            pill_text: t.badge_text.to_egui(),
            failed_fill: c.status.bad.solid.to_egui(),
            failed_text: c.text.on_status.to_egui(),
        }
    }
}
