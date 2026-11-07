//! Stories of the canvas family.

use super::crop::CORNER;
use super::{Canvas, CropOverlay, ToolPath};
use crate::contract::{Bind, EditResponse, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{CanvasTransform, Mixed};
use egui::{Pos2, Rect, Ui, Vec2};

/// How far the zoomed story magnifies the document.
const ZOOM: f32 = 2.0;

/// The narrowest crop the invalid story accepts, in document units.
const LEAST_WIDTH: f32 = 100.0;

const CROP_TO: ToolPath = ToolPath::Command("the Crop To... command");

fn crop() -> Rect {
    Rect::from_min_max(Pos2::new(40.0, 30.0), Pos2::new(200.0, 140.0))
}

/// Shows `control` as the only canvas control of a canvas and returns the control's output.
fn on_canvas(ui: &mut Ui, mut control: impl FnMut(&mut Ui, Rect) -> EditResponse<Rect>) -> StoryOutput {
    let shown = Canvas::new("Document", CROP_TO).show(ui, |ui, body| control(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

/// Drags the top left corner towards the middle of the crop by twice the drag threshold.
fn drag_top_left(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(grip) = fixture.part(id, CORNER.at(0)) else { return };
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&grip, Vec2::new(travel, 0.0));
}

#[story(Canvas, covers = [Default])]
fn crop_tool(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let shown = Canvas::new("Document", CROP_TO).show(ui, |ui, body| CropOverlay::new("Crop", Bind::given(crop())).show_at(ui, body.rect));
    shown.output.response.into()
}

#[story(Canvas, covers = [Default])]
fn zoomed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let shown = Canvas::new("Document", CROP_TO)
        .transform(CanvasTransform::new(ZOOM, Vec2::ZERO))
        .show(ui, |ui, body| CropOverlay::new("Crop", Bind::given(crop())).show_at(ui, body.rect));
    shown.output.response.into()
}

#[story(Canvas, covers = [Default])]
fn freehand(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Canvas::new("Painting", ToolPath::Freehand).show(ui, |_, _| ()).output.response.into()
}

#[story(CropOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_top_left))]
fn crop_rect(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rect = data.value("crop", crop());
    on_canvas(ui, |ui, at| CropOverlay::new("Crop", Bind::bound(&mut *rect)).show_at(ui, at))
}

#[story(CropOverlay<'static>, covers = [Mixed], drive = Drive::Custom(drag_top_left))]
fn crops_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rect = data.value("crop", Mixed::<Rect>::Mixed);
    on_canvas(ui, |ui, at| CropOverlay::new("Crop", Bind::bound_mixed(&mut *rect)).show_at(ui, at))
}

#[story(CropOverlay<'static>, covers = [Invalid], drive = Drive::Custom(drag_top_left))]
fn too_narrow(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rect = data.value("crop", Rect::from_min_max(Pos2::new(40.0, 30.0), Pos2::new(40.0 + LEAST_WIDTH / 2.0, 110.0)));
    on_canvas(ui, |ui, at| {
        let narrow = |crop: &Rect| {
            if crop.width() < LEAST_WIDTH { Validity::Refuse(Message::new("The crop is too narrow")) } else { Validity::Ok }
        };
        CropOverlay::new("Crop", Bind::bound(&mut *rect).check(narrow)).show_at(ui, at)
    })
}
