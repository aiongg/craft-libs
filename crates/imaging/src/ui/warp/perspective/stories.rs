//! Stories of Perspective Warp: the planes on a plain canvas, and the options bar.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, Grip, StoryData, StoryOutput, story};
use egui::{Ui, vec2};

use super::overlay::CORNER;
use super::{PerspectiveBar, PerspectiveEdit, PerspectiveSession, PerspectiveWarp, PerspectiveWarpOutput, Plane, PlaneMode};

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Plane {
    Plane::identity([Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)])
}

/// Two planes that share the corners on x = 150.
fn two_linked_planes() -> Vec<Plane> {
    vec![square(30.0, 40.0, 150.0, 200.0), square(150.0, 40.0, 270.0, 200.0)]
}

/// Shows `planes` on a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, bind: Bind<'_, Vec<Plane>>, mode: PlaneMode) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut control = Some(PerspectiveWarp::new("Planes", bind, mode, 0));
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| -> Option<PerspectiveWarpOutput> { control.take().map(|control| control.show_at(ui, body.rect)) });
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

#[story(PerspectiveWarp<'static>, covers = [Default], drive = Drive::Drag(Grip::part(CORNER.at(1)).toward(vec2(1.0, -1.0))))]
fn two_linked_planes_in_layout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let planes = data.value("planes", two_linked_planes());
    on_canvas(ui, Bind::bound(&mut *planes), PlaneMode::Layout)
}

#[story(PerspectiveWarp<'static>, covers = [Default], drive = Drive::Drag(Grip::part(CORNER.at(1)).toward(vec2(1.0, -1.0))))]
fn two_linked_planes_in_warp(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let planes = data.value("warped", two_linked_planes());
    on_canvas(ui, Bind::bound(&mut *planes), PlaneMode::Warp)
}

/// A session over two linked planes, in `mode`.
fn session(mode: PlaneMode) -> PerspectiveSession {
    let mut session = PerspectiveSession::new(DocRect::new(0.0, 0.0, 300.0, 240.0));
    session.apply(PerspectiveEdit::replace(&[], two_linked_planes(), "Planes"));
    session.apply(PerspectiveEdit::Mode(mode));
    session
}

#[story(PerspectiveBar<'static>, covers = [Default], drive = Drive::Never)]
fn bar_in_layout(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    PerspectiveBar::new(&session(PlaneMode::Layout)).show(ui);
    StoryOutput::default()
}

#[story(PerspectiveBar<'static>, covers = [], drive = Drive::Never)]
fn bar_in_warp(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    PerspectiveBar::new(&session(PlaneMode::Warp)).show(ui);
    StoryOutput::default()
}
