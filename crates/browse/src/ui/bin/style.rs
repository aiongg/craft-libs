//! `BinStyle`.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The bin's own look; the rest is the table, list, panel and footer families'. Until task TK3
/// the values come from the tokens they will derive from.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BinStyle {
    /// The bin glyph (`text.muted`, not FilmCraft's orange).
    pub bin_icon: Color32,
    /// The label chip's side: 12.
    pub chip: f32,
    /// The chip's radius: 2.5.
    pub chip_radius: f32,
    /// Offline names and glyphs (`status.bad.fg`).
    pub offline: Color32,
    /// Other columns' text (`text.muted`).
    pub cell_text: Color32,
    /// The count line (`text.faint`).
    pub count: Color32,
}

impl BinStyle {
    /// The style of `tokens`.
    pub fn of(tokens: &UiTokens) -> BinStyle {
        let c = &tokens.color;
        BinStyle {
            bin_icon: c.text.muted.to_egui(),
            chip: 12.0,
            chip_radius: 2.5,
            offline: c.status.bad.fg.to_egui(),
            cell_text: c.text.muted.to_egui(),
            count: c.text.faint.to_egui(),
        }
    }
}
