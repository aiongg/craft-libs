//! The action list, its file and the player.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `moved_index` cases (the moved item, items between, items outside); `move_step` within an action
// remaps `disabled` and `dialogs`, and between actions carries the flags along.
#[test]
#[ignore = "skeleton"]
fn action_list_moves_follow_the_selection() {
    todo!()
}

// A taken or empty name is refused; the same name changes nothing; `calls` sees every step with
// the old and new names.
#[test]
#[ignore = "skeleton"]
fn action_list_rename_keeps_names_unique_and_updates_callers() {
    todo!()
}

// PhotoCraft's `actions.json` shape (steps as `[id, params]`, colour names, no `sets`) loads and
// saves unchanged; empty sets survive in `sets`.
#[test]
#[ignore = "skeleton"]
fn action_list_file_round_trip_keeps_pc_json() {
    todo!()
}

// `toggle_all` turns every step on unless all already are, then every step off.
#[test]
#[ignore = "skeleton"]
fn action_list_toggle_all_rule() {
    todo!()
}

// Playback stops at the first failure, skips disabled steps uncharged, and reports `ran`.
#[test]
#[ignore = "skeleton"]
fn action_player_stops_at_first_error_and_skips_disabled() {
    todo!()
}

// One budget is charged across nested plays and stops at the limit; a call back to a running
// action is refused as `Recursive`.
#[test]
#[ignore = "skeleton"]
fn action_player_charges_a_shared_budget_and_detects_loops() {
    todo!()
}

// An `authorize` denial neither runs nor pauses the step.
#[test]
#[ignore = "skeleton"]
fn action_player_gates_each_step() {
    todo!()
}

// `Paused` reports `paused_at`; playing again from the next step finishes the action.
#[test]
#[ignore = "skeleton"]
fn action_player_pauses_for_a_dialog_step() {
    todo!()
}
