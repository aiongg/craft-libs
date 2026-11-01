//! Tests of the Polygonal Lasso on the tool host (task SE-F9).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// ⇧ turns a segment to 45° steps from the last vertex, not for the first vertex and not with ⌥.
#[test]
#[ignore = "skeleton"]
fn polygon_shift_constrains_to_45_degrees() {
    todo!()
}

// A plain press inside the selection places a vertex; only ⌘ drags it, and not while an outline
// is open.
#[test]
#[ignore = "skeleton"]
fn polygon_click_driven_needs_command_to_drag() {
    todo!()
}

// `retract` (the app's secondary click), Backspace and Delete remove the last vertex; the last one
// cancels with no event.
#[test]
#[ignore = "skeleton"]
fn polygon_retract_removes_last_vertex_then_cancels() {
    todo!()
}

// `replaces_selection` is true only while a New outline is open.
#[test]
#[ignore = "skeleton"]
fn polygon_new_outline_replaces_selection() {
    todo!()
}
