//! Stories of `Text`.

use super::{Elide, Shade, Text};
use crate::contract::{Props as _, Show as _};
use crate::style::TypeRole;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(Text, covers = [Default], drive = Drive::Never)]
fn text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::new("Layers").show(ui).into()
}

#[story(Text, covers = [], drive = Drive::Never)]
fn text_role(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::new("Blend mode").role(TypeRole::Caption).shade(Shade::Faint).show(ui).into()
}

#[story(Text, covers = [], drive = Drive::Never)]
fn text_elided(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::new("A layer name much longer than the room it is shown in").elide(Elide::End).show(ui).into()
}
