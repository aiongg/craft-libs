//! Stories of `ColorReadout`.

use craft_theme::Rgba;
use craft_ui::contract::Bind;
use craft_ui::contract::Show as _;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::ColorReadout;

#[story(ColorReadout<'static>, covers = [Default], drive = Drive::Never)]
fn color_readout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let color = data.value("color", Rgba::rgb(0x33, 0x66, 0x99));
    ColorReadout::new("Color", Bind::bound(color)).show(ui).into()
}
