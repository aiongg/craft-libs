//! Stories of the perspective crop quad.

use super::{CORNER, PerspectiveCropQuad, PerspectiveQuad};
use craft_behavior::Mixed;
use craft_behavior::kurbo::Point;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// A skewed page marked from corner to corner: a quad that can be committed.
const PAGE: [Point; 4] = [Point::new(250.0, 110.0), Point::new(430.0, 130.0), Point::new(400.0, 290.0), Point::new(220.0, 250.0)];
/// The same corners with the last one pulled in: the quad is not convex.
const BENT: [Point; 4] = [Point::new(250.0, 110.0), Point::new(430.0, 130.0), Point::new(400.0, 290.0), Point::new(380.0, 150.0)];

/// Drags the top left corner by twice the drag threshold.
fn drag_corner(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(grip) = fixture.part(id, CORNER.at(0)) else { return };
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&grip, Vec2::new(travel, 0.0));
}

fn on_canvas(ui: &mut Ui, bind: Bind<'_, PerspectiveQuad>) -> StoryOutput {
    let mut bind = Some(bind);
    let shown = Canvas::new("Document")
        .show(ui, |ui, body| bind.take().map(|bind| PerspectiveCropQuad::new("Perspective crop", bind).show_at(ui, body.rect)));
    shown.inner.flatten().map(|out| StoryOutput::from(out.edit)).unwrap_or_default()
}

#[story(PerspectiveCropQuad<'static>, covers = [Default], drive = Drive::Custom(drag_corner))]
fn skewed_page(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, Bind::bound(data.value("quad", Some(PAGE))))
}

#[story(PerspectiveCropQuad<'static>, covers = [Mixed], drive = Drive::Custom(drag_corner))]
fn quads_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, Bind::bound_mixed(data.value("quad", Mixed::<PerspectiveQuad>::Mixed)))
}

#[story(PerspectiveCropQuad<'static>, covers = [Invalid], drive = Drive::Custom(drag_corner))]
fn bent_page(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let quad = Bind::bound(data.value("quad", Some(BENT))).check(|quad: &PerspectiveQuad| match quad {
        Some(quad) if !super::is_valid(quad) => Validity::Refuse(Message::new("The corners do not make a convex quad")),
        _ => Validity::Ok,
    });
    on_canvas(ui, quad)
}
