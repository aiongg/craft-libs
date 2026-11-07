//! Stories of `Watermark`.

use super::{Watermark, WatermarkSettings, WatermarkStyle};
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(Watermark<'static>, covers = [], drive = Drive::Never)]
fn watermark(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let value = data.value("watermark", WatermarkSettings { style: WatermarkStyle::Text("© Studio".to_owned()), ..WatermarkSettings::default() });
    Watermark::new("Watermark", Bind::bound(value)).show(ui).into()
}

#[story(Watermark<'static>, covers = [], drive = Drive::Never)]
fn watermark_graphic(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let value =
        data.value("watermark-graphic", WatermarkSettings { style: WatermarkStyle::Graphic("logo.png".to_owned()), ..WatermarkSettings::default() });
    Watermark::new("Watermark", Bind::bound(value)).show(ui).into()
}
