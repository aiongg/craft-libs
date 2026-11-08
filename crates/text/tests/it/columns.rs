//! The Columns dialog: what comes back, the arithmetic of the grid and presets, the menu and the preview.

use craft_text::ui::DocUnit;
use craft_text::ui::columns::{
    ColumnsApply, ColumnsChoice, ColumnsPreset, MAX_COLUMNS, MAX_COLUMNS_WORD, MIN_COLUMN_WIDTH, TextColumns, TextColumnsDialog, TextColumnsWords,
    columns_menu, preview_plan,
};
use craft_ui::runtime::env::{Env, UnitKind};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, pos2};
use std::cell::RefCell;
use std::rc::Rc;

/// A text width of 6.5 in, points.
const WIDTH: f32 = 468.0;

fn two() -> TextColumns {
    TextColumns { count: Some(2), spacing: Some(36.0), equal_width: Some(true), line_between: Some(false), ..TextColumns::default() }
}

#[trial]
fn only_changed_fields_come_back() {
    let mut dialog = TextColumnsDialog::new(two());
    assert_eq!(dialog.changes(), None);
    dialog.set_count(3);
    assert_eq!(dialog.changes(), Some(TextColumns { count: Some(3), ..TextColumns::default() }));
}

#[trial]
fn out_of_range_values_keep_the_dialog_open() {
    let mut dialog = TextColumnsDialog::new(two());
    assert!(dialog.is_valid());
    dialog.set_count(MAX_COLUMNS + 1);
    assert!(!dialog.is_valid());
    dialog.set_count(0);
    assert!(!dialog.is_valid());
}

#[trial]
fn word_grid_follows_equal_width() {
    let mut dialog = TextColumnsDialog::new(two()).word(WIDTH);
    let rows = dialog.rows();
    assert_eq!(rows.len(), 2);
    assert!((rows[0].0 - (WIDTH - 36.0) / 2.0).abs() < 1e-3);
    dialog.set_count(3);
    let widths: Vec<f32> = dialog.rows().iter().map(|row| row.0).collect();
    assert!(widths.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 1e-3), "equal widths: {widths:?}");
}

#[trial]
fn the_menu_checks_the_current_preset() {
    let entries = Rc::new(RefCell::new(Vec::new()));
    let seen = entries.clone();
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<TextColumnsWords>();
        *seen.borrow_mut() = columns_menu(Some(ColumnsPreset::Two), &words);
    });
    fixture.settle();
    let entries = entries.borrow();
    let checked: Vec<ColumnsChoice> = entries.iter().filter(|entry| entry.is_checked()).filter_map(|entry| entry.value().copied()).collect();
    assert_eq!(checked, vec![ColumnsChoice::Preset(ColumnsPreset::Two)]);
    assert_eq!(entries.last().and_then(|entry| entry.value().copied()), Some(ColumnsChoice::More));
}

#[trial]
fn the_left_preset_splits_the_width_less_the_spacing() {
    let mut dialog = TextColumnsDialog::new(two()).word(WIDTH);
    dialog.apply_preset(ColumnsPreset::Left);
    let rows = dialog.edited().widths.clone();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].0 < rows[1].0);
    assert!((rows[0].0 + rows[0].1 + rows[1].0 - WIDTH).abs() < 1e-3);
}

#[trial]
fn the_most_columns_follows_the_family_and_the_builder() {
    assert_eq!(TextColumnsDialog::new(two()).most_columns(), MAX_COLUMNS);
    assert_eq!(TextColumnsDialog::new(two()).word(WIDTH).most_columns(), MAX_COLUMNS_WORD);
    assert_eq!(TextColumnsDialog::new(two()).word(WIDTH).max_columns(5).most_columns(), 5);
}

#[trial]
fn a_column_is_never_narrower_than_the_minimum() {
    for (width, spacing, count) in [(WIDTH, 400.0, 6), (60.0, 36.0, 3)] {
        let rows = TextColumnsDialog::even_rows(width, spacing, count);
        assert_eq!(rows.len(), count as usize);
        assert!(rows.iter().all(|row| row.0 >= MIN_COLUMN_WIDTH - 1e-3 || width < MIN_COLUMN_WIDTH * count as f32), "{rows:?}");
    }
}

#[trial]
fn the_preview_places_columns_and_the_line_between() {
    let page = Rect::from_min_max(pos2(0.0, 0.0), pos2(84.0, 110.0));
    let rows = [(100.0, 20.0), (100.0, 0.0)];
    let plan = preview_plan(&rows, 220.0, true, page);
    assert_eq!(plan.columns.len(), 2);
    assert_eq!(plan.separators.len(), 1);
    assert!(plan.columns[0].1 <= plan.separators[0] && plan.separators[0] <= plan.columns[1].0);
    assert!(preview_plan(&rows, 220.0, false, page).separators.is_empty());
}

#[trial]
fn apply_to_always_answers_with_the_choice() {
    let dialog = TextColumnsDialog::new(two()).apply_to(&[ColumnsApply::Section, ColumnsApply::Document], ColumnsApply::Document);
    assert_eq!(dialog.changes().and_then(|changes| changes.apply), Some(ColumnsApply::Document));
}

#[trial]
fn doc_unit_converts_points_to_each_length_kind_and_back() {
    for (points_per_inch, kind, per_inch) in [
        (72.0, UnitKind::Inches, 1.0),
        (72.0, UnitKind::Centimetres, 2.54),
        (72.0, UnitKind::Millimetres, 25.4),
        (72.0, UnitKind::Points, 72.0),
        (72.27, UnitKind::Points, 72.27),
        (72.27, UnitKind::Picas, 72.27 / 12.0),
    ] {
        let unit = DocUnit::new(kind, 96.0);
        let shown = unit.shown(144.0, points_per_inch);
        assert!((shown - 144.0 / points_per_inch * per_inch).abs() < 1e-9, "{kind:?} at {points_per_inch}");
        assert!((unit.to_points(shown, points_per_inch) - 144.0).abs() < 1e-9);
    }
    let pixels = DocUnit::new(UnitKind::Pixels, 96.0);
    assert!((pixels.shown(72.0, 72.0) - 96.0).abs() < 1e-9);
}
