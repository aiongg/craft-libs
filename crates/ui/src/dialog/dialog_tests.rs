//! Placement, width, outcomes and the accessible tree of a dialog shown in a harness.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

/// What one pass reported: opened, carried an outer gesture, initial focus.
type Pass = (bool, bool, Option<InitialFocus>);

struct App {
    state: DialogState,
    open: bool,
    modeless: bool,
    lines: usize,
    width: Width,
    first: Option<Vec2>,
    over: Option<Rect>,
    last: Option<DialogResponse>,
    outcomes: Vec<Outcome>,
}

impl App {
    fn new() -> Self {
        Self {
            state: DialogState::new(),
            open: true,
            modeless: false,
            lines: 2,
            width: Width::Standard,
            first: None,
            over: None,
            last: None,
            outcomes: Vec::new(),
        }
    }

    fn rect(&self) -> Rect {
        self.last.as_ref().map_or(Rect::NOTHING, |shown| shown.rect)
    }
}

fn harness<State>(size: Vec2) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size(size).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

fn build(app: App, size: Vec2) -> Harness<'static, App> {
    harness(size).build_ui_state(
        |ui, app: &mut App| {
            let ctx = &ui.ctx().clone();
            if !app.open {
                return;
            }
            let mut dialog = Dialog::new("test", "Canvas Size").width(app.width);
            if app.modeless {
                dialog = dialog.modeless();
            }
            if let Some(at) = app.first {
                dialog = dialog.first_place(move |_| Some(at));
            }
            if let Some(host) = app.over {
                dialog = dialog.center_over(host);
            }
            let lines = app.lines;
            let shown = dialog.show(ctx, &mut app.state, &mut (), |ui, ()| {
                for n in 0..lines {
                    ui.label(format!("Row {n}"));
                }
            });
            app.outcomes.extend(shown.outcome);
            // A host stops showing a dialog once it has answered.
            app.open &= !shown.outcome.is_some_and(Outcome::closes);
            app.last = Some(shown);
        },
        app,
    )
}

fn drag(harness: &mut Harness<'_, App>, from: Pos2, by: Vec2) {
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.event(button(from, true));
    harness.run();
    for step in [by / 3.0, by] {
        harness.event(Event::PointerMoved(from + step));
        harness.run();
    }
    harness.event(button(from + by, false));
    harness.run();
}

/// A point on the title band, left of the title text's end and clear of the close box.
fn on_title(rect: Rect) -> Pos2 {
    pos2(rect.left() + 40.0, rect.top() + 14.0)
}

#[test]
fn the_first_open_is_centred() {
    let mut harness = build(App::new(), WINDOW);
    harness.run();
    let rect = harness.state().rect();
    assert!((rect.center() - Rect::from_min_size(Pos2::ZERO, WINDOW).center()).length() < 1.5, "{rect:?}");
}

#[test]
fn the_first_open_is_centred_over_a_host_rect_by_its_measured_size() {
    let host = Rect::from_min_max(pos2(120.0, 40.0), pos2(620.0, 300.0));
    let mut app = App::new();
    app.over = Some(host);
    let mut harness = build(app, WINDOW);
    harness.run();
    let rect = harness.state().rect();
    assert!((rect.center() - host.center()).length() < 1.5, "{rect:?} over {host:?}");

    harness.state_mut().lines = 12;
    harness.run();
    let grown = harness.state().rect();
    assert!((grown.min - rect.min).length() < 1.5, "the top-left stays once placed: {rect:?} {grown:?}");
}

#[test]
fn first_place_is_used_and_the_top_left_stays_when_the_body_grows() {
    let mut app = App::new();
    app.first = Some(vec2(40.0, 30.0));
    let mut harness = build(app, WINDOW);
    harness.run();
    let before = harness.state().rect();
    assert!((before.min - pos2(40.0, 30.0)).length() < 1.5, "{before:?}");
    harness.state_mut().lines = 12;
    harness.run();
    let after = harness.state().rect();
    assert_eq!(after.min, before.min);
    assert!(after.height() > before.height());
}

#[test]
fn a_dragged_dialog_reopens_where_it_was_left() {
    let mut harness = build(App::new(), WINDOW);
    harness.run();
    let start = harness.state().rect();
    drag(&mut harness, on_title(start), vec2(-90.0, 40.0));
    let moved = harness.state().rect();
    assert!((moved.min - (start.min + vec2(-90.0, 40.0))).length() < 2.0, "{start:?} {moved:?}");

    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Cancel]);
    assert!(!harness.state().state.is_open() && !harness.state().open);
    harness.state_mut().open = true;
    harness.run();
    let reopened = harness.state().rect();
    assert!((reopened.min - moved.min).length() < 1.5, "{moved:?} {reopened:?}");
}

