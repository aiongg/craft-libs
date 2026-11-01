//! Tests of the Crop tool on the canvas tool host (task TO-C4).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Through `ToolHost`, a drag in the default frame draws, inside a drawn frame moves, on a corner resizes and outside turns, giving the frames `CropFrame::show` gives for the same pointer path, with Begin, Update and Commit.
#[test]
#[ignore = "skeleton"]
fn a_drag_draws_moves_resizes_and_turns_like_the_frame_show() {
    todo!()
}

// The first pass with a document makes the default frame; after Apply and `applied(true)` or a Discard the next pass makes a new one.
#[test]
#[ignore = "skeleton"]
fn the_tool_always_shows_a_frame() {
    todo!()
}

// Enter gives Apply with the commit of the settings, Esc gives Discard, Esc during a drag gives the frame's Cancel and keeps the frame.
#[test]
#[ignore = "skeleton"]
fn enter_applies_escape_discards_and_escape_in_a_drag_undoes_the_drag() {
    todo!()
}

// O, Shift+O, H and P return false during a drag and without a frame and give Settings otherwise; H is taken but inert in Classic Mode; I gives Settings whenever no drag runs.
#[test]
#[ignore = "skeleton"]
fn o_h_p_i_act_only_with_a_frame_and_no_drag() {
    todo!()
}

// In Image mode a turn sets the turned view through `set_view` and the frame stays upright; Classic Mode turns the frame; an undo step turns the view on the next pass.
#[test]
#[ignore = "skeleton"]
fn image_mode_requests_the_turned_view() {
    todo!()
}

// The armed button draws one line then disarms; Command held draws a line without arming; the release gives Straightened and a fitted frame as one box step; a line under `crop.straighten_min` changes nothing.
#[test]
#[ignore = "skeleton"]
fn straighten_one_shot_and_temporary_command() {
    todo!()
}

// `refit` with `editing` true twice (1:3, then 16:3) fits from the first frame, not from the 1:3 one (#7752).
#[test]
#[ignore = "skeleton"]
fn editing_a_ratio_refits_from_the_frame_the_edit_began_with() {
    todo!()
}

// A pass with another document's key gives Discard for a drawn frame and makes that document's default frame.
#[test]
#[ignore = "skeleton"]
fn a_second_document_discards_a_drawn_frame() {
    todo!()
}

// `parts` gives the Crop frame group with nine parts; Page Down through the host picks them in order and an arrow moves the picked corner.
#[test]
#[ignore = "skeleton"]
fn parts_name_every_corner_edge_and_the_whole_frame() {
    todo!()
}

// `leave` gives Apply for a drawn frame and nothing for the default; `applied(false)` gives the frame, its steps and the kept view back.
#[test]
#[ignore = "skeleton"]
fn leaving_applies_a_drawn_frame_and_a_refused_commit_restores_it() {
    todo!()
}

// A second click of a run inside a drawn frame gives Apply; on the default frame or a handle it does not.
#[test]
#[ignore = "skeleton"]
fn a_double_click_inside_a_drawn_frame_applies() {
    todo!()
}
