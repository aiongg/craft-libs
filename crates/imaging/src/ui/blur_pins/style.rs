//! [`BlurPinsStyle`]: the pins' marks, from tokens (task TK4).

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// The pins' sizes in screen points (the `blur_pins` family, the same at every Size) and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlurPinsStyle {
    /// A pin's dot radius: `blur_pins.dot` (5).
    pub dot: f32,
    /// How near a press grabs a pin: `.grab` (9).
    pub grab: f32,
    /// The ring's radius and band: `.ring` (24), `.ring_band` (5).
    pub ring: f32,
    pub ring_band: f32,
    /// A shape handle's size: `.handle` (4.5).
    pub handle: f32,
    /// How near a press grabs a Tilt-Shift line: `.line_grab` (5).
    pub line_grab: f32,
    /// The marks' line and fill (`overlay.handle_fill`) and halo (`overlay.halo`).
    pub line: Color32,
    pub halo: Color32,
    /// The ring's amount arc: `accent`.
    pub amount: Color32,
}

impl BlurPinsStyle {
    pub fn of(tokens: &UiTokens) -> BlurPinsStyle {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:44-51 (the sizes), :689 `draw` (the colours).
        let b = &tokens.component.blur_pins;
        BlurPinsStyle {
            dot: b.dot,
            grab: b.grab,
            ring: b.ring,
            ring_band: b.ring_band,
            handle: b.handle,
            line_grab: b.line_grab,
            line: tokens.color.overlay.handle_fill.to_egui(),
            halo: tokens.color.overlay.halo.to_egui(),
            amount: tokens.color.accent.solid.to_egui(),
        }
    }
}
