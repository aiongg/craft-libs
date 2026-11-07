//! Stories of `ApplyRow`.

use super::ApplyRow;
use crate::contract::{Bind, Role, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use crate::text::NumberField;
use craft_behavior::Mixed;
use egui::Ui;

fn toggle_apply(fixture: &mut Fixture) {
    if let Some(check) = fixture.by_role(Role::CHECK_BOX).into_iter().next() {
        fixture.click(&check);
    }
}

#[story(ApplyRow<'static>, covers = [Default], drive = Drive::Custom(toggle_apply))]
fn apply_row(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let value = data.value("opacity", Mixed::<Option<f64>>::Mixed).clone();
    ApplyRow::new("Opacity", Bind::bound(data.value("apply", false)))
        .show(ui, |ui, _| {
            NumberField::new("Opacity", Bind::given(value.clone())).show(ui);
        })
        .into()
}

#[story(ApplyRow<'static>, covers = [], drive = Drive::Never)]
fn apply_row_on(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ApplyRow::new("Opacity", Bind::bound(data.value("apply", true)))
        .show(ui, |ui, _| {
            NumberField::new("Opacity", Bind::given(Some(80.0))).show(ui);
        })
        .into()
}

#[story(ApplyRow<'static>, covers = [Mixed], drive = Drive::Never)]
fn apply_row_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ApplyRow::new("Opacity", Bind::bound_mixed(data.value("mixed", Mixed::<bool>::Mixed))).show(ui, |_, _| {}).into()
}

#[story(ApplyRow<'static>, covers = [Invalid], drive = Drive::Never)]
fn apply_row_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("Not available")) } else { Validity::Ok };
    ApplyRow::new("Opacity", Bind::bound(data.value("apply", true)).check(refuse_on)).show(ui, |_, _| {}).into()
}
