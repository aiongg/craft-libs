//! `PlacesStyle`.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

use super::PlacesTree;

/// The tree's own colours; rows, headings and indent are the list's. Colours only, from the semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PlacesStyle {
    /// The folder glyph (`text.muted`).
    pub icon: Color32,
    /// "(none)", "Loading…" and the headings (`text.faint`).
    pub faint: Color32,
}

impl<K> PlacesTree<'_, K> {
    /// The tree's own colours from `tokens`.
    pub fn style(tokens: &UiTokens) -> PlacesStyle {
        let c = &tokens.color;
        PlacesStyle { icon: c.text.muted.to_egui(), faint: c.text.faint.to_egui() }
    }
}
