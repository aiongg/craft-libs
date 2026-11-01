//! [`ConnectorStyle`]: the Connector tool's look, from tokens (task TK3).

use egui::{Color32, Stroke};

use super::ConnectorTool;
use craft_ui::theme::UiTokens;

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
    /// An end handle's radius: `connector.end_handle` (5, proposed), white with the line's ring.
    pub end_handle: f32,
    /// A path handle's radius: `connector.path_handle` (3.5, proposed), accent with a 1 pt white
    /// ring.
    pub path_handle: f32,
    pub handle_fill: Color32,
    /// A drag shorter than this many screen points makes nothing: `connector.click_slop` (4).
    pub click_slop: f32,
    /// An end within this many screen points of a site glues to it: `connector.attach_reach` (10,
    /// proposed; FigCraft's `ATTACH_PX`).
    pub attach_reach: f32,
}

impl ConnectorTool {
    /// The Connector tool's look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `connector` lands in task TK3")]
    pub fn style(tokens: &UiTokens) -> ConnectorStyle {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:364-376, :407-418 (the values above);
        // crates/engine/src/boards/board/tools.rs:12 `ATTACH_PX`, :129 (4 points).
        todo!()
    }
}
