//! The test `Harness` on craft-ui-rt and the headless platform (docs/contract-rt.md section 12).
//!
//! It runs a view (`FnMut(&mut Ui, &mut S)`) as the inline root of one window, steps the driver on
//! the platform's clock and drives the app the way a user does: pointer, keys and text go in as
//! platform input, and nodes are found in the access tree the runtime keeps, never through
//! internals. The clock moves only through [`Harness::step`] and its relatives, so a test decides
//! what time it is. Motion is off unless the test opts in with [`HarnessBuilder::with_motion`]: the
//! window's motion-mode env cell is `Off`, so a tween ends in the frame that starts it.
//!
//! Names follow the kittest subset the egui-backed tests use (`state`, `state_mut`, `step`,
//! `run_steps`, `run`, `get_by_label`, `query_by_label`, `query_*_by_role`, `key_press`, `render`,
//! `click`) and the egui `Fixture`'s (`by_label`, `by_role`, `press`, `release`, `drag`, `hold`,
//! `key`, `type_text`).

use accesskit::Role;
use craft_behavior::MotionMode;
use craft_theme::{Appearance, Contrast, Mode, TextSize, Tokens, builtins, resolve};
use craft_ui_platform_headless::{Contact, HeadlessPlatform, Input};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Driver, Key, Mods, Pos2, ReadBack, Rect, Renderer, RgbaImage, StepReport, Vec2, WindowId};

use super::{EnvCells, Ui, WindowEnv, window_root_with};

/// One simulated 60 Hz frame, in seconds: what [`Harness::step`] advances the clock by.
pub const FRAME: f64 = 1.0 / 60.0;

/// The most steps [`Harness::run`] takes before it reports that the app does not settle.
pub const MAX_RUN_STEPS: usize = 100;

/// The most steps [`Harness::render`] waits for a pending read-back.
const READ_BACK_TRIES: usize = 8;

/// The tokens a harness shows by default: the first builtin theme, dark, compact, standard contrast.
pub fn default_tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

/// A node of the access tree, as input addresses it.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    /// The node's id in the window's access tree.
    pub id: accesskit::NodeId,
    /// The node's role.
    pub role: Role,
    /// The node's name; empty when it has none.
    pub label: String,
    /// Its value text, for fields and sliders.
    pub value: Option<String>,
    /// Window coordinates, logical points.
    pub bounds: Rect,
    /// Whether it has key focus.
    pub focused: bool,
}

impl Target {
    /// The middle of the node's bounds, where a click lands.
    pub fn center(&self) -> Pos2 {
        self.bounds.center()
    }

    /// The bounds, as the egui `Target` named them.
    pub fn rect(&self) -> Rect {
        self.bounds
    }
}

/// What a harness shows and how: set before [`HarnessBuilder::build_ui_state`].
pub struct HarnessBuilder {
    size: Vec2,
    scale: f32,
    tokens: Tokens,
    motion: bool,
}

impl Default for HarnessBuilder {
    fn default() -> Self {
        HarnessBuilder { size: Vec2::new(400.0, 300.0), scale: 1.0, tokens: default_tokens(), motion: false }
    }
}

impl HarnessBuilder {
    /// The window's size in logical points.
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    /// The window's pixels per logical point.
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// The tokens the window shows ([`default_tokens`] otherwise).
    pub fn with_tokens(mut self, tokens: Tokens) -> Self {
        self.tokens = tokens;
        self
    }

    /// Whether animations take their time: off by default, where they end in the frame that starts
    /// them. With motion on, time passes only through the harness's steps.
    pub fn with_motion(mut self, on: bool) -> Self {
        self.motion = on;
        self
    }

    /// A harness over `state`, showing `view` on a recording renderer.
    pub fn build_ui_state<S: 'static>(self, view: impl FnMut(&mut Ui<'_, '_>, &mut S) + 'static, state: S) -> Harness<S> {
        self.build_with(RecordingRenderer::default(), view, state)
    }

    /// A harness over no state.
    pub fn build_ui(self, mut view: impl FnMut(&mut Ui<'_, '_>) + 'static) -> Harness<()> {
        self.build_ui_state(move |ui, ()| view(ui), ())
    }

    /// [`HarnessBuilder::build_ui_state`] on `renderer`, for a test that reads pixels.
    pub fn build_with<S: 'static, R: Renderer>(self, renderer: R, view: impl FnMut(&mut Ui<'_, '_>, &mut S) + 'static, state: S) -> Harness<S, R> {
        let window = HeadlessPlatform::MAIN;
        let platform = HeadlessPlatform::with_window(self.size, self.scale);
        let mut runtime = craft_ui_rt::Runtime::new(self.tokens);
        runtime.set_inline_root(window);
        let tokens = runtime.env(window).tokens;
        let cells = EnvCells::in_store(runtime.store_mut(), tokens);
        cells.motion.set(runtime.store_mut(), if self.motion { MotionMode::Full } else { MotionMode::Off });
        let mut driver = Driver::new(runtime, platform, renderer);
        driver.runtime.set_access_enabled(window, true);
        Harness { driver, window, state, view: Box::new(view), cells, held: Mods::NONE, rebuild: false, last: StepReport::default(), steps: 0 }
    }
}

