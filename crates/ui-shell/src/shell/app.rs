//! `craft_ui::app`: the app host. It drives craft-ui-rt's `Driver::step_with` for every window of an app, on winit
//! ([`run`], [`start_web`]) or on any platform a caller brings ([`start`], the headless one in tests).
//!
//! The host owns what an app shares across its windows: one [`CommandBoard`] built from the app's keymap, one
//! [`JobBoard`] that wakes the loop through the driver's waker, the env cells, and the [`WindowRegistry`] that keeps
//! window placement. Each window's inline root is [`window_root_with`] around [`App::ui`].
//!
//! The app's code runs at four points:
//! - [`App::input`] sees each platform event before the runtime does, and may take it;
//! - [`App::logic`] runs at event time for each fired command no handler of its own took (the app scope's), so a
//!   command changes app state by writing cells and only their readers run;
//! - [`App::frame`] runs before each turn, also while every window is hidden;
//! - [`App::ui`] builds a window's content when ui-rt runs its root.
//!
//! A command fired from code inside [`App::ui`] (`Ui::fire`) cannot reach the app while its build holds it: the host
//! runs [`App::logic`] for it right after that turn.
//!
//! Closing goes through the runtime's one close path (`Runtime::request_close`): a window with a close guard
//! (`Component::close_guard`) asks it, any other window closes. The host calls [`App::exit`] once the last window is
//! gone.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use craft_behavior::placement::Defaults;
use craft_host::Jobs;
use craft_host::target::{Backend, ShellInfo};
use craft_keymap::Keymap;
use craft_theme::{Appearance, Contrast, Mode, SUITE_DEFAULT, Size, TextSize, Tokens, builtins, resolve};
use craft_ui_rt::{Driver, Platform, Renderer, Runtime, State, StepReport, Store};
use craft_window::policy::Decision;
use craft_window::restore::{MAIN, Store as PlacementStore};

use crate::contract::{Context, PlatformRequest, Ui};
use crate::runtime::commands::{CommandBoard, CommandDefError, EditCommands, Fired, linked_keymap};
use crate::runtime::env::EnvCells;
use crate::runtime::memory::RuntimeMemory;
use crate::runtime::pending::JobBoard;
use crate::runtime::windows::{WindowEnv, WindowRegistry, WindowSpec, rt_spec, window_root_with};

pub use craft_ui_rt::{PlatformEvent, PointerPhase, WindowCommand, WindowEvent, WindowId};

/// The app the host drives.
pub trait App: 'static {
    /// Sees `event` before the runtime does. Returns true to take it: the runtime never gets it.
    fn input(&mut self, _event: &PlatformEvent, _cx: &mut LogicCx<'_>) -> bool {
        false
    }

    /// Runs at event time for each fired app command (one with no handler of its own): writes the cells the command
    /// changes.
    fn logic(&mut self, _fired: &Fired, _cx: &mut LogicCx<'_>) {}

    /// Runs before each turn of the loop, also while every window is hidden and no build runs. Returns the frame time
    /// at which the app next needs a turn without input (a pending save, a retry), or `None`.
    fn frame(&mut self, _cx: &mut LogicCx<'_>) -> Option<f64> {
        None
    }

    /// Builds `window`'s content. The first window the host opens is [`WindowId::ROOT`].
    fn ui(&mut self, window: WindowId, ui: &mut Ui<'_, '_>);

    /// Runs once, when the app's last window has closed.
    fn exit(&mut self) {}
}

/// What the app's hooks reach of the host: the cells, its windows and the loop.
pub struct LogicCx<'a> {
    store: &'a mut Store,
    host: &'a Shared,
}

impl<'a> LogicCx<'a> {
    fn new(store: &'a mut Store, host: &'a Shared) -> Self {
        LogicCx { store, host }
    }

    /// The runtime's cells: write the ones the command changes, and only their readers run.
    pub fn store(&mut self) -> &mut Store {
        self.store
    }

