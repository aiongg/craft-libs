//! Stories of the footer actions.

use super::{DropAction, FooterAction, FooterBar};
use crate::buttons::IconButton;
use crate::contract::{Props as _, Show as _, ShowContainer as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::IconId;
use egui::Ui;

#[story(FooterBar, covers = [], drive = Drive::Never)]
fn footer_actions(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let actions = [
        FooterAction::new(IconId::new("trash"), "Delete layer").accepts_drops::<u8>(|_| true).enabled(false),
        FooterAction::new(IconId::new("plus"), "New layer"),
        FooterAction::new(IconId::new("settings"), "Layer settings"),
    ];
    FooterBar::actions(actions).show_footer(ui).into()
}

#[story(DropAction<u8>, covers = [Default], drive = Drive::Never)]
fn drop_action(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    DropAction::new(|_: &u8| true)
        .show(ui, |ui, _| {
            IconButton::new(IconId::new("trash")).label("Delete layer").show(ui);
        })
        .output
        .into()
}
