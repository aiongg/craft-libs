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
use craft_behavior::PartRef;
use craft_theme::{Appearance, Contrast, Mode, TextSize, Tokens, builtins, resolve};
use craft_ui_platform_headless::{Contact, HeadlessPlatform, Input};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Driver, HitShape, Key, Mods, NodeId, Pos2, ReadBack, Rect, Renderer, RgbaImage, StepReport, Vec2, WindowId};

use std::cell::RefCell;
use std::rc::Rc;

use craft_theme::{Size, ThemeDef, ThemeId};

use super::commands::CommandBoard;
use super::node::{ComponentNode, Kept, ScopeNode};
use super::overlay::LayerScope;
use super::{EnvCells, Ui, WindowEnv, window_root_with};
use crate::contract::{Context, Id};

/// One simulated 60 Hz frame, in seconds: what [`Harness::step`] advances the clock by.
pub const FRAME: f64 = 1.0 / 60.0;

/// The most steps [`Harness::run`] takes before it reports that the app does not settle.
pub const MAX_RUN_STEPS: usize = 100;

/// The most steps [`Harness::render`] waits for a pending read-back.
const READ_BACK_TRIES: usize = 8;

/// The tokens a harness shows by default: the first builtin theme, dark, compact, standard contrast.
pub fn default_tokens() -> Tokens {
    let look = Look::default();
    resolve(&look.theme, look.appearance, None).tokens
}

/// What the tokens a harness shows resolve from: a theme and an appearance. The harness's `theme`,
/// `density`, `text_size` and `motion` setters change one part and resolve the tokens again.
#[derive(Clone)]
pub struct Look {
    /// The theme.
    pub theme: ThemeDef,
    /// Mode, contrast, density, text size and reduced motion.
    pub appearance: Appearance,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            theme: builtins()[0].clone(),
            appearance: Appearance {
                mode: Mode::Dark,
                contrast: Contrast::Standard,
                size: Size::Compact,
                text_size: TextSize::Small,
                reduced_motion: false,
            },
        }
    }
}

impl Look {
    fn tokens(&self) -> Tokens {
        resolve(&self.theme, self.appearance, None).tokens
    }
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
    /// A stand-in for a component with no access node: the area it shows, with no id of its own.
    pub fn area(bounds: Rect) -> Target {
        Target { id: accesskit::NodeId(0), role: Role::Unknown, label: String::new(), value: None, bounds, focused: false }
    }

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
    look: Look,
    tokens: Option<Tokens>,
    direction: craft_i18n::Direction,
    motion: bool,
    commands: Option<CommandBoard>,
}

impl Default for HarnessBuilder {
    fn default() -> Self {
        HarnessBuilder {
            size: Vec2::new(400.0, 300.0),
            scale: 1.0,
            look: Look::default(),
            tokens: None,
            direction: craft_i18n::Direction::Ltr,
            motion: false,
            commands: None,
        }
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
        self.tokens = Some(tokens);
        self
    }

    /// The theme and appearance the tokens resolve from ([`Look::default`] otherwise). Later
    /// `Harness::theme`, `density` and `text_size` calls change a part of it.
    pub fn with_look(mut self, look: Look) -> Self {
        self.look = look;
        self.tokens = None;
        self
    }

    /// The app's command board: the window root runs the command a key no node took is bound to.
    pub fn with_commands(mut self, board: CommandBoard) -> Self {
        self.commands = Some(board);
        self
    }

