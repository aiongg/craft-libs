//! Stories of the text family: `TextField<PlainText>` in every state.

use super::TextField;
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::{Mixed, PlainText};
use egui::Ui;

const LONG: &str = "A name that runs on well past the end of the field, so the field scrolls its text under the caret";

#[story(TextField<'static, PlainText>, covers = [Default])]
fn text_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::bound(data.value("name", String::new()))).placeholder("Untitled").show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [LongText], drive = Drive::Never)]
fn text_field_long(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::bound(data.value("long", LONG.to_owned()))).show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [Mixed], drive = Drive::Never)]
fn text_field_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::bound_mixed(data.value("mixed", Mixed::<String>::Mixed))).placeholder("Mixed").show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [Invalid], drive = Drive::Never)]
fn text_field_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_empty = |text: &String| if text.is_empty() { Validity::Refuse(Message::new("A name is required")) } else { Validity::Ok };
    TextField::new("Name", Bind::bound(data.value("empty", String::new())).check(refuse_empty)).show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [], drive = Drive::Never)]
fn text_field_read_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::bound(data.value("name", "Locked".to_owned()))).read_only(true).show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [], drive = Drive::Never)]
fn text_field_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::bound(data.value("name", "Off".to_owned()))).enabled(false).show(ui).into()
}

#[story(TextField<'static, PlainText>, covers = [], drive = Drive::Type("query"))]
fn text_field_search(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TextField::search("Search", Bind::bound(data.value("query", String::new()))).placeholder("Search").show(ui).into()
}
