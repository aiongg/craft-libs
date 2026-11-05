//! The Columns dialog and menu.

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::LocaleNumbers;
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
    TextColumns { count: Some(1), spacing: Some(36.0), equal_width: Some(true), line_between: Some(false), widths: Vec::new(), apply: None }
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
    let widths: Vec<_> = harness.get_all_by_role_and_label(Role::SpinButton, "Width").collect();
    assert_eq!(widths.len(), 3);
    assert_eq!(widths.iter().map(|w| w.accesskit_node().is_disabled()).collect::<Vec<_>>(), [false, true, true]);
    for width in &widths {
        assert!(width.value().is_some_and(|v| v.contains("1.83")), "{:?}", width.value());
    }
    let spacing = harness.get_all_by_role_and_label(Role::SpinButton, "Spacing").next().expect("spacing field");
    assert!(spacing.value().is_some_and(|v| v.contains("0.5")));

    harness.get_by_label("Equal column width").click();
    harness.run();
    let widths: Vec<_> = harness.get_all_by_role_and_label(Role::SpinButton, "Width").collect();
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

fn widths(harness: &Harness<'_, App>, name: &str) -> Vec<Option<String>> {
    harness.get_all_by_role_and_label(Role::SpinButton, name).map(|n| n.value()).collect()
}

// The Left preset over a 6.5 in text width and 0.5 in spacing makes a 2 in and a 4 in column (a third of the width less the spacing, then the rest); it sets the spacing to 0.5 in even when the columns had another.
#[test]
fn the_left_preset_splits_the_width_less_the_spacing() {
    let columns = TextColumns { spacing: Some(18.0), ..start() };
    let mut harness = open(TextColumnsDialog::new(columns).word(468.0).unit(LengthUnit::Inches));
    harness.get_by_label("Left").click();
    harness.run();
    let shown = widths(&harness, "Width");
    assert_eq!(shown.len(), 2);
    assert!(shown[0].as_deref().is_some_and(|v| v.starts_with('2') && !v.starts_with("2.")), "{shown:?}");
    assert!(shown[1].as_deref().is_some_and(|v| v.starts_with('4')), "{shown:?}");
    assert!(widths(&harness, "Spacing")[0].as_deref().is_some_and(|v| v.contains("0.5")));
}

// Each grid row is named by its column ("Col 2:") through the form grid; the row's first field keeps the name Width.
#[test]
fn grid_rows_are_labelled_by_column() {
    let columns = TextColumns { count: Some(3), ..start() };
    let harness = open(TextColumnsDialog::new(columns).word(468.0).unit(LengthUnit::Inches));
    harness.get_by_label_contains("Col 2");
    assert_eq!(widths(&harness, "Width").len(), 3);
}

// A Cancel after a preview puts the columns back: Begin and Update to 3 columns then Cancel keeps 1; a Commit keeps 3.
#[test]
fn cancelling_a_number_edit_restores_the_start() {
    let mut dialog = TextColumnsDialog::new(start());
    let id = GestureId::next();
    dialog.edit_count(Some(EditEvent::begin(id)));
    dialog.edit_count(Some(EditEvent::update(id, Some(3.0), egui::Modifiers::NONE)));
    assert_eq!(dialog.edited().count, Some(3));
    dialog.edit_count(Some(EditEvent::cancel(id, CancelReason::Escape)));
    assert_eq!(dialog.edited().count, Some(1));

    let id = GestureId::next();
    dialog.edit_count(Some(EditEvent::begin(id)));
    dialog.edit_count(Some(EditEvent::commit(id, Some(3.0), true, egui::Modifiers::NONE)));
    assert_eq!(dialog.edited().count, Some(3));
}

// The range message writes the bounds in the locale's digits: under ar-EG the maximum is not "16".
#[test]
fn the_number_range_message_uses_the_locales_digits() {
    let labels = TextColumnsLabels::default();
    let english = TextColumnsDialog::number_message(&labels, &LocaleNumbers::from_tag("en"), MAX_COLUMNS);
    assert!(english.contains("from 1 to 16"), "{english}");
    let arabic = TextColumnsDialog::number_message(&labels, &LocaleNumbers::from_tag("ar-EG"), MAX_COLUMNS);
    assert!(!arabic.contains("16") && !arabic.contains("to 1 "), "{arabic}");
}

