//! Stories of `FontPicker`: the closed field, the list open with recents and a typed query, and the list open with
//! the class column and a filter on. They draw from the two faces egui ships in an in-memory catalog.

use std::sync::{Arc, OnceLock};

use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Key, Modifiers, Ui};

use super::{FontFilters, FontPicker, FontPickerSettings};
use crate::Catalog;
use crate::panel::fixtures::story_catalog;

/// The catalog every story shares, so its families are read once.
fn catalog() -> Arc<Catalog> {
    static CATALOG: OnceLock<Arc<Catalog>> = OnceLock::new();
    Arc::clone(CATALOG.get_or_init(story_catalog))
}

/// What a story keeps between passes.
#[derive(Clone, Debug, PartialEq)]
struct Demo {
    family: String,
    settings: FontPickerSettings,
    filters: FontFilters,
}

fn demo(family: &str, settings: FontPickerSettings, filters: FontFilters) -> Demo {
    Demo { family: family.to_owned(), settings, filters }
}

fn show_picker(ui: &mut Ui, data: &mut StoryData, init: Demo) -> StoryOutput {
    let catalog = catalog();
    let Demo { family, settings, filters } = data.value("picker", init);
    FontPicker::new("Font family", Bind::bound(family), &catalog).settings(Bind::bound(settings)).filters(Bind::bound(filters)).show(ui).into()
}

#[story(FontPicker<'static>, covers = [Default], drive = Drive::Never)]
fn font_picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show_picker(ui, data, demo("Ubuntu", FontPickerSettings::default(), FontFilters::default()))
}

/// Focuses the field and types the start of a name, which opens the list on the matches.
fn type_query(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("u");
    fixture.idle(4);
}

#[story(FontPicker<'static>, covers = [Open], drive = Drive::Custom(type_query))]
fn font_picker_open_with_query(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let settings = FontPickerSettings { recent: vec!["Ubuntu".to_owned()], ..FontPickerSettings::default() };
    show_picker(ui, data, demo("Ubuntu", settings, FontFilters::default()))
}

/// Focuses the field and opens the list on every family with Alt+Down.
fn open_list(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
}

#[story(FontPicker<'static>, covers = [], drive = Drive::Custom(open_list))]
fn font_picker_open_with_filters(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let settings = FontPickerSettings { favorites: vec!["Hack".to_owned()], ..FontPickerSettings::default() };
    show_picker(ui, data, demo("Hack", settings, FontFilters { favorites: true, ..FontFilters::default() }))
}