/// A view on the runtime, the headless platform and a renderer, stepped by the test.
pub struct Harness<S = (), R: Renderer = RecordingRenderer> {
    /// `driver.platform` is the headless platform: injection, the clock, cursor, clipboard.
    pub driver: Driver<HeadlessPlatform, R>,
    window: WindowId,
    state: S,
    view: Box<dyn FnMut(&mut Ui<'_, '_>, &mut S)>,
    cells: EnvCells,
    held: Mods,
    /// The app's own state moved: the root builds in the next step.
    rebuild: bool,
    last: StepReport,
    steps: usize,
}

impl Harness<(), RecordingRenderer> {
    /// A builder for a harness with another size, scale, tokens or motion.
    pub fn builder() -> HarnessBuilder {
        HarnessBuilder::default()
    }

    /// A harness over no state, with the default size and tokens.
    pub fn new_ui(view: impl FnMut(&mut Ui<'_, '_>) + 'static) -> Harness<()> {
        HarnessBuilder::default().build_ui(view)
    }
}

impl<S: 'static> Harness<S, RecordingRenderer> {
    /// A harness over `state`, with the default size and tokens.
    pub fn new_ui_state(view: impl FnMut(&mut Ui<'_, '_>, &mut S) + 'static, state: S) -> Harness<S> {
        HarnessBuilder::default().build_ui_state(view, state)
    }
}

impl<S, R: Renderer> Harness<S, R> {
    /// The window the view runs in.
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// The app's state.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// The app's state to change. The view builds again in the next step, since the harness cannot
    /// tell which part the view reads.
    pub fn state_mut(&mut self) -> &mut S {
        self.rebuild = true;
        &mut self.state
    }

    /// The view builds again in the next step (state it reads through a handle moved).
    pub fn rebuild(&mut self) -> &mut Self {
        self.rebuild = true;
        self
    }

    /// Shows other tokens.
    pub fn set_tokens(&mut self, tokens: Tokens) -> &mut Self {
        self.driver.runtime.set_tokens(tokens);
        self
    }

    /// Turns motion on or off for the steps that follow.
    pub fn set_motion(&mut self, on: bool) -> &mut Self {
        let mode = if on { MotionMode::Full } else { MotionMode::Off };
        self.cells.motion.set(self.driver.runtime.store_mut(), mode);
        self
    }

    /// The frame time in seconds.
    pub fn now(&self) -> f64 {
        self.driver.platform.now()
    }

    /// What the last step did.
    pub fn last_report(&self) -> StepReport {
        self.last
    }

    /// Steps taken so far.
    pub fn steps_taken(&self) -> usize {
        self.steps
    }

    /// One turn at the platform's current time: the clock does not move.
    pub fn step_now(&mut self) -> StepReport {
        if std::mem::take(&mut self.rebuild) {
            self.driver.runtime.mark_root(self.window);
        }
        let now = self.driver.platform.now();
        let (state, view) = (&mut self.state, &mut self.view);
        let host = WindowEnv { cells: Some(self.cells), jobs: None, commands: None };
        let report = self.driver.step_with(now, &mut |_, cx| window_root_with(cx, host.clone(), |ui| view(ui, state)));
        self.last = report;
        self.steps += 1;
        report
    }

    /// Advances the clock by `dt` seconds, then one turn.
    pub fn step_dt(&mut self, dt: f64) -> StepReport {
        self.driver.platform.advance(dt);
        self.step_now()
    }

    /// One turn, [`FRAME`] on.
    pub fn step(&mut self) -> &mut Self {
        self.step_dt(FRAME);
        self
    }

