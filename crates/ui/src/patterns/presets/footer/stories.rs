//! Stories of `PresetFooter`.

use super::PresetFooter;
use crate::contract::Bind;
use crate::sections::FooterAction;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::IconId;
use egui::Ui;

#[story(PresetFooter<'static, u32>, covers = [], drive = Drive::Never)]
fn preset_footer(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PresetFooter::<u32>::new(Bind::bound(data.value("zoom", 1.0f64)))
        .count(12)
        .can_delete(false)
        .action(FooterAction::new(IconId::new("settings"), "Preset settings"))
        .show(ui)
        .into()
}
