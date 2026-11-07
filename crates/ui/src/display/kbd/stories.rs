//! Stories of `Kbd`.

use super::Kbd;
use crate::contract::Show as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(Kbd, covers = [Default], drive = Drive::Never)]
fn kbd(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Kbd::new("Ctrl+Shift+S").show(ui).into()
}

#[story(Kbd, covers = [], drive = Drive::Never)]
fn kbd_keycap(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Kbd::new("Ctrl+Shift+S").keycap(true).show(ui).into()
}
