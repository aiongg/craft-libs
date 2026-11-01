//! [`CornerStyle`]: the corner widgets' look, from tokens (task TK2). Every size is the same at
//! every Size: the widgets sit on the document.

use egui::Color32;

use super::CornerWidgets;
use craft_ui::theme::UiTokens;

/// What [`CornerWidgets`] draws and how far it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CornerStyle {
    /// The widget's diameter: `corner.widget` (10).
    pub widget: f32,
    /// The inner dot's diameter: `corner.dot` (5).
    pub dot: f32,
    /// The outline's width: `corner.line` (1.5).
    pub line: f32,
    /// The pointer's reach: `corner.hit` (7).
    pub hit: f32,
    /// The least distance inside a corner: `corner.inset` (10).
    pub inset: f32,
    /// The shortest side, on the screen, whose corners show widgets: `corner.min_side` (30).
    pub min_side: f32,
    /// The fill: `overlay.handle_fill` (white).
    pub fill: Color32,
    /// The outline and the dot: `overlay.line` (the accent).
    pub color: Color32,
}

impl CornerWidgets<'_> {
    /// The corner widgets' look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `corner` lands in task TK2")]
    pub fn style(tokens: &UiTokens) -> CornerStyle {
        // pc: crates/ui-egui/src/tools/corner_radius.rs:325-326 (radius 5, white, 1.5 pt accent,
        // dot radius 2.5), :213 (7), :22 (10); vectorcraft: crates/tools/src/corners.rs:25 (30).
        todo!()
    }
}
