//! Stories of `HarmonyWheel` and `HarmonyPanel`.

use super::{HarmonyPanel, HarmonyPanelState, HarmonyPicked, HarmonyWheel};
use craft_theme::Srgba;
use craft_ui::contract::Bind;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

const BASE: Srgba = Srgba::rgb(0.24, 0.39, 0.87);

#[story(HarmonyPanel<'static>, covers = [], drive = Drive::Never)]
fn harmony_panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (state, picked) = data.value("guide", (HarmonyPanelState::default(), HarmonyPicked::default()));
    HarmonyPanel::new(Bind::bound(state), Bind::bound(picked), BASE).show(ui).into()
}

#[story(HarmonyWheel<'static>, covers = [], drive = Drive::Never)]
fn harmony_wheel(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let colors = crate::harmony::Harmony::Triad.apply(BASE);
    HarmonyWheel::new("Harmony", &colors).selected(Some(1)).show(ui).into()
}
