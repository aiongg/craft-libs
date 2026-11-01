//! Tests of the pattern fill editor and dialog (task SH-T12). PhotoCraft's
//! `dialogs/pattern_fill_ui/tests.rs` cases move here.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Typing 2000 in Scale gives 1000, 0 gives 1; Snap to Origin reports `Set` with phase (0, 0).
#[test]
#[ignore = "skeleton"]
fn pattern_editor_scale_range_and_snap_to_origin() {
    todo!()
}

// With `colors` the editor shows the swatch grid and Foreground and Background chips, no Scale or
// Angle; a cell click reports `Set` with its id.
#[test]
#[ignore = "skeleton"]
fn pattern_editor_preset_mode_shows_foreground_and_background() {
    todo!()
}

// A Scale scrub in the dialog moves `current()` and sets `changed()` that pass; Cancel answers
// `DialogResult::Cancel`.
#[test]
#[ignore = "skeleton"]
fn pattern_dialog_reports_live_values_and_cancel_returns_nothing() {
    todo!()
}

// OK without a change answers the opening values and `unchanged()` is true.
#[test]
#[ignore = "skeleton"]
fn pattern_dialog_unchanged_ok_is_a_no_op() {
    todo!()
}
