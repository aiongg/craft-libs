//! Pointer and focus rules the pipeline keeps between the services: one press cancel path, parts and their focus.

use super::kit::{Panel, PartSpec, Probe, Seen};
use craft_behavior::{PartKey, PartRef, PressPhase, Sense};
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::testing::contract::findings;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const TAP: PartKey = PartKey::new("tap");
const A: PartKey = PartKey::new("a");
const B: PartKey = PartKey::new("b");

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

fn releases(seen: &Seen) -> usize {
    seen.press.iter().filter(|phase| matches!(phase, PressPhase::Release { .. })).count()
}

fn cancels(seen: &Seen) -> usize {
    seen.press.iter().filter(|phase| matches!(phase, PressPhase::Cancel { .. })).count()
}

/// A tool on a canvas: press, drag, then release in a frame egui discards and repeats.
#[trial]
fn canvas_tool_release_in_a_discarded_pass_reaches_the_tool_once() {
    let mut frames = Frames::new();
    let canvas = RefCell::new(Seen::default());
    let discard = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        let tool = RefCell::new(Seen::default());
        Panel::new().arena(true).sense(Sense::HOVER | Sense::DRAG).seen(&canvas).id_salt("canvas").show_at(
            ui,
            rect(10.0, 10.0, 300.0, 200.0),
            |ui, _| {
                Probe::new(&tool).size(Vec2::new(40.0, 20.0)).id_salt("control").show_at(ui, rect(20.0, 20.0, 40.0, 20.0));
            },
        );
        if discard.get() && ui.ctx().current_pass_index() == 0 {
            ui.ctx().request_discard("the release frame runs twice");
        }
    };
    let from = Pos2::new(200.0, 150.0);
    frames.frame(vec![], &mut show);
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![press(from)], &mut show);
    frames.frame(vec![moved(from + Vec2::new(30.0, 0.0))], &mut show);
    discard.set(true);
    let passes = frames.frame(vec![release(from + Vec2::new(30.0, 0.0))], &mut show);
    discard.set(false);
    frames.frame(vec![], &mut show);
    assert_eq!(passes, 2, "the release frame ran twice");
    assert_eq!(releases(&canvas.borrow()), 1, "the tool saw its release once: {:?}", canvas.borrow().press);
}

/// Lead-1: an Escape over a captured press cancels the press first and reaches no component.
#[trial]
fn escape_over_a_captured_press_cancels_the_press_and_not_the_component() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        Probe::new(&seen).sense(Sense::DRAG | Sense::FOCUS | Sense::TAKE_FOCUS).id_salt("probe").show_at(ui, rect(10.0, 10.0, 100.0, 40.0));
    };
    let from = Pos2::new(50.0, 30.0);
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![press(from)], &mut show);
    frames.frame(vec![moved(from + Vec2::new(20.0, 0.0))], &mut show);
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert_eq!(cancels(&seen.borrow()), 1, "the press was cancelled: {:?}", seen.borrow().press);
    assert_eq!(seen.borrow().cancels, 0, "the component saw no Cancel intent");

    // Control: with no press captured the same Escape reaches the focused component.
    let before = cancels(&seen.borrow());
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    assert_eq!(seen.borrow().cancels, 1, "an Escape with no captured press is the component's Cancel");
    assert_eq!(cancels(&seen.borrow()), before);
}

/// A press whose owner is not drawn for a pass ends, so it does not resume when the owner is drawn again.
#[trial]
fn a_press_ends_when_its_owner_is_not_drawn_for_a_pass() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let drawn = Cell::new(true);
    let mut show = |ui: &mut Ui| {
        if drawn.get() {
            Probe::new(&seen).sense(Sense::DRAG).id_salt("probe").show_at(ui, rect(10.0, 10.0, 100.0, 40.0));
        }
    };
    let from = Pos2::new(50.0, 30.0);
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![press(from)], &mut show);
    frames.frame(vec![moved(from + Vec2::new(20.0, 0.0))], &mut show);
    let before = seen.borrow().press.len();
    drawn.set(false);
    frames.frame(vec![moved(from + Vec2::new(30.0, 0.0))], &mut show);
    drawn.set(true);
    frames.frame(vec![release(from + Vec2::new(30.0, 0.0))], &mut show);
    assert_eq!(seen.borrow().press.len(), before, "the press did not resume: {:?}", seen.borrow().press);
}

/// Tab into a component whose only stops are parts lands on its first part.
#[trial]
fn tab_into_a_component_sets_its_first_part_active() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let parts = [
        PartSpec { key: A, at: rect(0.0, 0.0, 20.0, 20.0), sense: Sense::CLICK | Sense::FOCUS, focusable: true },
        PartSpec { key: B, at: rect(20.0, 0.0, 20.0, 20.0), sense: Sense::CLICK | Sense::FOCUS, focusable: true },
    ];
    let mut show = |ui: &mut Ui| {
        Probe::new(&seen).sense(Sense::HOVER).parts(&parts).id_salt("probe").show_at(ui, rect(10.0, 10.0, 40.0, 20.0));
    };
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert_eq!(seen.borrow().active, Some(PartRef { key: A, index: None }));
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert_eq!(seen.borrow().active, Some(PartRef { key: B, index: None }), "Tab moves to the next part");
}

/// A part that senses a press has a focus stop or names its keyboard path; one that does neither is a finding.
#[trial]
fn a_pointer_part_with_no_keyboard_path_is_a_finding_once() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let declared = [PartSpec { key: TAP, at: rect(0.0, 0.0, 20.0, 20.0), sense: Sense::CLICK, focusable: true }];
    let undeclared = [PartSpec { key: TAP, at: rect(0.0, 0.0, 20.0, 20.0), sense: Sense::CLICK, focusable: false }];
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            Probe::new(&seen).parts(&declared).id_salt("declared").show(ui);
        });
    }
    assert!(findings(&frames.ctx).is_empty(), "a part with a focus stop is no finding");
    for _ in 0..3 {
        frames.frame(vec![], |ui| {
            Probe::new(&seen).parts(&undeclared).id_salt("undeclared").show(ui);
        });
    }
    let found = findings(&frames.ctx);
    assert_eq!(found.len(), 1, "one finding however many frames: {found:?}");
    assert_eq!(found[0].rule, "pointer-part-without-keyboard-path");
    assert!(found[0].component.contains("Probe"), "{:?}", found[0]);
}
