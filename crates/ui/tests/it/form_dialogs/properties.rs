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

/// The top of the painted run `text`.
fn top_of(fixture: &Fixture, text: &str) -> f32 {
    fixture.texts().iter().find(|run| run.text == text).unwrap_or_else(|| panic!("{text} is painted")).rect.top()
}

