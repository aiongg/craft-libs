//! Stories of `ColorArea`: a hue-mode area and a web-safe one.

use super::ColorArea;
use crate::AreaMode;
use craft_theme::{ColorModel, HsbModel, Srgba};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(ColorArea<'static>, covers = [Default], drive = Drive::Auto)]
fn color_area(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", HsbModel.from_srgb(Srgba::rgb(0.2, 0.5, 0.8)));
    ColorArea::new("Saturation and brightness", AreaMode::Hue, Bind::bound(values)).show(ui).into()
}

#[story(ColorArea<'static>, covers = [], drive = Drive::Never)]
fn color_area_web_safe(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", HsbModel.from_srgb(Srgba::rgb(0.2, 0.6, 0.8)));
    ColorArea::new("Saturation and brightness", AreaMode::Hue, Bind::bound(values)).quantize(6).show(ui).into()
}
