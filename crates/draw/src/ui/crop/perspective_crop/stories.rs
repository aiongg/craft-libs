//! Stories of the perspective crop quad.

use super::{CORNER, PerspectiveCropQuad, PerspectiveQuad};
use craft_behavior::kurbo::Point;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Grip, StoryData, StoryOutput, story};
use egui::Ui;

/// A skewed page marked from corner to corner: a quad that can be committed.
const PAGE: [Point; 4] = [Point::new(250.0, 110.0), Point::new(430.0, 130.0), Point::new(400.0, 290.0), Point::new(220.0, 250.0)];
/// The same corners with the last one pulled in: the quad is not convex.
const BENT: [Point; 4] = [Point::new(250.0, 110.0), Point::new(430.0, 130.0), Point::new(400.0, 290.0), Point::new(380.0, 150.0)];

fn on_canvas(ui: &mut Ui, bind: Bind<'_, PerspectiveQuad>) -> StoryOutput {
    let mut bind = Some(bind);
    let shown = Canvas::new("Document")
        .show(ui, |ui, body| bind.take().map(|bind| PerspectiveCropQuad::new("Perspective crop", bind).show_at(ui, body.rect)));
    shown.inner.flatten().map(|out| StoryOutput::from(out.edit)).unwrap_or_default()
}

#[story(PerspectiveCropQuad<'static>, covers = [Default], drive = Drive::Drag(Grip::part(CORNER.at(0))))]
fn skewed_page(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, Bind::bound(data.value("quad", Some(PAGE))))
}

#[story(PerspectiveCropQuad<'static>, covers = [Invalid], drive = Drive::Drag(Grip::part(CORNER.at(0))))]
fn bent_page(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let quad = Bind::bound(data.value("quad", Some(BENT))).check(|quad: &PerspectiveQuad| match quad {
        Some(quad) if !super::is_valid(quad) => Validity::Refuse(Message::new("The corners do not make a convex quad")),
        _ => Validity::Ok,
    });
    on_canvas(ui, quad)
}
