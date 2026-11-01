//! Tests of the before and after view (task K14).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// SideBySide: two halves `gap_side` apart, before on the left; in RTL before on the right.
#[test]
#[ignore = "skeleton"]
fn side_by_side_panes_are_gapped_halves_and_mirror_in_rtl() {
    todo!()
}

// Stacked: two halves `gap_stacked` apart, before above; captions take a strip off each pane.
#[test]
#[ignore = "skeleton"]
fn stacked_panes_are_gapped_halves() {
    todo!()
}

// Split at 0.25: before's clip ends a quarter across the shown picture from the reading start,
// after's begins there; SplitStacked the same down.
#[test]
#[ignore = "skeleton"]
fn split_clips_before_on_the_reading_start_side_of_the_divider() {
    todo!()
}

// A press and drag on the picture report Begin, Update and Commit of the fraction; Esc cancels;
// with `drag_anywhere(false)` a press away from the line reports nothing.
#[test]
#[ignore = "skeleton"]
fn divider_drag_sets_the_fraction_and_reports_edit_events() {
    todo!()
}

// Knob focused: → 1 %, ⇧→ 10 %, Home 0, End 1, one Set each; in RTL ← moves toward the end.
#[test]
#[ignore = "skeleton"]
fn divider_keys_move_one_percent_and_shift_ten_and_home_end() {
    todo!()
}

// While the hold key is down the output mode is Original with only the before pane; released, the
// host's mode is back; not while a text field has focus.
#[test]
#[ignore = "skeleton"]
fn hold_to_show_original_shows_only_before_while_the_key_is_down() {
    todo!()
}

// The knob is a Slider named "Compare position" with value "25%"; the panes are Images named
// "Before" and "After".
#[test]
#[ignore = "skeleton"]
fn before_after_accesskit_slider_has_name_and_percent_value() {
    todo!()
}
