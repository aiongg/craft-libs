//! Tests of the Artboard tool (task SH-T5). PhotoCraft's `artboard_tool.rs` tests move here. The
//! "+" button press needs `AddBesideHandles::at` (SH-T8): these tests set
//! `ArtboardRules::plus_buttons` off; the "+" test is in `add_beside_tests.rs`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Drawing, resizing and moving a board each report one gesture (Begin … Commit) with whole-pixel
// values (PhotoCraft `artboard_tool_draws_resizes_and_moves_boards`).
#[test]
#[ignore = "skeleton"]
fn artboard_draw_resize_move_one_event_each() {
    todo!()
}

// A press and release 2 screen points apart on the pasteboard reports nothing.
#[test]
#[ignore = "skeleton"]
fn artboard_click_without_drag_draws_nothing() {
    todo!()
}

// The right handle's cursor is the horizontal resize arrow at 0° and the vertical one with the view
// turned 90° (PhotoCraft `resize_cursors_turn_with_the_view`).
#[test]
#[ignore = "skeleton"]
fn artboard_resize_cursors_turn_with_the_view() {
    todo!()
}

// ⇧ pressed during a corner drag keeps the board's proportions from that pass on (PhotoCraft
// `plus_buttons_add_boards_and_shift_keeps_proportions`).
#[test]
#[ignore = "skeleton"]
fn artboard_shift_corner_keeps_proportions_mid_drag() {
    todo!()
}

// Illustrator rules: ⇧-click toggles a board, ⇧-drag on the pasteboard selects the boards it
// touches, arrows move by `arrow_step` and ⇧ ten times, Esc reports `Leave`.
#[test]
#[ignore = "skeleton"]
fn artboard_ai_rules_multi_select_marquee_and_arrow_step() {
    todo!()
}

// A locked board selects on a press but reports no `Move` or `Resize`, by pointer or arrows.
#[test]
#[ignore = "skeleton"]
fn artboard_locked_board_selects_but_never_moves() {
    todo!()
}

// Page Down picks the next board (`Select`), arrows move it, Delete reports `Delete`, Enter
// `OpenOptions`, Insert `AddBeside` on the right.
#[test]
#[ignore = "skeleton"]
fn artboard_keyboard_picks_moves_and_deletes() {
    todo!()
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Move` and `Resize`.
#[test]
#[ignore = "skeleton"]
fn artboard_cancels_on_escape_and_on_tool_switch() {
    todo!()
}
