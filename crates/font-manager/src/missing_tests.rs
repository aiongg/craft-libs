//! Missing fonts: resolution and the dialog.

use std::sync::Arc;

use craft_font_catalog::{Limits, Os};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::theme::{Base, install};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::missing::*;
use crate::{Catalog, FontEnv, Source};

/// Hack, the face egui ships.
fn catalog() -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    let _ = catalog.add_bytes(epaint_default_fonts::HACK_REGULAR.to_vec(), Source::Bundled, &Limits::default());
    catalog
}

fn font(family: &str, style: &str, postscript_name: Option<&str>) -> DocumentFont {
    DocumentFont { family: family.to_owned(), style: style.to_owned(), postscript_name: postscript_name.map(str::to_owned) }
}

// A PostScript name that resolves counts as Installed even when the family name differs.
#[test]
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
        missing_font_warnings(&rows, &MissingFontsLabels::default()),
        ["font 'Nonesuch' is not installed; text may render using a fallback face"]
    );
}

struct App {
    host: DialogHost,
    results: Vec<MissingFontsResult>,
}

// Replace Fonts returns one replacement per missing row, the default font where none was picked.
#[test]
fn replace_returns_a_replacement_per_missing_row() {
    let catalog = catalog();
    let rows = document_fonts(&catalog, &[font("Nonesuch", "Bold", None), font("Hack", "Regular", None), font("Other", "Regular", None)]);
    let mut host = DialogHost::default();
    host.open(MissingFontsDialog::new(catalog, rows).default_family("Hack"));
    let mut harness =
        Harness::<App>::builder().with_size([760.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(result)) = app.host.take::<MissingFontsDialog>() {
                    app.results.push(result);
                }
            },
            App { host, results: Vec::new() },
        );
    harness.run();
    harness.get_by_label_contains("2 fonts used in this document are not installed.");
    harness.get_by_label("Replace Fonts").click();
    harness.run();
    let result = harness.state().results.first().expect("Replace Fonts answered");
    let names: Vec<(&str, &str)> = result.replacements.iter().map(|(f, to)| (f.family.as_str(), to.as_str())).collect();
    assert_eq!(names, [("Nonesuch", "Hack"), ("Other", "Hack")]);
}