    /// The layout direction.
    pub fn with_direction(mut self, direction: craft_i18n::Direction) -> Self {
        self.direction = direction;
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
        let tokens = self.tokens.unwrap_or_else(|| self.look.tokens());
        let mut runtime = craft_ui_rt::Runtime::new(tokens);
        runtime.set_inline_root(window);
        let token_cell = runtime.env(window).tokens;
        let cells = EnvCells::in_store(runtime.store_mut(), token_cell);
        cells.motion.set(runtime.store_mut(), if self.motion { MotionMode::Full } else { MotionMode::Off });
        cells.direction.set(runtime.store_mut(), self.direction);
        cells.density.set(runtime.store_mut(), self.look.appearance.size);
        // The view's `Ui` is read once, in the first build, for the harness's `ctx`.
        let ctx_seen: Rc<RefCell<Option<Context>>> = Rc::default();
        let seen = ctx_seen.clone();
        let mut view = view;
        let view = move |ui: &mut Ui<'_, '_>, state: &mut S| {
            if seen.borrow().is_none() {
                *seen.borrow_mut() = Some(ui.ctx().clone());
            }
            view(ui, state);
        };
        let mut driver = Driver::new(runtime, platform, renderer);
        driver.runtime.set_access_enabled(window, true);
        Harness {
            driver,
            window,
            state,
            view: Box::new(view),
            cells,
            held: Mods::NONE,
            rebuild: false,
            last: StepReport::default(),
            steps: 0,
            look: self.look,
            ctx: ctx_seen,
            commands: self.commands,
            host: None,
        }
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
    look: Look,
    ctx: Rc<RefCell<Option<Context>>>,
    commands: Option<CommandBoard>,
    /// The app host provided above the window root, as an app's host provides it (language, formats, jobs).
    host: Option<std::sync::Arc<craft_host::Host>>,
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
    /// The renderer, once the test is done with the window: a run that renders many stories keeps one device.
    pub fn into_renderer(self) -> R {
        self.driver.renderer
    }

    /// The window the view runs in.
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// The app's state.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// The app's state to change, with no rebuild: for state the view does not read (a record of outputs).
    pub fn state_mut_quiet(&mut self) -> &mut S {
        &mut self.state
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

    /// Gives the window `host` (its language, formats and jobs) as an app host provides it above every window root;
    /// the root builds in the next step.
    pub fn host(&mut self, host: std::sync::Arc<craft_host::Host>) -> &mut Self {
        self.host = Some(host);
        self.rebuild = true;
        self
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
        let host = WindowEnv { cells: Some(self.cells), jobs: None, commands: self.commands.clone() };
        let app_host = self.host.clone();
        let report = self.driver.step_with(now, &mut |_, cx| {
            if let Some(app_host) = &app_host {
                cx.provide(app_host.clone());
            }
            window_root_with(cx, host.clone(), |ui| view(ui, state))
        });
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

    /// The node with access id `id`, when the tree has it.
    pub fn target(&self, id: accesskit::NodeId) -> Option<Target> {
        self.nodes().into_iter().find(|node| node.id == id)
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

// ----- the look the tokens resolve from, the room and the scale

impl<S, R: Renderer> Harness<S, R> {
    /// The theme and appearance the shown tokens resolve from.
    pub fn look(&self) -> &Look {
        &self.look
    }

    fn apply_look(&mut self) -> &mut Self {
        let tokens = self.look.tokens();
        self.driver.runtime.set_tokens(tokens);
        let density = self.look.appearance.size;
        self.cells.density.set(self.driver.runtime.store_mut(), density);
        self
    }

    /// Shows `look`.
    pub fn set_look(&mut self, look: Look) -> &mut Self {
        self.look = look;
        self.apply_look()
    }

    /// Light or dark, and standard or high contrast.
    pub fn theme(&mut self, mode: Mode, contrast: Contrast) -> &mut Self {
        self.look.appearance.mode = mode;
        self.look.appearance.contrast = contrast;
        self.apply_look()
    }

    /// The theme: one of the built-ins (`craft.adwaita`). A panic names the id when there is none.
    pub fn theme_id(&mut self, id: ThemeId) -> &mut Self {
        match builtins().iter().find(|theme| theme.id == id) {
            Some(theme) => self.look.theme = theme.clone(),
            None => panic!("no built-in theme {id:?}"),
        }
        self.apply_look()
    }

    /// The density preset.
    pub fn density(&mut self, density: Size) -> &mut Self {
        self.look.appearance.size = density;
        self.apply_look()
    }

    /// The text-size step.
    pub fn text_size(&mut self, text_size: TextSize) -> &mut Self {
        self.look.appearance.text_size = text_size;
        self.apply_look()
    }

    /// The layout direction.
    pub fn direction(&mut self, direction: craft_i18n::Direction) -> &mut Self {
        self.cells.direction.set(self.driver.runtime.store_mut(), direction);
        self
    }

    /// The app's root values, as `App::frame` sets them through `EnvCells::set_root`.
    pub fn root_env(&mut self, root: &crate::runtime::env::RootEnv) -> &mut Self {
        self.cells.set_root(self.driver.runtime.store_mut(), root);
        self
    }

    /// The motion mode (the egui fixture's name; [`Harness::set_motion`] takes a flag).
    pub fn motion(&mut self, motion: MotionMode) -> &mut Self {
        self.cells.motion.set(self.driver.runtime.store_mut(), motion);
        self
    }

    /// The window's size in logical points, from the next step on.
    pub fn room(&mut self, size: Vec2) -> &mut Self {
        let window = self.window;
        self.driver.platform.inject(window, Input::Resize(size));
        self
    }

    /// The window's pixels per logical point, from the next step on.
    pub fn pixels_per_point(&mut self, scale: f32) -> &mut Self {
        let window = self.window;
        self.driver.platform.inject(window, Input::Scale(scale));
        self
    }

    /// Gives app-scope `command` this state, as an app's command states do; the nodes that read it
    /// run in the next step. A panic when the harness has no command board
    /// ([`HarnessBuilder::with_commands`]).
    pub fn command_state(&mut self, command: impl Into<craft_keymap::CommandRef>, state: crate::runtime::commands::CommandState) -> &mut Self {
        let Some(board) = self.commands.clone() else { panic!("the harness has no command board: build it with `with_commands`") };
        board.set_state(self.driver.runtime.store_mut(), command, state);
        self
    }

    /// Whether app-scope `command` may fire: enabled in the command board. `true` with no board.
    pub fn command_enabled(&self, command: impl Into<craft_keymap::CommandRef>) -> bool {
        match &self.commands {
            Some(board) => board.enabled(&command.into()),
            None => true,
        }
    }

    /// Fires app-scope `command` from `source` now, as automation does, and steps once; whether a handler ran. A
    /// panic when the harness has no command board.
    pub fn fire(&mut self, command: impl Into<craft_keymap::CommandRef>, source: crate::runtime::commands::FireSource) -> bool {
        let Some(board) = self.commands.clone() else { panic!("the harness has no command board: build it with `with_commands`") };
        let time = self.driver.platform.now();
        let mut fire = crate::contract::rt::commands::OutsideFire { rt: &mut self.driver.runtime, window: self.window };
        let ran = board.fire(&mut fire, command.into(), source, time);
        self.step();
        ran
    }

    /// The operating system the keys and chrome follow.
    pub fn os(&mut self, os: craft_host::target::Os) -> &mut Self {
        self.driver.runtime.set_os(os);
        self
    }

    /// The window's context handle, as the view's `ui.ctx()` gives it. A panic before the first
    /// step, which is the first build.
    pub fn ctx(&self) -> Context {
        match self.ctx.borrow().as_ref() {
            Some(ctx) => ctx.clone(),
            None => panic!("the view has not run yet: step the harness before asking for its context"),
        }
    }

    /// The system cursor the last step asked for.
    pub fn cursor_icon(&self) -> craft_ui_rt::Cursor {
        self.driver.platform.cursor(self.window)
    }

    /// The texts the last step put on the clipboard (the latest one, when it put several).
    pub fn copied(&self) -> Option<String> {
        self.driver.platform.clipboard_text().map(str::to_owned)
    }

    /// `input` as one platform event, then steps until the runtime asks for no further frame: what
    /// an app runs for an event whose effects take more than one frame (the egui fixture's
    /// `frame_through`).
    pub fn frame_through(&mut self, input: Input) -> &mut Self {
        self.send(input);
        self.run()
    }

    /// Steps until the runtime asks for no further frame (the egui fixture's `run_injected`: input
    /// reaches the runtime at event time, so nothing is left to inject after a step).
    pub fn run_injected(&mut self) -> &mut Self {
        self.run()
    }

    /// The press of `key` that types `text`, as a keyboard sends it: the key and its text, then
    /// the release.
    pub fn type_key(&mut self, key: Key, text: &str) -> &mut Self {
        let mods = self.held;
        self.inject(Input::Key { key, mods, down: true });
        self.inject(Input::Text(text.to_owned()));
        self.step();
        self.inject(Input::Key { key, mods, down: false });
        self.step()
    }

    /// The first half of a drag that pauses: press at `from`, move by `by` in four steps, and one
    /// step with the pointer still, button held. A test reads the running gesture, presses Escape
    /// or checks a snap line, then ends it with [`Harness::release_at`].
    pub fn drag_hold_at(&mut self, from: Pos2, by: Vec2) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Move, from).pointer(craft_ui_rt::PointerPhase::Down, from);
        for step in 1..=4 {
            self.pointer(craft_ui_rt::PointerPhase::Move, from + by * (step as f32 / 4.0));
        }
        self.step()
    }

    /// Releases the button at `at`, then a step with no input.
    pub fn release_at(&mut self, at: Pos2) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Up, at).step()
    }

    /// A drag from `from` by `by` that pauses before the release: [`Harness::drag_hold_at`],
    /// `halt` with the gesture running, then [`Harness::release_at`].
    pub fn drag_at_pausing(&mut self, from: Pos2, by: Vec2, halt: impl FnOnce(&mut Self)) -> &mut Self {
        self.drag_hold_at(from, by);
        halt(self);
        self.release_at(from + by)
    }

    /// The modifiers [`Harness::hold`] keeps.
    pub fn held(&self) -> Mods {
        self.held
    }
}

// ----- the runtime's own records: kept nodes, hit shapes, focus stops

impl<S, R: Renderer> Harness<S, R> {
    fn kept(&self, node: NodeId) -> Option<&Kept> {
        let rt = &self.driver.runtime;
        if let Some(kept) = rt.local::<ComponentNode>(node) {
            return Some(kept);
        }
        if let Some(kept) = rt.local::<ScopeNode>(node) {
            return Some(kept);
        }
        rt.local::<LayerScope>(node).map(|layer| &layer.kept)
    }

