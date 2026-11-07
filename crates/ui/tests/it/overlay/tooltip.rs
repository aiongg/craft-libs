//! Tooltips: shown after a delay, hidden on a press.

use super::kit::{Leaf, Seen};
use craft_ui::contract::tooltip::DELAY;
use craft_behavior::IconId;
use craft_ui::buttons::IconButton;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::controls::Slider;
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

/// Whether resting the pointer on the shown component past the delay opens a tooltip.
fn tooltip_opens(show_component: impl Fn(&mut Ui)) -> bool {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_component(ui);
    frames.idle(3, &mut show);
    frames.frame(vec![moved(TARGET.center())], &mut show);
    frames.advance(DELAY);
    frames.idle(3, &mut show);
    any_open(&frames.ctx, OpenFilter::Any)
}

#[trial]
fn an_icon_only_face_shows_its_label_as_tooltip_and_a_slider_does_not() {
    let icon = tooltip_opens(|ui| {
        IconButton::new(IconId::new("close")).label("Close").show_at(ui, TARGET);
    });
    assert!(icon, "an icon button shows its label as tooltip");
    let mut value = 40.0;
    let slider = tooltip_opens(|ui| {
        Slider::new(Bind::bound(&mut value), 0.0..=100.0).label("Opacity").show_at(ui, TARGET);
    });
    assert!(!slider, "a slider that paints no text shows no label tooltip");
}
