//! The browser's reorder rules: where a dragged preset or group may land.

use craft_ui::contract::ListDrag;
use craft_ui::patterns::presets::{PresetReorder, PresetRow};
use craft_ui::testing::trial;
use egui::Id;

const A: u32 = 1;
const A_ONE: u32 = 2;
const INNER: u32 = 3;
const INNER_ONE: u32 = 4;
const B: u32 = 5;
const B_ONE: u32 = 6;

/// Group A holding a preset and the folder Inner; group B holding a preset.
fn rows() -> Vec<PresetRow<u32>> {
    vec![
        PresetRow::group(A, "A", 0),
        PresetRow::item(A_ONE, "One", 1),
        PresetRow::group(INNER, "Inner", 1),
        PresetRow::item(INNER_ONE, "Two", 2),
        PresetRow::group(B, "B", 0),
        PresetRow::item(B_ONE, "Three", 1),
    ]
}

fn drag(key: u32) -> ListDrag<u32> {
    ListDrag::new(Id::NULL, vec![key], key)
}

/// A preset lands next to another preset, never next to a group header; groups stay put unless asked to move.
#[trial]
fn presets_land_next_to_presets() {
    let rows = rows();
    let rules = PresetReorder::<u32>::items();
    assert!(rules.allows(&rows, &drag(A_ONE), &B_ONE, 5));
    assert!(rules.allows(&rows, &drag(A_ONE), &INNER_ONE, 3));
    assert!(!rules.allows(&rows, &drag(A_ONE), &B, 4));
    assert!(!rules.allows(&rows, &drag(B), &A, 0));
}

/// A group lands next to a group at a depth `nest` allows, and never inside its own subtree.
#[trial]
fn groups_land_next_to_groups_within_the_nesting_limit() {
    let rows = rows();
    let flat = PresetReorder::<u32>::items_and_groups();
    assert!(flat.allows(&rows, &drag(B), &A, 0));
    assert!(!flat.allows(&rows, &drag(B), &A_ONE, 1));
    assert!(!flat.allows(&rows, &drag(B), &INNER, 2));
    assert!(!flat.allows(&rows, &drag(A), &INNER, 2));

    let nested = PresetReorder::<u32>::items_and_groups().nest(1);
    assert!(nested.allows(&rows, &drag(B), &INNER, 2));
    assert!(nested.allows(&rows, &drag(A), &B, 4));
}

/// The host's test refuses a landing the rules would allow.
#[trial]
fn the_hosts_test_refuses_a_landing() {
    let rows = rows();
    let rules = PresetReorder::<u32>::items().accepts(|drag, over, _| drag.grabbed != A_ONE || *over != B_ONE);
    assert!(!rules.allows(&rows, &drag(A_ONE), &B_ONE, 5));
    assert!(rules.allows(&rows, &drag(A_ONE), &INNER_ONE, 3));
}
