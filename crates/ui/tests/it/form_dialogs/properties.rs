//! The properties dialog: tabs of rows, read-only rows skipped by edits.

use craft_ui::contract::Bind;
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::patterns::form_dialogs::{DialogFields, HostedPropertiesDialog, Param, ParamKind, PropertiesDialog, PropertiesTab, PropertyRow};
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

/// The top of the painted run `text`.
fn top_of(fixture: &Fixture, text: &str) -> f32 {
    fixture.texts().iter().find(|run| run.text == text).unwrap_or_else(|| panic!("{text} is painted")).rect.top()
}

/// A space row puts a gap between the rows around it: they stand further apart than two plain rows do.
#[trial]
fn a_space_row_separates_the_rows_around_it() {
    let tabs = vec![PropertiesTab::new(
        "General",
        vec![PropertyRow::value("A", "alpha"), PropertyRow::value("B", "bravo"), PropertyRow::space(), PropertyRow::value("C", "charlie")],
    )];
    let values = Rc::new(RefCell::new(DialogFields::new()));
    let mut fixture = Fixture::new(move |ui| {
        let mut values = values.borrow_mut();
        PropertiesDialog::new("Properties", tabs.clone(), Bind::bound(&mut *values)).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    let (alpha, bravo, charlie) = (top_of(&fixture, "alpha"), top_of(&fixture, "bravo"), top_of(&fixture, "charlie"));
    assert!(charlie - bravo > bravo - alpha, "the space row widens the step from B to C: {alpha} {bravo} {charlie}");
}

/// A hosted properties dialog owns its values: an edit and OK hand the edited values to the host.
#[trial]
fn a_hosted_properties_dialog_returns_the_edited_values_on_ok() {
    let host = Rc::new(RefCell::new(DialogHost::new()));
    let tabs = vec![PropertiesTab::new("General", vec![PropertyRow::field(Param::new("locked", "Locked", ParamKind::bool(false)))])];
    host.borrow_mut().open(HostedPropertiesDialog::new("file.properties", DialogFields::new(), move |values| {
        PropertiesDialog::new("Properties", tabs.clone(), values)
    }));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.settle();
    let locked = fixture.by_label("Locked").expect("the row is shown");
    fixture.click(&locked);
    fixture.settle();
    assert!(host.borrow_mut().take::<HostedPropertiesDialog>().is_none(), "the dialog is still open");
    let ok = fixture.by_label("OK").expect("the OK button is shown");
    fixture.click(&ok);
    fixture.settle();
    let Some(DialogResult::Ok(fields)) = host.borrow_mut().take::<HostedPropertiesDialog>() else { panic!("OK answers with the values") };
    assert_eq!(fields.bool("locked"), Some(true));
}
