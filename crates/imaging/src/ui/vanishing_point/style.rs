//! [`VanishingPointStyle`]: Vanishing Point's marks, from tokens (task TK4).

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// The planes' and the stamp's look: the `vanishing_point` family, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct VanishingPointStyle {
    /// Outline: `vanishing_point.line` (1.25); selected `.line_selected` (2).
    pub line: f32,
    pub line_selected: f32,
    /// Grid lines: `.grid` (0.75) at `.grid_alpha` (0.6).
    pub grid: f32,
    pub grid_alpha: f32,
    /// Node squares: `.node` (7), white fill, outline in the plane's colour at `line`.
    pub node: f32,
    /// Create Plane's pending corners: `.pending` (6).
    pub pending: f32,
    /// The halo under lines, the ring and the cross: `.halo` (black at alpha 160), `.halo_width`
    /// (2.5).
    pub halo: Color32,
    pub halo_width: f32,
    /// The source cross's arms: `.source_arm` (7).
    pub source_arm: f32,
    /// The white line of the ring and the cross: `.ring_line` (1).
    pub ring_line: f32,
    /// How far from a node's centre a press reaches, in points: `.press_reach` (8).
    pub press_reach: f32,
    /// The click-versus-drag distance of a torn plane's far edge node, in document pixels:
    /// `.click_slop` (2).
    pub click_slop: f32,
    /// The pasted image's white tint alpha: `.paste_alpha` (230 of 255).
    pub paste_alpha: f32,
    pub node_fill: Color32,
    /// The check's colours: `accent`, `status.warn.fg`, `status.bad.fg`.
    pub valid: Color32,
    pub questionable: Color32,
    pub invalid: Color32,
}

impl VanishingPointStyle {
    pub(crate) fn of(tokens: &UiTokens) -> VanishingPointStyle {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1091-1135 (widths, alpha, nodes), :1148-1187
        // (the stamp ring and cross with the 2.5 pt black alpha-160 halo).
        let v = &tokens.component.vanishing_point;
        VanishingPointStyle {
            line: v.line,
            line_selected: v.line_selected,
            grid: v.grid,
            grid_alpha: v.grid_alpha,
            node: v.node,
            pending: v.pending,
            halo: v.halo.to_egui(),
            halo_width: v.halo_width,
            source_arm: v.source_arm,
            ring_line: v.ring_line,
            press_reach: v.press_reach,
            click_slop: v.click_slop,
            paste_alpha: v.paste_alpha,
            node_fill: Color32::WHITE,
            valid: tokens.color.accent.solid.to_egui(),
            questionable: tokens.color.status.warn.fg.to_egui(),
            invalid: tokens.color.status.bad.fg.to_egui(),
        }
    }
}
