//! New Document in a harness.

use std::sync::Arc;

use egui::accesskit::Role;
use egui::{Event, Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};
use craft_theme::Srgba;

struct App {
    host: DialogHost,
    results: Vec<Option<DocumentSettings>>,
}

/// The dialog in a harness whose Host keeps its settings in `store`, so a second harness on the
/// same store sees what the first remembered.
fn harness(dialog: NewDocumentDialog, store: Arc<craft_host::MemorySections>) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default()
        .with_size(vec2(1000.0, 800.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).settings(store.clone()).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(result) = app.host.take::<NewDocumentDialog>() {
                    app.results.push(result.ok());
                }
            },
            App { host, results: Vec::new() },
        )
}

fn open(dialog: NewDocumentDialog) -> Harness<'static, App> {
    let mut harness = harness(dialog, Arc::default());
    harness.run();
    harness
}

fn dialog<'a>(harness: &'a Harness<'_, App>) -> Option<&'a NewDocumentDialog> {
    harness.state().host.get::<NewDocumentDialog>()
}

fn settings(harness: &Harness<'_, App>) -> DocumentSettings {
    dialog(harness).map(|d| d.settings().clone()).unwrap_or_default()
}

fn px(harness: &Harness<'_, App>) -> (u32, u32) {
    let size = settings(harness).size;
    (size.width, size.height)
}

// The dialog opens with the name field focused and its text selected; Tab goes Name → Width →
// Height → Resolution.
#[test]
fn opens_with_name_selected_and_tabs_through_the_size_fields() {
    let mut harness = open(NewDocumentDialog::new());
    assert!(harness.get_by_role_and_label(Role::TextInput, "Name").accesskit_node().is_focused());
    harness.event(Event::Text("Poster".into()));
    harness.run();
    assert_eq!(settings(&harness).name, "Poster", "typing replaces the selected default name");

    for field in ["Width", "Height", "Resolution"] {
        harness.key_press(Key::Tab);
        harness.run();
        assert!(harness.get_by_role_and_label(Role::SpinButton, field).accesskit_node().is_focused(), "Tab reaches {field}");
    }
}

// A click on a card fills the details; the orientation buttons swap width and height and turn
// the built-in cards.
#[test]
fn card_click_fills_details_and_orientation_swaps_sides() {
    let mut harness = open(NewDocumentDialog::new());
    harness.get_by_role_and_label(Role::Tab, "Print").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Letter").click();
    harness.run();
    let letter = settings(&harness);
    assert_eq!((letter.size.width, letter.size.height, letter.size.pixels_per_inch), (2550, 3300, 300.0));
    assert_eq!(letter.unit, SizeUnit::Inches, "a print card applies inches");

    harness.get_by_label("Landscape").click();
    harness.run();
    assert_eq!(px(&harness), (3300, 2550));
    harness.get_by_role_and_label(Role::Tab, "Photo").click();
    harness.run();
    assert_eq!(
        harness.get_by_role_and_label(Role::Button, "Portrait, 4 x 6").accesskit_node().description().as_deref(),
        Some("6 × 4 in @ 300 ppi"),
        "the portrait card turns with the picked orientation"
    );

    harness.get_by_role_and_label(Role::Tab, "Film & Video").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "UHD 4K").click();
    harness.run();
    let uhd = settings(&harness);
    assert_eq!((uhd.size.width, uhd.size.pixels_per_inch, uhd.unit), (3840, 72.0, SizeUnit::Pixels));
}

// Create returns the settings and puts them first in Recent on the next open; equal settings are
// not repeated and the list keeps 20.
#[test]
fn create_returns_settings_and_updates_recent() {
    let store: Arc<craft_host::MemorySections> = Arc::default();
    let mut first = harness(NewDocumentDialog::new(), store.clone());
    first.run();
    first.get_by_role_and_label(Role::Tab, "Web").click();
    first.run();
    first.get_by_role_and_label(Role::Button, "Web Large").click();
    first.run();
    first.get_by_label("Create").click();
    first.run();
    let created = first.state().results[0].clone().expect("Create answers Ok");
    assert_eq!((created.size.width, created.size.height, created.name.as_str()), (1920, 1080, "Untitled-1"));

    let mut second = harness(NewDocumentDialog::new(), store);
    second.run();
    assert_eq!(px(&second), (1920, 1080), "the dialog opens with the last values");
    let titles = second.get_all_by_role(Role::Button).filter_map(|n| n.accesskit_node().label()).collect::<Vec<_>>();
    let recent = titles.iter().position(|t| t == "Web Large");
    let defaults = titles.iter().position(|t| t == "Default Test Size");
    assert!(recent.is_some() && recent < defaults, "the created document leads Recent: {titles:?}");

    let mut list = Vec::new();
    for width in 1..=25 {
        let size = PixelSize { width, height: 10, pixels_per_inch: 72.0 };
        push_recent(&mut list, DocumentSettings { size, ..DocumentSettings::default() }, 20);
    }
    assert_eq!((list.len(), list[0].size.width), (20, 25));
    let again =
        DocumentSettings { size: PixelSize { width: 10, height: 10, pixels_per_inch: 72.0 }, name: "Other".into(), ..DocumentSettings::default() };
    push_recent(&mut list, again, 20);
    assert_eq!((list.len(), list[0].size.width), (20, 10), "an equal document moves to the front");
}