    /// Every node of the window's tree, parents first, layers after the nodes they float over.
    fn walk(&self) -> Vec<NodeId> {
        let rt = &self.driver.runtime;
        let mut out = Vec::new();
        let mut pending: Vec<NodeId> = rt.window_node(self.window).into_iter().collect();
        while let Some(node) = pending.pop() {
            out.push(node);
            pending.extend(rt.children(node).iter().rev().copied());
        }
        out
    }

    /// The ui-rt node of the component `id` shows; `None` when no kept node has that id.
    pub(crate) fn rt_node(&self, id: Id) -> Option<NodeId> {
        self.walk().into_iter().find(|&node| self.kept(node).is_some_and(|kept| kept.id == id))
    }

    /// The access node of the component `id` shows, as a [`Target`]; `None` when it has none.
    pub fn node_of(&self, id: Id) -> Option<Target> {
        let node = self.rt_node(id)?;
        self.nodes_of(|entry| entry.owner == node && entry.sub.is_none()).into_iter().next()
    }

    /// The access sub-node of `part` below the component `component`.
    pub fn part(&self, component: Id, part: impl Into<PartRef>) -> Option<Target> {
        let node = self.rt_node(component)?;
        let sub = super::node::part_id(part.into());
        self.nodes_of(|entry| entry.owner == node && entry.sub == Some(sub)).into_iter().next()
    }

