//! Count trials of the test `Harness` on craft-ui-rt: input reaches the view through the platform,
//! `run` stops when no frame is asked for, and motion is off unless a test turns it on.

use std::cell::Cell;
use std::rc::Rc;

use accesskit::Role;
use craft_behavior::IconId;
use craft_ui_contract::contract::rt::harness::{FRAME, Harness};
use craft_ui_contract::contract::rt::skeleton::{IconButton, Stack};
use craft_ui_rt::{BuildCx, Constraints, Element, LayoutCx, PaintCx, Vec2};

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

/// A fade over the hover duration of the tokens, evaluated at paint time.
struct Fade {
    alpha: Rc<Cell<f32>>,
}

impl Element for Fade {
    type Output = ();
    type Local = Cell<Option<f64>>;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Self::Local {
        Cell::new(None)
    }

    fn layout(&self, _: &mut Self::Local, _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(Vec2::new(10.0, 10.0))
    }

    fn paint(&self, start: &Self::Local, cx: &mut PaintCx<'_>) {
        let seconds = f64::from(cx.tokens().motion.hover.millis) / 1000.0;
        let now = cx.time_until(|_| None);
        let from = start.get().unwrap_or(now);
        start.set(Some(from));
        let alpha = if seconds <= 0.0 { 1.0 } else { ((now - from) / seconds).min(1.0) as f32 };
        if alpha < 1.0 {
            cx.time_until(|t| Some(t + FRAME));
        }
        self.alpha.set(alpha);
    }
}

fn fading(alpha: &Rc<Cell<f32>>) -> impl FnMut(&mut craft_ui_contract::contract::rt::Ui<'_, '_>) + 'static {
    let alpha = alpha.clone();
    move |ui| ui.build_cx().child(Fade { alpha: alpha.clone() })
}

#[test]
fn motion_off_finishes_a_fade_in_one_step_and_motion_on_takes_the_clock() {
    let alpha = Rc::new(Cell::new(0.0));
    let mut off = Harness::new_ui(fading(&alpha));
    off.step();
    assert_eq!(alpha.get(), 1.0, "a fade ends in the step that starts it");
    assert!(off.try_run().is_ok_and(|n| n <= 2), "no frame is asked for after it");

    let alpha = Rc::new(Cell::new(0.0));
    let mut on = Harness::builder().with_motion(true).build_ui(fading(&alpha));
    on.step();
    assert!(alpha.get() < 1.0, "with motion on the fade has not ended in its first step: {}", alpha.get());
    let first = on.now();
    on.run();
    assert!(alpha.get() < 1.0, "run follows frame requests, not the fade's wakes");
    on.run_to_rest();
    assert_eq!(alpha.get(), 1.0, "run_to_rest steps through the fade's wakes until it ends");
    assert!(on.now() - first > 0.0 && on.now() < 2.0, "the fade took simulated time only: {}", on.now());
}
