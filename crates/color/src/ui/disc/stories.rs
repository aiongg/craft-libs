//! Stories of `ColorDisc`: three markers with one selected.

use super::{ColorDisc, DiscMove};
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

const MARKERS: [Srgba; 3] = [Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.7, 0.4), Srgba::rgb(0.3, 0.4, 0.9)];

#[story(ColorDisc<'static>, covers = [Default], drive = Drive::Never)]
fn color_disc(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let moved = data.value("move", DiscMove::new(0, 0.0, 0.7));
    ColorDisc::new("Harmony", &MARKERS, Bind::bound(moved)).selected(1).show(ui).into()
}