    /// Builds `window`'s root in the next frame although no cell it reads changed: for app state the app keeps
    /// outside cells. Within an event or [`App::frame`], the build is this turn's.
    pub fn request_build(&mut self, window: WindowId) {
        let cell = self.host.builds.borrow().get(&window).copied();
        if let Some(cell) = cell {
            cell.update(self.store, |builds| *builds = builds.wrapping_add(1));
        }
    }

    /// Asks the platform to change `window` (title, size, state, close).
    pub fn window_command(&mut self, window: WindowId, command: WindowCommand) {
        self.host.actions.borrow_mut().push(Action::Window(window, command));
    }

    /// Asks every window to close through the close path: a window with a close guard asks it first.
    pub fn quit(&mut self) {
        self.host.actions.borrow_mut().push(Action::Quit);
    }

    /// The env cells every window root provides: tokens, theme base, direction, density, motion.
    pub fn env(&self) -> EnvCells {
        self.host.env
    }

    /// The job board: work spawned on it delivers into cells on the UI thread.
    pub fn jobs(&self) -> &JobBoard {
        &self.host.jobs
    }

    /// The command board: command states, and handlers of commands the app does not take in [`App::logic`].
    pub fn commands(&self) -> &CommandBoard {
        &self.host.commands
    }

    /// The frame time of the running turn, in seconds: the clock [`App::frame`]'s wake time is on.
    pub fn time(&self) -> f64 {
        self.host.time.get()
    }
}

/// What the app asked of the platform from a hook that had only the cells.
enum Action {
    Window(WindowId, WindowCommand),
    Quit,
}

/// The host's state the app's hooks reach.
struct Shared {
    /// Per window, the cell its root reads so that [`LogicCx::request_build`] builds it.
    builds: RefCell<BTreeMap<WindowId, State<u64>>>,
    actions: RefCell<Vec<Action>>,
    /// App commands fired from inside [`App::ui`], run after the turn.
    deferred: RefCell<Vec<Fired>>,
    env: EnvCells,
    jobs: JobBoard,
    commands: CommandBoard,
    time: Cell<f64>,
    /// The session and saved memory of the runtime, provided above every window root.
    memory: RuntimeMemory,
    /// Wakes the loop: what each window's `Context` asks for outside a build reaches the host's next turn.
    waker: craft_ui_rt::Waker,
    /// Each window's `Context`, made when the window first builds and provided above its root.
    contexts: RefCell<BTreeMap<WindowId, Context>>,
}

impl Shared {
    /// The context of `window`, made on first use over the runtime's memory store.
    fn context(&self, window: WindowId) -> Context {
        let mut contexts = self.contexts.borrow_mut();
        contexts.entry(window).or_insert_with(|| Context::for_window(window, Some(self.waker.clone()), self.memory.clone())).clone()
    }

    /// Hands the platform what the app asked for. Returns whether there was anything.
    fn apply(&self, runtime: &mut Runtime) -> bool {
        let actions = std::mem::take(&mut *self.actions.borrow_mut());
        let any = !actions.is_empty();
        for action in actions {
            match action {
                Action::Window(window, command) => runtime.window_command(window, command),
                Action::Quit => runtime.quit(),
            }
        }
        any
    }
}

/// Which renderer [`run`] draws with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RendererKind {
    /// epaint's tessellator on wgpu.
    #[default]
    Epaint,
    /// Skia's CPU raster (native only, with the `skia` feature).
    Skia,
}

impl RendererKind {
    /// `CRAFT_RENDERER=skia` picks Skia where this build has it; anything else epaint.
    pub fn from_env() -> RendererKind {
        let skia = cfg!(all(feature = "skia", not(target_arch = "wasm32")));
        match std::env::var("CRAFT_RENDERER") {
            Ok(name) if skia && name.eq_ignore_ascii_case("skia") => RendererKind::Skia,
            _ => RendererKind::Epaint,
        }
    }
}

