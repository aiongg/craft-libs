//! Stories of Perspective Warp: the planes on a plain canvas, and the options bar.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

use super::overlay::CORNER;
use super::{PerspectiveBar, PerspectiveEdit, PerspectiveSession, PerspectiveWarp, PerspectiveWarpOutput, Plane, PlaneCorner, PlaneMode};

/// The most planes the invalid story accepts.
const MOST_PLANES: usize = 1;

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

/// Drags the top corner the two planes share by twice the drag threshold.
fn drag_shared_corner(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let corner = PlaneCorner::new(0, 1);
    let Some(target) = fixture.part(id, CORNER.at((corner.plane * 4 + corner.corner) as u64)) else { return };
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&target, Vec2::new(travel, -travel));
}

#[story(PerspectiveWarp<'static>, covers = [Default], drive = Drive::Custom(drag_shared_corner))]
fn two_linked_planes_in_layout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let planes = data.value("planes", two_linked_planes());
    on_canvas(ui, Bind::bound(&mut *planes), PlaneMode::Layout)
}

#[story(PerspectiveWarp<'static>, covers = [Default], drive = Drive::Custom(drag_shared_corner))]
fn two_linked_planes_in_warp(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let planes = data.value("warped", two_linked_planes());
    on_canvas(ui, Bind::bound(&mut *planes), PlaneMode::Warp)
}

#[story(PerspectiveWarp<'static>, covers = [Invalid], drive = Drive::Never)]
fn too_many_planes(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let planes = data.value("invalid", two_linked_planes());
    let few =
        |planes: &Vec<Plane>| if planes.len() > MOST_PLANES { Validity::Refuse(Message::new("There are too many planes")) } else { Validity::Ok };
    on_canvas(ui, Bind::bound(&mut *planes).check(few), PlaneMode::Layout)
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
