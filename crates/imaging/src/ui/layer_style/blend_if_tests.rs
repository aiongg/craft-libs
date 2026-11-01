//! Tests of the Blend If bar (task AD-10).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A drag of the unsplit black handle moves both halves; with Alt held it moves only the half on
// the pointer's side; a gesture Begin, Update, Commit; Esc cancels.
#[test]
#[ignore = "skeleton"]
fn blend_if_bar_drag_moves_handles_and_alt_drag_splits() {
    todo!()
}

// Tab visits the four handles; → moves the focused pair one level, ⇧→ ten, ⌥→ only the focused
// half; each press is one Set; AccessKit sliders are named "Black low, This Layer" and so on.
#[test]
#[ignore = "skeleton"]
fn blend_if_bar_arrow_keys_move_the_focused_handle_and_alt_arrow_moves_one_half() {
    todo!()
}
