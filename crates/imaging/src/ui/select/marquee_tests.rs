//! Tests of the marquees and the selection drag (task SE-F8). Each drives a `MarqueeTool` through
//! a `ToolHost` with injected pointer input (`ToolHost::inject`) and a fake `SelectionProbe` (a
//! rectangle selection, optionally floating), then reads `take_events`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Normal style: ⇧ pressed after the press squares, ⌥ pressed after it centres, ⇧ held at the press
// does not (it is Add); Fixed Ratio 2 : 1 and Fixed Size 10 × 10 give PhotoCraft's pixel
// rectangles (`marquee_tests.rs` cases).
#[test]
#[ignore = "skeleton"]
fn marquee_corners_follow_constraint_and_modifiers() {
    todo!()
}

// Single Row and Single Column select the one-pixel line under the pointer across the whole
// document, follow the pointer until the release, and stay on the canvas past its edge.
#[test]
#[ignore = "skeleton"]
fn marquee_single_row_and_column_follow_pointer() {
    todo!()
}

// A release within the click slop reports `Deselect`; with Fixed Size the click commits the box at
// the pointer.
#[test]
#[ignore = "skeleton"]
fn marquee_click_deselects_and_fixed_size_click_places() {
    todo!()
}

// A plain press inside the selection with the bar on New drags the outline; with ⇧, ⌥ or a bar on
// Add it draws a marquee instead.
#[test]
#[ignore = "skeleton"]
fn press_inside_selection_drags_outline_only_for_new() {
    todo!()
}

// ⌘ inside the selection drags `Pixels { copy: false }`, ⌘⌥ `Pixels { copy: true }`; ⌘ outside it
// is `Ignored` (the host's `ignored_press`).
#[test]
#[ignore = "skeleton"]
fn command_press_cuts_and_option_copies() {
    todo!()
}

// With a floating piece a plain press on it drags `Pixels`; one in the margin the cut left draws.
#[test]
#[ignore = "skeleton"]
fn floating_piece_moves_with_plain_drag() {
    todo!()
}

// A selection drag reports Begin, whole-pixel Updates (only when the rounded delta changes) and
// Commit; Esc reports Cancel and nothing else; a release without travel commits unchanged.
#[test]
#[ignore = "skeleton"]
fn drag_selection_reports_whole_pixel_delta_and_cancel() {
    todo!()
}

// Space held during the drag moves the marquee without resizing it, and the tool takes the key.
#[test]
#[ignore = "skeleton"]
fn marquee_space_moves_the_marquee() {
    todo!()
}
