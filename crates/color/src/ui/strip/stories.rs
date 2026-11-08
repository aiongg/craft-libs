//! Stories of `ColorStrip`: a hue strip and a web-safe one.

use super::ColorStrip;
use crate::AreaMode;
use craft_theme::{ColorModel, HsbModel, Srgba};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(ColorStrip<'static>, covers = [Default], drive = Drive::Step)]
fn color_strip(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", HsbModel.from_srgb(Srgba::rgb(0.2, 0.5, 0.8)));
    ColorStrip::new("Hue", AreaMode::Hue, Bind::bound(values)).show(ui).into()
}

#[story(ColorStrip<'static>, covers = [], drive = Drive::Never)]
fn color_strip_web_safe(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", HsbModel.from_srgb(Srgba::rgb(0.2, 0.6, 0.8)));
    ColorStrip::new("Hue", AreaMode::Hue, Bind::bound(values)).quantize(6).show(ui).into()
}
