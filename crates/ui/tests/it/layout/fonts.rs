//! The font discard hook.

use craft_ui::fonts;
use craft_ui::testing::layout::reordered_fonts;
use craft_ui::testing::{Frames, trial};

#[trial]
fn loading_fonts_runs_one_discard_pass_in_the_frame_that_loads_them() {
    let mut frames = Frames::new();
    frames.idle(2, |_| {});
    let loading = frames.frame(vec![], |ui| fonts::load(ui.ctx(), reordered_fonts()));
    assert_eq!(loading, 2, "the pass that changed the metrics runs again, once");
    assert_eq!(frames.frame(vec![], |_| {}), 1, "and the next frame needs no extra pass");
}

#[trial]
fn loading_fonts_between_frames_runs_no_extra_pass() {
    let mut frames = Frames::new();
    frames.idle(2, |_| {});
    fonts::load(&frames.ctx, reordered_fonts());
    assert_eq!(frames.frame(vec![], |_| {}), 1, "the next frame measures with the new fonts and needs no discard");
}
