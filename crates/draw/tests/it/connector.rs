//! The Connector tool, its handles and its bar. The view maps one document unit to one screen point.

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CanvasView, PointerButton};
use craft_draw::geometry::connector::ConnectionSide;
use craft_draw::ui::connector::{ConnectionTarget, ConnectorEnd, ConnectorEvent, ConnectorTool};
use craft_ui::canvas::{CanvasSpace, CanvasTool as _, ToolContext, ToolPointer};
use craft_ui::testing::{ToolFixture, trial};
use egui::{Pos2, Vec2, pos2};

const CANVAS: Vec2 = Vec2::new(400.0, 300.0);
const SLOP: f32 = 6.0;

const A: Rect = Rect::new(20.0, 100.0, 100.0, 200.0);

fn target(id: u64, rect: Rect) -> ConnectionTarget {
    ConnectionTarget::new(id, rect)
}

/// Three boxes: A (1) on the left, B (2) on the right at the same height, C (3) above A's right.
fn boxes() -> Vec<ConnectionTarget> {
    vec![target(1, A), target(2, Rect::new(250.0, 100.0, 330.0, 200.0)), target(3, Rect::new(150.0, 20.0, 200.0, 60.0))]
}

fn glued(target: u64, side: ConnectionSide) -> ConnectorEnd {
    ConnectorEnd::glued(target, side)
}

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(200.0, 150.0), 1.0)
}

/// The view of a canvas filling `room`, with its middle at the document's middle.
fn room_view(room: egui::Rect) -> CanvasView {
    CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0)
}

fn cx<'a>() -> ToolContext<'a> {
    ToolContext::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Some(CanvasSpace::new(view(), 1.0))).slop(SLOP)
}

fn at(x: f32, y: f32) -> ToolPointer {
    ToolPointer::new(pos2(x, y), PointerButton::Primary, 0.0)
}

fn tool(targets: Vec<ConnectionTarget>) -> ConnectorTool {
    let mut tool = ConnectorTool::new();
    tool.targets(&targets);
    tool
}

/// A press, a move through the middle, a move to `b` and the release.
fn drag(tool: &mut ConnectorTool, a: (f32, f32), b: (f32, f32)) -> Vec<ConnectorEvent> {
    tool.press(&at(a.0, a.1), &mut cx());
    tool.drag(&at((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), &mut cx());
    tool.drag(&at(b.0, b.1), &mut cx());
    tool.release(&at(b.0, b.1), &mut cx());
    tool.take_events()
}

#[trial]
fn the_connector_tool_owes_the_canvas_what_every_tool_owes() {
    ToolFixture::with_view("Connector", tool(boxes()), room_view, |_, _| {}).check();
}
