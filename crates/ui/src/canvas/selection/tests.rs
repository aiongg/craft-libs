//! Tests of the selection frame, outlines and range helpers (task K11).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Resize-only rules: a corner drag stretches freely and ⇧ keeps the proportions, ⌥ scales about the
// centre, an edge stretches one axis, the inside moves and ⇧ locks it to an axis.
#[test]
#[ignore = "skeleton"]
fn frame_resize_rules_follow_legacy_free() {
    todo!()
}

// With focus, Page Down picks the whole frame, then the corners and edges clockwise; an arrow moves
// the picked part by the step (⇧ the big step), one `Set` per press.
#[test]
#[ignore = "skeleton"]
fn frame_keyboard_parts_and_nudge() {
    todo!()
}

// Esc during a drag gives Cancel and leaves the frame focused; Esc with no drag gives nothing.
#[test]
#[ignore = "skeleton"]
fn frame_esc_cancels_drag_only() {
    todo!()
}

// Parts the rules refuse show no grip and take no press; `grips(false)` hides all and the inside
// still moves.
#[test]
#[ignore = "skeleton"]
fn frame_hides_grips_when_refused() {
    todo!()
}

// A press outside the frame is not claimed and reaches the tool.
#[test]
#[ignore = "skeleton"]
fn frame_press_outside_is_the_tools() {
    todo!()
}

// `range_bands` gives the four sides cut to the clip, none for non-finite input; `rect_around` never
// overlaps, leaves the hole clear and keeps the area whole for a broken hole.
#[test]
#[ignore = "skeleton"]
fn range_bands_split_hole_and_clip() {
    todo!()
}
