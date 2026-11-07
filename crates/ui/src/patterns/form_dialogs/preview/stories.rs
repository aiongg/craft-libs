//! Stories of `PreviewPane` and `PreviewZoom`.

use super::{PreviewPane, PreviewZoom, WELL};
use crate::contract::Show as _;
use crate::primitives::TextureCache;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::CanvasTransform;
use egui::load::SizedTexture;
use egui::{Color32, ColorImage, Ui, Vec2};

/// The picture's side in points at zoom 1, wider than the well so the pane pans.
const PICTURE: [usize; 2] = [480, 320];

fn picture(ui: &Ui) -> SizedTexture {
    let id = TextureCache::texture(ui.ctx(), "form-dialogs-preview-story", || ColorImage::filled(PICTURE, Color32::from_rgb(0x4a, 0x7f, 0xc1)));
    SizedTexture::new(id, Vec2::new(PICTURE[0] as f32, PICTURE[1] as f32))
}

#[story(PreviewPane, covers = [Default], drive = Drive::Drag(WELL))]
fn preview_pane(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", CanvasTransform::new(1.0, Vec2::ZERO));
    let shown = PreviewPane::new("Preview", picture(ui), *view).show(ui);
    if let Some(next) = shown.view {
        *view = next;
    }
    shown.into()
}

#[story(PreviewZoom, covers = [], drive = Drive::Never)]
fn preview_zoom(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", CanvasTransform::new(1.0, Vec2::ZERO));
    let shown = PreviewZoom::new(*view).show(ui);
    if let Some(next) = shown.view {
        *view = next;
    }
    shown.into()
}
