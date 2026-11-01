//! Tests of the outline draft and the selection operation (task K9).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Mixed, ⌥ pressed after the press: clicks add straight vertices, drags add freehand points;
// `straight()` holds exactly the clicked indices.
#[test]
#[ignore = "skeleton"]
fn draft_alt_latch_straight_and_freehand() {
    todo!()
}

// Mixed with a selection, ⌥ held at the press: `Started.intent` keeps ⌥ and the outline is
// freehand until ⌥ is released and pressed again.
#[test]
#[ignore = "skeleton"]
fn draft_alt_before_press_subtracts_until_released() {
    todo!()
}

// Mixed and Polygonal with nothing selected: ⌥ at the press starts straight segments and is
// dropped from `Started.intent`.
#[test]
#[ignore = "skeleton"]
fn draft_alt_with_nothing_selected_is_polygonal() {
    todo!()
}

// Close within 8 pt of the first point (3 or more points) and within 3 pt of the last point, at
// zoom 1 and 4; a segment shorter than 8 pt still places its vertex.
#[test]
#[ignore = "skeleton"]
fn draft_closes_near_first_and_on_repeat_click_only() {
    todo!()
}

// Mixed: ⌥ released with the button up closes; with the button down drawing goes on.
#[test]
#[ignore = "skeleton"]
fn draft_release_alt_with_button_up_closes() {
    todo!()
}

// `retract` removes the last vertex between clicks, not while the button is down; the last one
// gives Cancelled; `straight` indices past the end go.
#[test]
#[ignore = "skeleton"]
fn draft_retract_removes_last_vertex_then_cancels() {
    todo!()
}

// `cancel` and `for_document` with another key drop an open outline; an empty draft gives None.
#[test]
#[ignore = "skeleton"]
fn draft_dropped_on_tool_or_document_change() {
    todo!()
}

// Polygonal with ⇧: the second vertex turns to the nearest 45° keeping its distance; not the first
// vertex, not with ⌥; a close with 2 points cancels.
#[test]
#[ignore = "skeleton"]
fn draft_polygonal_shift_turns_to_45_degrees() {
    todo!()
}

// `resolve`: modifiers override the bar; ⌥ or ⇧⌥ with nothing selected and bar New give New; a bar
// Subtract or Intersect is kept with nothing selected.
#[test]
#[ignore = "skeleton"]
fn resolve_alt_without_selection_is_new() {
    todo!()
}