#[test]
fn a_dialog_is_dragged_no_further_than_the_window() {
    let mut harness = build(App::new(), WINDOW);
    harness.run();
    let start = harness.state().rect();
    drag(&mut harness, on_title(start), vec2(-2000.0, -2000.0));
    let rect = harness.state().rect();
    assert!(rect.min.x >= -0.5 && rect.min.y >= -0.5, "{rect:?}");
}

#[test]
fn width_classes_set_the_frame_width_within_the_window() {
    // The frame is the content width plus `dialog.pad` on each side.
    for (width, expected) in [(Width::Compact, 274.0), (Width::Standard, 454.0), (Width::Wide, 614.0), (Width::Fixed(300.0), 314.0)] {
        let mut app = App::new();
        app.width = width;
        let mut harness = build(app, vec2(1000.0, 480.0));
        harness.run();
        assert!((harness.state().rect().width() - expected).abs() < 1.0, "{width:?}: {:?}", harness.state().rect());
    }
    let mut app = App::new();
    app.width = Width::Wide;
    let mut harness = build(app, vec2(400.0, 480.0));
    harness.run();
    let rect = harness.state().rect();
    assert!(rect.left() >= 24.0 - 1.0 && rect.right() <= 400.0 - 24.0 + 1.0, "{rect:?}");
}

#[test]
fn the_close_box_cancels_a_modal_dialog_and_closes_a_modeless_one() {
    for (modeless, expected) in [(false, Outcome::Cancel), (true, Outcome::Close)] {
        let mut app = App::new();
        app.modeless = modeless;
        let mut harness = build(app, WINDOW);
        harness.run();
        harness.get_by_label("Close").click();
        harness.run();
        assert_eq!(harness.state().outcomes, [expected]);
    }
}

#[test]
fn esc_cancels_a_modal_dialog_only() {
    for (modeless, expected) in [(false, vec![Outcome::Cancel]), (true, vec![])] {
        let mut app = App::new();
        app.modeless = modeless;
        let mut harness = build(app, WINDOW);
        harness.run();
        harness.key_press(Key::Escape);
        harness.run();
        assert_eq!(harness.state().outcomes, expected, "modeless: {modeless}");
    }
}

#[test]
fn the_dialog_reports_its_opening_once_and_carries_an_outer_gesture_when_asked() {
    let mut harness = harness(WINDOW).build_ui_state(
        |ui, seen: &mut (DialogState, Vec<Pass>)| {
            let ctx = &ui.ctx().clone();
            let shown = Dialog::new("outer", "Levels").outer_gesture(true).show(ctx, &mut seen.0, &mut (), |ui, ()| {
                ui.label("body");
            });
            seen.1.push((shown.opened, shown.outer.is_some(), shown.initial_focus));
        },
        (DialogState::new(), Vec::new()),
    );
    harness.run();
    let (_, passes) = harness.state();
    assert_eq!(passes.iter().filter(|(opened, ..)| *opened).count(), 1, "{passes:?}");
    assert!(passes.iter().all(|(_, outer, _)| *outer));
    assert_eq!(passes.iter().filter(|(.., focus)| focus.is_some()).count(), 1, "the initial focus is reported once");
}

#[test]
fn the_dialog_is_named_by_its_title_with_a_heading_and_a_close_button() {
    let mut harness = build(App::new(), WINDOW);
    harness.run();
    let dialog = harness.get_by(|n| n.role() == Role::Dialog);
    assert_eq!(dialog.accesskit_node().label().as_deref(), Some("Canvas Size"));
    let heading = harness.get_by(|n| n.role() == Role::Heading);
    assert_eq!(heading.accesskit_node().label().as_deref(), Some("Canvas Size"));
    assert_eq!(harness.get_by_label("Close").accesskit_node().role(), Role::Button);
}

#[test]
fn a_dialog_closed_without_an_outcome_opens_afresh() {
    let mut harness = build(App::new(), WINDOW);
    harness.run();
    let centred = harness.state().rect();

    harness.state_mut().state.close();
    harness.state_mut().first = Some(vec2(40.0, 30.0));
    harness.run();
    let reopened = harness.state().rect();
    assert!((reopened.min - pos2(40.0, 30.0)).length() < 1.5, "{centred:?} {reopened:?}");
}
