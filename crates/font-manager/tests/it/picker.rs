//! `FontPicker` through the fixture over an in-memory catalog of the two faces egui ships.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use craft_font_catalog::{Limits, Os};
use craft_font_manager::picker::{FontClass, FontFilters, FontPicker, FontPickerSettings, family_rows};
use craft_font_manager::{Catalog, FontEnv, Source};
use craft_host::Section as _;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers};

/// Hack and Ubuntu Light, the two faces egui ships.
fn catalog() -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    for bytes in [epaint_default_fonts::UBUNTU_LIGHT, epaint_default_fonts::HACK_REGULAR] {
        let _ = catalog.add_bytes(bytes.to_vec(), Source::Bundled, &Limits::default());
    }
    catalog
}

struct State {
    catalog: Arc<Catalog>,
    family: String,
    settings: FontPickerSettings,
    filters: FontFilters,
    picked: Vec<String>,
    preview: Option<String>,
}

/// A picker in a fixture with the focus on its field.
fn rig(family: &str, settings: FontPickerSettings, filters: FontFilters) -> (Fixture, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State { catalog: catalog(), family: family.to_owned(), settings, filters, picked: Vec::new(), preview: None }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut state = shown.borrow_mut();
        let State { catalog, family, settings, filters, picked, preview } = &mut *state;
        let out = FontPicker::new("Font family", Bind::bound(family), catalog).settings(Bind::bound(settings)).filters(Bind::bound(filters)).show(ui);
        picked.extend(out.picked);
        *preview = out.preview;
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    (fixture, state)
}

fn options(fixture: &Fixture) -> Vec<String> {
    fixture.by_role(Role::LIST_BOX_OPTION).iter().filter_map(|row| fixture.tree().name_of(row.node).map(str::to_owned)).collect()
}

fn open(fixture: &mut Fixture) {
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(3);
}

/// Typing "ubu" shows Ubuntu and hides Hack; Enter picks Ubuntu and closes the list.
#[trial]
fn typing_filters_and_enter_picks_the_match() {
    let (mut fixture, state) = rig("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut fixture);
    assert_eq!(options(&fixture).len(), 2, "both families before typing");
    fixture.type_text("ubu");
    fixture.idle(3);
    assert_eq!(options(&fixture), ["Ubuntu"]);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(10);
    assert_eq!(state.borrow().picked, ["Ubuntu"]);
    assert_eq!(state.borrow().family, "Ubuntu");
    assert!(options(&fixture).is_empty(), "the list closes");
}

/// Down moves the keyboard row and previews it without applying it; Escape closes the list and restores the field.
#[trial]
fn arrows_preview_and_escape_restores() {
    let (mut fixture, state) = rig("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut fixture);
    assert_eq!(state.borrow().preview.as_deref().unwrap_or("Hack"), "Hack");
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(state.borrow().preview.as_deref(), Some("Ubuntu"), "the keyboard row previews");
    assert_eq!(state.borrow().family, "Hack", "stepping applies nothing yet");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(10);
    assert_eq!(state.borrow().family, "Hack");
    assert!(state.borrow().picked.is_empty());
    assert!(options(&fixture).is_empty());
}

/// Recents are kept newest first within the limit, show first in the list, and a star in the list is remembered.
#[trial]
fn stars_and_recents_are_remembered_in_the_settings_section() {
    assert_eq!(FontPickerSettings::KEY, "font_picker");
    let mut settings = FontPickerSettings { recent_limit: 2, ..FontPickerSettings::default() };
    for family in ["Hack", "Ubuntu", "Hack", "Other"] {
        settings.note_recent(family);
    }
    assert_eq!(settings.recent, ["Other", "Hack"], "newest first, no duplicate, within the limit");

    let catalog = catalog();
    let listed = FontPickerSettings { recent: vec!["Ubuntu".into(), "Hack".into()], ..FontPickerSettings::default() };
    let rows = family_rows(&catalog, "", Some(&listed), &FontFilters::default(), "Hack", "en");
    let order: Vec<(&str, bool)> = rows.iter().map(|r| (r.family.as_str(), r.recent)).collect();
    assert_eq!(order, [("Ubuntu", true), ("Hack", true), ("Hack", false), ("Ubuntu", false)]);

    let (mut fixture, state) = rig("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut fixture);
    let star = fixture.by_label("Favorite Ubuntu").expect("a star on the Ubuntu row");
    fixture.click(&star);
    fixture.idle(3);
    assert_eq!(state.borrow().settings.favorites, ["Ubuntu"]);
}

/// The class filter keeps only families of that class; Clear All shows every family again.
#[trial]
fn class_filter_and_clear_all() {
    let catalog = catalog();
    assert_eq!(FontClass::of(&catalog, "Hack"), Some(FontClass::Monospace));
    let filters = FontFilters { class: Some(FontClass::Monospace), ..FontFilters::default() };
    let rows = family_rows(&catalog, "", None, &filters, "Ubuntu", "en");
    assert_eq!(rows.iter().map(|r| r.family.as_str()).collect::<Vec<_>>(), ["Hack"]);

    let (mut fixture, state) = rig("Ubuntu", FontPickerSettings::default(), filters);
    open(&mut fixture);
    assert_eq!(options(&fixture), ["Hack"]);
    let clear = fixture.by_label("Clear All").expect("Clear All");
    fixture.click(&clear);
    fixture.idle(3);
    assert!(!state.borrow().filters.any());
    assert_eq!(options(&fixture).len(), 2);
}

/// The wheel on the closed, focused field picks the next family in list order and leaves the list closed.
#[trial]
fn wheel_on_the_closed_field_steps_families() {
    let (mut fixture, state) = rig("Hack", FontPickerSettings::default(), FontFilters::default());
    fixture.frame(vec![pointer::wheel_lines(-1.0)]);
    fixture.idle(3);
    assert_eq!(state.borrow().family, "Ubuntu");
    assert_eq!(state.borrow().picked, ["Ubuntu"]);
    assert!(options(&fixture).is_empty(), "the list stays closed");
}
