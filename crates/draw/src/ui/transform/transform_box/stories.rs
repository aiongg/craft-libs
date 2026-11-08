//! Stories of the transform box: Free Transform with its reference point, the Move tool's controls and Distort.

use super::{RotateZone, TransformBox};
use craft_behavior::geometry::quad::{BoxRules, BoxState, TransformMode};
use craft_behavior::kurbo::Rect as DocRect;
use craft_ui::canvas::{Canvas, ToolPath};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{StoryData, StoryOutput, story};
use egui::Ui;

/// The picture before any transform, in document units.
const PICTURE: DocRect = DocRect::new(200.0, 120.0, 440.0, 280.0);

const FREE_TRANSFORM: ToolPath = ToolPath::Command("the Transform fields of the options bar");

/// Shows the box `configure` builds over the story's box as the only canvas control of a canvas.
fn on_canvas(ui: &mut Ui, data: &mut StoryData, configure: impl for<'a> FnOnce(TransformBox<'a>) -> TransformBox<'a>) -> StoryOutput {
    let state = data.value("box", BoxState::from_rect(PICTURE));
    let shown = Canvas::new("Document", FREE_TRANSFORM)
        .show(ui, |ui, body| configure(TransformBox::new("Free Transform", Bind::bound(&mut *state), PICTURE, 0.0)).show_at(ui, body.rect));
    shown.inner.map(|output| StoryOutput::from(output.edit)).unwrap_or_default()
}

#[story(TransformBox<'static>, covers = [Default])]
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
