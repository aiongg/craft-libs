//! A measuring run delivers nothing and consumes nothing: what the real run after it needs is still there.

mod record;

use craft_ui::contract::Politeness;
use craft_ui::runtime::commands::{FireSource, fire};
use craft_ui::runtime::toast::{self, Toast};
use craft_ui::testing::{Frames, measuring_run, take_effects, trial};

#[trial]
fn a_measuring_run_fires_and_announces_nothing() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        let ctx = ui.ctx().clone();
        measuring_run(&ctx, || {
            assert!(!fire(&ctx, "save", FireSource::Palette));
            craft_ui::contract::announce(&ctx, "Saved", Politeness::Polite);
            let id = toast::post(&ctx, Toast::new("Saved"));
            toast::dismiss(&ctx, id);
        });
        assert_eq!(take_effects(&ctx), ["commands::fire", "announce", "toast::post"]);
        assert!(toast::cards(&ctx).is_empty(), "no toast was posted");
    });
    assert!(craft_ui::runtime::frame::take_output(&frames.ctx).announcements.is_empty(), "nothing was announced");
}
