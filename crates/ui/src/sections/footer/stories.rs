//! Stories of the footer actions.

use super::{DropAction, FooterAction, FooterBar};
use crate::buttons::IconButton;
use crate::contract::{PopupKind, Props as _, Show as _, ShowContainer as _};
use crate::display::Text;
use crate::layout_components::Stack;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::IconId;
use craft_theme::variant::Tone;
use egui::Ui;

#[story(FooterBar, covers = [], drive = Drive::Never)]
fn footer_actions(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let actions = [
        FooterAction::new(IconId::new("adjustment_layer"), "New fill or adjustment layer").popup(PopupKind::Menu).expanded(false),
        FooterAction::new(IconId::new("settings"), "Layer settings").pressed(true).tone(Tone::Accent),
        FooterAction::new(IconId::new("plus"), "New layer"),
        FooterAction::new(IconId::new("trash"), "Delete layer").accepts_drops::<u8>(|_| true).takes_drops_disabled(true).enabled(false),
    ];
    let mut output = None;
    let _ = Stack::new().footer(|ui, _| output = Some(FooterBar::actions(actions.clone()).show_footer(ui))).show(ui, |ui, _| {
        Text::new("Layers").show(ui);
        Text::new("Background layer, Group 1, Title and Levels 1").show(ui);
    });
    output.map_or_else(StoryOutput::default, Into::into)
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
