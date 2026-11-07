//! The properties dialog: tabs of rows, read-only rows skipped by edits.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{Param, ParamKind, PropertiesDialog, PropertiesTab, PropertyRow};
use craft_ui::testing::{Fixture, trial};

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
    let mut fixture = Fixture::new(move |ui| {
        PropertiesDialog::new("Properties", tabs.clone()).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    let texts = fixture.texts();
    assert!(texts.iter().any(|text| text.text.contains("Photoshop document")), "the value row shows its text");
    assert!(!texts.iter().any(|text| text.text.contains("CMYK")), "the other tab's rows are not shown");
}
