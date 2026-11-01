//! Tests of Select and Mask's session and edits (task WS-SM1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Edits and strokes are added in order and undo and redo (through `LocalHistory` over the session)
// take them back and forth.
#[test]
#[ignore = "skeleton"]
fn select_mask_tools_make_edits_and_undo_redo_them() {
    todo!()
}

// Refine Edge Brush strokes go to `strokes`, never `edits`, and serialise as `refineStrokes`.
#[test]
#[ignore = "skeleton"]
fn select_mask_refine_edge_strokes_are_kept_apart() {
    todo!()
}

// At the edit count or point limit `add_edit` returns the message and changes nothing; the same
// for strokes.
#[test]
#[ignore = "skeleton"]
fn select_mask_limits_refuse_at_the_brush_with_a_message() {
    todo!()
}

// Reset clears edits, strokes and the refinement values in one step; undo gives all of them back.
#[test]
#[ignore = "skeleton"]
fn select_mask_reset_is_one_undo_step_that_keeps_what_it_replaced() {
    todo!()
}

// Decontaminate Colors moves Output To from Selection or Layer Mask to New Layer with Mask and
// allows only the new targets.
#[test]
#[ignore = "skeleton"]
fn select_mask_decontaminate_limits_output_to_new_targets() {
    todo!()
}

// `command` serialises as PhotoCraft's `commit_params` (refinement keys at the top, tool-tagged
// edits, `target: "mask"` for a layer mask, empty lists left out).
#[test]
#[ignore = "skeleton"]
fn select_mask_command_matches_the_engine_params() {
    todo!()
}

// The settings round-trip through serde with defaults; `view_state` and `take_view` keep the
// view's fields.
#[test]
#[ignore = "skeleton"]
fn select_mask_settings_round_trip_and_carry_the_view() {
    todo!()
}
