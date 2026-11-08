//! Stories of the transform box: Free Transform with its reference point, the Move tool's controls and Distort.

use super::{BOX, RotateZone, TransformBox};
use craft_behavior::geometry::quad::{BoxPart, BoxRules, BoxState, TransformMode};
use craft_behavior::kurbo::Rect as DocRect;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// The picture before any transform, in document units.
const PICTURE: DocRect = DocRect::new(200.0, 120.0, 440.0, 280.0);

/// Shows the box `configure` builds over the story's box as the only canvas control of a canvas.
fn on_canvas(ui: &mut Ui, data: &mut StoryData, configure: impl for<'a> FnOnce(TransformBox<'a>) -> TransformBox<'a>) -> StoryOutput {
    let state = data.value("box", BoxState::from_rect(PICTURE));
    let mut configure = Some(configure);
    let shown = Canvas::new("Document").show(ui, |ui, body| {
        configure
            .take()
            .map(|configure| configure(TransformBox::new("Free Transform", Bind::bound(&mut *state), PICTURE, 0.0)).show_at(ui, body.rect))
    });
    shown.inner.flatten().map(|output| StoryOutput::from(output.edit)).unwrap_or_default()
}

/// Drags the bottom right corner handle.
fn drag_a_corner(fixture: &mut Fixture) {
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    let corner = fixture.root_id().and_then(|id| fixture.part(id, BOX.at(BoxPart::Corner(2).index())));
    if let Some(corner) = corner {
        fixture.drag(&corner, Vec2::new(travel, travel));
    }
}

#[story(TransformBox<'static>, covers = [Default], drive = Drive::Custom(drag_a_corner))]
fn free_transform(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, |control| control.reference_point(true))
}

#[story(TransformBox<'static>, covers = [Default])]
fn move_tool_controls(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, |control| control.rotate_zone(RotateZone::Band).inside_moves(false).reference_point(false))
}

#[story(TransformBox<'static>, covers = [Default])]
fn distort(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, |control| control.rules(BoxRules::in_mode(TransformMode::Distort)))
}
