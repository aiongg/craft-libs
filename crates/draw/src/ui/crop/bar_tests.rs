//! Tests of the Crop options bar (task TO-C5).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Clicking ✓, ⊘, New Crop Preset… and Delete Crop Preset… gives their events in order; a preset pick and a ratio field edit change the settings and give `Ratio`, with `editing` true while the field has focus.
#[test]
#[ignore = "skeleton"]
fn the_bar_reports_commit_cancel_new_and_delete_and_edits_options() {
    todo!()
}

// W x H x Resolution shows the W, H and Resolution fields with the unit select; any ratio shows the two ratio fields with ⇄; Original shows the document's size.
#[test]
#[ignore = "skeleton"]
fn the_bar_shows_size_fields_in_size_mode_and_ratio_fields_otherwise() {
    todo!()
}

// New Crop Preset… is disabled for Original and a free ratio, Delete Crop Preset… without user presets.
#[test]
#[ignore = "skeleton"]
fn new_and_delete_are_disabled_with_nothing_to_save_or_delete() {
    todo!()
}
