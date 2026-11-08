//! Stories of the LUT picker.

use super::{LutChoice, LutPicker};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(LutPicker<'static>, covers = [Default], drive = Drive::Never)]
fn lut_picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    LutPicker::new(Bind::bound(data.value("lut", LutChoice::Builtin("warm")))).label("3D LUT File").show(ui).into()
}

#[story(LutPicker<'static>, covers = [], drive = Drive::Never)]
fn lut_picker_loaded(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    LutPicker::new(Bind::bound(data.value("lut", LutChoice::loaded("Film", 33)))).label("3D LUT File").show(ui).into()
}