    fn nodes_of(&self, keep: impl Fn(&craft_ui_rt::AccessEntry<'_>) -> bool) -> Vec<Target> {
        let focus = self.driver.platform.updates_for(self.window).last().map(|update| update.focus);
        self.driver
            .runtime
            .access_snapshot(self.window)
            .into_iter()
            .filter(|entry| keep(entry))
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

    /// The window rect of the component `id` shows.
    pub fn rect_of(&self, id: Id) -> Option<Rect> {
        self.driver.runtime.window_rect(self.rt_node(id)?)
    }

    /// The window rect of the part `part` of the component `owner` as its kept hit shape gives it,
    /// for parts that have no access node (pointer-only ones). Callers assert how rects relate
    /// (inside, below), not their size.
    pub fn part_rect(&self, owner: Id, part: impl Into<PartRef>) -> Option<Rect> {
        let part = part.into();
        let node = self.rt_node(owner)?;
        let origin = self.driver.runtime.window_rect(node)?.min.to_vec2();
        let kept = self.kept(node)?.parts.iter().find(|kept| kept.part == part)?;
        Some(match kept.shape {
            HitShape::Rect(rect) | HitShape::RoundedRect(rect, _) => rect.translate(origin),
            HitShape::Circle(center, radius) => Rect::from_center_size(center, Vec2::splat(radius * 2.0)).translate(origin),
        })
    }

    /// How far a press may move and still be a click, in points: the shown tokens' slop.
    pub fn slop(&self) -> f32 {
        let rt = &self.driver.runtime;
        rt.store().peek(rt.env(self.window).tokens).size.hit.slop
    }

    /// The window rect of the part `part` of a component shown inside `owner` (a colour area's point
    /// picker inside the subject), as its kept hit shape gives it; `None` when no node under `owner`
    /// has such a part sensing `sense`.
    pub fn child_part_rect(&self, owner: Id, part: PartRef, sense: craft_behavior::Sense) -> Option<Rect> {
        let rt = &self.driver.runtime;
        let mut pending = vec![self.rt_node(owner)?];
        while let Some(node) = pending.pop() {
            if let Some(kept) = self.kept(node)
                && let Some(found) = kept.parts.iter().find(|kept| kept.part == part && kept.sense.intersects(sense))
            {
                let origin = rt.window_rect(node)?.min.to_vec2();
                return Some(match found.shape {
                    HitShape::Rect(rect) | HitShape::RoundedRect(rect, _) => rect.translate(origin),
                    HitShape::Circle(center, radius) => Rect::from_center_size(center, Vec2::splat(radius * 2.0)).translate(origin),
                });
            }
            pending.extend(rt.children(node).iter().rev().copied());
        }
        None
    }

    /// Whether a component holds a press: some node of the window is on the path of a pressed shape.
    pub fn press_captured(&self) -> bool {
        let rt = &self.driver.runtime;
        self.walk().into_iter().any(|node| rt.interaction(node).pressed)
    }

    /// Whether the part `part` of the component `owner` senses `sense`.
    pub fn part_senses(&self, owner: Id, part: PartRef, sense: craft_behavior::Sense) -> bool {
        self.sensing_parts(owner, sense).contains(&part)
    }

    /// Every kept part of the window that senses a press, with its component and window rect, in tree order.
    pub fn press_parts(&self) -> Vec<(Id, PartRef, Rect)> {
        let rt = &self.driver.runtime;
        let sense = craft_behavior::Sense::CLICK | craft_behavior::Sense::DRAG;
        let mut out = Vec::new();
        for node in self.walk() {
            let (Some(kept), Some(rect)) = (self.kept(node), rt.window_rect(node)) else { continue };
            for part in kept.parts.iter().filter(|part| part.sense.intersects(sense)) {
                let origin = rect.min.to_vec2();
                let bounds = match part.shape {
                    HitShape::Rect(r) | HitShape::RoundedRect(r, _) => r.translate(origin),
                    HitShape::Circle(c, radius) => Rect::from_center_size(c, Vec2::splat(radius * 2.0)).translate(origin),
                };
                out.push((kept.id, part.part, bounds));
            }
        }
        out
    }

    /// The first part of the component `owner` that senses a drag.
    pub fn drag_part(&self, owner: Id) -> Option<PartRef> {
        self.sensing_parts(owner, craft_behavior::Sense::DRAG).into_iter().next()
    }

    /// The first part of the component `owner` that senses a click: what a click activates when
    /// the component has parts of its own.
    pub fn click_part(&self, owner: Id) -> Option<PartRef> {
        self.sensing_parts(owner, craft_behavior::Sense::CLICK).into_iter().next()
    }

    /// The parts of the component `owner` that sense any of `wanted`, in declaration order.
    pub fn sensing_parts(&self, owner: Id, wanted: craft_behavior::Sense) -> Vec<PartRef> {
        let Some(kept) = self.rt_node(owner).and_then(|node| self.kept(node)) else { return Vec::new() };
        kept.parts.iter().filter(|part| part.sense.intersects(wanted)).map(|part| part.part).collect()
    }

    /// The component with key focus and, when the focus is on one of its parts, the part. The
    /// runtime's focus is per node, so the part is `None` until a part holds a stop of its own.
    pub fn focused_stop(&self) -> Option<(Id, Option<PartRef>)> {
        let node = self.driver.runtime.focused(self.window)?;
        Some((self.kept(node)?.id, None))
    }

    /// The keys the focused stop takes, as its component declared them.
    pub fn focused_keys(&self) -> Option<crate::runtime::focus::KeyUse> {
        let node = self.driver.runtime.focused(self.window)?;
        self.kept(node)?.focus.stop
    }

    /// The window rects of the stops only a press reaches (read-only text, selection parts), in
    /// tree order: nodes that declare a stop and that Tab does not visit.
    pub fn press_only_stops(&self) -> Vec<Rect> {
        let rt = &self.driver.runtime;
        let tabbed = rt.focus_order(self.window);
        self.walk()
            .into_iter()
            .filter(|node| !tabbed.contains(node) && self.kept(*node).is_some_and(|kept| kept.focus.stop.is_some()))
            .filter_map(|node| rt.window_rect(node))
            .collect()
    }

    /// The nodes of the window that declare a focus stop, in tree order, as component ids.
    pub fn stops(&self) -> Vec<Id> {
        self.walk().into_iter().filter_map(|node| self.kept(node).filter(|kept| kept.focus.stop.is_some()).map(|kept| kept.id)).collect()
    }

    /// The piece of `target` the window shows, where a press on it lands; `None` when none of it
    /// is inside the window.
    pub fn visible(&self, target: &Target) -> Option<Target> {
        let window = Rect::from_min_size(Pos2::ZERO, self.driver.platform.window_state(self.window)?.size);
        let bounds = target.bounds.intersect(window);
        (bounds.width() >= 0.0 && bounds.height() >= 0.0).then(|| Target { bounds, ..target.clone() })
    }
}

/// A scrolling region: the view it shows and the content behind it, in window points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollRegion {
    /// The visible part.
    pub view: Rect,
    /// The whole content, as laid out.
    pub content: Rect,
}

/// An open layer of the window: a popup, a menu, a dialog or a tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerInfo {
    /// The component that opened it.
    pub id: Id,
    /// The key its owner gave it.
    pub key: u64,
    /// Its window rect.
    pub rect: Rect,
    /// It is a tooltip, which never takes input.
    pub tip: bool,
    /// The focus policy of its scope: whether it traps Tab and what Escape does.
    pub policy: crate::runtime::focus::FocusPolicy,
}

impl<S, R: Renderer> Harness<S, R> {
    /// The window's open layers, bottom first.
    pub fn layers(&self) -> Vec<LayerInfo> {
        let rt = &self.driver.runtime;
        self.walk()
            .into_iter()
            .filter_map(|node| {
                let scope = rt.local::<LayerScope>(node)?;
                Some(LayerInfo {
                    id: scope.kept.id,
                    key: scope.key,
                    rect: rt.window_rect(node)?,
                    tip: scope.surface == super::overlay::Surface::Tip,
                    policy: scope.kept.focus.scope,
                })
            })
            .collect()
    }

