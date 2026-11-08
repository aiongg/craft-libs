//! Stories of the connector handles and the connector bar.

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CanvasView, Mixed};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{ConnectionTarget, ConnectorBar, ConnectorEnd, ConnectorHandles, ConnectorItem, ConnectorLine};
use crate::geometry::connector::{ConnectionSide, Routing};
use crate::ui::stroke::{StrokeChoice, StrokeChoiceRow};

fn targets() -> Vec<ConnectionTarget> {
    vec![
        ConnectionTarget::new(1, Rect::new(40.0, 80.0, 120.0, 140.0)).named("Rectangle 1"),
        ConnectionTarget::new(2, Rect::new(260.0, 40.0, 340.0, 100.0)).named("Rectangle 2"),
    ]
}

fn glued(target: u64, side: ConnectionSide) -> ConnectorEnd {
    ConnectorEnd::Glued { target, side, site: None, at: None }
}

/// Shows `handles` as the only canvas control of a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, handles: ConnectorHandles<'_>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut handles = Some(handles);
    let shown = Canvas::new("Document").view(view).show(ui, |ui, body| handles.take().map(|handles| handles.show_at(ui, body.rect)));
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

#[story(ConnectorHandles<'static>, covers = [Default], drive = Drive::Never)]
fn connector_glued_at_both_ends(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let targets = targets();
    let item = data.value(
        "item",
        ConnectorItem { id: 9, from: glued(1, ConnectionSide::Right), to: glued(2, ConnectionSide::Left), routing: Routing::Elbow, path: None },
    );
    on_canvas(ui, ConnectorHandles::new("Connector", Bind::bound(&mut *item), &targets))
}

#[story(ConnectorHandles<'static>, covers = [], drive = Drive::Never)]
fn connector_with_one_free_end(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let targets = targets();
    let item = data.value(
        "item",
        ConnectorItem {
            id: 9,
            from: glued(1, ConnectionSide::Right),
            to: ConnectorEnd::Free(Point::new(300.0, 180.0)),
            routing: Routing::Curved,
            path: None,
        },
    );
    on_canvas(ui, ConnectorHandles::new("Connector", Bind::bound(&mut *item), &targets))
}

fn caps() -> Vec<StrokeChoice> {
    vec![StrokeChoice::new("none", "None"), StrokeChoice::new("arrow", "Arrow"), StrokeChoice::new("dot", "Dot")]
}

#[story(ConnectorBar<'static>, covers = [Default], drive = Drive::Never)]
fn connector_bar_for_one_connector(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let choices = caps();
    Bar::new("Options", BarKind::Options)
        .show(ui, |ui, _| {
            let start = StrokeChoiceRow::new("start", "Start", &choices).selected(Mixed::Same("none"));
            let end = StrokeChoiceRow::new("end", "End", &choices).selected(Mixed::Same("arrow"));
            ConnectorBar::new(Mixed::Same(Routing::Elbow), Mixed::Same(ConnectorLine::default())).caps(start, end).show(ui);
        })
        .output
        .into()
}

#[story(ConnectorBar<'static>, covers = [Mixed], drive = Drive::Never)]
fn connector_bar_for_a_mixed_selection(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let choices = caps();
    Bar::new("Options", BarKind::Options)
        .show(ui, |ui, _| {
            let start = StrokeChoiceRow::new("start", "Start", &choices).selected(Mixed::Mixed);
            let end = StrokeChoiceRow::new("end", "End", &choices).selected(Mixed::Same("arrow"));
            ConnectorBar::new(Mixed::Mixed, Mixed::Mixed).caps(start, end).show(ui);
        })
        .output
        .into()
}
