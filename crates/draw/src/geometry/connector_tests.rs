//! Tests of connector routes (task SH-T2), ported from FigCraft's engine tests.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Straight, elbow and curved routes between two boxes and to a free point match FigCraft's cases:
// named sides and free points (`board_tools_tests.rs:94`), curves leave and arrive along the
// sides' normals (`board_connector_tests.rs:102`).
#[test]
#[ignore = "skeleton"]
fn connector_route_straight_elbow_curved_match_figcraft_cases() {
    todo!()
}

// An elbow whose short way would cross a joined box goes around it, keeping `ROOM` from it
// (`board_connector_tests.rs:141`).
#[test]
#[ignore = "skeleton"]
fn connector_elbow_goes_around_joined_boxes_by_room() {
    todo!()
}

// `reshaped` of a first or last run keeps it on its end through an extra run; a straight route
// has no path; `attached_sides` gives the sides it left by (`board_reshape_tests.rs:67`, `:123`,
// `:130`; `connector_anywhere_tests.rs:81`).
#[test]
#[ignore = "skeleton"]
fn connector_reshape_keeps_first_and_last_runs_on_their_ends() {
    todo!()
}

// `path_handles`: none for a straight route, one per run of an elbow at its middle, one for a
// curve at its middle.
#[test]
#[ignore = "skeleton"]
fn connector_path_handles_per_routing() {
    todo!()
}
