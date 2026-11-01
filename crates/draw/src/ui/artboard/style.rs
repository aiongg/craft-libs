//! [`ArtboardStyle`]: the Artboard tool's look, from tokens (task TK3).

use egui::{Color32, Stroke};

use super::ArtboardTool;
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{ToEgui, UiTokens};

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
    pub fn style(tokens: &UiTokens) -> ArtboardStyle {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:24 `CLICK_PX`, :266 (1.5 pt accent), :284-288
        // (handles 7 × 7 white, 1 pt accent edge).
        let a = &tokens.component.artboard;
        let accent = tokens.color.accent.solid.to_egui();
        let handle = Handle::style(tokens, HandleShape::Square);
        ArtboardStyle {
            preview: Stroke::new(a.preview_line, accent),
            click_slop: a.click_slop,
            handle: handle.size,
            handle_hit: handle.hit,
            handle_fill: handle.fill,
            handle_edge: Stroke::new(handle.stroke.width, accent),
        }
    }
}