/// What the host needs of the app.
#[non_exhaustive]
pub struct AppSpec {
    /// The main window's title.
    pub title: String,
    /// The app id: the Wayland app id and the name the OS keeps the app under.
    pub app_id: String,
    /// The platform the app runs on.
    pub shell_info: ShellInfo,
    /// The first window's size and the smallest window.
    pub defaults: Defaults,
    /// Where window placement is kept.
    pub placement: PlacementStore,
    /// Who draws the main window's frame (`craft_window::policy::decide_for`). Unset: the OS draws it.
    pub frame: Option<Decision>,
    /// Whether the stories' command sets (`#[command(story)]`) join the keymap: the gallery's spec sets it.
    pub stories: bool,
    /// The keymap of the app's command board. Unset: the command sets linked into the binary.
    pub keymap: Option<Keymap>,
    /// The tokens the windows open with; the app writes others through [`LogicCx::env`].
    pub tokens: Tokens,
    /// Where background jobs run.
    pub jobs: Arc<dyn Jobs>,
    /// The renderer [`run`] draws with.
    pub renderer: RendererKind,
}

impl AppSpec {
    /// A spec for `app_id` titled `title`, with the running build's shell on the backend winit opens by default, an
    /// in-memory placement store, the suite's default tokens, the default job pool and the renderer `CRAFT_RENDERER`
    /// names.
    pub fn new(app_id: impl Into<String>, title: impl Into<String>, defaults: Defaults) -> Self {
        AppSpec {
            title: title.into(),
            app_id: app_id.into(),
            shell_info: with_browser_caps(ShellInfo::default().backend(Backend::from_env())),
            defaults,
            placement: PlacementStore::in_memory(),
            frame: None,
            stories: false,
            keymap: None,
            tokens: default_tokens(),
            jobs: craft_host::default_jobs(),
            renderer: RendererKind::from_env(),
        }
    }

    /// Loads the stories' command sets too, for an app that shows stories (the gallery).
    pub fn stories(mut self) -> Self {
        self.stories = true;
        self
    }

    /// Sets the placement store.
    pub fn placement(mut self, store: PlacementStore) -> Self {
        self.placement = store;
        self
    }

    /// Sets the decision on who draws the main window's frame.
    pub fn frame(mut self, decision: Decision) -> Self {
        self.frame = Some(decision);
        self
    }

    /// Sets the shell info.
    pub fn shell_info(mut self, shell_info: ShellInfo) -> Self {
        self.shell_info = shell_info;
        self
    }

    /// Sets the command board's keymap (an app catalog, the user's overrides).
    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = Some(keymap);
        self
    }

    /// Sets the tokens the windows open with.
    pub fn tokens(mut self, tokens: Tokens) -> Self {
        self.tokens = tokens;
        self
    }

    /// Sets where background jobs run.
    pub fn jobs(mut self, jobs: Arc<dyn Jobs>) -> Self {
        self.jobs = jobs;
        self
    }

    /// Sets the renderer [`run`] draws with.
    pub fn renderer(mut self, renderer: RendererKind) -> Self {
        self.renderer = renderer;
        self
    }
}

/// The suite's default theme, dark, in its default density.
fn default_tokens() -> Tokens {
    let themes = builtins();
    let theme = themes.iter().find(|theme| theme.id.as_str() == SUITE_DEFAULT).unwrap_or(&themes[0]);
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::default(), text_size: TextSize::default(), reduced_motion: false };
    resolve(theme, appearance, None).tokens
}

/// The shell with what the running browser offers: a browser build captures a screen colour only where the browser
/// has the EyeDropper. Native capabilities are the build's own.
fn with_browser_caps(shell: ShellInfo) -> ShellInfo {
    #[cfg(target_arch = "wasm32")]
    {
        let mut shell = shell;
        if craft_platform::screen::available() {
            shell.caps.insert(craft_host::target::Caps::SCREEN_CAPTURE);
        }
        shell
    }
    #[cfg(not(target_arch = "wasm32"))]
    shell
}

