//! The fit rule of `BarItems`: which items stay in the strip and which move to the overflow menu.

use craft_ui::bars::{FitItem, FitKeep, fit_items};
use craft_ui::testing::trial;

const EXTENT: f32 = 10.0;
const GAP: f32 = 2.0;
const SECTION_GAP: f32 = 8.0;
const CONTROL: f32 = 8.0;

fn items(sections: &[u16]) -> Vec<FitItem> {
    sections.iter().map(|&section| FitItem::new(EXTENT, section, FitKeep::Free)).collect()
}

/// Items that fit whole leave no overflow and no room for the control.
#[trial]
fn items_that_fit_leave_no_overflow() {
    let fit = fit_items(&items(&[0, 0, 0, 0]), EXTENT * 4.0 + GAP * 3.0, GAP, SECTION_GAP, CONTROL);
    assert_eq!(fit.shown, vec![0, 1, 2, 3]);
    assert!(fit.overflow.is_empty());
}

/// The last items leave first, and the items kept leave room for the control and its gap.
#[trial]
fn the_last_items_overflow_first() {
    let fit = fit_items(&items(&[0, 0, 0, 0]), 40.0, GAP, SECTION_GAP, CONTROL);
    assert_eq!(fit.shown, vec![0, 1]);
    assert_eq!(fit.overflow, vec![2, 3]);
}

/// A section change costs the section gap, not the item gap.
#[trial]
fn a_section_change_costs_the_section_gap() {
    let all = EXTENT * 3.0 + GAP + SECTION_GAP;
    let fit = fit_items(&items(&[0, 0, 1]), all, GAP, SECTION_GAP, CONTROL);
    assert_eq!(fit.shown, vec![0, 1, 2]);
    let fit = fit_items(&items(&[0, 0, 1]), all - 1.0, GAP, SECTION_GAP, CONTROL);
    assert_eq!(fit.shown, vec![0, 1]);
    assert_eq!(fit.overflow, vec![2]);
}

/// A pinned item stays when the strip is shorter than the item alone.
#[trial]
fn a_pinned_item_stays() {
    let mut items = items(&[0, 0, 0]);
    items[1].keep = FitKeep::Pinned;
    let fit = fit_items(&items, EXTENT * 1.5, GAP, SECTION_GAP, CONTROL);
    assert_eq!(fit.shown, vec![1]);
    assert_eq!(fit.overflow, vec![0, 2]);
}
