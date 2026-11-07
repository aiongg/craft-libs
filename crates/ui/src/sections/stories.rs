//! Stories of the sections family: `GroupBox` and `TitleRow`.

use super::{GroupBox, TitleRow};
use crate::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _, Validity};
use crate::controls::check::Checkbox;
use crate::controls::slider::Slider;
use crate::layout_components::Stack;
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

/// What the story boxes hold: a check box and a slider of fixed values.
fn body(ui: &mut Ui) {
    Stack::new().show(ui, |ui, _| {
        Checkbox::new(Bind::given(true)).label("Snap to grid").show(ui);
        Slider::new(Bind::given(30.0), 0.0..=100.0).label("Spacing").show(ui);
    });
}

fn toggle_gate(fixture: &mut Fixture) {
    let gate = fixture.by_role(Role::CHECK_BOX).into_iter().find(|check| fixture.tree().name_of(check.node) == Some("Enable"));
    if let Some(gate) = gate {
        fixture.click(&gate);
    }
}

#[story(GroupBox<'static>, covers = [Default], drive = Drive::Custom(toggle_gate))]
fn group_box(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    GroupBox::new("Grid", Some(Bind::bound(data.value("gate", true)))).show(ui, |ui, _| body(ui)).into()
}

#[story(GroupBox<'static>, covers = [], drive = Drive::Never)]
fn group_box_without_gate(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    GroupBox::new("Grid", None).show(ui, |ui, _| body(ui)).into()
}

#[story(GroupBox<'static>, covers = [], drive = Drive::Never)]
fn group_box_gate_off(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    GroupBox::new("Grid", Some(Bind::bound(data.value("gate", false)))).show(ui, |ui, _| body(ui)).into()
}

#[story(GroupBox<'static>, covers = [Mixed], drive = Drive::Never)]
fn group_box_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    GroupBox::new("Grid", Some(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)))).show(ui, |ui, _| body(ui)).into()
}

#[story(GroupBox<'static>, covers = [Invalid], drive = Drive::Never)]
fn group_box_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("Not available")) } else { Validity::Ok };
    GroupBox::new("Grid", Some(Bind::bound(data.value("gate", true)).check(refuse_on))).show(ui, |ui, _| body(ui)).into()
}

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
