//! `CompareStyle` from the semantic tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The width kept for a tile's name in the caption strip.
const NAME_ROOM: f32 = 110.0;
/// The caption strip's height, the margin round the tiles and the gap between them.
const CAPTION: f32 = 26.0;
const MARGIN: f32 = 18.0;
const GAP: f32 = 16.0;

/// The compare tile's look.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompareStyle {
    /// The caption strip's height.
    pub caption: f32,
    /// The margin round the tiles.
    pub margin: f32,
    /// The gap between tiles.
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
            caption: CAPTION,
            margin: MARGIN,
            gap: GAP,
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
