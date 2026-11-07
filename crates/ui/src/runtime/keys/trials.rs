//! Trials of the bare-Tab chrome toggle: when a Tab press toggles a window's panels and when it moves focus.

use crate::runtime::commands::{ChromeCommand, CommandInput, CommandSet as _};
use crate::runtime::env::RootEnv;
use crate::runtime::focus::{FocusTarget, KeyUse, request};
use crate::runtime::frame::{ChromeToggle, take_output};
use crate::testing::focus::{Stop, egui_focused, named_id, register_stop, widget};
use crate::testing::input::{key_press, key_repeat};
use crate::testing::{Frames, trial};
use craft_behavior::Input;
use craft_keymap::Overrides;
use egui::{Id, Key, Modifiers, Ui, ViewportId};
use std::sync::Arc;

fn id(name: &str) -> Id {
    named_id(name)
}

fn stop(ui: &mut Ui, name: &str, keys: KeyUse) {
    widget(ui, id(name));
    register_stop(ui.ctx(), Stop::new(id(name), keys));
}

/// A window with a document canvas, a button and a text field.
fn with_canvas(ui: &mut Ui) {
    stop(ui, "button", KeyUse::of(Input::Pointer));
    stop(ui, "canvas", KeyUse::CANVAS);
    stop(ui, "field", KeyUse::of(Input::Text));
}

/// A window without a canvas: a button and a slider.
fn without_canvas(ui: &mut Ui) {
    stop(ui, "button", KeyUse::of(Input::Pointer));
    stop(ui, "slider", KeyUse::of(Input::Pointer).with_value(true));
}

/// Frames with the app's switch `enabled`, past the first frame that registers the stops.
fn shown(enabled: bool, show: &mut impl FnMut(&mut Ui)) -> Frames {
    let mut frames = Frames::new();
    frames.root(RootEnv { tab_toggles_chrome: enabled, ..RootEnv::default() });
    frames.frame(Vec::new(), &mut *show);
    take_output(&frames.ctx);
    frames
}

fn press(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), key: Key, modifiers: Modifiers) -> Vec<(ViewportId, ChromeToggle)> {
    frames.frame(vec![key_press(key, modifiers)], &mut *show);
    take_output(&frames.ctx).chrome_toggles
}

fn focus_on(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), name: &str) {
    request(&frames.ctx, FocusTarget::Id(id(name)));
    frames.frame(Vec::new(), &mut *show);
    take_output(&frames.ctx);
}

#[trial]
fn tab_in_a_window_with_a_canvas_toggles_that_window_only() {
    let mut show = with_canvas;
    let mut frames = shown(true, &mut show);
    let child = ViewportId::from_hash_of("child window");
    frames.frame_in_viewport(child, without_canvas);
    take_output(&frames.ctx);
    assert_eq!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE), [(ViewportId::ROOT, ChromeToggle::Panels)], "nothing focused");
    assert_eq!(egui_focused(&frames.ctx), None, "the press moved no focus");
    focus_on(&mut frames, &mut show, "canvas");
    assert_eq!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE), [(ViewportId::ROOT, ChromeToggle::Panels)], "canvas focused");
    assert_eq!(egui_focused(&frames.ctx), Some(id("canvas")));
}

#[trial]
fn shift_tab_toggles_the_panels_and_keeps_the_tools() {
    let mut show = with_canvas;
    let mut frames = shown(true, &mut show);
    assert_eq!(press(&mut frames, &mut show, Key::Tab, Modifiers::SHIFT), [(ViewportId::ROOT, ChromeToggle::PanelsKeepTools)]);
}

#[trial]
fn tab_in_a_window_with_a_canvas_moves_focus_off_a_control() {
    let mut show = with_canvas;
    let mut frames = shown(true, &mut show);
    focus_on(&mut frames, &mut show, "button");
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty());
    assert_eq!(egui_focused(&frames.ctx), Some(id("canvas")));
    focus_on(&mut frames, &mut show, "field");
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty(), "a text field never loses Tab");
}

#[trial]
fn tab_in_a_window_without_a_canvas_moves_focus() {
    let mut show = without_canvas;
    let mut frames = shown(true, &mut show);
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty(), "nothing focused");
    assert_eq!(egui_focused(&frames.ctx), Some(id("button")));
    focus_on(&mut frames, &mut show, "slider");
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty(), "a slider");
    assert_ne!(egui_focused(&frames.ctx), Some(id("slider")), "focus left the slider");
}

#[trial]
fn tab_moves_focus_when_the_app_switch_is_off() {
    let mut show = with_canvas;
    let mut frames = shown(false, &mut show);
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty());
    assert_eq!(egui_focused(&frames.ctx), Some(id("button")));
}

#[trial]
fn a_repeated_tab_toggles_nothing_and_moves_no_focus() {
    let mut show = with_canvas;
    let mut frames = shown(true, &mut show);
    frames.frame(vec![key_repeat(Key::Tab, Modifiers::NONE)], &mut show);
    assert!(take_output(&frames.ctx).chrome_toggles.is_empty());
    assert_eq!(egui_focused(&frames.ctx), None);
}

#[trial]
fn tab_inside_an_open_dialog_toggles_nothing() {
    use crate::contract::Bind;
    use crate::dialog::Alert;
    let mut show = |ui: &mut Ui| {
        with_canvas(ui);
        let mut open = true;
        Alert::new("Error", "The name is taken.").opened(Bind::bound(&mut open)).show(ui);
    };
    let mut frames = shown(true, &mut show);
    frames.frame(Vec::new(), &mut show);
    take_output(&frames.ctx);
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty(), "no toggle under a dialog");
}

#[trial]
fn a_rebound_command_fires_on_its_new_key_and_tab_then_moves_focus() {
    let mut overrides = Overrides::default();
    overrides.insert(ChromeCommand::TogglePanels.id(), vec!["F9".to_owned()]);
    let mut show = with_canvas;
    let mut frames = Frames::new();
    frames.root(RootEnv { tab_toggles_chrome: true, ..RootEnv::default() });
    frames.commands(CommandInput::new().overrides(Arc::new(overrides)));
    frames.frame(Vec::new(), &mut show);
    take_output(&frames.ctx);
    assert_eq!(press(&mut frames, &mut show, Key::F9, Modifiers::NONE), [(ViewportId::ROOT, ChromeToggle::Panels)]);
    assert!(press(&mut frames, &mut show, Key::Tab, Modifiers::NONE).is_empty());
    assert_eq!(egui_focused(&frames.ctx), Some(id("button")), "Tab moves focus once its command is rebound");
}
