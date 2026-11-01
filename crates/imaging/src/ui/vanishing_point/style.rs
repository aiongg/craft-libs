//! [`VanishingPointStyle`]: Vanishing Point's marks, from tokens (task TK4).

use egui::Color32;

use craft_ui::theme::UiTokens;

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
    pub node_fill: Color32,
    /// The check's colours: `accent`, `status.warn.fg`, `status.bad.fg`.
    pub valid: Color32,
    pub questionable: Color32,
    pub invalid: Color32,
}

impl VanishingPointStyle {
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `vanishing_point` lands in task TK4")]
    pub fn of(tokens: &UiTokens) -> VanishingPointStyle {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1091-1135 (widths, alpha, nodes), :1148-1187
        // (the stamp ring and cross with the 2.5 pt black alpha-160 halo).
        todo!()
    }
}
