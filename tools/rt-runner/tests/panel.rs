//! The Layers panel on craft-ui-rt, headless on the recording renderer: what a frame runs, and one
//! behaviour the v1 reference checks (a range pick).

use craft_rt_runner::gesture::{Gesture, Target};
use craft_rt_runner::observe::observe_rig;
use craft_rt_runner::run::fresh_rig;
use craft_rt_runner::scenarios::gesture_steps;
use craft_ui_rt::Mods;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_test_kit::rt::Scenario;

fn row(n: u32) -> Target {
    Target::Row(format!("Layer {n}"))
}

#[test]
fn idle_runs_no_frame_and_hover_builds_nothing() {
    let (mut rig, _) = fresh_rig(RecordingRenderer::default(), 20, 1.0);
    assert_eq!(rig.settle(10), 0, "a settled panel runs no frame");
    let mut totals = (0, 0, 0);
    for step in gesture_steps("hover", &Gesture::Hover(row(3))) {
        step.inject(&mut rig);
        let report = rig.frame(step.dt);
        totals = (totals.0 + report.frame.built, totals.1 + report.frame.laid_out, totals.2 + report.frame.painted);
    }
    assert_eq!((totals.0, totals.1), (0, 0), "hover rebuilds and lays out nothing");
    // The rows' own nodes only: a repaint of the whole 20-row panel is several times this.
    assert!((1..40).contains(&totals.2), "hover repaints the row entered, painted {}", totals.2);
}

#[test]
fn shift_click_picks_the_range() {
    let (mut rig, cell) = fresh_rig(RecordingRenderer::default(), 12, 1.0);
    let mut steps = gesture_steps("click", &Gesture::Click { target: row(2), mods: Mods::NONE });
    steps.extend(gesture_steps("shift-click", &Gesture::Click { target: row(5), mods: Mods::SHIFT }));
    for step in &steps {
        Scenario::run_step(step, &mut rig).expect("steps carry no expectations");
    }
    let doc = observe_rig(&rig, cell).doc;
    assert_eq!(doc.selected, ["Layer 2", "Layer 3", "Layer 4", "Layer 5"]);
}
