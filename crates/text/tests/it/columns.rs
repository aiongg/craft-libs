//! The Columns dialog: what comes back, the arithmetic of the grid and presets, the menu and the preview.

use crate::set;
use craft_text::ui::columns::{
    ColumnsApply, ColumnsChoice, ColumnsPreset, MAX_COLUMNS, MAX_COLUMNS_WORD, MIN_COLUMN_WIDTH, TextColumns, TextColumnsDialog, TextColumnsWords,
    columns_menu, preview_plan,
};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, pos2};
use std::cell::RefCell;
use std::rc::Rc;

/// A text width of 6.5 in, points.
const WIDTH: f32 = 468.0;

fn two() -> TextColumns {
    set(TextColumns::default(), |c| {
        c.count = Some(2);
        c.spacing = Some(36.0);
        c.equal_width = Some(true);
        c.line_between = Some(false);
    })
}

#[trial]
fn only_changed_fields_come_back() {
    let mut dialog = TextColumnsDialog::new(two());
    assert_eq!(dialog.changes(), None);
    dialog.set_count(3);
    assert_eq!(dialog.changes(), Some(set(TextColumns::default(), |c| c.count = Some(3))));
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

