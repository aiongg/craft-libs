//! A drag on a side label scrubs the control's value; Escape mid-drag cancels it, and a disabled control's label
//! does not scrub.

use crate::label::kit::{Dial, Field, Seen};
use craft_ui::contract::{Props as _, ShowContainer as _, SideLabel};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{drag, moved, press};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Ui, Vec2};
use std::cell::RefCell;

const START: f64 = 10.0;
/// How far the drag travels, in points; the value changes by about one per point.
const TRAVEL: f32 = 30.0;

/// A container whose body labels a dial with a scrubbing side label, the dial disabled when `enabled` is false.
fn show(ui: &mut Ui, value: &mut f64, seen: &RefCell<Seen>, enabled: bool) {
    Field::new("Group", SideLabel::new()).id_salt("field").show(ui, |ui, cx| {
        let dial = Dial::new(&mut *value, seen).enabled(enabled);
        cx.labelled(ui, "dial", "Opacity".into(), SideLabel::new().scrub(true), dial);
    });
}

fn settled(value: &mut f64, seen: &RefCell<Seen>, enabled: bool) -> (Frames, Pos2) {
    let mut frames = Frames::new();
    frames.idle(3, |ui| show(ui, value, seen, enabled));
    let label = seen.borrow().label.clone().expect("the dial ran inside a label scope");
    (frames, label.rect.center())
}

#[trial]
fn escape_during_a_label_scrub_returns_the_value() {
    let seen = RefCell::new(Seen::default());
    let mut value = START;
    let (mut frames, from) = settled(&mut value, &seen, true);
    frames.frame(vec![moved(from)], |ui| show(ui, &mut value, &seen, true));
    frames.frame(vec![press(from)], |ui| show(ui, &mut value, &seen, true));
    for step in 1..=4 {
        let at = from + Vec2::new(TRAVEL * step as f32 / 4.0, 0.0);
        frames.frame(vec![moved(at)], |ui| show(ui, &mut value, &seen, true));
    }
    assert!(value > START, "the drag scrubbed the value before Escape: {value}");
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], |ui| show(ui, &mut value, &seen, true));
    frames.frame(Vec::new(), |ui| show(ui, &mut value, &seen, true));
    assert_eq!(value, START, "Escape cancelled the scrub and the value returned");
    frames.frame(vec![craft_ui::testing::pointer::release(from + Vec2::new(TRAVEL, 0.0))], |ui| show(ui, &mut value, &seen, true));
    assert_eq!(value, START, "the release after the cancel commits nothing");
}

#[trial]
fn a_disabled_controls_label_does_not_scrub() {
    let seen = RefCell::new(Seen::default());
    let mut value = START;
    let (mut frames, from) = settled(&mut value, &seen, false);
    drag(&mut frames, from, Vec2::new(TRAVEL, 0.0), &mut |ui| show(ui, &mut value, &seen, false));
    assert_eq!(value, START, "a drag on the label of a disabled control changes nothing");
}
