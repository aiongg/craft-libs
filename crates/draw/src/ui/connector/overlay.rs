//! [`ConnectorOverlay`]: a selected connector's handles and a target's attach points.

use egui::epaint::CubicBezierShape;
use egui::{Color32, Painter, Pos2, Shape};
use kurbo::Point;

use super::{ConnectionTarget, ConnectorStyle};
use crate::geometry::connector::{Routing, path_handles};
use craft_ui::canvas::CanvasView;

/// Draws what the Connector tool shows over the document, for the tool and for an app that shows
/// a selected connector's handles under another tool (FigCraft's selection overlay).
///
/// - The route: an accent polyline (the line 1.5 pt), a curve as its cubic.
/// - End handles: white discs with a 1.5 pt accent ring; path handles: accent discs with a 1 pt
///   white ring, one per elbow run or one for a curve.
/// - Attach points ([`paint_sites`](Self::paint_sites)): see [`ConnectorTool`](super::ConnectorTool).
#[derive(Clone, Copy, Debug)]
pub struct ConnectorOverlay<'a> {
    points: &'a [Point],
    routing: Routing,
    handles: bool,
}

/// The path handles of the route through `points` that are drawn and can be grabbed at `view`'s
/// zoom: [`path_handles`] without the elbow runs too short on screen to hold one. The portion is
/// the index [`reshaped`](crate::geometry::connector::reshaped) takes.
pub(super) fn visible_path_handles(points: &[Point], routing: Routing, view: &CanvasView, style: &ConnectorStyle) -> Vec<(usize, Point)> {
    let min = style.min_run;
    let long = |i: usize| match (points.get(i), points.get(i + 1)) {
        (Some(&a), Some(&b)) => view.to_screen(a).distance(view.to_screen(b)) >= min,
        _ => false,
    };
    path_handles(points, routing).into_iter().filter(|&(i, _)| routing == Routing::Curved || long(i)).collect()
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
        // new: the route itself (FigCraft draws it in the document).
        let screen: Vec<Pos2> = self.points.iter().map(|&p| view.to_screen(p)).collect();
        if screen.len() < 2 {
            return;
        }
        match (self.routing, screen.as_slice()) {
            (Routing::Curved, &[a, b, c, d]) => {
                painter.add(CubicBezierShape::from_points_stroke([a, b, c, d], false, Color32::TRANSPARENT, style.line))
            }
            _ => painter.add(Shape::line(screen.clone(), style.line)),
        };
        if !self.handles {
            return;
        }
        for (_, at) in visible_path_handles(self.points, self.routing, view, style) {
            let c = view.to_screen(at);
            painter.circle_filled(c, style.path_handle, style.line.color);
            painter.circle_stroke(c, style.path_handle, style.path_ring);
        }
        for end in [screen.first(), screen.last()].into_iter().flatten() {
            painter.circle_filled(*end, style.end_handle, style.handle_fill);
            painter.circle_stroke(*end, style.end_handle, style.line);
        }
    }

    /// Draws `target`'s sites through `view`, `active` (a site index) as the larger see-through
    /// ring.
    pub fn paint_sites(painter: &Painter, view: &CanvasView, style: &ConnectorStyle, target: &ConnectionTarget, active: Option<u32>) {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:407-418 `attach_points` (7 at accent ×
        // 0.25 for the active one, else 4 white; 1.5 pt accent ring).
        for site in target.side_sites() {
            let c = view.to_screen(site.point);
            let (radius, fill) =
                if active == Some(site.index) { (style.attach_active, style.attach_active_fill) } else { (style.attach, style.handle_fill) };
            painter.circle_filled(c, radius, fill);
            painter.circle_stroke(c, radius, style.line);
        }
    }
}
