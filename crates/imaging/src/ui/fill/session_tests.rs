//! Tests of Content-Aware Fill's session, edits and tools (task WS-CA1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// In Auto and Rectangular the brush subtracts, in Custom it adds; Alt at the press reverses; a
// stroke keeps its points from before the drag threshold.
#[test]
#[ignore = "skeleton"]
fn fill_brush_adds_in_custom_and_subtracts_in_auto_and_alt_reverses() {
    todo!()
}

// The lasso's operation is the bar's or Shift, Alt, Shift+Alt at the press; Expand and Contract
// clamp to 1..=100; each reports a fill-area edit and serialises as PhotoCraft's `fillEdits`.
#[test]
#[ignore = "skeleton"]
fn fill_lasso_modes_and_expand_contract_report_fill_area_edits() {
    todo!()
}

// Reset Sampling clears the strokes and returns to Auto; Reset Fill only the fill settings; the
// footer's Reset everything; each session change is one undo step that gives back what it replaced.
#[test]
#[ignore = "skeleton"]
fn fill_reset_scopes_sampling_fill_and_all() {
    todo!()
}

// At the stroke or points limit a stroke does not begin and the message is reported; a stroke at
// the points limit stops taking points.
#[test]
#[ignore = "skeleton"]
fn fill_limits_refuse_at_the_brush() {
    todo!()
}

// The settings round-trip through serde with defaults; `FillCommand` serialises as `plan_params`.
#[test]
#[ignore = "skeleton"]
fn fill_settings_and_command_serialise_as_photocraft() {
    todo!()
}

// `rebase` clears the history and keeps the strokes and edits; `refuse_fill_edit` takes the last
// edit back with no redo.
#[test]
#[ignore = "skeleton"]
fn fill_rebase_clears_history_and_a_refused_edit_is_taken_back() {
    todo!()
}
