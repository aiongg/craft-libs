//! Stories of `Divider`.

use super::Divider;
use crate::contract::Show as _;
use crate::contract::ShowContainer as _;
use crate::display::Text;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::Ui;

#[story(Divider, covers = [Default], drive = Drive::Never)]
fn divider(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Divider::horizontal().show(ui).into()
}

#[story(Divider, covers = [], drive = Drive::Never)]
fn divider_vertical(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    crate::layout_components::Row::new()
        .show(ui, |ui, _| {
            Text::new("Left").show(ui);
            Divider::vertical().show(ui)
        })
        .output
        .into()
}
