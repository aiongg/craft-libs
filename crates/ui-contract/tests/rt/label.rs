//! Count trials of the label step and the edit registry on craft-ui-rt: a label shows again only when a fact its control
//! told it changed, and a cancel builds only the nodes that run a gesture. Relations only, no times.

use crate::skeleton::{Slider, Stack};
use craft_behavior::CancelReason;
use craft_ui_contract::contract::{Bind, Ui};
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;

type View<A> = fn(&mut Ui<'_, '_>, &mut A);

/// An app on a rig, run until it settles; the app is boxed so that a binding's key keeps its address.
struct Run<A> {
    rig: Rig<RecordingRenderer>,
    app: Box<A>,
    view: View<A>,
}

impl<A> Run<A> {
    fn new(app: A, view: View<A>) -> Run<A> {
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        let main = rig.main;
        rig.driver.runtime.set_access_enabled(main, true);
        let mut run = Run { rig, app: Box::new(app), view };
        for _ in 0..8 {
            if !run.step().ran {
                return run;
            }
        }
        panic!("the rig did not settle");
    }

    fn step(&mut self) -> StepReport {
        let (app, view) = (&mut *self.app, self.view);
        self.rig.step_with(&mut |_, cx| window_root(cx, |ui| view(ui, app)))
    }

    fn input(&mut self, input: Input) -> StepReport {
        let main = self.rig.main;
        self.rig.inject(main, input);
        self.step()
    }
}

fn runs(report: &StepReport) -> usize {
    report.frame.runs as usize
}

/// A slider with a reset value in a labelled stack.
struct Labelled {
    value: f64,
    track: Rect,
}

impl Default for Labelled {
    fn default() -> Labelled {
        Labelled { value: 0.0, track: Rect::NOTHING }
    }
}

fn labelled(ui: &mut Ui<'_, '_>, app: &mut Labelled) {
    let Labelled { value, track } = app;
    Stack::new().labelled("Amount").show(ui, |ui| {
        let out = Slider::new(Bind::bound(&mut *value).reset(0.0), 0.0..=100.0).show(ui);
        *track = out.response.rect;
    });
}

#[test]
fn an_unchanged_reset_fact_runs_no_label() {
    let mut run = Run::new(Labelled::default(), labelled);
    let track = run.app.track;
    let at = |f: f32| Pos2::new(track.min.x + f * track.width(), track.center().y);
    run.input(Input::mouse_move(at(0.25)));
    // The press moves the value off its reset value: the fact the label shows (the dot) changes, so the label runs too.
    let down = run.input(Input::mouse_down(at(0.25)));
    assert!(runs(&down) >= 2, "the first change of the reset fact runs the label as well as the slider: {down:?}");
    // The fact stays the same while the drag goes on: each move runs the slider alone.
    for f in [0.4, 0.55, 0.7] {
        let moved = run.input(Input::mouse_move(at(f)));
        assert_eq!(runs(&moved), 1, "a move that leaves the reset fact as it was runs the slider only: {moved:?}");
    }
}

/// Three sliders; the app cancels the running edits in the step after `cancel` is set.
struct Cancelling {
    values: [f64; 3],
    tracks: [Rect; 3],
    cancel: bool,
}

impl Default for Cancelling {
    fn default() -> Cancelling {
        Cancelling { values: [0.0; 3], tracks: [Rect::NOTHING; 3], cancel: false }
    }
}

fn cancelling(ui: &mut Ui<'_, '_>, app: &mut Cancelling) {
    let Cancelling { values, tracks, cancel } = app;
    if std::mem::take(cancel) {
        ui.cancel_edits(CancelReason::HostChanged);
    }
    for (value, track) in values.iter_mut().zip(tracks.iter_mut()) {
        *track = Slider::new(Bind::bound(value), 0.0..=100.0).show(ui).response.rect;
    }
}

#[test]
fn a_cancel_builds_only_the_nodes_that_run_a_gesture() {
    let mut run = Run::new(Cancelling::default(), cancelling);
    let track = run.app.tracks[0];
    let at = |f: f32| Pos2::new(track.min.x + f * track.width(), track.center().y);
    run.input(Input::mouse_move(at(0.25)));
    run.input(Input::mouse_down(at(0.25)));
    // The pointer moves with the first slider's gesture running: the cancel reaches it, and no idle slider builds.
    run.app.cancel = true;
    let cancelled = run.input(Input::mouse_move(at(0.5)));
    assert_eq!(runs(&cancelled), 1, "only the slider with the running gesture builds; the two idle sliders are kept: {cancelled:?}");
    let value = run.app.values[0];
    let after = run.input(Input::mouse_move(at(0.9)));
    assert!((run.app.values[0] - value).abs() < f64::EPSILON, "the cancelled gesture stays cancelled under its press: {after:?}");
}
