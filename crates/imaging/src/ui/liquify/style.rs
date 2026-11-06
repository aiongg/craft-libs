//! [`LiquifyStyle`]: Liquify's marks on the canvas, from tokens, read through
//! [`LiquifyWorkspace::style`].

use egui::Color32;

use craft_ui::theme::{ToEgui, UiTokens};

use super::LiquifyWorkspace;

/// The freeze mask's opacity over the image.
const MASK_OPACITY: f32 = 0.5;

/// What Liquify draws on the canvas besides the mesh (`warp::MeshStyle`) and the brush outline
/// (`craft_ui::canvas::brush::BrushStyle`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LiquifyStyle {
    /// The freeze mask's tint and the Freeze Lasso's line while it freezes: `status.bad.fg`.
    pub freeze: Color32,
    /// The Freeze Lasso's line while it thaws: `accent.solid`.
    pub thaw: Color32,
    /// The lasso line's width: `stroke.medium`.
    pub lasso_width: f32,
    /// The freeze mask's opacity over the image: 0.5.
    pub mask_opacity: f32,
}

impl LiquifyWorkspace {
    /// The marks Liquify draws on the canvas, from `tokens`.
    pub fn style(tokens: &UiTokens) -> LiquifyStyle {
        LiquifyStyle {
            freeze: tokens.color.status.bad.fg.to_egui(),
            thaw: tokens.color.accent.solid.to_egui(),
            lasso_width: tokens.stroke.medium,
            mask_opacity: MASK_OPACITY,
        }
    }
}
