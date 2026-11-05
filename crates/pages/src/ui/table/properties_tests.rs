//! The table properties panel.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::edit::EditEvent;
use craft_ui::localized::units::LengthUnit;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::properties::*;

struct App {
    values: TableValues,
    edits: Vec<EditEvent<TableChange>>,
}

fn open(values: TableValues) -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(360.0, 900.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let out = TableProperties::new("table", &app.values, LengthUnit::Points).show(ui);
                app.edits.extend(out.edits);
            },
            App { values, edits: Vec::new() },
        )
}

// Typing 5 in Rows and Enter reports Begin then Commit of Rows(5); choosing Exactly reports Set(RowHeightMode(Exactly)); Merge Cells is disabled with one cell selected.
#[test]
fn fields_report_one_change_each() {
    let mut harness = open(TableValues { rows: 3, columns: 4, row_height_mode: Some(RowHeight::AtLeast), ..TableValues::default() });
    harness.run();

    harness.get_by_role_and_label(Role::SpinButton, "Rows").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role_and_label(Role::SpinButton, "Rows").type_text("5");
    harness.key_press(Key::Enter);
    harness.run();
    let changes: Vec<_> = harness
        .state()
        .edits
        .iter()
        .map(|e| (matches!(e, EditEvent::Begin { .. }), matches!(e, EditEvent::Commit { .. }), e.value().cloned()))
        .collect();
    assert!(matches!(changes.first(), Some((true, _, _))), "{changes:?}");
    assert_eq!(changes.last(), Some(&(false, true, Some(TableChange::Rows(5)))));
    assert!(changes.iter().all(|(_, _, v)| v.is_none() || *v == Some(TableChange::Rows(5))), "{changes:?}");

    harness.state_mut().edits.clear();
    harness.get_by_role_and_label(Role::ComboBox, "Row Height").click();
    harness.run();
    harness.get_by_label("Exactly").click();
    harness.run();
    assert!(
        matches!(harness.state().edits.as_slice(), [EditEvent::Set { value: TableChange::RowHeightMode(RowHeight::Exactly), .. }]),
        "{:?}",
        harness.state().edits
    );

    harness.state_mut().edits.clear();
    harness.get_by_label("Merge Cells").click();
    harness.run();
    assert!(harness.state().edits.is_empty(), "Merge Cells is disabled with one cell");

    harness.state_mut().values.several_cells = true;
    harness.run();
    harness.get_by_label("Merge Cells").click();
    harness.run();
    assert!(matches!(harness.state().edits.as_slice(), [EditEvent::Set { value: TableChange::Merge, .. }]));
}
