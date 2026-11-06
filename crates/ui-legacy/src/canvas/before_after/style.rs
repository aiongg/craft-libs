//! [`BeforeAfterStyle`]: the before and after view's look, from tokens.

use egui::{Color32, FontId};

use super::BeforeAfter;
use craft_theme::TypeRole;

use crate::theme::{ToEgui, UiTokens};

/// What [`BeforeAfter`] draws besides the host's pictures.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterStyle {
    /// The divider line's width: `stroke.thick`.
    pub line: f32,
    /// The divider line's colour: the overlay's light mark, white in every theme
    /// (`overlay.handle_fill`).
    pub line_color: Color32,
    /// The halo under the line, `stroke.thin` wider on each side: `overlay.halo`.
    pub halo: Color32,
    /// The halo's width across the line: `stroke.thin` on each side of the line's own width.
    pub halo_width: f32,
    /// The knob's diameter: `size.control.md - space.gap.tight`.
    pub knob: f32,
    /// The knob's fill and its glyph: `accent.strong` and `text.on_strong`.
    pub knob_fill: Color32,
    #[allow(missing_docs)]
    pub knob_glyph: Color32,
    /// Between two side-by-side panes: `space.pad.panel`.
    pub gap_side: f32,
    /// Between two stacked panes: `space.pad.dialog`.
    pub gap_stacked: f32,
    /// Below a caption: `space.gap.tight`.
    pub caption_gap: f32,
    /// The captions: `type.label` in `text.muted`.
    pub caption_font: FontId,
    #[allow(missing_docs)]
    pub caption_color: Color32,
}

impl BeforeAfter {
    /// The before and after look in `tokens`.
    pub fn style(tokens: &UiTokens) -> BeforeAfterStyle {
        let c = &tokens.color;
        BeforeAfterStyle {
            line: tokens.stroke.thick,
            line_color: c.overlay.handle_fill.to_egui(),
            halo: c.overlay.halo.to_egui(),
            halo_width: tokens.stroke.thin,
            knob: tokens.size.control.md - tokens.space.gap.tight,
            knob_fill: c.accent.strong.to_egui(),
            knob_glyph: c.text.on_strong.to_egui(),
            gap_side: tokens.space.pad.panel,
            gap_stacked: tokens.space.pad.dialog,
            caption_gap: tokens.space.gap.tight,
            caption_font: tokens.font(TypeRole::Label),
            caption_color: c.text.muted.to_egui(),
        }
    }
}
