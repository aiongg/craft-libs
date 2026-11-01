//! Tests of the mask views (task SE-F6).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// O M V A T K Y pick their views, F cycles in menu order (On Layers to Onion Skin), J and P toggle,
// X held hides the view until released; nothing is read with ⌘ held or while a text field has
// focus.
#[test]
#[ignore = "skeleton"]
fn mask_view_keys_pick_cycle_and_hold() {
    todo!()
}

// Each view keeps its own opacity: set Overlay to 30, switch to On Black (100) and back, Overlay
// reads 30; the defaults are 50, 50, 100, 100.
#[test]
#[ignore = "skeleton"]
fn mask_view_opacity_kept_per_view() {
    todo!()
}

// The recipe per view: veil colours and masks, the layer floor `1 − o` (Onion Skin) and 0 (On
// Layers), ants only in Marching Ants, the coverage from Show Edge and Show Original, `hidden`
// while X is held.
#[test]
#[ignore = "skeleton"]
fn mask_view_recipe_per_mode() {
    todo!()
}

// Picking a view in the drop-down and moving the Opacity slider report one change each, applied to
// the state; the slider is absent for Marching Ants.
#[test]
#[ignore = "skeleton"]
fn mask_view_controls_report_changes() {
    todo!()
}
