//! Stories of `Select` and its option list.

use super::{Choices, OptionList, OptionRow, Select, SelectOption};
use crate::contract::{Bind, Props as _, Role, Show as _, ShowCollection as _, Validity};
use crate::runtime::env::{Label, Message};
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{Mixed, Selection};
use egui::Ui;

fn modes() -> [SelectOption<u8>; 3] {
    [SelectOption::new(0, "Normal"), SelectOption::new(1, "Multiply"), SelectOption::new(2, "Screen")]
}

#[story(Select<'static, u8>, covers = [Default], drive = Drive::Pick(1))]
fn select(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let options = modes();
    Select::new("Blend mode", Bind::bound(data.value("mode", 0u8)), &options).show(ui).into()
}

#[story(Select<'static, u8>, covers = [Mixed], drive = Drive::Never)]
fn select_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let options = modes();
    Select::new("Blend mode", Bind::bound_mixed(data.value("mixed", Mixed::<u8>::Mixed)), &options).show(ui).into()
}

#[story(Select<'static, u8>, covers = [Invalid], drive = Drive::Never)]
fn select_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let options = modes();
    let bind = Bind::bound(data.value("invalid", 2u8))
        .check(|mode| if *mode == 2 { Validity::Refuse(Message::new("Screen is not available here")) } else { Validity::Ok });
    Select::new("Blend mode", bind, &options).show(ui).into()
}

fn open_listbox(fixture: &mut Fixture) {
    if let Some(field) = fixture.by_role(Role::COMBO_BOX).first().copied() {
        fixture.click(&field);
    }
}

#[story(Select<'static, u8>, covers = [Open], drive = Drive::Custom(open_listbox))]
fn select_open(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let options = modes();
    Select::new("Blend mode", Bind::bound(data.value("open", 1u8)), &options).show(ui).into()
}

fn list(ui: &mut Ui, count: usize) -> StoryOutput {
    let labels: Vec<Label> = (1..=count).map(|n| Label::from(format!("Option {n}"))).collect();
    let choices = Choices { labels: &labels };
    OptionList::new(Bind::given(Selection::default()))
        .label("Blend mode")
        .show_items(ui, &choices, &mut |key, _| OptionRow::new(labels[*key].as_str()))
        .into()
}

#[story(OptionList<'static>, covers = [Empty], drive = Drive::Never)]
fn option_list_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    list(ui, 0)
}

#[story(OptionList<'static>, covers = [One], drive = Drive::Never)]
fn option_list_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    list(ui, 1)
}

#[story(OptionList<'static>, covers = [Default, Many], drive = Drive::Never)]
fn option_list_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    list(ui, 12)
}

#[story(OptionRow<'static>, covers = [Default], drive = Drive::Never)]
fn option_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    crate::testing::show_as_only_item(ui, OptionRow::new("Option")).into()
}
