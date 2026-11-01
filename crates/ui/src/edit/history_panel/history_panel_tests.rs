//! Screen and AccessKit tests of the History panel (task ST3; they need the `history` style from TK1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on a past state reports one `GoTo` with its id and nothing else.
#[test]
#[ignore = "skeleton"]
fn clicking_a_state_reports_go_to_once() {
    todo!()
}

// A click on the current state reports nothing; with a snapshot selected the state row highlights again.
#[test]
#[ignore = "skeleton"]
fn clicking_the_current_state_reports_nothing_and_deselects_a_snapshot() {
    todo!()
}

// Later rows paint `list.dimmed` and a click reports `GoTo` forward.
#[test]
#[ignore = "skeleton"]
fn later_states_are_dimmed_and_clickable() {
    todo!()
}

// A click on a snapshot row reports `SelectSnapshot`; after `snapshot_selected` it is the one highlighted row.
#[test]
#[ignore = "skeleton"]
fn clicking_a_snapshot_selects_it_and_reports_select_snapshot() {
    todo!()
}

// A record after `snapshot_selected` moves the revision and the current state highlights again.
#[test]
#[ignore = "skeleton"]
fn a_selected_snapshot_is_deselected_when_the_list_changes() {
    todo!()
}

// A click on a state's source cell reports only `SetSource(Step(id))`; on the source row nothing.
#[test]
#[ignore = "skeleton"]
fn the_source_cell_sets_the_source_and_does_not_step() {
    todo!()
}

// Press on one source cell, drag over three rows, release: one `SetSource` at most, for the pressed row.
#[test]
#[ignore = "skeleton"]
fn a_press_on_the_source_cell_does_not_sweep_other_rows() {
    todo!()
}

// Double-click opens the rename field; Enter reports `RenameSnapshot`, Esc cancels, a blank name reports nothing.
#[test]
#[ignore = "skeleton"]
fn double_click_renames_a_snapshot_in_place() {
    todo!()
}

// A state dragged onto the trash reports `DeleteStep` once; a snapshot reports `DeleteSnapshot`.
#[test]
#[ignore = "skeleton"]
fn dragging_a_state_to_the_trash_reports_delete() {
    todo!()
}

// A drag from the oldest row or a later row (linear) never outlines the trash and reports nothing.
#[test]
#[ignore = "skeleton"]
fn the_oldest_state_and_later_states_in_linear_history_cannot_be_dragged() {
    todo!()
}

// Trash click with a snapshot selected reports `DeleteSnapshot`, else `DeleteStep(current)`; disabled at the oldest.
#[test]
#[ignore = "skeleton"]
fn the_trash_deletes_the_selected_snapshot_else_the_current_state() {
    todo!()
}

// The camera reports `NewSnapshot { dialog: false }`, with Alt held `dialog: true`.
#[test]
#[ignore = "skeleton"]
fn alt_click_on_the_camera_asks_for_the_dialog() {
    todo!()
}

// A record whose row is below the viewport scrolls it into view; a visible one keeps the offset.
#[test]
#[ignore = "skeleton"]
fn a_new_state_scrolls_into_view_only_when_hidden() {
    todo!()
}

// Passes with an unchanged revision keep the user's scroll offset.
#[test]
#[ignore = "skeleton"]
fn unrelated_changes_leave_the_scroll_where_it_is() {
    todo!()
}

// In a 160 pt dock with 6 snapshots and 20 states the footer stays inside the panel and the rows scroll.
#[test]
#[ignore = "skeleton"]
fn snapshots_scroll_with_the_states_above_the_footer() {
    todo!()
}

// Down moves the selection one row and reports `GoTo`; held keys report one event per pass for the last row.
#[test]
#[ignore = "skeleton"]
fn arrow_keys_select_and_report_one_event_per_pass() {
    todo!()
}

// Step Forward, Step Backward, Delete and Clear History follow the rows; New Snapshot and New Document always enabled.
#[test]
#[ignore = "skeleton"]
fn the_menu_enables_rows_by_state() {
    todo!()
}

// `run_command(delete)` with a snapshot selected reports `DeleteSnapshot` and clears the selection.
#[test]
#[ignore = "skeleton"]
fn delete_in_the_menu_removes_the_selected_snapshot() {
    todo!()
}

// AccessKit: `ListBox` "History"; state rows "Current state" or "Later state"; snapshot rows "Snapshot"; the source cell a checked `Button`.
#[test]
#[ignore = "skeleton"]
fn rows_have_names_descriptions_and_selected_state() {
    todo!()
}

// In right to left the source column sits at the right edge and the label ends before it.
#[test]
#[ignore = "skeleton"]
fn columns_mirror_in_right_to_left() {
    todo!()
}

// A `StepRows` with depths indents tree rows and dims `is_later` rows.
#[test]
#[ignore = "skeleton"]
fn tree_rows_indent_by_depth_and_dim_off_line_rows() {
    todo!()
}

// `HistoryOrder::NewestFirst` lists states newest first, snapshots still on top, the current state highlighted.
#[test]
#[ignore = "skeleton"]
fn newest_first_reverses_the_rows_and_keeps_the_selection() {
    todo!()
}

// Features off remove their buttons; with all off no footer is drawn.
#[test]
#[ignore = "skeleton"]
fn the_footer_leaves_out_buttons_for_features_that_are_off() {
    todo!()
}
