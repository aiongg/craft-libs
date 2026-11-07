//! Stories of the text family: `TextField<PlainText>`, `NumberField` and `SelectableText`.

use super::{NumberField, SelectableText, TextField};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::primitives::{Completer, Completion};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{Mixed, PlainText};
use egui::{Key, Modifiers, Ui};
use std::ops::Range;

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

/// Completes the words that start with the text before the caret.
struct Prefixes;

impl Completer for Prefixes {
    fn complete(&self, text: &str, caret: usize, out: &mut Vec<Completion>) -> Option<Range<usize>> {
        let before = text.get(..caret)?;
        let hits: Vec<_> = ["alpha", "alps", "beta"].into_iter().filter(|word| !before.is_empty() && word.starts_with(before)).collect();
        out.extend(hits.iter().map(|word| Completion::new(*word, *word)));
        (!hits.is_empty()).then_some(0..caret)
    }
}

/// Focuses the field and types the start of a word, which opens the completion list.
fn open_completions(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("al");
    fixture.idle(4);
}

#[story(TextField<'static, PlainText>, covers = [Open], drive = Drive::Custom(open_completions))]
fn text_field_open(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    TextField::new("Name", Bind::given(String::new())).complete(Prefixes).show(ui).into()
}

const SIZE: f64 = 12.5;
const HUGE: f64 = 123_456_789_012_345.0;

#[story(NumberField<'static>, covers = [Default], drive = Drive::Type("42"))]
fn number_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("size", Some(SIZE)))).show(ui).into()
}

#[story(NumberField<'static>, covers = [LongText], drive = Drive::Never)]
fn number_field_long(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("long", Some(HUGE)))).show(ui).into()
}

#[story(NumberField<'static>, covers = [Mixed], drive = Drive::Never)]
fn number_field_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound_mixed(data.value("mixed", Mixed::<Option<f64>>::Mixed))).show(ui).into()
}

#[story(NumberField<'static>, covers = [Invalid], drive = Drive::Never)]
fn number_field_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_negative = |size: &Option<f64>| match size {
        Some(size) if *size < 0.0 => Validity::Refuse(Message::new("Size cannot be negative")),
        _ => Validity::Ok,
    };
    NumberField::new("Size", Bind::bound(data.value("negative", Some(-SIZE))).check(refuse_negative)).show(ui).into()
}

#[story(SelectableText<'static>, covers = [Default], drive = Drive::Never)]
fn selectable_text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    SelectableText::new("Path", "/projects/poster.indd").show(ui).into()
}

#[story(SelectableText<'static>, covers = [LongText], drive = Drive::Never)]
fn selectable_text_long(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    SelectableText::new("Path", LONG).show(ui).into()
}