fn nth_field(harness: &Harness<'_, App>, name: &str, n: usize) -> String {
    widths(harness, name).swap_remove(n).unwrap_or_default()
}

fn type_into_nth(harness: &mut Harness<'_, App>, name: &str, n: usize, text: &str) {
    harness.get_all_by_role_and_label(Role::SpinButton, name).nth(n).expect("field").focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

// Word's dialog takes 12 columns, PowerPoint's 16; `max_columns` replaces either.
#[test]
fn the_most_columns_follows_the_family_and_the_builder() {
    let mut harness = open(TextColumnsDialog::new(start()).word(468.0));
    type_into(&mut harness, "Number:", "13");
    harness.get_by_label_contains("from 1 to 12");
    let mut harness = open(TextColumnsDialog::new(start()).word(468.0).max_columns(16));
    type_into(&mut harness, "Number:", "13");
    assert!(harness.query_by_label_contains("Number must be").is_none());
    let mut harness = open(TextColumnsDialog::new(start()).max_columns(4));
    type_into(&mut harness, "Number:", "5");
    harness.get_by_label_contains("from 1 to 4");
}

// A width typed below 18 pt becomes 18 pt (0.25 in), and a width that would take the neighbour below 18 pt leaves it at 18 pt (WordCraft `MIN_COL_PT`).
#[test]
fn a_column_is_never_narrower_than_the_minimum() {
    let mut harness = open(TextColumnsDialog::new(start()).word(468.0).unit(LengthUnit::Inches));
    harness.get_by_label("Left").click();
    harness.run();
    type_into_nth(&mut harness, "Width", 0, "0.1");
    assert!(nth_field(&harness, "Width", 0).starts_with("0.25"), "{:?}", widths(&harness, "Width"));
    type_into_nth(&mut harness, "Width", 0, "6.4");
    assert!(nth_field(&harness, "Width", 1).starts_with("0.25"), "{:?}", widths(&harness, "Width"));
}

// The preview puts each column at its share of the text width and the line between in the middle of the spacing, only when the line is on.
#[test]
fn the_preview_places_columns_and_the_line_between() {
    let page = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(100.0, 130.0));
    let rows = [(100.0, 36.0), (200.0, 0.0)];
    let plan = preview_plan(&rows, 336.0, true, page);
    let scale = plan.text.width() / 336.0;
    let left = plan.text.left();
    assert_eq!(plan.columns.len(), 2);
    assert!((plan.columns[0].1 - (left + 100.0 * scale)).abs() < 1e-3 && (plan.columns[1].0 - (left + 136.0 * scale)).abs() < 1e-3);
    assert!((plan.separators[0] - (left + 118.0 * scale)).abs() < 1e-3);
    assert!((plan.columns[1].1 - plan.text.right()).abs() < 1e-3);
    assert!(preview_plan(&rows, 336.0, false, page).separators.is_empty());
}

// Apply to answers with the choice, changed or not, and lists the parts offered.
#[test]
fn apply_to_always_answers_with_the_choice() {
    let choices = [ColumnsApply::Section, ColumnsApply::Document, ColumnsApply::Forward];
    let mut harness = open(TextColumnsDialog::new(start()).word(468.0).apply_to(&choices, ColumnsApply::Section));
    ok(&mut harness);
    assert_eq!(harness.state().answers, vec![Some(TextColumns { apply: Some(ColumnsApply::Section), ..TextColumns::default() })]);

    let mut harness = open(TextColumnsDialog::new(start()).word(468.0).apply_to(&choices, ColumnsApply::Section));
    choose(&mut harness, "Apply to:", "Whole document");
    ok(&mut harness);
    assert_eq!(harness.state().answers, vec![Some(TextColumns { apply: Some(ColumnsApply::Document), ..TextColumns::default() })]);

    let harness = open(TextColumnsDialog::new(start()).word(468.0));
    assert!(harness.query_by_role_and_label(Role::ComboBox, "Apply to:").is_none());
}

fn choose(harness: &mut Harness<'_, App>, list: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, list).click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}
