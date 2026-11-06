//! [`CornerStyle`]: the corner widgets' look, from tokens. Every size is the same at
//! every Size: the widgets sit on the document.

use egui::Color32;

use super::CornerWidgets;
use craft_ui::theme::{ToEgui, UiTokens};

/// The shortest side, in screen points, whose corners show widgets.
const MIN_SIDE: f32 = 30.0;

/// What [`CornerWidgets`] draws and how far it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CornerStyle {
    /// The widget's diameter.
    pub widget: f32,
    /// The inner dot's diameter.
    pub dot: f32,
    /// The outline's width.
    pub line: f32,
    /// The pointer's reach.
    pub hit: f32,
    /// Farthest a press may move and still be a click.
    pub click_slop: f32,
    /// The least distance inside a corner.
    pub inset: f32,
    /// The shortest side, on the screen, whose corners show widgets.
    pub min_side: f32,
    /// The fill: `overlay.handle_fill` (white).
    pub fill: Color32,
    /// The outline and the dot: `overlay.line` (the accent).
    pub color: Color32,
}

impl CornerWidgets<'_> {
    /// The corner widgets' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> CornerStyle {
        // pc: crates/ui-egui/src/tools/corner_radius.rs:325-326 (radius 5, white, 1.5 pt accent,
        // dot radius 2.5), :213 (7), :22 (10); vectorcraft: crates/tools/src/corners.rs:25 (30).
        let o = &tokens.color.overlay;
        CornerStyle {
            widget: tokens.size.handle.md + 2.0 * tokens.stroke.thin,
            dot: tokens.size.handle.sm,
            line: tokens.stroke.medium,
            hit: tokens.size.hit.target,
            click_slop: tokens.size.hit.slop,
            inset: tokens.space.gap.stack,
            min_side: MIN_SIDE,
            fill: o.handle_fill.to_egui(),
            color: o.line.to_egui(),
        }
    }
}