    /// `n` turns, [`FRAME`] apart.
    pub fn run_steps(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            self.step_dt(FRAME);
        }
        self
    }

    /// Steps until the runtime asks for no further frame; the number of steps taken. Wakes at a
    /// later time (a caret's blink) do not keep it going: [`Harness::run_to_wake`] takes those.
    /// `Err` with the limit when the app still asks for frames after [`MAX_RUN_STEPS`].
    pub fn try_run(&mut self) -> Result<usize, usize> {
        self.driver.platform.take_frame_request();
        for taken in 1..=MAX_RUN_STEPS {
            self.step_dt(FRAME);
            if !self.driver.platform.take_frame_request() {
                return Ok(taken);
            }
        }
        Err(MAX_RUN_STEPS)
    }

    /// [`Harness::try_run`]; panics when the app does not settle.
    pub fn run(&mut self) -> &mut Self {
        if let Err(limit) = self.try_run() {
            panic!("the app still asks for frames after {limit} steps");
        }
        self
    }

    /// Moves the clock to the time the runtime asked to be woken at, exactly, and steps there.
    /// `None` when no wake is pending.
    pub fn run_to_wake(&mut self) -> Option<StepReport> {
        self.driver.platform.advance_to_wake()?;
        Some(self.step_now())
    }

    /// [`Harness::run`], then each pending wake in turn (an animation's frames) until none is
    /// left; the steps taken. `Err` after [`MAX_RUN_STEPS`], as for an animation that never ends.
    pub fn try_run_to_rest(&mut self) -> Result<usize, usize> {
        let mut taken = self.try_run()?;
        while self.run_to_wake().is_some() {
            taken += 1;
            if taken > MAX_RUN_STEPS {
                return Err(MAX_RUN_STEPS);
            }
            taken += self.try_run()?;
        }
        Ok(taken)
    }

    /// [`Harness::try_run_to_rest`]; panics when the app never comes to rest.
    pub fn run_to_rest(&mut self) -> &mut Self {
        if let Err(limit) = self.try_run_to_rest() {
            panic!("the app is still animating after {limit} steps");
        }
        self
    }

    /// `n` steps with no input (the egui fixture's name).
    pub fn idle(&mut self, n: usize) -> &mut Self {
        self.run_steps(n)
    }

    /// [`Harness::run`] (the egui fixture's name).
    pub fn settle(&mut self) -> &mut Self {
        self.run()
    }

    /// Moves the clock by `seconds` and steps once (the egui fixture's name).
    pub fn advance(&mut self, seconds: f64) -> &mut Self {
        self.step_dt(seconds);
        self
    }

    // ----- access tree

    /// Every node of the access tree with a role or a name, in tree order.
    pub fn nodes(&self) -> Vec<Target> {
        let focus = self.driver.platform.updates_for(self.window).last().map(|u| u.focus);
        self.driver
            .runtime
            .access_snapshot(self.window)
            .into_iter()
            .map(|entry| Target {
                id: entry.id,
                role: entry.node.role(),
                label: entry.node.label().unwrap_or_default().to_owned(),
                value: entry.node.value().map(str::to_owned),
                bounds: entry.bounds,
                focused: Some(entry.id) == focus,
            })
            .collect()
    }

    /// The nodes named `label`, in tree order.
    pub fn query_all_by_label(&self, label: &str) -> Vec<Target> {
        self.nodes().into_iter().filter(|n| n.label == label).collect()
    }

    /// The node named `label`; `None` when there is none, a panic when there are several.
    pub fn query_by_label(&self, label: &str) -> Option<Target> {
        one(self.query_all_by_label(label), &format!("label {label:?}"))
    }

    /// The node named `label`; a panic naming the labels there are when none, or several, match.
    pub fn get_by_label(&self, label: &str) -> Target {
        match self.query_by_label(label) {
            Some(t) => t,
            None => panic!("no node named {label:?}; the access tree has {:?}", self.labels()),
        }
    }

    /// The first node named `label` (the egui fixture's rule: a repeated label is the first on screen).
    pub fn by_label(&self, label: &str) -> Option<Target> {
        self.query_all_by_label(label).into_iter().next()
    }

    /// The nodes of `role`, in tree order.
    pub fn query_all_by_role(&self, role: Role) -> Vec<Target> {
        self.nodes().into_iter().filter(|n| n.role == role).collect()
    }

    /// The node of `role`; `None` when there is none, a panic when there are several.
    pub fn query_by_role(&self, role: Role) -> Option<Target> {
        one(self.query_all_by_role(role), &format!("role {role:?}"))
    }

    /// The node of `role`; a panic when none or several match.
    pub fn get_by_role(&self, role: Role) -> Target {
        match self.query_by_role(role) {
            Some(t) => t,
            None => panic!("no node of role {role:?}; the access tree has {:?}", self.nodes().iter().map(|n| n.role).collect::<Vec<_>>()),
        }
    }

    /// The node of `role` named `label`; `None` when there is none, a panic when there are several.
    pub fn query_by_role_and_label(&self, role: Role, label: &str) -> Option<Target> {
        one(self.nodes().into_iter().filter(|n| n.role == role && n.label == label).collect(), &format!("role {role:?} and label {label:?}"))
    }

    /// The node of `role` named `label`; a panic when none or several match.
    pub fn get_by_role_and_label(&self, role: Role, label: &str) -> Target {
        match self.query_by_role_and_label(role, label) {
            Some(t) => t,
            None => panic!("no {role:?} named {label:?}; the access tree has {:?}", self.labels()),
        }
    }

    /// The nodes of `role` in tree order (the egui fixture's name).
    pub fn by_role(&self, role: Role) -> Vec<Target> {
        self.query_all_by_role(role)
    }

    /// The node with key focus.
    pub fn focused(&self) -> Option<Target> {
        self.nodes().into_iter().find(|n| n.focused)
    }

    fn labels(&self) -> Vec<String> {
        self.nodes().into_iter().filter(|n| !n.label.is_empty()).map(|n| n.label).collect()
    }

    // ----- input

    fn inject(&mut self, input: Input) {
        let window = self.window;
        self.driver.platform.inject(window, input);
    }

    fn contact(&self) -> Contact {
        Contact::MOUSE.with_mods(self.held)
    }

    fn pointer(&mut self, phase: craft_ui_rt::PointerPhase, pos: Pos2) -> &mut Self {
        let contact = self.contact();
        self.inject(Input::pointer(contact, phase, pos));
        self.step();
        self
    }

    /// The modifiers held in every following input, until the next call.
    pub fn hold(&mut self, mods: Mods) -> &mut Self {
        self.held = mods;
        self
    }

    /// Moves the pointer to the middle of `target`.
    pub fn hover(&mut self, target: &Target) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Move, target.center())
    }

    /// Hover, press and release at the middle of `target`, one step each. The step of the release
    /// is the one in which the view sees the click ([`Harness::last_report`] counts it).
    pub fn click(&mut self, target: &Target) -> &mut Self {
        self.click_at(target.center())
    }

    /// [`Harness::click`] at a point.
    pub fn click_at(&mut self, pos: Pos2) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Move, pos).pointer(craft_ui_rt::PointerPhase::Down, pos).pointer(craft_ui_rt::PointerPhase::Up, pos)
    }

    /// Hover, then press on `target`.
    pub fn press(&mut self, target: &Target) -> &mut Self {
        let at = target.center();
        self.pointer(craft_ui_rt::PointerPhase::Move, at).pointer(craft_ui_rt::PointerPhase::Down, at)
    }

    /// Release at the middle of `target`.
    pub fn release(&mut self, target: &Target) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Up, target.center())
    }

    /// Press on `target`, move by `by` in four steps, release.
    pub fn drag(&mut self, target: &Target, by: Vec2) -> &mut Self {
        self.drag_at(target.center(), by)
    }

    /// [`Harness::drag`] from a point.
    pub fn drag_at(&mut self, from: Pos2, by: Vec2) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Move, from).pointer(craft_ui_rt::PointerPhase::Down, from);
        for step in 1..=4 {
            self.pointer(craft_ui_rt::PointerPhase::Move, from + by * (step as f32 / 4.0));
        }
        self.pointer(craft_ui_rt::PointerPhase::Up, from + by)
    }

    /// A key press and release with the held modifiers, one step each.
    pub fn key_press(&mut self, key: Key) -> &mut Self {
        self.key(key, Mods::NONE)
    }

    /// A key press and release with `mods` and the held ones.
    pub fn key(&mut self, key: Key, mods: Mods) -> &mut Self {
        let mods = Mods {
            cmd: self.held.cmd || mods.cmd,
            ctrl: self.held.ctrl || mods.ctrl,
            alt: self.held.alt || mods.alt,
            shift: self.held.shift || mods.shift,
        };
        self.inject(Input::Key { key, mods, down: true });
        self.step();
        self.inject(Input::Key { key, mods, down: false });
        self.step()
    }

    /// Typed `text`, one step.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        self.inject(Input::Text(text.to_owned()));
        self.step()
    }

    /// An assistive technology's action on `target`, one step.
    pub fn access(&mut self, action: accesskit::Action, target: &Target) -> &mut Self {
        self.inject(Input::action(action, target.id));
        self.step()
    }

    /// Injects `input` into the window and steps once.
    pub fn send(&mut self, input: Input) -> &mut Self {
        self.inject(input);
        self.step()
    }

    // ----- pixels

    /// The window's last rendered frame, in physical pixels, read back from the renderer. A
    /// renderer without read-back (none is attached to the recording one's pixels beyond its
    /// coarse picture) makes this panic with the reason.
    pub fn render(&mut self) -> RgbaImage {
        let id = self.driver.renderer.request_read_back(self.window, None);
        for _ in 0..=READ_BACK_TRIES {
            match self.driver.renderer.poll_read_back(id) {
                ReadBack::Ready(image) => return image,
                ReadBack::Failed => panic!("the renderer cannot read back window {:?}", self.window),
                ReadBack::Pending => {
                    self.step_now();
                }
            }
        }
        panic!("the read-back of window {:?} stayed pending for {READ_BACK_TRIES} steps", self.window);
    }
}

fn one(mut found: Vec<Target>, what: &str) -> Option<Target> {
    if found.len() > 1 {
        panic!("{} nodes have {what}: {found:?}", found.len());
    }
    found.pop()
}
