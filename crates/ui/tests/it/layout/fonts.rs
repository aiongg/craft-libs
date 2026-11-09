//! The font discard hook.

use craft_ui::contract::Show as _;
use craft_ui::display::Text;
use craft_ui::fonts;
use craft_ui::runtime::env::Scale;
use craft_ui::testing::layout::reordered_fonts;
use craft_ui::testing::{Fixture, Frames, trial};

/// Loading fonts changes the metrics of the pass that loads them, which runs again once; loaded between frames,
/// the next frame measures with them and needs no extra pass.
#[trial]
fn loading_fonts_runs_one_discard_pass_only_in_the_frame_that_loads_them() {
    for in_frame in [true, false] {
        let mut frames = Frames::new();
        frames.idle(2, |_| {});
        let loading = if in_frame {
            frames.frame(vec![], |ui| fonts::load(ui.ctx(), reordered_fonts()))
        } else {
            fonts::load(&frames.ctx, reordered_fonts());
            frames.frame(vec![], |_| {})
        };
        assert_eq!(loading, if in_frame { 2 } else { 1 }, "passes of the loading frame (in frame: {in_frame})");
        assert_eq!(frames.frame(vec![], |_| {}), 1, "the next frame needs no extra pass (in frame: {in_frame})");
    }
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
    const SAME: f32 = 1.0;
    assert!((one - two).abs() < SAME, "\"Layers\" is {one} wide at scale 1 and {two} at scale 2");
}
