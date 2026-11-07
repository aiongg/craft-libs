//! Pointer input, press focus and the wheel service read through the sensing core's shape record: double
//! clicks, buttons, press targets, hit sizes, the zoom modifier and armed controls.

use craft_behavior::{Buttons, HitLevel, PartKey, PointerButton, PressFocus, Sense, WheelTaker};
use craft_ui::runtime::pointer::frame;
use craft_ui::testing::pointer::{self, settle};
use craft_ui::testing::sensing::{ProbeMember, ProbePart, ProbeRoot, RootReport, press_focus, show};
use craft_ui::testing::wheel::{register, take};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const OVER: PartKey = PartKey::new("over");

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

/// A double click arrives as press, release, press, release in one batch; the input hook spreads the presses
/// over frames, so one pass sees a release and the next press together. Both clicks activate.
#[trial]
fn a_double_click_activates_twice() {
    let button = ProbeRoot::new("button", rect(100.0, 100.0, 80.0, 30.0), Sense::CLICK);
    let activations = Cell::new(0);
    let mut show_frame = |ui: &mut Ui| {
        let reports = show(ui, std::slice::from_ref(&button));
        activations.set(activations.get() + usize::from(reports[0].root.activated));
    };
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let at = Pos2::new(120.0, 115.0);
    frames.frame(vec![pointer::moved(at)], &mut show_frame);
    frames.frame(vec![pointer::press(at), pointer::release(at), pointer::press(at), pointer::release(at)], &mut show_frame);
    frames.idle(4, &mut show_frame);
    assert_eq!(activations.get(), 2);
}

/// A release of the secondary button is reported, and `buttons` lists every button held.
#[trial]
fn the_frame_reports_every_button() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let mut show_frame = |ui: &mut Ui| {
        let frame = frame(ui.ctx());
        seen.borrow_mut().push((frame.buttons, frame.release, frame.press.map(|press| press.button)));
    };
    settle(&mut frames, &mut show_frame);
    let at = Pos2::new(50.0, 50.0);
    frames.frame(vec![pointer::moved(at)], &mut show_frame);
    seen.borrow_mut().clear();
    frames.frame(vec![pointer::secondary(at, true)], &mut show_frame);
    frames.frame(vec![pointer::secondary(at, false)], &mut show_frame);
    let seen = seen.borrow();
    assert_eq!(seen[0], (Buttons::SECONDARY, None, Some(PointerButton::Secondary)));
    assert_eq!(seen[1], (Buttons::empty(), Some(PointerButton::Secondary), None));
}

/// A press on a group form nested in a canvas is classified by the form's shapes, not by the canvas's
/// top-of-stack part: stack positions only compare inside one arena.
#[trial]
fn a_press_on_a_nested_group_is_classified_inside_the_group() {
    let area = rect(100.0, 100.0, 400.0, 300.0);
    let canvas = ProbeRoot::new("canvas", area, Sense::empty())
        .arena()
        .part(ProbePart::new(OVER, area, Sense::DRAG | Sense::TAKE_FOCUS).level(HitLevel::Background, 0).over());
    let group = ProbeRoot::new("group", rect(150.0, 150.0, 200.0, 100.0), Sense::empty()).arena().member(ProbeMember::new(
        "item",
        rect(160.0, 160.0, 60.0, 30.0),
        Sense::CLICK,
    ));
    let (group_item_center, canvas_corner) = (Pos2::new(180.0, 170.0), Pos2::new(450.0, 350.0));
    let roots = [canvas, group];
    let mut show_frame = |ui: &mut Ui| {
        show(ui, &roots);
    };
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let mut at = |point: Pos2| {
        frames.frame(vec![pointer::moved(point)], &mut show_frame);
        press_focus(&frames.ctx, point)
    };
    let (item, outside) = (group_item_center, canvas_corner);
    assert_eq!(at(item), Some(PressFocus::Release), "the item of the nested group acts without taking focus");
    assert_eq!(at(outside), Some(PressFocus::Take), "outside the group the canvas takes it");
}

/// The click target decides: a card that senses click and drag starts its press although a drag-only handle
/// lies over it, and the handle starts its own.
#[trial]
fn the_click_target_decides_which_roots_start_a_press() {
    let card = ProbeRoot::new("card", rect(100.0, 100.0, 200.0, 100.0), Sense::CLICK | Sense::DRAG);
    let handle = ProbeRoot::new("handle", rect(100.0, 100.0, 40.0, 100.0), Sense::DRAG);
    let reports: RefCell<Vec<Vec<RootReport>>> = RefCell::new(Vec::new());
    let roots = [card, handle];
    let mut show_frame = |ui: &mut Ui| reports.borrow_mut().push(show(ui, &roots));
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    reports.borrow_mut().clear();
    pointer::click(&mut frames, Pos2::new(120.0, 150.0), &mut show_frame);
    let activated = |root: usize| reports.borrow().iter().filter(|frame| frame[root].root.activated).count();
    assert_eq!((activated(0), activated(1)), (1, 0), "the card clicks, the drag-only handle never does");
}

