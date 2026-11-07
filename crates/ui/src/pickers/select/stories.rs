//! Stories of `Select`.

use super::Select;
use crate::contract::{Bind, Entry, Props as _, Role, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

fn modes() -> [Entry<u8>; 3] {
    [Entry::new(0, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")]
}

#[story(Select<'static, u8>, covers = [Default], drive = Drive::Pick { row: 1 })]
fn select(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let entries = modes();
    Select::new("Blend mode", Bind::bound(data.value("mode", 0u8)), &entries).show(ui).into()
}

#[story(Select<'static, u8>, covers = [Mixed], drive = Drive::Never)]
fn select_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let entries = modes();
    Select::new("Blend mode", Bind::bound_mixed(data.value("mixed", Mixed::<u8>::Mixed)), &entries).show(ui).into()
}

#[story(Select<'static, u8>, covers = [Invalid], drive = Drive::Never)]
fn select_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let entries = modes();
    let bind = Bind::bound(data.value("invalid", 2u8))
        .check(|mode| if *mode == 2 { Validity::Refuse(Message::new("Screen is not available here")) } else { Validity::Ok });
    Select::new("Blend mode", bind, &entries).show(ui).into()
}

fn open_listbox(fixture: &mut Fixture) {
    if let Some(field) = fixture.by_role(Role::COMBO_BOX).first().copied() {
        fixture.click(&field);
    }
}

#[story(Select<'static, u8>, covers = [Open], drive = Drive::Custom(open_listbox))]
fn select_open(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let entries = modes();
    Select::new("Blend mode", Bind::bound(data.value("open", 1u8)), &entries).show(ui).into()
}

#[story(Select<'static, u8>, covers = [], disabled_text = "Pick a layer first")]
fn select_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let entries = modes();
    let tooltip = data.tooltip("Blend mode of the layer");
    Select::new("Blend mode", Bind::bound(data.value("mode", 0u8)), &entries).tooltip(tooltip).show(ui).into()
}
