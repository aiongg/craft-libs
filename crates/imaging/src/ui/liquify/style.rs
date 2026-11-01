//! [`LiquifyStyle`]: Liquify's marks on the canvas, from tokens (task TK4).

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

/// What Liquify draws on the canvas besides the mesh (`warp::MeshStyle`) and the brush outline
/// (`craft_ui::canvas::brush::BrushStyle`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LiquifyStyle {
    /// The freeze mask's tint and the Freeze Lasso's line while it freezes: `liquify.freeze` →
    /// `status.bad.fg` (PhotoCraft `t.danger`).
    pub freeze: Color32,
    /// The Freeze Lasso's line while it thaws: `liquify.thaw` → `accent` (PhotoCraft `t.accent`).
    pub thaw: Color32,
    /// The lasso line's width: 1.5 (PhotoCraft's), a `liquify.lasso` size leaf.
    pub lasso_width: f32,
    /// The freeze mask's opacity over the image: 0.5 (PhotoCraft's red overlay at alpha 128).
    pub mask_opacity: f32,
}

impl LiquifyStyle {
    pub fn of(tokens: &UiTokens) -> LiquifyStyle {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1142-1149 (lasso colours).
        let l = &tokens.component.liquify;
        LiquifyStyle { freeze: l.freeze.to_egui(), thaw: l.thaw.to_egui(), lasso_width: l.lasso, mask_opacity: l.mask_opacity }
    }
}
