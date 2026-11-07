//! Tooltips: shown after a delay, hidden on a press.

use super::kit::{Leaf, Seen};
use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::runtime::overlay::{OpenFilter, any_open};
use craft_ui::testing::pointer::{moved, press};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const TARGET: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(180.0, 130.0) };

#[trial]
fn a_tooltip_shows_after_its_delay_and_hides_on_press() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        Leaf::new(&seen, Vec2::new(80.0, 30.0)).id_salt("target").tooltip("Hint").show_at(ui, TARGET);
    };
    frames.idle(3, &mut show);
    let at = TARGET.center();
    frames.frame(vec![moved(at)], &mut show);
    frames.idle(10, &mut show);
    assert!(!any_open(&frames.ctx, OpenFilter::Any), "the tooltip waits for its delay");
    frames.advance(DELAY);
    frames.idle(3, &mut show);
    assert!(any_open(&frames.ctx, OpenFilter::Any), "the tooltip shows after its delay");
    frames.frame(vec![press(at)], &mut show);
    frames.idle(30, &mut show);
    assert!(!any_open(&frames.ctx, OpenFilter::Any), "a press hides the tooltip");
}
