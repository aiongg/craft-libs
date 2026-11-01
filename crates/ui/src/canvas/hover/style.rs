//! [`HoverStyle`]: the hover outline's look and timing, from tokens (task TK1).

use std::time::Duration;

use egui::Color32;

use super::HoverOutline;
use crate::theme::UiTokens;

/// What [`HoverOutline`] draws and how [`HoverLookup`](super::HoverLookup) waits. Every size is a
/// token; the same at every Size (the outline sits on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverStyle {
    /// The default colour of a rect, path and mask tint: `overlay.line` (the accent as a bare
    /// line, as PhotoCraft's rollover and Object Finder use it).
    pub colour: Color32,
    /// The line width of a rect or path: `hover.outline` (1.5, PhotoCraft `move_hover.rs:130`).
    pub outline: f32,
    /// A mask texel's alpha per unit of coverage: `hover.fill_alpha` (0.42).
    pub fill_alpha: f32,
    /// A mask edge texel's alpha: `hover.edge_alpha` (0.95).
    pub edge_alpha: f32,
    /// How long the pointer rests before a lookup: `hover.debounce_ms` (120).
    pub debounce: Duration,
    /// PhotoCraft's repaint interval while busy: `hover.busy_ms` (40). The library repaints from
    /// the job queue's wake instead and keeps it only for a host without one.
    pub busy: Duration,
}

impl HoverOutline {
    /// The hover look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `hover` lands in task TK1")]
    pub fn style(tokens: &UiTokens) -> HoverStyle {
        // pc: crates/ui-egui/src/tools/move_hover.rs:130 (1.5, accent);
        // crates/ui-egui/src/tools/object_finder.rs:26 (0.12 s), :166 (0.95, 0.42), :206 (40 ms).
        todo!()
    }
}
