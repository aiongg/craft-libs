//! Stories of the corner widgets: a rectangle and a hexagon on a canvas.

use super::widgets::WIDGET;
use super::{CornerKind, CornerPoint, CornerTarget, CornerWidgets, CornerWidgetsOutput};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// The rectangle's corner radius and the hexagon's.
const RECT_RADIUS: f64 = 12.0;
const HEXAGON_RADIUS: f64 = 8.0;

/// A closed polygon whose vertices are `points`, each corner rounded by `radius`.
fn polygon(key: u64, points: &[(f64, f64)], radius: f64, individual: bool) -> CornerTarget {
    let vertices: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let n = vertices.len();
    let corners = (0..n)
        .map(|i| CornerPoint { vertex: vertices[i], prev: vertices[(i + n - 1) % n], next: vertices[(i + 1) % n], radius, kind: CornerKind::Round })
        .collect();
    CornerTarget { key, corners, individual }
}

/// A rectangle (each corner its own radius) and a hexagon (one radius for all corners), side by side.
fn targets() -> Vec<CornerTarget> {
    let rect = polygon(1, &[(40.0, 40.0), (200.0, 40.0), (200.0, 140.0), (40.0, 140.0)], RECT_RADIUS, true);
    let hexagon = polygon(2, &[(300.0, 60.0), (360.0, 40.0), (420.0, 60.0), (420.0, 120.0), (360.0, 140.0), (300.0, 120.0)], HEXAGON_RADIUS, false);
    vec![rect, hexagon]
}

/// Shows `control` as the only canvas control of a canvas whose view has the document origin at its centre.
fn on_canvas(ui: &mut Ui, control: CornerWidgets<'_>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut control = Some(control);
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| -> Option<CornerWidgetsOutput> { control.take().map(|control| control.show_at(ui, body.rect)) });
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

/// Drags the first widget, the rectangle's top left corner, towards the inside.
fn drag_the_first_widget(fixture: &mut Fixture) {
    let first = fixture.root_id().and_then(|id| fixture.part(id, WIDGET.at(0)));
    if let Some(first) = first {
        fixture.drag_at(first.center(), Vec2::new(15.0, 15.0));
    }
}

#[story(CornerWidgets<'static>, covers = [Default], drive = Drive::Custom(drag_the_first_widget))]
fn rectangle_and_hexagon(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let targets = data.value("targets", targets());
    on_canvas(ui, CornerWidgets::new("Corner radius", Bind::bound(&mut *targets)))
}

#[story(CornerWidgets<'static>, covers = [], drive = Drive::Never)]
fn cycling_kinds_and_a_wide_angle_limit(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let targets = data.value("kinds", targets());
    on_canvas(ui, CornerWidgets::new("Corner radius", Bind::bound(&mut *targets)).kinds(true).hide_above(120.0))
}