/// Why the host could not start.
#[derive(Debug)]
#[non_exhaustive]
pub enum StartError {
    /// A command set's definitions are unusable, or two commands share an id.
    Commands(CommandDefError),
    /// The renderer could not start (no GPU adapter).
    Renderer(String),
    /// The platform's event loop could not start.
    EventLoop(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::Commands(error) => write!(f, "command registry: {error}"),
            StartError::Renderer(error) => write!(f, "the renderer could not start: {error}"),
            StartError::EventLoop(error) => write!(f, "the event loop could not start: {error}"),
        }
    }
}

impl std::error::Error for StartError {}

/// A spec with its keymap built: what is left of starting cannot fail.
struct Prepared {
    spec: AppSpec,
    keymap: Keymap,
}

impl Prepared {
    fn new(mut spec: AppSpec) -> Result<Prepared, StartError> {
        let keymap = match spec.keymap.take() {
            Some(keymap) => keymap,
            None => linked_keymap(spec.stories, spec.shell_info.os).map_err(StartError::Commands)?,
        };
        Ok(Prepared { spec, keymap })
    }
}

/// The running app over a driver: [`Host::step`] is one turn of the loop.
pub struct Host<A: App> {
    app: Rc<RefCell<A>>,
    shared: Rc<Shared>,
    registry: WindowRegistry,
    env: WindowEnv,
    shell: ShellInfo,
    frame: Option<Decision>,
    had_windows: bool,
    exited: bool,
}

