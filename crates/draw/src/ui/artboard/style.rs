//! [`ArtboardStyle`]: the Artboard tool's look, from tokens (task TK3).

use egui::{Color32, Stroke};

use super::ArtboardTool;
use craft_ui::theme::UiTokens;

/// What [`ArtboardTool`] draws. The same at every Size (it sits on the document). The frames and
/// names are the `Pasteboard`'s; the "+" buttons have their own
/// [`AddBesideStyle`](crate::ui::add_beside::AddBesideStyle).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ArtboardStyle {
    /// The drag preview: `artboard.preview_line` (1.5) of `accent.solid`.
    pub preview: Stroke,
    /// A drag shorter than this many screen points draws nothing: `artboard.click_slop` (3).
    pub click_slop: f32,
    /// Handle size and reach from the `Handle` painter, `Handle::style(tokens, HandleShape::Square)`
    /// (7 and 14).
    pub handle: f32,
    pub handle_hit: f32,
    /// A handle's fill: `overlay.handle_fill` (white).
    pub handle_fill: Color32,
    /// A handle's edge: `Handle::style`'s stroke width in `accent.solid`.
    pub handle_edge: Stroke,
}

impl ArtboardTool {
    /// The Artboard tool's look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `artboard` lands in task TK3")]
    pub fn style(tokens: &UiTokens) -> ArtboardStyle {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:24 `CLICK_PX`, :266 (1.5 pt accent), :284-288
        // (handles 7 × 7 white, 1 pt accent edge).
        todo!()
    }
}