    /// The window's scroll regions, outermost first.
    pub fn scroll_regions(&self) -> Vec<ScrollRegion> {
        let rt = &self.driver.runtime;
        self.walk()
            .into_iter()
            .filter(|node| rt.local::<super::scroll::ScrollNode>(*node).is_some())
            .filter_map(|node| {
                let view = rt.window_rect(node)?;
                let content = rt.children(node).first().and_then(|child| rt.window_rect(*child))?;
                Some(ScrollRegion { view, content })
            })
            .collect()
    }

    /// The topmost open layer.
    pub fn top_layer(&self) -> Option<LayerInfo> {
        self.layers().pop()
    }

    /// A wheel step of `delta` points with the pointer at `pos`, one step.
    pub fn wheel_at(&mut self, pos: Pos2, delta: Vec2) -> &mut Self {
        self.pointer(craft_ui_rt::PointerPhase::Move, pos);
        let mods = self.held;
        self.send(Input::Wheel { pos, delta, mods })
    }

    /// Pasted `text`, as the platform's clipboard delivers it, one step.
    pub fn paste(&mut self, text: &str) -> &mut Self {
        self.send(Input::Paste(text.to_owned()))
    }

    /// An input method's preedit `text`, one step.
    pub fn ime_preedit(&mut self, text: &str) -> &mut Self {
        self.send(Input::ImePreedit { text: text.to_owned(), cursor: None })
    }

    /// An input method's commit of `text`, one step.
    pub fn ime_commit(&mut self, text: &str) -> &mut Self {
        self.send(Input::ImeCommit(text.to_owned()))
    }

    /// The window loses (`false`) or gains (`true`) the system's focus, one step.
    pub fn window_focus(&mut self, focused: bool) -> &mut Self {
        self.send(Input::Focus(focused))
    }
}
