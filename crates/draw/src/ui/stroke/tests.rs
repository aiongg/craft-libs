//! Tests of the stroke editor and its button (task PE-B3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Clicking the Dashed and Dotted rows reports Sets of the preset style (Dotted with round caps);
// the row matching the value is selected, none for a custom pattern.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_preset_rows_report_the_preset_style() {
    todo!()
}

// PhotoCraft's rules: Alignment is disabled for an open path, Caps for a solid closed path,
// Corners without corners.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_disables_alignment_on_open_paths_and_caps_on_solid_closed_paths() {
    todo!()
}

// The Miter limit field is enabled only for miter corners.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_miter_limit_only_for_miter_corners() {
    todo!()
}

// Add pair appends [4, 2] (or 2 to an odd pattern) up to 32 entries; Remove pair keeps one pair;
// the dash offset field reports its value.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_dash_pairs_add_remove_and_offset() {
    todo!()
}

// A pattern of zeros shows "Use finite lengths and a positive pattern total" and reports no event.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_shows_the_error_for_an_invalid_pattern_and_reports_nothing() {
    todo!()
}

// With `mixed.cap` no cap button or select value is lit; picking one reports a Set for all.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_mixed_values_light_no_button() {
    todo!()
}

// An app row's pick reports `choice` with the row's and the choice's ids and no style event.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_choice_row_reports_the_pick() {
    todo!()
}

// The popover closes when the target or the value changes from outside; Esc closes it and the
// opener has focus again.
#[test]
#[ignore = "skeleton"]
fn stroke_button_closes_on_target_change_and_escape_returns_focus() {
    todo!()
}

// A press outside the open popover on a canvas closes it and the canvas sees neither the press
// nor its release.
#[test]
#[ignore = "skeleton"]
fn stroke_button_closing_press_reaches_nothing() {
    todo!()
}

// The opener is a button named "Stroke Options" with the style's name as its value; presets are
// radio buttons with their selected state; number fields carry their names and values.
#[test]
#[ignore = "skeleton"]
fn stroke_editor_accesskit_names_and_values() {
    todo!()
}
