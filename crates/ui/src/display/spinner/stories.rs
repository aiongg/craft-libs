//! Stories of `Spinner`.

use super::Spinner;
use crate::contract::{Props as _, Show as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(Spinner, covers = [Default], drive = Drive::Never)]
fn spinner(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Spinner::new().label("Loading").show(ui).into()
}