/// A part's hit area is at least the pointer's target across.
#[trial]
fn a_small_part_is_hit_within_the_target_size() {
    let area = rect(100.0, 100.0, 300.0, 300.0);
    let part = PartKey::new("dot");
    let side = 2.0;
    let root = ProbeRoot::new("root", area, Sense::empty()).arena().member(
        ProbeMember::new("dot", rect(200.0, 200.0, side, side), Sense::empty()).part(ProbePart::new(
            part,
            rect(200.0, 200.0, side, side),
            Sense::CLICK,
        )),
    );
    let clicks = Cell::new(0);
    let target = Cell::new(0.0);
    let mut show_frame = |ui: &mut Ui| {
        let reports = show(ui, std::slice::from_ref(&root));
        target.set(frame(ui.ctx()).target);
        clicks.set(clicks.get() + usize::from(reports[0].member("dot").is_some_and(|dot| dot.activated)));
    };
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let margin = (target.get() - side) / 2.0;
    assert!(margin > 0.0, "the target size is larger than the part");
    pointer::click(&mut frames, Pos2::new(200.0 - margin + 1.0, 201.0), &mut show_frame);
    assert_eq!(clicks.get(), 1, "a press just outside the part but inside its target size hits it");
    pointer::click(&mut frames, Pos2::new(200.0 - margin - 3.0, 201.0), &mut show_frame);
    assert_eq!(clicks.get(), 1, "a press beyond the target size does not");
}

/// Ctrl or Cmd with the wheel zooms and does not also scroll.
#[trial]
fn the_zoom_modifier_wheel_zooms_and_does_not_scroll() {
    let panel = ProbeRoot::id("zoom_panel");
    let area = rect(0.0, 0.0, 400.0, 400.0);
    let read = Cell::new((None, 1.0));
    let mut show_frame = |ui: &mut Ui| {
        register(ui.ctx(), WheelTaker::new(panel, area).vertical(true));
        read.set((take(ui.ctx(), panel), frame(ui.ctx()).zoom));
    };
    let mut frames = Frames::new();
    frames.idle(2, &mut show_frame);
    frames.frame(vec![pointer::moved(Pos2::new(50.0, 50.0))], &mut show_frame);
    frames.frame(vec![pointer::wheel(-30.0)], &mut show_frame);
    assert_eq!(read.get(), (Some(Vec2::new(0.0, -30.0)), 1.0), "control: a plain wheel scrolls");
    frames.frame(vec![pointer::zoom_wheel(-30.0)], &mut show_frame);
    let (scroll, zoom) = read.get();
    assert!(zoom != 1.0, "the pass zooms");
    assert!(scroll.is_none_or(|scroll| scroll == Vec2::ZERO), "and scrolls nothing: {scroll:?}");
}

/// A press on a member of an arena arms the member's value control for the wheel.
#[trial]
fn a_press_on_a_member_arms_its_wheel_taker() {
    let panel = ProbeRoot::id("member_panel");
    let slider = ProbeRoot::id("member_slider");
    let panel_rect = rect(0.0, 0.0, 400.0, 400.0);
    let slider_rect = rect(50.0, 50.0, 100.0, 20.0);
    let root = ProbeRoot::new("member_form", rect(0.0, 0.0, 300.0, 300.0), Sense::empty()).arena().member(ProbeMember::new(
        "member_slider",
        slider_rect,
        Sense::CLICK | Sense::DRAG,
    ));
    let read = Cell::new((None, None));
    let mut show_frame = |ui: &mut Ui| {
        show(ui, std::slice::from_ref(&root));
        register(ui.ctx(), WheelTaker::new(panel, panel_rect).vertical(true));
        register(ui.ctx(), WheelTaker::new(slider, slider_rect).vertical(true).gated(true));
        read.set((take(ui.ctx(), panel), take(ui.ctx(), slider)));
    };
    let on_slider = Pos2::new(70.0, 60.0);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    pointer::click(&mut frames, on_slider, &mut show_frame);
    frames.frame(vec![pointer::wheel(-30.0)], &mut show_frame);
    let scroll = Some(Vec2::new(0.0, -30.0));
    assert_eq!(read.get(), (None, scroll), "the armed member slider takes the wheel");
}

/// A control turning disabled while its press is open cancels the press: the release activates nothing.
#[trial]
fn a_press_open_while_the_control_turns_disabled_never_activates() {
    let at = Pos2::new(120.0, 115.0);
    let activations = Cell::new(0);
    let disabled = Cell::new(false);
    let mut show_frame = |ui: &mut Ui| {
        let button = ProbeRoot::new("button", rect(100.0, 100.0, 80.0, 30.0), Sense::CLICK);
        let button = if disabled.get() { button.disabled() } else { button };
        let reports = show(ui, std::slice::from_ref(&button));
        activations.set(activations.get() + usize::from(reports[0].root.activated));
    };
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    frames.frame(vec![pointer::moved(at)], &mut show_frame);
    frames.frame(vec![pointer::press(at)], &mut show_frame);
    disabled.set(true);
    frames.frame(vec![], &mut show_frame);
    frames.frame(vec![pointer::release(at)], &mut show_frame);
    frames.idle(2, &mut show_frame);
    assert_eq!(activations.get(), 0);
}
