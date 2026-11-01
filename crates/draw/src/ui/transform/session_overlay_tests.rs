//! Tests of the session overlay and the box's hidden reference point (task TR-T8).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A corner drag on the overlay reports Begin, Update and Commit as TransformEdit::Box.
#[test]
#[ignore = "skeleton"]
fn the_overlay_turns_box_events_into_session_edits() {
    todo!()
}

// With `reference_point(false)` the box draws no pivot glyph, a press on the pivot's place hits
// the inside, Page keys skip it and AccessKit lists 9 parts; ⌥-press away from handles does
// nothing.
#[test]
#[ignore = "skeleton"]
fn a_hidden_reference_point_is_not_drawn_hit_or_picked() {
    todo!()
}

// In the Warp phase the overlay paints nothing, claims no press and reports no edit.
#[test]
#[ignore = "skeleton"]
fn in_the_warp_phase_the_overlay_claims_nothing() {
    todo!()
}

// Enter, a click outside and a double click inside each report TransformEdit::Commit; Esc with no
// drag reports TransformEdit::Cancel.
#[test]
#[ignore = "skeleton"]
fn enter_applies_a_click_outside_applies_a_double_click_inside_applies() {
    todo!()
}

// Tab reaches the box; Page Down walks whole box, corners, edges (and the pivot when shown);
// arrows move the picked part and report Set.
#[test]
#[ignore = "skeleton"]
fn every_overlay_part_is_reachable_from_the_keyboard() {
    todo!()
}
