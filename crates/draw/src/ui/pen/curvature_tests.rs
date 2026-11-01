//! Tests of the pen draft's Curvature mode (task PE-B2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Clicks add smooth anchors (straight until the third); a drag after a press moves the new anchor
// past the click slop and the draft reflows; each change is an Update.
#[test]
#[ignore = "skeleton"]
fn curvature_draft_adds_smooth_anchors_that_follow_the_pointer() {
    todo!()
}

// With 3 or more anchors a press on the first closes at once: `closed`, Commit, `finished`; with
// two it adds an anchor there instead.
#[test]
#[ignore = "skeleton"]
fn curvature_draft_closes_on_the_first_anchor_press() {
    todo!()
}

// A double-click on empty canvas leaves a corner anchor; a double-click on a corner makes it
// smooth again.
#[test]
#[ignore = "skeleton"]
fn curvature_draft_double_click_places_a_corner_point() {
    todo!()
}

// A press on an inner anchor grabs it; the drag moves it and every smooth knot reflows.
#[test]
#[ignore = "skeleton"]
fn curvature_draft_grabs_an_existing_anchor_and_reflows() {
    todo!()
}

// Backspace and `remove_last` remove the last anchor and reflow; Enter ends and keeps the draft.
#[test]
#[ignore = "skeleton"]
fn curvature_draft_backspace_removes_the_last_anchor_and_reflows() {
    todo!()
}

// The hover draws the dashed preview segment through the pointer with 2 or more anchors and a
// straight dashed line with one, with the rubber band setting off and on.
#[test]
#[ignore = "skeleton"]
fn curvature_rubber_band_previews_the_next_curve() {
    todo!()
}
