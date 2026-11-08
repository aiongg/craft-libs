//! Missing fonts: resolution and the dialog.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use craft_font_catalog::{Limits, Os};
use craft_font_manager::missing::{
    DocumentFont, FontStatus, MissingFontsDialog, MissingFontsResult, MissingFontsWords, document_fonts, missing_font_warnings,
};
use craft_font_manager::{Catalog, FontEnv, Source};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::testing::{Fixture, Room, trial};

/// Hack, the face egui ships.
fn catalog() -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    let _ = catalog.add_bytes(epaint_default_fonts::HACK_REGULAR.to_vec(), Source::Bundled, &Limits::default());
    catalog
}

fn font(family: &str, style: &str, postscript_name: Option<&str>) -> DocumentFont {
    DocumentFont::new(family, style, postscript_name)
}

/// A PostScript name that resolves counts as Installed even when the family name differs.
#[trial]
fn postscript_name_resolves_before_family_and_style() {
    let catalog = catalog();
    let fonts = [
        font("Hack Renamed", "Regular", Some("Hack-Regular")),
        font("Hack Renamed", "Regular", Some("Hack-Regular")),
        font("Hack", "Bold Oblique", None),
        font("Nonesuch", "Regular", Some("Nonesuch-Regular")),
    ];
    let rows = document_fonts(&catalog, &fonts);
    assert_eq!(rows.len(), 3, "the duplicate is merged");
    assert_eq!(rows[0].status, FontStatus::Installed);
    assert!(matches!(rows[1].status, FontStatus::OtherStyle(_)));
    assert_eq!(rows[2].status, FontStatus::Missing);
    assert_eq!(
        missing_font_warnings(&rows, &MissingFontsWords::default()),
        ["font 'Nonesuch' is not installed; text may render using a fallback face"]
    );
}

struct App {
    dialog: MissingFontsDialog,
    stack: DialogStack,
    results: Vec<MissingFontsResult>,
}

/// Replace Fonts returns one replacement per missing row, the default font where none was picked.
#[trial]
fn replace_returns_a_replacement_per_missing_row() {
    let catalog = catalog();
    let rows = document_fonts(&catalog, &[font("Nonesuch", "Bold", None), font("Hack", "Regular", None), font("Other", "Regular", None)]);
    let dialog = MissingFontsDialog::new(catalog, rows).default_family("Hack");
    let mut stack = DialogStack::default();
    stack.push(dialog.id(), None);
    let app = Rc::new(RefCell::new(App { dialog, stack, results: Vec::new() }));
    let shown = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { dialog, stack, results } = &mut *app;
        if let Some(DialogResult::Ok(result)) = dialog.show(ui, stack) {
            results.push(result);
        }
    });
    fixture.in_room(Room::Dialog);
    fixture.settle();
    assert!(fixture.by_label("Replace Fonts").is_some());
    assert!(fixture.texts().iter().any(|painted| painted.text == "2 fonts used in this document are not installed."));
    let replace = fixture.by_label("Replace Fonts").expect("Replace Fonts");
    fixture.click(&replace);
    fixture.settle();
    let app = app.borrow();
    let result = app.results.first().expect("Replace Fonts answered");
    let names: Vec<(&str, &str)> = result.replacements.iter().map(|(f, to)| (f.family.as_str(), to.as_str())).collect();
    assert_eq!(names, [("Nonesuch", "Hack"), ("Other", "Hack")]);
}
