//! `BinStyle`.

use craft_ui::canvas::textures::TextureKey;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

use super::MediaBin;

/// The bin's own look; the rest is the table, list, panel and footer families'. The chip and the colours are
/// semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BinStyle {
    /// The bin glyph (`text.muted`, not FilmCraft's orange).
    pub bin_icon: Color32,
    /// The label chip's side (`size.icon.sm`).
    pub chip: f32,
    /// The chip's radius (`shape.radius.sm`).
    pub chip_radius: f32,
    /// Offline names and glyphs (`status.bad.fg`).
    pub offline: Color32,
    /// Other columns' text (`text.muted`).
    pub cell_text: Color32,
    /// The count line (`text.faint`).
    pub count: Color32,
}

impl<K: TextureKey, M: ?Sized> MediaBin<'_, K, M> {
    /// The bin's own look from the tokens.
    pub fn style(tokens: &UiTokens) -> BinStyle {
        let c = &tokens.color;
        BinStyle {
            bin_icon: c.text.muted.to_egui(),
            chip: tokens.size.icon.sm,
            chip_radius: tokens.shape.radius.sm,
            offline: c.status.bad.fg.to_egui(),
            cell_text: c.text.muted.to_egui(),
            count: c.text.faint.to_egui(),
        }
    }
}
