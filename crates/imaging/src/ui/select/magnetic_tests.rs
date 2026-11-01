//! Tests of the Magnetic Lasso (task SE-F10), with a fake `EdgeTracer` that snaps to a circle and
//! traces along it, and a fake `SelectionProbe`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Points fasten every `spacing` along the edge at frequencies 0, 57 and 100, and a long jump is
// walked in at most 256 steps of `spacing / 2`.
#[test]
#[ignore = "skeleton"]
fn magnetic_fastens_every_spacing_and_walks_jumps() {
    todo!()
}

// ⌥-click adds a straight segment to the pointer; ⌥-drag draws freehand and fastens where it
// stops.
#[test]
#[ignore = "skeleton"]
fn magnetic_alt_click_straight_alt_drag_freehand() {
    todo!()
}

// Backspace removes fastening points back to the first; one more cancels with no event.
#[test]
#[ignore = "skeleton"]
fn magnetic_remove_last_point_and_cancel() {
    todo!()
}

// `[` and `]` change the width by 1 within 1..=256, with or without an outline open.
#[test]
#[ignore = "skeleton"]
fn magnetic_bracket_keys_change_width_within_range() {
    todo!()
}

// Enter closes along the edges (the tracer's path), ⌥ Enter with a straight segment, a click on the
// first point closes too; each commits one `Polygon` with the latched operation.
#[test]
#[ignore = "skeleton"]
fn magnetic_closes_along_edges_or_straight() {
    todo!()
}

// With `pressure_width` on, a pen sample at pressure 0.5 halves the searched width (minus one);
// a mouse sample never narrows it.
#[test]
#[ignore = "skeleton"]
fn magnetic_stylus_pressure_narrows_width() {
    todo!()
}

// While the Magnetic Lasso's outline is open the tool's `node` is "Selection outline" with its
// point count (fastening points and the live trace) and bounds.
#[test]
#[ignore = "skeleton"]
fn magnetic_outline_being_drawn_is_one_accesskit_object() {
    todo!()
}
