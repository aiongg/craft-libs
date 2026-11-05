//! [`ConnectorStyle`]: the Connector tool's look, from tokens (task TK3).

use egui::{Color32, Stroke};

use super::ConnectorTool;
use craft_ui::theme::{ToEgui, UiTokens};

/// What [`ConnectorTool`] and [`ConnectorOverlay`](super::ConnectorOverlay) draw. The same at
/// every Size (it sits on the document). Look source: FigCraft.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ConnectorStyle {
    /// The route and rings: `connector.line` (1.5) of `accent.solid`.
    pub line: Stroke,
    /// An attach point's radius: `connector.attach` (4); filled `overlay.handle_fill`.
    pub attach: f32,
    /// The active attach point's radius: `connector.attach_active` (7).
    pub attach_active: f32,
    /// The active attach point's fill: `accent.solid` at `connector.attach_fill_alpha` (0.25).
    pub attach_active_fill: Color32,
    /// An end handle's radius: `connector.end_handle` (5), white with the line's ring.
    pub end_handle: f32,
    /// A path handle's radius: `connector.path_handle` (3.5), accent with a 1 pt white
    /// ring.
    pub path_handle: f32,
    /// A path handle's fill.
    pub handle_fill: Color32,
    /// A drag shorter than this many screen points makes nothing: `connector.click_slop` (4).
    pub click_slop: f32,
    /// An end within this many screen points of a site glues to it: `connector.attach_reach` (10,
    /// FigCraft's `ATTACH_PX`).
    pub attach_reach: f32,
    /// An end or path handle within this many screen points of a press is grabbed:
    /// `connector.handle_reach` (8, FigCraft's `END_HANDLE_PX`).
    pub handle_reach: f32,
    /// An elbow run shorter than this many screen points has no path handle:
    /// `connector.min_run` (28, FigCraft's `MIN_RUN_PX`).
    pub min_run: f32,
    /// A target is hit within this many screen points of its bounds: `connector.target_tolerance` (2).
    pub target_tolerance: f32,
    /// A path handle's ring: `connector.path_ring` (1) of `overlay.handle_fill`.
    pub path_ring: Stroke,
    /// The hovered target's outline: `connector.drop_outline` (1) of `accent.solid`.
    pub drop_outline: Stroke,
}

impl ConnectorTool {
    /// The Connector tool's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ConnectorStyle {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:364-376, :407-418 (the values above);
        // crates/engine/src/boards/board/tools.rs:12 `ATTACH_PX`, :129 (4 points).
        let (k, c) = (&tokens.component.connector, &tokens.color);
        let accent = c.accent.solid.to_egui();
        ConnectorStyle {
            line: Stroke::new(k.line, accent),
            attach: k.attach,
            attach_active: k.attach_active,
            attach_active_fill: accent.linear_multiply(k.attach_fill_alpha),
            end_handle: k.end_handle,
            path_handle: k.path_handle,
            handle_fill: c.overlay.handle_fill.to_egui(),
            click_slop: k.click_slop,
            attach_reach: k.attach_reach,
            handle_reach: k.handle_reach,
            min_run: k.min_run,
            target_tolerance: k.target_tolerance,
            path_ring: Stroke::new(k.path_ring, c.overlay.handle_fill.to_egui()),
            drop_outline: Stroke::new(k.drop_outline, accent),
        }
    }
}
