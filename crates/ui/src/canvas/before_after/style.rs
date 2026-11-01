//! [`BeforeAfterStyle`]: the before and after view's look, from tokens (task TK1).

use egui::{Color32, FontId};

use super::BeforeAfter;
use crate::theme::UiTokens;

/// What [`BeforeAfter`] draws besides the host's pictures.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterStyle {
    /// The divider line's width: `before_after.line` (2, FigCraft `dev_compare.rs:354`).
    pub line: f32,
    /// The divider line's colour: the overlay's light mark, white in every theme
    /// (`overlay.handle_fill`; LightCraft's white line).
    pub line_colour: Color32,
    /// The halo under the line, `handle.halo` wider on each side: `overlay.halo`.
    pub halo: Color32,
    pub halo_width: f32,
    /// The knob's diameter: `before_after.knob` (20, FigCraft `dev_compare.rs:356`).
    pub knob: f32,
    /// The knob's fill and its glyph: `accent.strong` and `text.on_strong`.
    pub knob_fill: Color32,
    pub knob_glyph: Color32,
    /// Between two side-by-side panes: `before_after.gap_side` (12, LightCraft `detail.rs:479`).
    pub gap_side: f32,
    /// Between two stacked panes: `before_after.gap_stacked` (28, LightCraft `detail.rs:486`).
    pub gap_stacked: f32,
    /// Below a caption: `before_after.caption_gap` (`space.s4`).
    pub caption_gap: f32,
    /// The captions: `type.label` in `text.muted`.
    pub caption_font: FontId,
    pub caption_colour: Color32,
}

impl BeforeAfter {
    /// The before and after look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `before_after` lands in task TK1")]
    pub fn style(tokens: &UiTokens) -> BeforeAfterStyle {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:479 (6 each side), :486 (14 each side);
        // figcraft: crates/ui-egui/src/panels/dev_compare.rs:354 (2 pt), :356 (knob radius 10).
        todo!()
    }
}