// Save Preset… then Enter saves the preset under Saved without creating the document; Delete
// removes it.
#[test]
fn enter_in_the_preset_name_saves_without_creating() {
    let mut harness = open(NewDocumentDialog::new());
    harness.get_by_label("Save Preset…").click();
    harness.run();
    harness.event(Event::Text("My size".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().results.is_empty(), "Enter in the name row does not create");
    assert!(harness.query_by_label("My size").is_some(), "the preset is listed under Saved");

    harness.get_by_label("Delete").click();
    harness.run();
    assert!(harness.query_by_label("My size").is_none());
    assert!(harness.query_by_label("No saved presets yet.").is_some());
}

// Custom… asks for a colour once; set_color makes the background Custom, cancel_color restores
// the previous choice.
#[test]
fn custom_background_round_trips_through_the_app() {
    let mut harness = open(NewDocumentDialog::new());
    let choose = |harness: &mut Harness<'_, App>, option: &str| {
        harness.get_by_role_and_label(Role::ComboBox, "Background Contents").click();
        harness.run();
        harness.get_by_label(option).click();
        harness.run();
    };
    let take = |harness: &mut Harness<'_, App>| harness.state_mut().host.get_mut::<NewDocumentDialog>().expect("open").pick_color();
    choose(&mut harness, "Black");
    assert_eq!(settings(&harness).background, BackgroundContents::Black);

    choose(&mut harness, "Custom…");
    let request = take(&mut harness).expect("Custom… asks for a colour");
    assert_eq!(request.key, BACKGROUND_COLOR_KEY);
    assert_eq!(request.color, Srgba::BLACK, "the picker opens at the colour that was shown");
    assert!(take(&mut harness).is_none(), "the request is taken once");
    harness.state_mut().host.get_mut::<NewDocumentDialog>().expect("open").cancel_color();
    harness.run();
    assert_eq!(settings(&harness).background, BackgroundContents::Black, "a cancelled pick restores the choice");

    choose(&mut harness, "Custom…");
    assert!(take(&mut harness).is_some());
    harness.state_mut().host.get_mut::<NewDocumentDialog>().expect("open").set_color("other", Srgba::WHITE);
    assert_ne!(settings(&harness).background, BackgroundContents::Custom([1.0; 3]), "an answer for another key is ignored");
    harness.state_mut().host.get_mut::<NewDocumentDialog>().expect("open").set_color(BACKGROUND_COLOR_KEY, Srgba::rgb(1.0, 0.5, 0.0));
    harness.run();
    assert_eq!(settings(&harness).background, BackgroundContents::Custom([1.0, 0.5, 0.0]));
}

// The built-in default card carries the application's name, and an app can replace the cards.
#[test]
fn default_card_is_named_after_the_app_and_the_cards_can_be_replaced() {
    let harness = open(NewDocumentDialog::new());
    assert!(harness.query_by_label_contains("Default Test Size").is_some(), "the harness app is named Test");

    let replaced = open(NewDocumentDialog::new().recent_defaults(vec![DocumentPreset::new("mine", "Mine", 640, 480, 72.0)]));
    assert!(replaced.query_by_label_contains("Mine").is_some());
    assert!(replaced.query_by_label_contains("HDTV 1080p").is_none());
}

// Enter on a focused card applies it and creates, as PhotoCraft's Enter (the dialog confirms
// whatever has the focus); the tabs are one Tab stop moved with the arrows.
#[test]
fn enter_on_a_card_creates_and_the_tabs_are_one_stop() {
    let mut harness = open(NewDocumentDialog::new());
    harness.get_by_role_and_label(Role::Tab, "Web").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.get_by_role_and_label(Role::Tab, "Mobile").accesskit_node().is_focused(), "an arrow moves the tab focus");

    harness.get_by_role_and_label(Role::Button, "Android 1080p").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let created = harness.state().results.first().cloned().flatten();
    assert_eq!(created.map(|s| (s.size.width, s.size.height)), Some((1080, 1920)), "Enter applies the card and creates once");
    assert_eq!(harness.state().results.len(), 1);
}
