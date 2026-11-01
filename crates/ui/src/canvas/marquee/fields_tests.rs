//! Tests of the marquee style cells (task K13).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Picking Fixed Ratio gives Ratio(1, 1), Fixed Size Size(64, 64), Normal Free, each one `Set`.
#[test]
#[ignore = "skeleton"]
fn constraint_fields_style_pick_resets_the_numbers() {
    todo!()
}

// Typing 2.5 in Width of Ratio(1, 1) gives Ratio(2.5, 1) on Enter, not while typing; out of range
// clamps to 0.001..=999.
#[test]
#[ignore = "skeleton"]
fn constraint_fields_typed_width_commits_on_enter() {
    todo!()
}

// The swap button turns Size(100, 50) into Size(50, 100).
#[test]
#[ignore = "skeleton"]
fn constraint_fields_swap_exchanges_width_and_height() {
    todo!()
}

// Fixed Size in inches at 300 ppi shows 1 in for 300 px; typing "2in" gives 600 px; a picked unit
// comes back in `unit_picked`.
#[test]
#[ignore = "skeleton"]
fn constraint_fields_fixed_size_converts_units() {
    todo!()
}

// Free greys Width, swap and Height; `enabled(false)` greys every cell and reports nothing.
#[test]
#[ignore = "skeleton"]
fn constraint_fields_grey_for_normal_and_when_disabled() {
    todo!()
}
