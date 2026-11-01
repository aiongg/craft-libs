//! The Columns dialog and menu.

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::localized::units::LengthUnit;
use craft_ui::menu::MenuEntry;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::columns::*;

struct App {
    host: DialogHost,
    answers: Vec<Option<TextColumns>>,
}

fn open(dialog: TextColumnsDialog) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 900.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<TextColumnsDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        );
    harness.run();
    harness
}

fn start() -> TextColumns {
    TextColumns { count: Some(1), spacing: Some(36.0), equal_width: Some(true), line_between: Some(false), widths: Vec::new() }
}

fn type_into(harness: &mut Harness<'_, App>, field: &str, text: &str) {
    harness.get_by_role_and_label(Role::SpinButton, field).focus();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

fn ok(harness: &mut Harness<'_, App>) {
    harness.get_by_label("OK").click();
    harness.run();
}

// Changing Number from 1 to 2 returns count Some(2) and spacing None; OK with nothing changed returns None.
#[test]
fn only_changed_fields_come_back() {
    let mut harness = open(TextColumnsDialog::new(start()).unit(LengthUnit::Inches));
    type_into(&mut harness, "Number:", "2");
    ok(&mut harness);
    assert_eq!(harness.state().answers, vec![Some(TextColumns { count: Some(2), ..TextColumns::default() })]);

    let mut harness = open(TextColumnsDialog::new(start()).unit(LengthUnit::Inches));
    ok(&mut harness);
    assert_eq!(harness.state().answers, vec![None]);
}

// Number 17 or Spacing 23 in keep it open with the range message.
#[test]
fn out_of_range_values_keep_the_dialog_open() {
    let mut harness = open(TextColumnsDialog::new(start()).unit(LengthUnit::Inches));
    type_into(&mut harness, "Number:", "17");
    harness.get_by_label_contains("Number must be a whole number from 1 to 16");
    ok(&mut harness);
    assert!(harness.state().answers.is_empty() && harness.state().host.get::<TextColumnsDialog>().is_some());

    type_into(&mut harness, "Number:", "3");
    type_into(&mut harness, "Spacing:", "23");
    harness.get_by_label_contains("Spacing must be from");
    ok(&mut harness);
    assert!(harness.state().answers.is_empty() && harness.state().host.get::<TextColumnsDialog>().is_some());
}

// With Equal column width the width fields after the first are disabled; three columns over 6.5 in split 1.83 in each with 0.5 in spacing.
#[test]
fn word_grid_follows_equal_width() {
    let columns = TextColumns { count: Some(3), ..start() };
    let mut harness = open(TextColumnsDialog::new(columns).word(468.0).unit(LengthUnit::Inches));
    let widths: Vec<_> = harness.get_all_by_role_and_label(Role::SpinButton, "Width:").collect();
    assert_eq!(widths.len(), 3);
    assert_eq!(widths.iter().map(|w| w.accesskit_node().is_disabled()).collect::<Vec<_>>(), [false, true, true]);
    for width in &widths {
        assert!(width.value().is_some_and(|v| v.contains("1.83")), "{:?}", width.value());
    }
    let spacing = harness.get_all_by_role_and_label(Role::SpinButton, "Spacing:").next().expect("spacing field");
    assert!(spacing.value().is_some_and(|v| v.contains("0.5")));

    harness.get_by_label("Equal column width").click();
    harness.run();
    let widths: Vec<_> = harness.get_all_by_role_and_label(Role::SpinButton, "Width:").collect();
    assert!(widths.iter().all(|w| !w.accesskit_node().is_disabled()));
}

// The menu checks the current preset and ends with More Columns….
#[test]
fn the_menu_checks_the_current_preset() {
    let entries = columns_menu(Some(ColumnsPreset::Two), &TextColumnsLabels::default());
    let checked: Vec<_> =
        entries.iter().filter_map(MenuEntry::item).filter(|i| i.item_kind().checked() == Some(true)).map(|i| i.label().to_owned()).collect();
    assert_eq!(checked, ["Two"]);
    assert_eq!(entries.last().and_then(MenuEntry::item).and_then(|i| i.command()), Some(&ColumnsChoice::More));
}
