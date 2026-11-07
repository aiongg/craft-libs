//! The font discard hook.

use craft_ui::contract::Show as _;
use craft_ui::display::Text;
use craft_ui::fonts;
use craft_ui::runtime::env::Scale;
use craft_ui::testing::layout::reordered_fonts;
use craft_ui::testing::{Fixture, Frames, trial};

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

/// 6.3: a text's width in points does not depend on the interface scale: glyph advances are not grid-fitted to the
/// pixel size.
#[trial]
fn a_text_measures_the_same_width_at_every_scale() {
    let width_at = |scale: f32| {
        let mut fixture = Fixture::new(|ui| {
            Text::new("Layers").show(ui);
        });
        fixture.scale(Scale::new(scale));
        fixture.settle();
        fixture.shown().width()
    };
    let (one, two) = (width_at(1.0), width_at(2.0));
    assert!((one - two).abs() < 1.0, "\"Layers\" is {one} wide at scale 1 and {two} at scale 2");
}
