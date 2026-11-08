//! The properties dialog: tabs of rows, read-only rows skipped by edits.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{DialogFields, Param, ParamKind, PropertiesDialog, PropertiesTab, PropertyRow};
use craft_ui::testing::{Fixture, trial};
use egui::Modifiers;
use std::cell::RefCell;
use std::rc::Rc;

fn tabs() -> Vec<PropertiesTab> {
    vec![
        PropertiesTab::new(
            "General",
            vec![PropertyRow::value("Kind", "Photoshop document"), PropertyRow::field(Param::new("title", "Title", ParamKind::text("Poster")))],
        ),
        PropertiesTab::new("Colour", vec![PropertyRow::value("Mode", "CMYK")]),
    ]
}

/// A read-only value row shows its text, and only the shown tab's rows are on screen.
#[trial]
fn the_dialog_shows_the_rows_of_the_first_tab() {
    let tabs = tabs();
    let values = Rc::new(RefCell::new(DialogFields::new()));
    let mut fixture = Fixture::new(move |ui| {
        let mut values = values.borrow_mut();
        PropertiesDialog::new("Properties", tabs.clone(), Bind::bound(&mut *values)).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    let texts = fixture.texts();
    assert!(texts.iter().any(|text| text.text.contains("Photoshop document")), "the value row shows its text");
    assert!(!texts.iter().any(|text| text.text.contains("CMYK")), "the other tab's rows are not shown");
}

/// Alt turns Cancel into Reset, which puts back the values the dialog opened with.
#[trial]
fn alt_reset_restores_the_opening_values() {
    let tabs = vec![PropertiesTab::new("General", vec![PropertyRow::field(Param::new("locked", "Locked", ParamKind::bool(false)))])];
    let values = Rc::new(RefCell::new(DialogFields::new()));
    let held = Rc::clone(&values);
    let mut fixture = Fixture::new(move |ui| {
        let mut values = held.borrow_mut();
        PropertiesDialog::new("Properties", tabs.clone(), Bind::bound(&mut *values)).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    let locked = fixture.by_label("Locked").expect("the row is shown");
    fixture.click(&locked);
    fixture.settle();
    assert_eq!(values.borrow().bool("locked"), Some(true), "the edit wrote through");
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(values.borrow().bool("locked"), Some(false), "Reset wrote the opening value back");
}
