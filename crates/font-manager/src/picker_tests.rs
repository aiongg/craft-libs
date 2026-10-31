//! The font picker in a harness over an in-memory catalog.

use std::sync::Arc;

use craft_font_catalog::{Limits, Os};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, ImeEvent, Key, Modifiers, MouseWheelUnit, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use craft_host::Section as _;

use crate::picker::*;
use crate::{Catalog, FontEnv, Source};

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

fn harness(family: &str, settings: FontPickerSettings, filters: FontFilters) -> Harness<'static, State> {
    let state = State { catalog: catalog(), family: family.to_owned(), settings, filters, picked: Vec::new(), preview: None };
    Harness::<State>::builder().with_size([760.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, s: &mut State| {
            let catalog = Arc::clone(&s.catalog);
            let r = FontPicker::new("t", &catalog, &mut s.family).settings(&mut s.settings).filters(&mut s.filters).width(220.0).show(ui);
            s.picked.extend(r.picked);
            s.preview = r.preview;
        },
        state,
    )
}

fn options(harness: &Harness<'_, State>) -> Vec<String> {
    harness
        .query_all_by_role(Role::ListBoxOption)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| ["Hack", "Ubuntu"].contains(&l.as_str()))
        .collect()
}

fn open(harness: &mut Harness<'_, State>) {
    harness.get_by_role(Role::TextInput).click();
    harness.run();
}

fn field_text(harness: &Harness<'_, State>) -> String {
    harness.get_by_role(Role::TextInput).accesskit_node().value().unwrap_or_default()
}

// Typing "ubu" shows Ubuntu and hides Hack; Enter picks Ubuntu.
#[test]
fn typing_filters_and_enter_picks_the_match() {
    let mut h = harness("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut h);
    assert_eq!(options(&h).len(), 2, "both families before typing");
    h.event(Event::Text("ubu".into()));
    h.run();
    assert_eq!(options(&h), ["Ubuntu"]);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().picked, ["Ubuntu"]);
    assert_eq!(h.state().family, "Ubuntu");
    assert!(options(&h).is_empty(), "the list closes");
}

// ↓ moves the keyboard row and previews it; Esc closes and restores the field text.
#[test]
fn arrows_preview_and_escape_restores() {
    let mut h = harness("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut h);
    assert_eq!(h.state().preview, None);
    h.get_by_role_and_label(Role::ListBoxOption, "Ubuntu").hover();
    h.run();
    assert_eq!(h.state().preview.as_deref(), Some("Ubuntu"), "the row under the pointer previews");
    h.get_by_role(Role::TextInput).hover();
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().preview.as_deref(), Some("Ubuntu"));
    assert_eq!(field_text(&h), "Ubuntu");
    assert_eq!(h.state().family, "Hack", "stepping applies nothing");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(field_text(&h), "Hack");
    assert!(h.state().picked.is_empty());
    assert!(options(&h).is_empty());
}

// Enter during an IME composition belongs to the composition; a later Enter picks.
#[test]
fn enter_while_composing_does_not_pick() {
    let mut h = harness("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut h);
    h.event(Event::Ime(ImeEvent::Preedit { text: "ubu".into(), active_range_chars: None }));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().picked.is_empty(), "Enter confirmed the composition");
    h.event(Event::Ime(ImeEvent::Commit("ubu".into())));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().picked, ["Ubuntu"]);
}

// Starring a row moves it into the favourites filter; recents show first, newest first, within the limit.
#[test]
fn stars_and_recents_are_remembered_in_the_settings_section() {
    assert_eq!(FontPickerSettings::KEY, "font_picker");
    let mut settings = FontPickerSettings { recent_limit: 2, ..FontPickerSettings::default() };
    for family in ["Hack", "Ubuntu", "Hack", "Other"] {
        settings.note_recent(family);
    }
    assert_eq!(settings.recent, ["Other", "Hack"], "newest first, no duplicate, within the limit");

    let catalog = catalog();
    let mut family = String::from("Hack");
    settings.recent = vec!["Ubuntu".into(), "Hack".into()];
    let rows = FontPicker::new("t", &catalog, &mut family).settings(&mut settings).rows("");
    let order: Vec<(&str, bool)> = rows.iter().map(|r| (r.family.as_str(), r.recent)).collect();
    assert_eq!(order, [("Ubuntu", true), ("Hack", true), ("Hack", false), ("Ubuntu", false)]);

    // The star in the list stars the row; the favourites filter then keeps it alone.
    let mut h = harness("Hack", FontPickerSettings::default(), FontFilters::default());
    open(&mut h);
    h.get_by_label("Favorite Ubuntu").click();
    h.run();
    assert_eq!(h.state().settings.favorites, ["Ubuntu"]);
    h.state_mut().filters.favorites = true;
    h.run();
    assert_eq!(options(&h), ["Ubuntu"]);
    h.state_mut().settings.set_favorite("Ubuntu", false);
    assert!(h.state().settings.favorites.is_empty());
}

// The class filter keeps only families of that class; Clear All shows every family again.
#[test]
fn class_filter_and_clear_all() {
    let catalog = catalog();
    assert_eq!(FontClass::of(&catalog, "Hack"), Some(FontClass::Monospace));
    let mut family = String::from("Ubuntu");
    let mut filters = FontFilters { class: Some(FontClass::Monospace), ..FontFilters::default() };
    let rows = FontPicker::new("t", &catalog, &mut family).filters(&mut filters).rows("");
    assert_eq!(rows.iter().map(|r| r.family.as_str()).collect::<Vec<_>>(), ["Hack"]);

    let mut h = harness("Ubuntu", FontPickerSettings::default(), FontFilters { class: Some(FontClass::Monospace), ..FontFilters::default() });
    open(&mut h);
    assert_eq!(options(&h), ["Hack"]);
    h.get_by_label("Clear All").click();
    h.run();
    assert!(!h.state().filters.any());
    assert_eq!(options(&h).len(), 2);
}

// The wheel on the closed field picks the next family in list order.
#[test]
fn wheel_on_the_closed_field_steps_families() {
    let mut h = harness("Hack", FontPickerSettings::default(), FontFilters::default());
    h.run();
    h.get_by_role(Role::TextInput).hover();
    h.run();
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, -1.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });
    h.run();
    assert_eq!(h.state().family, "Ubuntu");
    assert_eq!(h.state().picked, ["Ubuntu"]);
    assert!(options(&h).is_empty(), "the list stays closed");
}
