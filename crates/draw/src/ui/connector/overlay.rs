//! [`ConnectorOverlay`]: a selected connector's handles and a target's attach points (task
//! SH-T10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Painter;
use kurbo::Point;

use super::{ConnectionTarget, ConnectorStyle};
use crate::geometry::connector::Routing;
use craft_ui::canvas::CanvasView;

/// Draws what the Connector tool shows over the document, for the tool and for an app that shows
/// a selected connector's handles under another tool (FigCraft's selection overlay).
///
/// - The route: an accent polyline (`connector.line` 1.5 pt), a curve as its cubic.
/// - End handles: white discs with a 1.5 pt accent ring; path handles: accent discs with a 1 pt
///   white ring, one per elbow run or one for a curve.
/// - Attach points ([`paint_sites`](Self::paint_sites)): see [`ConnectorTool`](super::ConnectorTool).
#[derive(Clone, Copy, Debug)]
pub struct ConnectorOverlay<'a> {
    points: &'a [Point],
    routing: Routing,
    handles: bool,
}

impl<'a> ConnectorOverlay<'a> {
    /// The connector through `points` (document units), routed `routing`, with its handles.
    pub fn new(points: &'a [Point], routing: Routing) -> Self {
        ConnectorOverlay { points, routing, handles: true }
    }

    /// Whether the end and path handles show; on by default (off for a draft).
    pub fn handles(mut self, on: bool) -> Self {
        self.handles = on;
        self
    }

    /// Draws the route and its handles through `view`.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &ConnectorStyle) {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:364-376 (path handles 3.5 accent with a
        // 1 pt white ring, end handles 5 white with a 1.5 pt accent ring).
        todo!()
    }

    /// Draws `target`'s sites through `view`, `active` (a site index) as the larger see-through
    /// ring.
    pub fn paint_sites(painter: &Painter, view: &CanvasView, style: &ConnectorStyle, target: &ConnectionTarget, active: Option<u32>) {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:407-418 `attach_points` (7 at accent ×
        // 0.25 for the active one, else 4 white; 1.5 pt accent ring).
        todo!()
    }
}
