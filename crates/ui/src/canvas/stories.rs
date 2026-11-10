//! Stories of the canvas family.

use super::{Canvas, CropOverlay};
use crate::contract::{Bind, Show as _, ShowContainer as _};
use crate::testing::canvas_story::{StoryDocument, origin_view, show_document};
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::CanvasTransform;
use craft_behavior::geometry::CropBox;
use craft_behavior::kurbo::Rect as DocRect;
use egui::{Ui, Vec2};

/// How far the zoomed story magnifies the document.
const ZOOM: f32 = 2.0;

fn crop() -> CropBox {
    CropBox::upright(DocRect::new(40.0, 30.0, 200.0, 140.0))
}

#[story(Canvas, covers = [Default])]
fn crop_tool(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let room = crate::testing::stage(ui);
    let shown = Canvas::new("Document").view(origin_view(room, 1.0)).show_at(ui, room, |ui, body| {
        show_document(ui, body.rect, &StoryDocument::STANDARD);
        CropOverlay::new("Crop", Bind::given(crop())).show_at(ui, body.rect)
    });
    shown.output.into()
}

#[story(Canvas, covers = [Default])]
fn zoomed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let room = crate::testing::stage(ui);
    let shown = Canvas::new("Document")
        .transform(CanvasTransform::new(ZOOM, Vec2::ZERO))
        .show_at(ui, room, |ui, body| CropOverlay::new("Crop", Bind::given(crop())).show_at(ui, body.rect));
    shown.output.into()
}

#[story(Canvas, covers = [Default])]
fn freehand(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let room = crate::testing::stage(ui);
    Canvas::new("Painting")
        .view(origin_view(room, 1.0))
        .show_at(ui, room, |ui, body| show_document(ui, body.rect, &StoryDocument::STANDARD))
        .output
        .into()
}
