//! [`ConnectorStyle`]: the Connector tool's look, from tokens.

use egui::{Color32, Stroke};

use super::ConnectorTool;
use craft_ui::theme::{ToEgui, UiTokens};

/// Alpha of the active attach point's fill.
const ATTACH_FILL_ALPHA: f32 = 0.25;
/// Shortest elbow run, in screen points, that has a path handle.
const MIN_RUN: f32 = 28.0;
/// How far from a target's bounds, in screen points, it is still hit.
const TARGET_TOLERANCE: f32 = 2.0;

/// What [`ConnectorTool`] and [`ConnectorOverlay`](super::ConnectorOverlay) draw. The same at
/// every Size (it sits on the document). Look source: FigCraft.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ConnectorStyle {
    /// The route and rings, in `accent.solid`.
    pub line: Stroke,
    /// An attach point's radius; filled `overlay.handle_fill`.
    pub attach: f32,
    /// The active attach point's radius.
    pub attach_active: f32,
    /// The active attach point's fill: `accent.solid` at low alpha.
    pub attach_active_fill: Color32,
    /// An end handle's radius, white with the line's ring.
    pub end_handle: f32,
    /// A path handle's radius, accent with a thin white ring.
    pub path_handle: f32,
    /// A path handle's fill.
    pub handle_fill: Color32,
    /// A drag shorter than this many screen points makes nothing.
    pub click_slop: f32,
    /// An end within this many screen points of a site glues to it.
    pub attach_reach: f32,
    /// An end or path handle within this many screen points of a press is grabbed.
    pub handle_reach: f32,
    /// An elbow run shorter than this many screen points has no path handle.
    pub min_run: f32,
    /// A target is hit within this many screen points of its bounds.
    pub target_tolerance: f32,
    /// A path handle's ring, in `overlay.handle_fill`.
    pub path_ring: Stroke,
    /// The hovered target's outline, in `accent.solid`.
    pub drop_outline: Stroke,
}

impl ConnectorTool {
    /// The Connector tool's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ConnectorStyle {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:364-376, :407-418 (the values above);
        // crates/engine/src/boards/board/tools.rs:12 `ATTACH_PX`, :129 (4 points).
        let (c, stroke, handle, hit) = (&tokens.color, &tokens.stroke, &tokens.size.handle, &tokens.size.hit);
        let accent = c.accent.solid.to_egui();
        ConnectorStyle {
            line: Stroke::new(stroke.medium, accent),
            attach: handle.sm,
            attach_active: handle.md,
            attach_active_fill: accent.linear_multiply(ATTACH_FILL_ALPHA),
            end_handle: handle.sm,
            path_handle: handle.sm / 2.0 + stroke.thin / 2.0,
            handle_fill: c.overlay.handle_fill.to_egui(),
            click_slop: hit.slop,
            attach_reach: hit.target,
            handle_reach: hit.target,
            min_run: MIN_RUN,
            target_tolerance: TARGET_TOLERANCE,
            path_ring: Stroke::new(stroke.thin, c.overlay.handle_fill.to_egui()),
            drop_outline: Stroke::new(stroke.thin, accent),
        }
    }
}
