//! Tests of the crop session (task TO-C2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `ensure_frame` frames the selection's bounds clipped to the canvas, the canvas without a selection or with an empty one, marks it default, and makes none for an empty canvas.
#[test]
#[ignore = "skeleton"]
fn default_frame_is_the_selection_bounds_or_the_canvas() {
    todo!()
}

// A default frame made for another document key or another size is replaced; `for_document` drops a drawn frame of another document and reports it.
#[test]
#[ignore = "skeleton"]
fn another_document_or_size_gets_its_own_default_frame() {
    todo!()
}

// A busy drag settles as one step on its release, a key step is one step each; Undo and Redo go back and forth and give the frame with its default mark; a new step drops the redo side.
#[test]
#[ignore = "skeleton"]
fn box_undo_redo_steps_committed_gestures_only() {
    todo!()
}

// `leave`: Dropped for the untouched default frame, Apply for a drawn one, Kept while a gesture runs or the eraser stands in, Nothing without a frame; Straighten disarms.
#[test]
#[ignore = "skeleton"]
fn leaving_drops_a_default_frame_and_applies_a_drawn_one() {
    todo!()
}

// `keep` then `clear` then `restore`: frame, default mark, history and the kept view come back.
#[test]
#[ignore = "skeleton"]
fn a_refused_commit_gives_frame_and_steps_back() {
    todo!()
}

// `ratio_base(true)` keeps the first frame of the run until `ratio_base(false)` (#7752).
#[test]
#[ignore = "skeleton"]
fn a_ratio_edit_fits_from_the_frame_it_began_with() {
    todo!()
}
