//! The properties dialog in a harness.

use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

struct App {
    dialog: PropertiesDialog,
    result: Option<DialogResult<DialogFields>>,
}

fn tabs() -> Vec<PropertiesTab> {
    vec![
        PropertiesTab::new("General", vec![PropertyRow::Value { label: "Page size:".into(), text: "210 × 297 mm".into() }]),
        PropertiesTab::new("Fonts", vec![PropertyRow::Value { label: "Font:".into(), text: "Helvetica".into() }]),
    ]
}

fn harness(dialog: PropertiesDialog) -> Harness<'static, App> {
    Harness::<App>::builder().with_size(vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = ui.ctx().clone();
            let mut stack = DialogStack::default();
            stack.push(app.dialog.id(), None);
            if let Some(result) = app.dialog.show(&ctx, &stack) {
                app.result = Some(result);
            }
        },
        App { dialog, result: None },
    )
}

// The tab strip switches tabs with ←/→ and Enter; each tab shows its rows; the open tab is remembered.
#[test]
fn tabs_switch_by_keyboard_and_are_remembered() {
    let mut harness = harness(PropertiesDialog::new("props-tabs", "Document Properties", tabs()));
    harness.run();
    assert!(harness.query_by_label("210 × 297 mm").is_some());
    assert!(harness.query_by_label("Helvetica").is_none());

    harness.get_by_label("General").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.query_by_label("Helvetica").is_some());
    assert!(harness.query_by_label("210 × 297 mm").is_none());

    harness.state_mut().dialog = PropertiesDialog::new("props-tabs", "Document Properties", tabs());
    harness.run();
    assert!(harness.query_by_label("Helvetica").is_some(), "the reopened dialog shows the tab it was left on");
}

// Read-only mode shows every row as text and a single OK; editable mode returns the fields.
#[test]
fn read_only_shows_text_and_editable_returns_fields() {
    let mut read_only = harness(PropertiesDialog::new("props-ro", "File Info", tabs()).read_only(true));
    read_only.run();
    assert!(read_only.query_by_label("210 × 297 mm").is_some());
    assert!(read_only.query_by_label("Cancel").is_none());
    read_only.get_by_label("OK").click();
    read_only.run();
    assert_eq!(read_only.state().result, Some(DialogResult::Ok(DialogFields::new())));

    let values = DialogFields::from_pairs([("title", "Report")]);
    let mut editable = harness(PropertiesDialog::new("props-edit", "Document Properties", tabs()).values(values.clone()));
    editable.run();
    assert!(editable.query_by_label("Cancel").is_some());
    editable.get_by_label("OK").click();
    editable.run();
    assert_eq!(editable.state().result, Some(DialogResult::Ok(values)));
}