/// Opens the main window on `platform`, builds the runtime and the driver over `renderer`, then the app with
/// `make`. The platform hands out window ids in order, so a platform with no window yet gives the main window
/// [`WindowId::ROOT`].
pub fn start<A: App, P: Platform, R: Renderer>(
    spec: AppSpec,
    platform: P,
    renderer: R,
    make: impl FnOnce(&mut LogicCx<'_>) -> A,
) -> Result<(Driver<P, R>, Host<A>), StartError> {
    Ok(start_on(Prepared::new(spec)?, platform, renderer, make, |platform, spec| platform.create_window(spec)))
}

fn start_on<A: App, P: Platform, R: Renderer>(
    prepared: Prepared,
    mut platform: P,
    renderer: R,
    make: impl FnOnce(&mut LogicCx<'_>) -> A,
    open: impl FnOnce(&mut P, &craft_ui_rt::WindowSpec) -> WindowId,
) -> (Driver<P, R>, Host<A>) {
    let Prepared { spec, keymap } = prepared;
    let registry = WindowRegistry::new(spec.placement);
    let frame = spec.frame.as_ref().map(|decision| decision.frame.clone());
    let mut main = rt_spec(&WindowSpec::document(), &spec.title, Some(MAIN), &|_| frame.clone());
    // The platform groups the app's windows and finds its icon and desktop entry by it.
    main.app_id = Some(spec.app_id.clone());
    let main = registry.place(main, &spec.defaults, platform.surfaces());
    let window = open(&mut platform, &main);
    debug_assert_eq!(window, WindowId::ROOT, "the main window is the platform's first");
    let mut runtime = Runtime::new(spec.tokens);
    runtime.set_os(spec.shell_info.os);
    runtime.set_inline_root(window);
    runtime.set_window_key(window, main.key.clone());
    let tokens = runtime.env(window).tokens;
    let env = EnvCells::in_store(runtime.store_mut(), tokens);
    // An id that is not a valid `AppId` names no theme pins.
    env.set_app(runtime.store_mut(), craft_theme::AppId::parse(spec.app_id.as_str()).ok());
    let mut driver = Driver::new(runtime, platform, renderer);
    let jobs = JobBoard::new(spec.jobs, driver.waker());
    let commands = CommandBoard::new(keymap).with_edits::<EditCommands>();
    let commands = if spec.stories { commands.with_stories() } else { commands };
    let shared = Rc::new(Shared {
        builds: RefCell::new(BTreeMap::new()),
        actions: RefCell::new(Vec::new()),
        deferred: RefCell::new(Vec::new()),
        env,
        jobs: jobs.clone(),
        commands: commands.clone(),
        time: Cell::new(0.0),
        memory: RuntimeMemory::new(),
        waker: driver.waker(),
        contexts: RefCell::new(BTreeMap::new()),
    });
    let app = Rc::new(RefCell::new(make(&mut LogicCx::new(driver.runtime.store_mut(), &shared))));

    driver.set_wake_handler(move |store, images| {
        jobs.deliver(store, images);
    });
    let (input_app, input_host) = (app.clone(), shared.clone());
    driver.set_input_filter(move |event, runtime| {
        // The OS's light or dark mode reaches the root env through the path `EnvCells::set_root` takes.
        if let PlatformEvent::Window(_, WindowEvent::Theme(theme)) = event {
            let store = runtime.store_mut();
            let mode = match theme {
                craft_ui_rt::SystemTheme::Light => Mode::Light,
                craft_ui_rt::SystemTheme::Dark => Mode::Dark,
            };
            let system = craft_theme::SystemAppearance { mode: Some(mode), ..store.peek(input_host.env.root).system };
            input_host.env.set_system(store, system);
        }
        let Ok(mut app) = input_app.try_borrow_mut() else { return false };
        let taken = app.input(event, &mut LogicCx::new(runtime.store_mut(), &input_host));
        drop(app);
        input_host.apply(runtime);
        taken
    });
    // The board holds the handler and `Shared` holds the board: the handler reaches `Shared` weakly.
    let (logic_app, logic_host): (Rc<RefCell<A>>, Weak<Shared>) = (app.clone(), Rc::downgrade(&shared));
    commands.on_app(move |fired, store| {
        let Some(host) = logic_host.upgrade() else { return };
        match logic_app.try_borrow_mut() {
            Ok(mut app) => app.logic(fired, &mut LogicCx::new(store, &host)),
            // Fired from inside `App::ui`: it runs after the turn.
            Err(_) => host.deferred.borrow_mut().push(fired.clone()),
        }
    });

    let env = WindowEnv { cells: Some(env), jobs: Some(shared.jobs.clone()), commands: Some(commands) };
    let mut host = Host { app, shared, registry, env, shell: spec.shell_info, frame: spec.frame, had_windows: false, exited: false };
    host.sync_windows(&mut driver.runtime);
    (driver, host)
}

impl<A: App> Host<A> {
    /// One turn at frame time `time`: [`App::frame`], then `Driver::step_with` over every window's root, then the app
    /// commands fired from inside the build, placement and the platform's hit test. Calls [`App::exit`] once the last
    /// window is gone.
    pub fn step<P: Platform, R: Renderer>(&mut self, driver: &mut Driver<P, R>, time: f64) -> StepReport {
        self.shared.time.set(time);
        self.sync_windows(&mut driver.runtime);
        let wake = match self.app.try_borrow_mut() {
            Ok(mut app) => app.frame(&mut LogicCx::new(driver.runtime.store_mut(), &self.shared)),
            Err(_) => None,
        };
        self.shared.apply(&mut driver.runtime);
        self.registry.before_step(&mut driver.runtime);

        let (app, shared, env, shell, frame) = (&self.app, &*self.shared, &self.env, &self.shell, &self.frame);
        let report = driver.step_with(time, &mut |window, cx| {
            if let Some(cell) = shared.builds.borrow().get(&window).copied() {
                cell.get(cx);
            }
            cx.provide(shell.clone());
            if let Some(decision) = frame {
                cx.provide(decision.clone());
            }
            cx.provide(shared.memory.clone());
            cx.provide(shared.context(window));
            window_root_with(cx, env.clone(), |ui| {
                if let Ok(mut app) = app.try_borrow_mut() {
                    app.ui(window, ui);
                }
            });
        });

        self.drain_contexts(driver);
        let fired = std::mem::take(&mut *self.shared.deferred.borrow_mut());
        if !fired.is_empty()
            && let Ok(mut app) = self.app.try_borrow_mut()
        {
            for fired in &fired {
                app.logic(fired, &mut LogicCx::new(driver.runtime.store_mut(), &self.shared));
            }
        }
        // Cells written and windows asked to change after the turn's own frame request need the next turn.
        if self.shared.apply(&mut driver.runtime) || driver.runtime.needs_frame() {
            driver.platform.request_frame();
        }
        self.registry.after_step(&mut driver.runtime, driver.platform.surfaces(), report.ran, time);
        #[cfg(windows)]
        if report.ran {
            self.install_hit_sinks(driver);
        }
        if let Some(at) = wake {
            // The platform keeps one wake time: the earlier of the app's and the runtime's.
            driver.platform.request_frame_at(driver.runtime.next_wake().map_or(at, |due| due.min(at)));
        }

        if driver.runtime.windows().next().is_some() {
            self.had_windows = true;
        } else if self.had_windows {
            self.finish();
        }
        report
    }

    /// Hands the platform what code outside a build asked of each window's `Context` (clipboard writes, URLs, window
    /// commands), and forgets the contexts of closed windows.
    fn drain_contexts<P: Platform, R: Renderer>(&mut self, driver: &mut Driver<P, R>) {
        let open: Vec<WindowId> = driver.runtime.windows().collect();
        let contexts: Vec<(WindowId, Context)> = {
            let mut contexts = self.shared.contexts.borrow_mut();
            contexts.retain(|window, _| open.contains(window));
            contexts.iter().map(|(window, ctx)| (*window, ctx.clone())).collect()
        };
        for (window, ctx) in contexts {
            for request in ctx.take_platform_requests() {
                match request {
                    PlatformRequest::CopyText(text) => driver.platform.clipboard().set_text(&text),
                    PlatformRequest::CopyImage(image) => driver.platform.clipboard().set_image(&image),
                    PlatformRequest::OpenUrl { url, new_tab } => driver.platform.open_url(&url, new_tab),
                }
            }
            for command in ctx.take_window_commands() {
                driver.runtime.window_command(window, command);
            }
            if ctx.wants_root_build() {
                driver.runtime.mark_root(window);
            }
        }
    }

    /// Writes the placements and calls [`App::exit`], once: the loop calls it when the last window has closed.
    pub fn finish(&mut self) {
        if std::mem::replace(&mut self.exited, true) {
            return;
        }
        self.registry.flush(self.shared.time.get());
        if let Ok(mut app) = self.app.try_borrow_mut() {
            app.exit();
        }
    }

    /// The window registry: saved placements and the Windows hit-test sinks.
    pub fn registry(&mut self) -> &mut WindowRegistry {
        &mut self.registry
    }

    /// Gives each window a build cell when it opens and drops it when the window closes.
    fn sync_windows(&mut self, runtime: &mut Runtime) {
        let open: Vec<WindowId> = runtime.windows().collect();
        let mut builds = self.shared.builds.borrow_mut();
        let closed: Vec<WindowId> = builds.keys().copied().filter(|window| !open.contains(window)).collect();
        for window in closed {
            if let Some(cell) = builds.remove(&window) {
                cell.remove(runtime.store_mut());
            }
        }
        for window in open {
            builds.entry(window).or_insert_with(|| runtime.store_mut().create(0u64));
        }
    }

    /// Routes the OS hit test of each window whose frame the app draws (it has window zones) to the registry.
    #[cfg(windows)]
    fn install_hit_sinks<P: Platform, R: Renderer>(&mut self, driver: &mut Driver<P, R>) {
        let waker = driver.waker();
        let windows: Vec<WindowId> = driver.runtime.windows().collect();
        for window in windows {
            if self.registry.hit_sink(window).is_some() || driver.runtime.window_zones(window).is_empty() {
                continue;
            }
            let native = driver.platform.surfaces().surface(window).and_then(|surface| surface.native().cloned());
            if let Some(native) = native {
                self.registry.install_hit_sink(window, &native, waker.clone());
            }
        }
    }
}

/// The epaint renderer with the suite's interface faces.
#[cfg(feature = "app")]
fn epaint_fonts() -> craft_ui_render_epaint::FontDefinitions {
    crate::fonts::UiFonts::new(&crate::fonts::I18nScripts).stack().flat_definitions()
}

/// Opens the main window and runs `make`'s app until its last window closes.
#[cfg(all(feature = "app", not(target_arch = "wasm32")))]
pub fn run<A: App>(spec: AppSpec, make: impl FnOnce(&mut LogicCx<'_>) -> A + 'static) -> Result<(), StartError> {
    let prepared = Prepared::new(spec)?;
    match prepared.spec.renderer {
        #[cfg(feature = "skia")]
        RendererKind::Skia => run_winit(prepared, craft_ui_render_skia::SkiaRenderer::new(Default::default()), make, open_native),
        _ => {
            let options = craft_ui_render_epaint::EpaintOptions::default();
            let renderer = craft_ui_render_epaint::EpaintRenderer::create_blocking(options, epaint_fonts(), None)
                .map_err(|error| StartError::Renderer(error.to_string()))?;
            run_winit(prepared, renderer, make, open_native)
        }
    }
}

#[cfg(all(feature = "app", not(target_arch = "wasm32")))]
fn open_native(platform: &mut craft_ui_platform_winit::WinitPlatform, spec: &craft_ui_rt::WindowSpec) -> WindowId {
    platform.create_window(spec)
}

/// Runs `make`'s app on the page's canvas with id `canvas`. Returns once the loop runs; the page keeps it running.
#[cfg(all(feature = "app", target_arch = "wasm32"))]
pub async fn start_web<A: App>(canvas: &str, spec: AppSpec, make: impl FnOnce(&mut LogicCx<'_>) -> A + 'static) -> Result<(), StartError> {
    let prepared = Prepared::new(spec)?;
    let options = craft_ui_render_epaint::EpaintOptions::default();
    let renderer = craft_ui_render_epaint::EpaintRenderer::create(options, epaint_fonts(), None)
        .await
        .map_err(|error| StartError::Renderer(error.to_string()))?;
    let canvas = canvas.to_owned();
    run_winit(prepared, renderer, make, move |platform: &mut craft_ui_platform_winit::WinitPlatform, spec: &craft_ui_rt::WindowSpec| {
        platform.create_window_on_canvas(spec, &canvas)
    })
}

/// The winit loop over the host: setup starts the app, each redraw is one [`Host::step`].
#[cfg(feature = "app")]
fn run_winit<A: App, R: Renderer + 'static>(
    prepared: Prepared,
    renderer: R,
    make: impl FnOnce(&mut LogicCx<'_>) -> A + 'static,
    open: impl FnOnce(&mut craft_ui_platform_winit::WinitPlatform, &craft_ui_rt::WindowSpec) -> WindowId + 'static,
) -> Result<(), StartError> {
    let slot: Rc<RefCell<Option<Host<A>>>> = Rc::new(RefCell::new(None));
    let (started, stepped) = (slot.clone(), slot.clone());
    craft_ui_platform_winit::run_with(
        move |platform| {
            let (driver, host) = start_on(prepared, platform, renderer, make, open);
            *started.borrow_mut() = Some(host);
            std::future::ready(driver)
        },
        move |driver, time| {
            if let Some(host) = stepped.borrow_mut().as_mut() {
                host.step(driver, time);
            }
        },
    )
    .map_err(|error| StartError::EventLoop(error.to_string()))?;
    // On the desktop the loop returns when the last window has closed, maybe before a turn saw it.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(host) = slot.borrow_mut().as_mut() {
        host.finish();
    }
    Ok(())
}
