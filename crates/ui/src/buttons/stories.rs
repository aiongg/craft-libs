//! Stories of the buttons.

use super::{Button, IconButton, Link, SplitButton, ToggleButton};
use crate::contract::{Bind, Entry, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{IconId, Mixed};
use egui::{Key, Modifiers, Ui};

#[story(Button<'static>, covers = [Default])]
fn button(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Button::new("Save the document as a copy").icon(IconId::new("save")).show(ui).into()
}

#[story(Button<'static>, covers = [], disabled_text = "Nothing to save")]
fn button_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Button::new("Save").tooltip(data.tooltip("Save the document")).show(ui).into()
}

#[story(Button<'static>, covers = [], drive = Drive::Never)]
fn button_loading(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Button::new("Saving").icon(IconId::new("save")).loading(true).show(ui).into()
}

#[story(IconButton<'static>, covers = [Default])]
fn icon_button(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    IconButton::new(IconId::new("settings")).label("Settings").show(ui).into()
}

#[story(ToggleButton<'static>, covers = [Default])]
fn toggle_button(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ToggleButton::new("Snap to grid", Bind::bound(data.value("on", false))).icon(IconId::new("eye")).show(ui).into()
}

#[story(ToggleButton<'static>, covers = [Mixed], drive = Drive::Never)]
fn toggle_button_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ToggleButton::new("Snap to grid", Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).show(ui).into()
}

#[story(ToggleButton<'static>, covers = [Invalid], drive = Drive::Never)]
fn toggle_button_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("Snapping is not available here")) } else { Validity::Ok };
    ToggleButton::new("Snap to grid", Bind::bound(data.value("invalid", true)).check(refuse_on)).show(ui).into()
}

fn presets() -> [Entry<u8>; 4] {
    [
        Entry::new(0, "Black and white").icon(IconId::new("eye")).checked(true),
        Entry::new(1, "Colour"),
        Entry::separator(),
        Entry::new(2, "Grayscale").enabled(false),
    ]
}

#[story(SplitButton<'static, u8>, covers = [Default], drive = Drive::Activate)]
fn split_button(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let entries = presets();
    SplitButton::new("Image Trace", &entries).arrow_label("Tracing presets").show(ui).into()
}

fn open_menu(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
}

#[story(SplitButton<'static, u8>, covers = [Open], drive = Drive::Custom(open_menu))]
fn split_button_open(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let entries = presets();
    SplitButton::new("Image Trace", &entries).arrow_label("Tracing presets").show(ui).into()
}

#[story(Link, covers = [Default])]
fn link(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Link::new("Show all shortcuts").url("https://example.com/shortcuts").show(ui).into()
}
