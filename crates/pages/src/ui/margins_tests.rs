//! Margin handles.

// Dragging the left margin of a Letter page to 100 pt gives 99 (snapped to 4.5); to 600 pt it stops leaving 72 pt of text; Alt drags freely.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn drags_snap_and_leave_room() {
    todo!()
}

// On an even page of mirrored margins the visible left edge edits the stored right margin with the pointer reflected.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn mirrored_even_pages_swap_edges() {
    todo!()
}

// Press, two moves and release report Begin, Update, Update, Commit with changed true; Esc mid-drag reports Cancel.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_drag_is_one_gesture() {
    todo!()
}
