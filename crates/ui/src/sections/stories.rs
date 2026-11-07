//! Stories of the sections family: `TitleRow`.

use super::TitleRow;
use crate::contract::{Bind, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

#[story(TitleRow<'static>, covers = [Default])]
fn title_row(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TitleRow::new("Layers").disclosure(Bind::bound(data.value("open", false))).show(ui).edit.into()
}

#[story(TitleRow<'static>, covers = [Mixed], drive = Drive::Never)]
fn title_row_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TitleRow::new("Grid").gate(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).show(ui).edit.into()
}

#[story(TitleRow<'static>, covers = [Invalid], drive = Drive::Never)]
fn title_row_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("Not available")) } else { Validity::Ok };
    TitleRow::new("Grid").gate(Bind::bound(data.value("gate", true)).check(refuse_on)).show(ui).edit.into()
}
