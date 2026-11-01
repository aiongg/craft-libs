//! Tests of the Brush Preset picker (task BR-BP2).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on a card reports `List(Select)` and keeps the picker open; a double-click reports
// `List(Activate)` and closes it with `CloseReason::Content`.
#[test]
#[ignore = "skeleton"]
fn a_click_picks_and_a_double_click_picks_and_closes() {
    todo!()
}

// Esc closes; Enter closes unless a card rename is open (then it commits the rename only).
#[test]
#[ignore = "skeleton"]
fn escape_closes_and_enter_closes_unless_renaming() {
    todo!()
}

// A press on the `toggle` control does not close the picker as outside (the control's own click
// toggles it); a press elsewhere closes it; a press in the open gear menu does not.
#[test]
#[ignore = "skeleton"]
fn a_press_on_the_toggle_control_does_not_count_as_outside() {
    todo!()
}

// `hardness: None` shows no Hardness slider; `Some` shows it in percent and reports 0..=1.
#[test]
#[ignore = "skeleton"]
fn hardness_hides_for_a_sampled_tip() {
    todo!()
}

// Typing "SOFT" shows the presets whose names contain "soft" and opens their groups.
#[test]
#[ignore = "skeleton"]
fn search_filters_in_any_case_and_opens_groups() {
    todo!()
}

// A size dragged with the grip is the size the next open starts at.
#[test]
#[ignore = "skeleton"]
fn picker_size_is_remembered_between_opens() {
    todo!()
}

// Gear: Rename opens the current card's rename field and Delete reports `List(Delete(current))`;
// both are disabled without a current preset; Import shows only with `import(true)`.
#[test]
#[ignore = "skeleton"]
fn gear_rename_and_delete_act_on_the_current_preset() {
    todo!()
}

// Gear part boxes: clearing the last checked box leaves it on; a change reports `settings`.
#[test]
#[ignore = "skeleton"]
fn gear_part_boxes_keep_one_on_and_report_settings() {
    todo!()
}

// Brush Settings, New Brush Preset and Import report their picks and close the picker.
#[test]
#[ignore = "skeleton"]
fn picks_that_open_something_else_close_the_picker() {
    todo!()
}
