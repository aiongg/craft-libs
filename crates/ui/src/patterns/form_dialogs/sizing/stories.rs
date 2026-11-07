//! Stories of `ImageSize` and `CanvasSize`.

use super::{CanvasSize, ImageSize};
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::PixelSize;
use egui::Ui;

const PICTURE: PixelSize = PixelSize::new(1600, 900);

#[story(ImageSize<'static>, covers = [], drive = Drive::Never)]
fn image_size(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ImageSize::new(PICTURE).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(CanvasSize<'static>, covers = [], drive = Drive::Never)]
fn canvas_size(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    CanvasSize::new(PICTURE).opened(Bind::bound(data.value("open", true))).show(ui).into()
}
