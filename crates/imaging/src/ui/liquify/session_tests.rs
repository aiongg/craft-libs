//! Tests of Liquify's session, tools and keys (task WS-LQ1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// W R E C S B O F D L H Z are the tools' letters (Twirl Counterclockwise has none); Alt reverses
// the twirls and swaps Pucker and Bloat, and leaves the others.
#[test]
#[ignore = "skeleton"]
fn liquify_tool_letters_and_alt_swaps() {
    todo!()
}

// A press, two drags and a release report Begin, Update, Update, Commit with every point; a
// press and release one pixel apart (under the drag threshold) is recorded whole with both points.
#[test]
#[ignore = "skeleton"]
fn liquify_stroke_events_begin_update_commit_and_short_strokes_are_recorded_whole() {
    todo!()
}

// Undo after two strokes reports Replay with the first; redo reports Replay with both; 101 undos
// keep at most 100 redo steps.
#[test]
#[ignore = "skeleton"]
fn liquify_undo_redo_ask_the_app_to_replay_and_redo_is_bounded() {
    todo!()
}

// A closed Freeze Lasso records one stroke with amount 1 (0 with Alt at the press) and the first
// point again at its end; fewer than 3 points record nothing.
#[test]
#[ignore = "skeleton"]
fn liquify_freeze_lasso_records_a_stroke_and_alt_thaws() {
    todo!()
}

// Reconstruct, Mask All, None and Invert All, and ⌘I and ⌘D, record the global step and report
// it; Restore All empties the session and reports Replay with nothing.
#[test]
#[ignore = "skeleton"]
fn liquify_global_buttons_and_keys_report_the_global_event() {
    todo!()
}

// [ and ] change the size by at least one pixel within 1..=15000; ⌘H toggles Show Mask.
#[test]
#[ignore = "skeleton"]
fn liquify_brackets_scale_the_brush_and_command_h_toggles_the_mask() {
    todo!()
}

// The settings round-trip through serde; a partial record fills the defaults; `clamped` puts
// out-of-range numbers back in range.
#[test]
#[ignore = "skeleton"]
fn liquify_settings_round_trip_with_serde_defaults() {
    todo!()
}

// A stroke serialises with the engine's names (`twirlCw`, `lassoMask`, `points` as triples).
#[test]
#[ignore = "skeleton"]
fn liquify_strokes_serialise_as_the_engine_reads_them() {
    todo!()
}
