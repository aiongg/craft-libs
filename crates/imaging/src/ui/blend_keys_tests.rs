//! Tests of the blend mode keys (task AD-12).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// ⇧+ from the last entry wraps to the first and ⇧− from the first to the last; an unlisted
// current steps from the top (+) or the bottom (−); ⇧= counts as ⇧+.
#[test]
#[ignore = "skeleton"]
fn shift_plus_and_minus_step_and_wrap() {
    todo!()
}

// ⇧⌥M picks the Multiply entry as `BlendKeyPick::Lettered`; the event is gone from the input.
#[test]
#[ignore = "skeleton"]
fn shift_alt_letter_picks_the_listed_mode() {
    todo!()
}

// Every `BlendShortcut` maps to its own letter and back (`letter`, `from_key`), 26 distinct keys.
#[test]
#[ignore = "skeleton"]
fn every_shortcut_has_its_own_letter() {
    todo!()
}

// ⇧⌥P with no Pass Through entry: consumed (later readers don't see it) and no pick; ⇧⌥1 is not
// a table letter and stays in the input.
#[test]
#[ignore = "skeleton"]
fn a_letter_for_a_missing_mode_is_consumed_and_does_nothing() {
    todo!()
}

// key Â with physical key M picks Multiply; key Z with physical key Y picks Pin Light (the
// layout's letter wins).
#[test]
#[ignore = "skeleton"]
fn physical_key_is_used_when_the_layout_letter_is_not_a_mode() {
    todo!()
}

// With a focused text field (and with an open menu) the presses stay in the input and nothing is
// picked; with ⌘ or Ctrl held nothing is read either.
#[test]
#[ignore = "skeleton"]
fn keys_are_ignored_when_a_text_field_has_focus() {
    todo!()
}

// ⇧⌥Q with a `BlendKeyEntry::behind` entry picks it; without one it is consumed and picks nothing.
#[test]
#[ignore = "skeleton"]
fn behind_is_consumed_and_reported() {
    todo!()
}

// Two ⇧+ presses in one pass step twice from the current entry and report one pick.
#[test]
#[ignore = "skeleton"]
fn presses_in_one_pass_combine_into_one_pick() {
    todo!()
}
