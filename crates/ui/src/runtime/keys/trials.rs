//! Trials of the bare-Tab chrome toggle: when a Tab press toggles the chrome and when it moves focus.

use super::tab_toggles_chrome;
use crate::runtime::env::RootEnv;
use crate::runtime::focus::{FocusTarget, KeepKeys, KeyUse, request};
use crate::runtime::frame::take_output;
use crate::testing::focus::{Stop, egui_focused, named_id, register_stop, widget};
use crate::testing::input::{key_press, key_repeat};
use crate::testing::{Frames, trial};
use craft_behavior::Input;
use egui::{Id, Key, Modifiers, Ui};

fn id(name: &str) -> Id {
    named_id(name)
}

fn stop(ui: &mut Ui, name: &str, keys: KeyUse) {
    widget(ui, id(name));
    register_stop(ui.ctx(), Stop::new(id(name), keys));
}

/// A widget that keeps Tab, standing for a canvas until the answer to Q3 says how a canvas is recognised.
const TAB_CANVAS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::Tab);

/// Frames with the flag `enabled`, showing a button, a Tab-keeping canvas and a code editor, settled so the key stage
/// has read the flag.
fn settled(enabled: bool) -> (Frames, impl FnMut(&mut Ui)) {
    let mut frames = Frames::new();
    frames.root(RootEnv { tab_toggles_chrome: enabled, ..RootEnv::default() });
    let mut show = |ui: &mut Ui| {
        stop(ui, "button", KeyUse::of(Input::Pointer));
        stop(ui, "other", KeyUse::of(Input::Pointer));
        stop(ui, "canvas", TAB_CANVAS);
        stop(ui, "code", KeyUse::CODE);
    };
    frames.idle(2, &mut show);
    take_output(&frames.ctx);
    (frames, show)
}

fn press_tab(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), modifiers: Modifiers) -> bool {
    frames.frame(vec![key_press(Key::Tab, modifiers)], &mut *show);
    take_output(&frames.ctx).toggle_chrome
}

fn focus_on(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), name: &str) {
    request(&frames.ctx, FocusTarget::Id(id(name)));
    frames.frame(Vec::new(), &mut *show);
    frames.idle(1, &mut *show);
    take_output(&frames.ctx);
}

#[trial]
fn tab_moves_focus_when_the_chrome_flag_is_off() {
    let (mut frames, mut show) = settled(false);
    focus_on(&mut frames, &mut show, "button");
    assert!(!press_tab(&mut frames, &mut show, Modifiers::NONE));
    assert_eq!(egui_focused(&frames.ctx), Some(id("other")));
    let (mut frames, mut show) = settled(false);
    assert!(!press_tab(&mut frames, &mut show, Modifiers::NONE), "nothing focused and the flag off: no toggle");
}

#[trial]
fn tab_with_nothing_focused_toggles_the_chrome_once_and_moves_no_focus() {
    let (mut frames, mut show) = settled(true);
    assert!(press_tab(&mut frames, &mut show, Modifiers::NONE));
    assert_eq!(egui_focused(&frames.ctx), None);
    assert!(press_tab(&mut frames, &mut show, Modifiers::NONE), "every press toggles");
    assert_eq!(egui_focused(&frames.ctx), None);
}

#[trial]
fn shift_tab_with_nothing_focused_toggles_the_chrome() {
    let (mut frames, mut show) = settled(true);
    assert!(press_tab(&mut frames, &mut show, Modifiers::SHIFT));
    assert_eq!(egui_focused(&frames.ctx), None);
}

#[trial]
fn tab_in_a_component_that_keeps_tab_toggles_the_chrome() {
    let (mut frames, mut show) = settled(true);
    focus_on(&mut frames, &mut show, "canvas");
    assert!(press_tab(&mut frames, &mut show, Modifiers::NONE));
    assert_eq!(egui_focused(&frames.ctx), Some(id("canvas")));
}

#[trial]
fn tab_on_a_button_moves_focus_and_toggles_nothing() {
    let (mut frames, mut show) = settled(true);
    focus_on(&mut frames, &mut show, "button");
    assert!(!press_tab(&mut frames, &mut show, Modifiers::NONE));
    assert_eq!(egui_focused(&frames.ctx), Some(id("other")));
}

#[trial]
fn tab_in_a_text_part_that_keeps_tab_does_neither() {
    let (mut frames, mut show) = settled(true);
    focus_on(&mut frames, &mut show, "code");
    assert!(!press_tab(&mut frames, &mut show, Modifiers::NONE));
    assert_eq!(egui_focused(&frames.ctx), Some(id("code")));
}

#[trial]
fn a_repeated_tab_toggles_nothing_and_moves_no_focus() {
    let (mut frames, mut show) = settled(true);
    frames.frame(vec![key_repeat(Key::Tab, Modifiers::NONE)], &mut show);
    assert!(!take_output(&frames.ctx).toggle_chrome);
    assert_eq!(egui_focused(&frames.ctx), None);
}

#[trial]
fn only_a_widget_that_keeps_tab_lets_it_toggle() {
    assert!(tab_toggles_chrome(true, None));
    assert!(!tab_toggles_chrome(false, None));
    assert!(tab_toggles_chrome(true, Some(&TAB_CANVAS)));
    assert!(!tab_toggles_chrome(true, Some(&KeyUse::CODE)));
    assert!(!tab_toggles_chrome(true, Some(&KeyUse::of(Input::Pointer))));
}
