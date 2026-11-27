//! Scenarios: steps of injected input, each followed by one frame and a snapshot.

use craft_ui_platform_headless::Input;
use craft_ui_rt::{Renderer, StepReport, WindowId};

use super::observe::Snapshot;
use super::rig::{FRAME, Rig};

type Resolve = Box<dyn Fn(&Snapshot) -> Vec<(Option<WindowId>, Input)>>;
type Expect = Box<dyn Fn(&Snapshot) -> Result<(), String>>;

enum Inputs {
    Fixed(Vec<(Option<WindowId>, Input)>),
    /// Built from the snapshot after the previous step, so a step can name its target ("click the
    /// row called Layer 3") instead of its coordinates.
    Resolved(Resolve),
}

/// Input for one frame. `None` as a window means the rig's first window.
pub struct Step {
    pub label: String,
    inputs: Inputs,
    /// Seconds the clock advances before the frame.
    pub dt: f64,
    expect: Option<Expect>,
}

impl Step {
    /// A frame with no input.
    pub fn idle(label: &str) -> Step {
        Step { label: label.into(), inputs: Inputs::Fixed(Vec::new()), dt: FRAME, expect: None }
    }

    /// Input for the first window.
    pub fn input(label: &str, inputs: impl IntoIterator<Item = Input>) -> Step {
        Step { label: label.into(), inputs: Inputs::Fixed(inputs.into_iter().map(|i| (None, i)).collect()), dt: FRAME, expect: None }
    }

    /// Input for named windows.
    pub fn on(label: &str, inputs: impl IntoIterator<Item = (WindowId, Input)>) -> Step {
        Step { label: label.into(), inputs: Inputs::Fixed(inputs.into_iter().map(|(w, i)| (Some(w), i)).collect()), dt: FRAME, expect: None }
    }

    /// Input built from the snapshot taken after the previous step (the first step sees the
    /// snapshot of the rig as it stands). An empty list is a step without input.
    pub fn resolved(label: &str, resolve: impl Fn(&Snapshot) -> Vec<(Option<WindowId>, Input)> + 'static) -> Step {
        Step { label: label.into(), inputs: Inputs::Resolved(Box::new(resolve)), dt: FRAME, expect: None }
    }

    pub fn dt(mut self, dt: f64) -> Step {
        self.dt = dt;
        self
    }

    /// A check of the snapshot this step produced; its `Err` fails the scenario run.
    pub fn expect(mut self, check: impl Fn(&Snapshot) -> Result<(), String> + 'static) -> Step {
        self.expect = Some(Box::new(check));
        self
    }
}

impl Step {
    /// Queues the step's input on `rig`; the frame is the caller's (the bench times it).
    pub fn inject<R: Renderer>(&self, rig: &mut Rig<R>) {
        let inputs = match &self.inputs {
            Inputs::Fixed(inputs) => inputs.clone(),
            Inputs::Resolved(resolve) => resolve(&rig.snapshot()),
        };
        for (window, input) in inputs {
            rig.inject(window.unwrap_or(rig.main), input);
        }
    }
}

/// A named list of steps.
pub struct Scenario {
    pub name: String,
    pub steps: Vec<Step>,
}

/// What one step did.
#[derive(Clone, Debug)]
pub struct StepRun {
    pub label: String,
    pub report: StepReport,
    pub snapshot: Snapshot,
}

impl Scenario {
    pub fn new(name: &str, steps: Vec<Step>) -> Scenario {
        Scenario { name: name.into(), steps }
    }

    pub fn labels(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.label.clone()).collect()
    }

    /// Runs one step on `rig`: input, clock, one frame, snapshot. The step's own check, if any,
    /// runs on the snapshot.
    pub fn run_step<R: Renderer>(step: &Step, rig: &mut Rig<R>) -> Result<StepRun, String> {
        step.inject(rig);
        let report = rig.frame(step.dt);
        let snapshot = rig.snapshot();
        if let Some(check) = &step.expect {
            check(&snapshot).map_err(|e| format!("step {:?}: {e}", step.label))?;
        }
        Ok(StepRun { label: step.label.clone(), report, snapshot })
    }

    /// Runs every step on `rig`. The first failed expectation stops the run.
    pub fn run<R: Renderer>(&self, rig: &mut Rig<R>) -> Result<Vec<StepRun>, String> {
        self.steps.iter().map(|s| Scenario::run_step(s, rig)).collect()
    }
}
