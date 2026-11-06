//! [`VanishingPointStyle`]: Vanishing Point's marks, from tokens.

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// The grid lines' width.
const GRID_STROKE: f32 = 0.75;
/// The grid lines' alpha.
const GRID_ALPHA: f32 = 0.6;
/// The source cross's arm length.
const SOURCE_ARM: f32 = 7.0;
/// How far from a node's centre a press reaches, in points.
const PRESS_REACH: f32 = 8.0;
/// The click-versus-drag distance of a torn plane's far edge node, in document pixels.
const CLICK_SLOP: f32 = 2.0;
/// The pasted image's white tint alpha.
const PASTE_ALPHA: f32 = 0.9;

/// The planes' and the stamp's look.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct VanishingPointStyle {
    /// Outline: `stroke.thin`; selected `stroke.thick`.
    pub line: f32,
    #[allow(missing_docs)]
    pub line_selected: f32,
    /// Grid lines: 0.75 at an alpha of 0.6.
    pub grid: f32,
    #[allow(missing_docs)]
    pub grid_alpha: f32,
    /// Node squares: `size.handle.sm / 2 + stroke.thin`, white fill, outline in the plane's colour at `line`.
    pub node: f32,
    /// Create Plane's pending corners: `size.handle.sm / 2 + stroke.thin`.
    pub pending: f32,
    /// The halo under lines, the ring and the cross: `surface.panel`, the line's width plus a
    /// `stroke.thin` on each side.
    pub halo: Color32,
    #[allow(missing_docs)]
    pub halo_width: f32,
    /// The source cross's arms: 7.
    pub source_arm: f32,
    /// The white line of the ring and the cross: `stroke.thin`.
    pub ring_line: f32,
    /// How far from a node's centre a press reaches, in points: 8.
    pub press_reach: f32,
    /// The click-versus-drag distance of a torn plane's far edge node, in document pixels:
    /// 2.
    pub click_slop: f32,
    /// The pasted image's white tint alpha: 0.9.
    pub paste_alpha: f32,
    /// The node squares' fill.
    pub node_fill: Color32,
    /// The check's colours: `accent`, `status.warn.fg`, `status.bad.fg`.
    pub valid: Color32,
    #[allow(missing_docs)]
    pub questionable: Color32,
    #[allow(missing_docs)]
    pub invalid: Color32,
}

impl VanishingPointStyle {
    pub(crate) fn of(tokens: &UiTokens) -> VanishingPointStyle {
        let stroke = &tokens.stroke;
        let node = tokens.size.handle.sm / 2.0 + stroke.thin;
        VanishingPointStyle {
            line: stroke.thin,
            line_selected: stroke.thick,
            grid: GRID_STROKE,
            grid_alpha: GRID_ALPHA,
            node,
            pending: node,
            halo: tokens.color.surface.panel.to_egui(),
            halo_width: stroke.thin + 2.0 * stroke.thin,
            source_arm: SOURCE_ARM,
            ring_line: stroke.thin,
            press_reach: PRESS_REACH,
            click_slop: CLICK_SLOP,
            paste_alpha: PASTE_ALPHA,
            node_fill: Color32::WHITE,
            valid: tokens.color.accent.solid.to_egui(),
            questionable: tokens.color.status.warn.fg.to_egui(),
            invalid: tokens.color.status.bad.fg.to_egui(),
        }
    }
}
