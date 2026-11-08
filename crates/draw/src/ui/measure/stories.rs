//! Stories of the Ruler tool's overlay and options bar.

use super::overlay::POINT;
use super::{MIN_LINE, RulerBar, RulerLine, RulerOverlay, RulerPart};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

fn line() -> RulerLine {
    RulerLine::new(Point::new(60.0, 80.0), Point::new(260.0, 140.0))
}

/// Shows the overlay over `line` as the only canvas control of a canvas.
fn on_canvas(ui: &mut Ui, line: &mut Option<RulerLine>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let shown = Canvas::new("Document").view(view).show(ui, |ui, body| RulerOverlay::new("Ruler", Bind::bound(&mut *line)).show_at(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

/// Drags the line's end.
fn drag_the_end(fixture: &mut Fixture) {
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    let end = fixture.root_id().and_then(|id| fixture.part(id, POINT.at(RulerPart::End.key())));
    if let Some(end) = end {
        fixture.drag(&end, Vec2::new(travel, 0.0));
    }
}

#[story(RulerOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_the_end))]
fn ruler_overlay_with_a_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data.value("line", Some(line())))
}

#[story(RulerOverlay<'static>, covers = [], drive = Drive::Never)]
fn ruler_overlay_with_a_protractor_arm(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data.value("arm", Some(line().with_protractor(Point::new(120.0, 200.0)))))
}

#[story(RulerBar<'static>, covers = [], drive = Drive::Never)]
fn ruler_bar_with_a_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let use_scale = data.value("scale", false);
    let mut output = None;
    Bar::new("Options", BarKind::Options).show(ui, |ui, _| output = Some(RulerBar::new(Some(line()), Bind::bound(&mut *use_scale)).show(ui)));
    output.map(StoryOutput::from).unwrap_or_default()
}

#[story(RulerBar<'static>, covers = [], drive = Drive::Never)]
fn ruler_bar_without_a_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let use_scale = data.value("none", false);
    let mut output = None;
    Bar::new("Options", BarKind::Options).show(ui, |ui, _| output = Some(RulerBar::new(None, Bind::bound(&mut *use_scale)).show(ui)));
    output.map(StoryOutput::from).unwrap_or_default()
}

/// The overlay refuses a line shorter than the least the tool keeps.
fn refuse_a_short_line(line: &Option<RulerLine>) -> Validity<Option<RulerLine>> {
    if line.is_some_and(|line| line.reading().l1 < MIN_LINE) { Validity::Refuse(Message::new("The line is too short")) } else { Validity::Ok }
}

#[story(RulerOverlay<'static>, covers = [Invalid], drive = Drive::Never)]
fn ruler_overlay_with_a_short_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let line = data.value("short", Some(RulerLine::new(Point::new(60.0, 80.0), Point::new(60.0, 80.0))));
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| RulerOverlay::new("Ruler", Bind::bound(&mut *line).check(refuse_a_short_line)).show_at(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}
