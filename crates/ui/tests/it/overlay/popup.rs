//! Popups: opening, staying open, and the dismissal rules.

use super::kit::{Leaf, Opened, Opener, Seen};
use craft_ui::contract::{Overlay, Props as _, Show as _, Trigger};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

const OPENER: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(180.0, 120.0) };
const BELOW: Rect = Rect { min: Pos2::new(300.0, 300.0), max: Pos2::new(360.0, 330.0) };

/// An opener of the preset `make` and a leaf below it at a fixed place; returns whether the popup body ran.
fn scene(frames: &mut Frames, events: Vec<egui::Event>, trigger: Trigger, make: fn() -> Overlay, open: bool, below: &RefCell<Seen>) -> bool {
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    frames.frame(events, |ui: &mut Ui| {
        Opener::new(&opened, &content, trigger, make).open(open).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    });
    opened.shown.get()
}

#[trial]
fn manual_popup_opens_only_through_open() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, Overlay::dropdown, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    let at = OPENER.center();
    frame(&mut frames, vec![moved(at)], false);
    frame(&mut frames, vec![press(at)], false);
    frame(&mut frames, vec![release(at)], false);
    let alt_down = key_press(Key::ArrowDown, Modifiers::ALT);
    let after_keys = [frame(&mut frames, vec![alt_down], false), frame(&mut frames, vec![], false)];
    assert_eq!(after_keys, [false, false], "presses and Intent::Open never open a manual popup");
    frame(&mut frames, vec![], true);
    assert!(frame(&mut frames, vec![], false), "the popup is open after `open`");
    assert!(frame(&mut frames, vec![], false), "and stays open");
}

#[trial]
fn a_popup_stays_open_over_idle_frames() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    frames.frame(vec![], |_| {});
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Click, Overlay::dropdown).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(&below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    };
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(2, &mut show);
    assert!(opened.shown.get(), "a click on the opener opens its popup");
    for _ in 0..120 {
        frames.frame(vec![], &mut show);
        assert!(opened.shown.get(), "the popup stays open over two seconds of idle frames");
    }
}

/// Opens the popup, then presses outside it on the leaf below; returns whether the popup still shows after the press
/// and what the leaf below saw.
fn press_outside(make: fn() -> Overlay) -> (bool, u32) {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, make, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    frame(&mut frames, vec![], true);
    for _ in 0..30 {
        frame(&mut frames, vec![], false);
    }
    assert!(frame(&mut frames, vec![], false), "the popup is open before the press");
    let at = BELOW.center();
    frame(&mut frames, vec![moved(at)], false);
    frame(&mut frames, vec![press(at)], false);
    frame(&mut frames, vec![release(at)], false);
    let still = [frame(&mut frames, vec![], false), frame(&mut frames, vec![], false)];
    let presses = below.borrow().presses;
    (still[1], presses)
}

#[trial]
fn an_outside_press_closes_a_dropdown_and_reaches_the_widget_below() {
    let (open, presses) = press_outside(Overlay::dropdown);
    assert!(!open, "an outside press closes the popup");
    assert!(presses > 0, "the press reaches the widget below");
}

#[trial]
fn an_outside_press_closes_a_menu_and_does_not_reach_the_widget_below() {
    let (open, presses) = press_outside(Overlay::menu);
    assert!(!open, "an outside press closes the menu");
    assert_eq!(presses, 0, "the menu absorbs the press");
}

#[trial]
fn a_press_inside_the_popup_keeps_it_open() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, Overlay::dropdown, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    frame(&mut frames, vec![], true);
    for _ in 0..30 {
        frame(&mut frames, vec![], false);
    }
    // The dropdown hangs below its anchor, as wide as the anchor.
    let inside = rect(OPENER.left(), OPENER.bottom(), OPENER.width(), 30.0).center();
    frame(&mut frames, vec![moved(inside)], false);
    frame(&mut frames, vec![press(inside)], false);
    frame(&mut frames, vec![release(inside)], false);
    assert!(frame(&mut frames, vec![], false), "a press inside the popup does not close it");
    assert!(frame(&mut frames, vec![], false));
}

#[trial]
fn a_click_on_the_opener_closes_its_open_menu() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Click, Overlay::menu).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(&below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    };
    frames.idle(3, &mut show);
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(30, &mut show);
    assert!(opened.shown.get(), "a click opens the menu");
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(30, &mut show);
    assert!(!opened.shown.get(), "the press on the opener reaches it, and its toggle closes the menu");
}

#[trial]
fn a_press_trigger_opens_on_the_press() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events| scene(frames, events, Trigger::Press, Overlay::popover, false, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![]);
    }
    let at = OPENER.center();
    frame(&mut frames, vec![moved(at)]);
    frame(&mut frames, vec![press(at)]);
    let shown = [frame(&mut frames, vec![]), frame(&mut frames, vec![])];
    assert!(shown[1], "the popup opens while the button is still down");
}

#[trial]
fn a_hover_popup_closes_after_a_grace_period_with_the_pointer_on_neither() {
    let mut frames = Frames::new();
    let (opened, content) = (Opened::default(), RefCell::new(Seen::default()));
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Hover, Overlay::popover).id_salt("opener").show_at(ui, OPENER);
    };
    frames.idle(3, &mut show);
    frames.frame(vec![moved(OPENER.center())], &mut show);
    frames.advance(1.0);
    frames.idle(3, &mut show);
    assert!(opened.shown.get(), "hovering the opener opens the popup after the dwell");
    frames.frame(vec![moved(Pos2::new(700.0, 500.0))], &mut show);
    frames.idle(2, &mut show);
    assert!(opened.shown.get(), "the pointer on neither the opener nor the popup does not close it at once");
    frames.advance(1.0);
    frames.idle(3, &mut show);
    assert!(!opened.shown.get(), "it closes once the grace period passed");
}
