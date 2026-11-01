//! `Thumbnails`, `ThumbRequest`, `thumb_image` and `fit_rect` over `FakeThumbSource`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// 20 synchronous renders in one pass: 16 land, 4 are Loading, and a repaint is requested.
#[test]
#[ignore = "skeleton"]
fn thumbnails_defer_renders_over_budget() {
    todo!()
}

// While held, a new revision returns the old picture as Loading and no render runs.
#[test]
#[ignore = "skeleton"]
fn thumbnails_hold_keeps_shown_pictures() {
    todo!()
}

// A Later render shows Missing (placeholder) until ManualJobs runs it, then Ready (task XT).
#[test]
#[ignore = "skeleton"]
fn thumbnails_later_renders_arrive_through_the_queue() {
    todo!()
}

// `paint` returns false for Empty and draws the placeholder for Missing and the icon for Failed.
#[test]
#[ignore = "skeleton"]
fn thumbnails_paint_each_state() {
    todo!()
}

// Landscape 400×300 at 64 gives 64×48; portrait 300×400 gives 48×64.
#[test]
#[ignore = "skeleton"]
fn thumb_image_has_the_frames_aspect() {
    todo!()
}

// 28 pt at 2× gives 64; 3 pt at 1× gives 16; past MAX_TEXTURE gives MAX_TEXTURE; `state` lowers to max_side.
#[test]
#[ignore = "skeleton"]
fn thumb_request_rounds_side_up_to_16() {
    todo!()
}

// `fit_rect` of a 4:3 picture in a square cell is centred, full width, three quarters high.
#[test]
#[ignore = "skeleton"]
fn fit_rect_keeps_the_aspect() {
    todo!()
}
