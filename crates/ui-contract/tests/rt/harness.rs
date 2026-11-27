//! Count trials of the test `Harness` on craft-ui-rt: input reaches the view through the platform,
//! `run` stops when no frame is asked for, and motion is off unless a test turns it on.

use accesskit::Role;
use craft_behavior::IconId;
use craft_ui_contract::contract::rt::harness::{FRAME, Harness};
use crate::skeleton::{IconButton, Stack};
use craft_ui_contract::contract::rt::{Component, ComponentKey, Cx, MeasureCx, Paint, Response, show_leaf, spec};
use craft_ui_contract::contract::{MemoKey, MemoProps};
use craft_ui_rt::Vec2;
use egui::Color32;

#[derive(Default)]
struct App {
    a_clicks: u32,
    b_clicks: u32,
    view_runs: u32,
}

fn view(ui: &mut craft_ui_contract::contract::rt::Ui<'_, '_>, app: &mut App) {
    app.view_runs += 1;
    Stack::new().show(ui, |ui| {
        if IconButton::new(IconId::new("brush"), "A").show(ui).clicked() {
            app.a_clicks += 1;
        }
        if IconButton::new(IconId::new("eraser"), "B").show(ui).clicked() {
            app.b_clicks += 1;
        }
    });
}

#[test]
fn a_harness_click_returns_clicked_and_runs_only_the_clicked_path() {
    let mut h = Harness::new_ui_state(view, App::default());
    h.run();
    assert_eq!(h.query_all_by_role(Role::Button).len(), 2);
    let a = h.get_by_label("A");
    assert!(h.query_by_label("nothing").is_none());
    h.click(&a);
    assert_eq!((h.state().a_clicks, h.state().b_clicks), (1, 0));
    let up = h.last_report();
    assert_eq!(up.frame.runs, 1, "only A's hook runs; B and the stack are kept: {up:?}");
    assert_eq!(up.frame.scopes, 1, "only the root scope on A's path runs");
    h.click(&h.get_by_label("B"));
    assert_eq!((h.state().a_clicks, h.state().b_clicks), (1, 1));
}

#[test]
fn run_stops_when_no_frame_is_asked_for_and_state_mut_builds_the_view_again() {
    let mut h = Harness::new_ui_state(view, App::default());
    let taken = h.try_run().expect("a still view settles");
    assert!(taken <= 3, "settled in {taken} steps");
    let runs = h.state().view_runs;
    assert_eq!(h.try_run(), Ok(1), "an idle view takes one step");
    assert!(!h.last_report().ran, "nothing ran in it");
    assert_eq!(h.state().view_runs, runs, "no step builds the view while nothing moved");
    h.state_mut().a_clicks = 7;
    h.run();
    assert_eq!(h.state().view_runs, runs + 1, "a change of the state builds the view once");
    assert_eq!(h.now(), h.steps_taken() as f64 * FRAME, "the clock moves only through steps");
}

/// A leaf that aims its opacity at `open` through B9's fade.
struct Fader {
    open: bool,
}

impl ComponentKey for Fader {
    fn key(&self, key: &mut MemoKey) {
        self.open.key(key);
    }
}

impl Component for Fader {
    const ROLE: Role = Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.fade_to(if self.open { 1.0 } else { 0.0 }, spec::OPEN_FADE);
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        paint.draw().canvas().rect_filled(paint.bounds(), 0.0, Color32::RED);
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn close_fader(motion: bool) -> Harness<bool> {
    let mut h = Harness::builder().with_motion(motion).build_ui_state(|ui, open: &mut bool| drop(show_leaf(Fader { open: *open }, ui)), true);
    h.run_to_rest();
    *h.state_mut() = false;
    h.step();
    h
}

#[test]
fn motion_off_finishes_a_fade_in_one_step_and_motion_on_takes_the_clock() {
    let off = close_fader(false);
    assert!(!off.driver.runtime.wants_time(), "with motion off the fade is at its target in the step of the run");
    let mut off = off;
    assert!(off.try_run().is_ok_and(|n| n <= 2) && !off.driver.platform.take_frame_request(), "no frame is asked for after it");

    let mut on = close_fader(true);
    assert!(on.driver.runtime.wants_time(), "with motion on the fade is still running after its first step");
    let first = on.now();
    on.run_to_rest();
    assert!(!on.driver.runtime.wants_time(), "run_to_rest steps until the fade ends");
    assert!(on.now() > first && on.now() < 2.0, "the fade took simulated time only: {}", on.now());
}
