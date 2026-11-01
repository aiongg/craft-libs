//! Tests of the Slice tools and the slice overlay (task SH-T4). PhotoCraft's
//! `tools/slice_ui/tests.rs` cases move here.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Normal: a 2 × 2 px drag selects the slice under the press (`Select`), a 3 × 1 one draws; Fixed
// Size draws its size on a click.
#[test]
#[ignore = "skeleton"]
fn slice_draw_needs_a_drag_over_two_pixels_except_fixed_size() {
    todo!()
}

// A press on the visible piece of a slice covered by a later one moves the whole slice (one
// `Move` with its full rect).
#[test]
#[ignore = "skeleton"]
fn slice_select_moves_whole_slice_when_press_hits_a_covered_piece() {
    todo!()
}

// Dragging the right handle of a selected user slice reports `Resize`; past the left edge it
// flips; layer and auto slices show no handles.
#[test]
#[ignore = "skeleton"]
fn slice_handles_resize_the_edges_they_hang_off() {
    todo!()
}

// A drag on an auto slice selects it and moves nothing; with `hide_auto` it can't be picked.
#[test]
#[ignore = "skeleton"]
fn auto_slices_do_not_move_and_hide_when_asked() {
    todo!()
}

// Delete in Select with a user slice selected reports `Command(Delete)` and takes the key; with an
// auto slice, or in Slice mode, the key is not taken (PhotoCraft: Layer › Delete never sees it).
#[test]
#[ignore = "skeleton"]
fn slice_delete_key_deletes_the_slice_not_the_layer() {
    todo!()
}

// Locked: a press in either mode reports `Refused(Locked)` and nothing else.
#[test]
#[ignore = "skeleton"]
fn locked_slices_refuse_every_gesture() {
    todo!()
}

// The overlay draws no badge on a 17 × 30 slice, "03" on a user slice and "04 ▪" on a layer slice.
#[test]
#[ignore = "skeleton"]
fn slice_badge_hides_on_small_slices_and_marks_layer_slices() {
    todo!()
}

// Page Down picks the next slice part (`Select`), arrows move a user slice, Enter reports
// `Command(Options)`, double click too.
#[test]
#[ignore = "skeleton"]
fn slice_keyboard_picks_moves_and_opens_options() {
    todo!()
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Move` and `Resize`.
#[test]
#[ignore = "skeleton"]
fn slice_cancels_on_escape_and_on_tool_switch() {
    todo!()
}
