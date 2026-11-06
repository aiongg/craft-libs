//! [`BlurPinsStyle`]: the pins' marks, from tokens.

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// The ring's band width.
const RING_BAND_WIDTH: f32 = 5.0;
/// How far a feather handle sits beyond its shape.
const FEATHER_EXTRA: f32 = 1.0;
/// How far a turn handle sits beyond its shape.
const TURN_EXTRA: f32 = 0.5;
/// The corner radius of a rounded shape handle.
const ROUND_CORNER: f32 = 1.0;

/// The pins' sizes in screen points and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlurPinsStyle {
    /// A pin's dot radius: `size.handle.sm / 2`.
    pub dot: f32,
    /// How near a press grabs a pin: `size.hit.target`.
    pub grab: f32,
    /// The ring's radius and band: `size.control.sm`, 5.
    pub ring: f32,
    #[allow(missing_docs)]
    pub ring_band: f32,
    /// A shape handle's size: `size.handle.sm / 2`.
    pub handle: f32,
    /// How near a press grabs a Tilt-Shift line: `size.hit.target`.
    pub line_grab: f32,
    /// `stroke.medium`.
    pub line_width: f32,
    /// `stroke.medium` plus a `stroke.thin` halo on each side.
    pub line_halo: f32,
    /// `stroke.thick`.
    pub arc: f32,
    /// `stroke.thick` plus a `stroke.thin` halo on each side.
    pub arc_halo: f32,
    /// `stroke.dash_gap`.
    pub dash: f32,
    /// `stroke.dash_gap`.
    pub dash_gap: f32,
    /// `stroke.thin`.
    pub handle_outline: f32,
    /// How far a feather handle sits beyond its shape.
    pub feather_extra: f32,
    /// How far a turn handle sits beyond its shape.
    pub turn_extra: f32,
    /// A rounded handle's corner radius.
    pub round_corner: f32,
    /// The marks' line and fill (`overlay.handle_fill`) and halo (`surface.panel`).
    pub line: Color32,
    #[allow(missing_docs)]
    pub halo: Color32,
    /// The ring's amount arc: `accent`.
    pub amount: Color32,
}

impl BlurPinsStyle {
    pub(crate) fn of(tokens: &UiTokens) -> BlurPinsStyle {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:44-51 (the sizes), :689 `draw` (the colours).
        let (size, stroke) = (&tokens.size, &tokens.stroke);
        BlurPinsStyle {
            dot: size.handle.sm / 2.0,
            grab: size.hit.target,
            ring: size.control.sm,
            ring_band: RING_BAND_WIDTH,
            handle: size.handle.sm / 2.0,
            line_grab: size.hit.target,
            line_width: stroke.medium,
            line_halo: stroke.medium + 2.0 * stroke.thin,
            arc: stroke.thick,
            arc_halo: stroke.thick + 2.0 * stroke.thin,
            dash: stroke.dash_gap,
            dash_gap: stroke.dash_gap,
            handle_outline: stroke.thin,
            feather_extra: FEATHER_EXTRA,
            turn_extra: TURN_EXTRA,
            round_corner: ROUND_CORNER,
            line: tokens.color.overlay.handle_fill.to_egui(),
            halo: tokens.color.surface.panel.to_egui(),
            amount: tokens.color.accent.solid.to_egui(),
        }
    }
}
