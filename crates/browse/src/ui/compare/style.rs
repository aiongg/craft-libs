//! `CompareStyle` from the semantic tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The width kept for a tile's name in the caption strip.
const NAME_ROOM: f32 = 110.0;

/// The compare tile's look.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompareStyle {
    /// `size.control.sm`.
    pub caption: f32,
    /// `space.gap.section`.
    pub margin: f32,
    /// `space.gap.section`.
    pub gap: f32,
    /// The room kept for a name.
    pub name_room: f32,
    /// The divider (`line.divider`).
    pub divider: Color32,
    /// The active ring (`accent.solid`).
    pub ring: Color32,
    /// Its halo (`overlay.halo`).
    pub ring_halo: Color32,
    /// The veil (`surface.scrim`).
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
            caption: tokens.size.control.sm,
            margin: tokens.space.gap.section,
            gap: tokens.space.gap.section,
            name_room: NAME_ROOM,
            divider: c.line.divider.to_egui(),
            ring: c.accent.solid.to_egui(),
            ring_halo: c.overlay.halo.to_egui(),
            veil: c.surface.scrim.to_egui(),
            pending_fill: c.surface.scrim.to_egui(),
            pill_text: c.text.on_strong.to_egui(),
            failed_fill: c.status.bad.solid.to_egui(),
            failed_text: c.text.on_status.to_egui(),
        }
    }
}
