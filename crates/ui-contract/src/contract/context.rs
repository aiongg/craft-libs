//! The craft `Context` (docs/contract-rt.md section 15.3): a cheap handle to one window's facts and
//! services, held by every `Ui` of the window and cloned where code needs the window without a
//! `Ui`.
//!
//! Reads of window facts (`pixels_per_point`, `window_focused`, `content_rect`) inside a build are
//! tracked: they go to a per-thread log that the scope's build drains when it ends
//! ([`end_cell_reads`], called from the bridge's `end_reads`), which registers each as a build read
//! of the cell it answered from, so a change reruns exactly the scopes that read it. Effects asked
//! for inside a build (a repaint, a window command) go to the same log and reach the reading scope's
//! `BuildCx` there; outside a build they go to the waker and the host's queue.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use craft_ui_rt::{BuildCx, Margin, Os, Rect, State, Vec2, Waker, WindowCommand, WindowId, Writes as _};

use super::rt::EnvCells;
use super::rt::commands::CommandBoard;
use crate::primitives::Id;
use crate::runtime::commands::CommandState;
use crate::runtime::memory::RuntimeMemory;
use crate::style::StyleCaches;
use craft_keymap::CommandRef;

/// A handle to one window's facts and services. Cloning it is cheap; every clone answers for the
/// same window.
#[derive(Clone)]
pub struct Context {
    shared: Rc<Shared>,
}

/// What a platform does for the app outside the runtime: the host takes these after each step
/// ([`Context::take_platform_requests`]).
#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub enum PlatformRequest {
    /// Put `text` on the clipboard.
    CopyText(String),
    /// Put `image` on the clipboard (straight alpha).
    CopyImage(craft_ui_rt::RgbaImage),
    /// Open `url`, in a new tab where the platform has tabs and `new_tab` is set.
    OpenUrl { url: String, new_tab: bool },
}

/// The window facts a build reads, refreshed from the window's cells whenever a `Ui` of the window
/// is made (they change only between frames).
#[derive(Clone, Copy)]
struct Facts {
    time: f64,
    scale: f32,
    focused: bool,
    viewport: Vec2,
    safe_area: Margin,
    os: Os,
    mods: craft_keymap::Mods,
    caps_lock: bool,
}

/// The ui-rt cells the facts come from, for read registration.
#[derive(Clone, Copy)]
struct WindowCells {
    scale: State<f32>,
    focused: State<bool>,
    viewport: State<Vec2>,
    modifiers: State<craft_keymap::Mods>,
    caps_lock: State<bool>,
    held: State<Vec<craft_ui_rt::Key>>,
}

struct Shared {
    window: WindowId,
    waker: RefCell<Option<Waker>>,
    facts: Cell<Facts>,
    cells: Cell<Option<WindowCells>>,
    /// The window's style caches (section 15.7).
    styles: StyleCaches,
    /// The session and saved memory of the runtime: one store above every window root, so a panel moved to another
    /// window keeps its state.
    memory: RuntimeMemory,
    /// The window root's env cells, whose tokens and theme-base revisions the style caches are filled at.
    root_cells: Cell<Option<EnvCells>>,
    /// The window's open-layer cell: its revision moves whenever a layer opens, changes or closes (`rt::layers`).
    layers: Cell<Option<State<u64>>>,
    /// The keys held down in the window, in the order they went down, as the window's cell last held them.
    held: RefCell<Vec<craft_ui_rt::Key>>,
    /// Builds by `Id` asked for outside a build (`Ui::request_rename`), drained by the window root.
    builds: RefCell<Vec<Id>>,
    /// The command board the window root provides, for code with no build at hand (`Commands`).
    board: RefCell<Option<CommandBoard>>,
    /// Crate-private services of the window, one value per type: caches and readers that egui kept
    /// in its context memory.
    services: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    /// Window commands asked for outside a build, for the host.
    commands: RefCell<Vec<WindowCommand>>,
    platform: RefCell<Vec<PlatformRequest>>,
}

impl Context {
    /// The context of `window` over the runtime's memory store, with no facts read yet. The host makes one per window
    /// and provides it above the window root ([`window_root_with`](crate::runtime::windows::window_root_with)), with
    /// the same `memory` for every window.
    #[doc(hidden)]
    pub fn for_window(window: WindowId, waker: Option<Waker>, memory: RuntimeMemory) -> Context {
        let facts = Facts {
            time: 0.0,
            scale: 1.0,
            focused: true,
            viewport: Vec2::ZERO,
            safe_area: Margin::default(),
            os: Os::default(),
            mods: craft_keymap::Mods::NONE,
            caps_lock: false,
        };
        Context {
            shared: Rc::new(Shared {
                window,
                waker: RefCell::new(waker),
                facts: Cell::new(facts),
                cells: Cell::new(None),
                styles: StyleCaches::default(),
                memory,
                root_cells: Cell::new(None),
                layers: Cell::new(None),
                held: RefCell::new(Vec::new()),
                builds: RefCell::new(Vec::new()),
                board: RefCell::new(None),
                services: RefCell::new(HashMap::new()),
                commands: RefCell::new(Vec::new()),
                platform: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The context of the node `cx` builds: the one its window root provided, brought up to date
    /// with the window's cells (read without recording). Outside a window root (a bare test build)
    /// the node's own, as a window root would make it.
    pub(crate) fn of_build(cx: &mut BuildCx<'_>) -> Context {
        let ctx = match cx.provided::<Context>() {
            Some(ctx) => ctx,
            None => Context::enter_root(cx, EnvCells::of(cx)),
        };
        ctx.refresh(cx);
        ctx
    }

    /// The window root's part (section 15.7): takes the context the host provided above the root,
    /// else one kept in a slot of the root's node (over the runtime memory store the host
    /// provided, else a store of its own), and provides it and its style caches to the window.
    /// The root reads the tokens, theme-base and scale cells, so a theme, contrast, density or
    /// scale write runs it, and it clears the style caches then, before any child resolves a style.
    pub(crate) fn enter_root(cx: &mut BuildCx<'_>, cells: Option<EnvCells>) -> Context {
        let ctx = match cx.provided::<Context>() {
            Some(ctx) => ctx,
            None => {
                let window = cx.window();
                let waker = cx.provided::<super::rt::JobBoard>().and_then(|jobs| jobs.waker());
                let memory = cx.provided::<RuntimeMemory>().unwrap_or_default();
                let kept = cx.slot("craft-context", move || Context::for_window(window, waker, memory));
                cx.peek_state(kept).clone()
            }
        };
        ctx.shared.root_cells.set(cells);
        *ctx.shared.board.borrow_mut() = cx.provided::<CommandBoard>();
        cx.provide(ctx.clone());
        cx.provide(ctx.styles().clone());
        let scale = cx.env().scale;
        let tokens = match cells {
            Some(cells) => {
                cells.tokens.get(cx);
                cells.theme.get(cx);
                cells.tokens
            }
            None => cx.env().tokens,
        };
        scale.get(cx);
        let store = cx.runtime().store();
        let theme = cells.map_or(0, |cells| cells.theme.version(store));
        ctx.shared.styles.clear_if_moved((tokens.version(store), theme, scale.version(store)));
        ctx
    }

    /// Reads the window's facts from its cells, without recording a read.
    fn refresh(&self, cx: &mut BuildCx<'_>) {
        let env = cx.env();
        let facts = Facts {
            time: cx.now(),
            scale: *cx.peek_state(env.scale),
            focused: *cx.peek_state(env.window_focused),
            viewport: *cx.peek_state(env.viewport),
            safe_area: *cx.peek_state(env.safe_area),
            os: cx.runtime().os(),
            mods: *cx.peek_state(env.modifiers),
            caps_lock: *cx.peek_state(env.caps_lock),
        };
        self.shared.facts.set(facts);
        let held = cx.peek_state(env.held_keys);
        if *self.shared.held.borrow() != *held {
            *self.shared.held.borrow_mut() = held.clone();
        }
        self.shared.cells.set(Some(WindowCells {
            scale: env.scale,
            focused: env.window_focused,
            viewport: env.viewport,
            modifiers: env.modifiers,
            caps_lock: env.caps_lock,
            held: env.held_keys,
        }));
    }

    fn facts(&self) -> Facts {
        self.shared.facts.get()
    }

    /// Logs a tracked read for the scope whose build is running, if any.
    fn track(&self, read: impl FnOnce(WindowCells) -> CellRead) {
        if let Some(cells) = self.shared.cells.get() {
            log(read(cells));
        }
    }

    /// The window this context answers for.
    pub fn window(&self) -> WindowId {
        self.shared.window
    }

    /// The window's pixels per point. Tracked inside a build.
    pub fn pixels_per_point(&self) -> f32 {
        self.track(|cells| CellRead::Scale(cells.scale));
        self.facts().scale
    }

    /// Inside a build: the calling scope builds again in this frame's next round. Outside a build: the
    /// loop wakes for a frame.
    pub fn request_repaint(&self) {
        if building() {
            log(CellRead::Wake(Duration::ZERO));
        } else if let Some(waker) = self.shared.waker.borrow().as_ref() {
            waker.wake();
        }
    }

    /// Inside a build: the calling scope builds again `after` from now. Outside a build: the loop wakes
    /// (the host schedules no timed wake from outside a build).
    pub fn request_repaint_after(&self, after: Duration) {
        if building() {
            log(CellRead::Wake(after));
        } else if let Some(waker) = self.shared.waker.borrow().as_ref() {
            waker.wake();
        }
    }

    /// The frame time in seconds, untracked.
    pub fn time(&self) -> f64 {
        self.facts().time
    }

    /// The modifiers held at the window's last key, pointer or wheel event. Tracked inside a build.
    pub fn modifiers(&self) -> craft_keymap::Mods {
        self.track(|cells| CellRead::Mods(cells.modifiers));
        self.facts().mods
    }

    /// Whether the window has the platform's key focus. Tracked inside a build.
    pub fn window_focused(&self) -> bool {
        self.track(|cells| CellRead::Focused(cells.focused));
        self.facts().focused
    }

    /// Whether caps lock is on, as the platform last reported it. Tracked inside a build.
    pub fn caps_lock(&self) -> bool {
        self.track(|cells| CellRead::Caps(cells.caps_lock));
        self.facts().caps_lock
    }

    /// The keys held down in the window, in the order they went down. Tracked inside a build: a reader runs again when
    /// a key goes down or up.
    pub fn held_keys(&self) -> Vec<craft_ui_rt::Key> {
        self.track(|cells| CellRead::Held(cells.held));
        self.shared.held.borrow().clone()
    }

    /// The operating system the window runs on.
    pub fn os(&self) -> Os {
        self.facts().os
    }

    /// The window's content rect in points: its size less the platform's safe area. Tracked inside a
    /// build.
    pub fn content_rect(&self) -> Rect {
        self.track(|cells| CellRead::Viewport(cells.viewport));
        let facts = self.facts();
        let safe = facts.safe_area;
        Rect::from_min_max(craft_ui_rt::pos2(safe.left, safe.top), craft_ui_rt::pos2(facts.viewport.x - safe.right, facts.viewport.y - safe.bottom))
    }

    /// Puts `text` on the clipboard.
    pub fn copy_text(&self, text: String) {
        self.platform(PlatformRequest::CopyText(text));
    }

    /// Puts `image` (straight alpha) on the clipboard; platforms without image support ignore it.
    pub(crate) fn copy_image(&self, image: craft_ui_rt::RgbaImage) {
        self.platform(PlatformRequest::CopyImage(image));
    }

    /// Opens `url`, in a new tab where the platform has tabs and `new_tab` is set.
    pub fn open_url(&self, url: &str, new_tab: bool) {
        self.platform(PlatformRequest::OpenUrl { url: url.to_owned(), new_tab });
    }

    fn platform(&self, request: PlatformRequest) {
        self.shared.platform.borrow_mut().push(request);
        if let Some(waker) = self.shared.waker.borrow().as_ref() {
            waker.wake();
        }
    }

    /// Asks the platform to change this window. Inside a build it reaches the runtime when the scope's
    /// build ends; outside one it waits for the host.
    pub fn window_command(&self, command: WindowCommand) {
        if building() {
            log(CellRead::Command(command));
        } else {
            self.shared.commands.borrow_mut().push(command);
            if let Some(waker) = self.shared.waker.borrow().as_ref() {
                waker.wake();
            }
        }
    }

    /// The clipboard writes and URL opens asked for since the last call, for the host.
    #[doc(hidden)]
    pub fn take_platform_requests(&self) -> Vec<PlatformRequest> {
        std::mem::take(&mut *self.shared.platform.borrow_mut())
    }

    /// The window commands asked for outside a build since the last call, for the host.
    #[doc(hidden)]
    pub fn take_window_commands(&self) -> Vec<WindowCommand> {
        std::mem::take(&mut *self.shared.commands.borrow_mut())
    }

    /// Installs the app's style replacements (`Looks`) for this window: the host calls it once per window.
    #[doc(hidden)]
    pub fn install_looks(&self, looks: crate::style::StyleReplacements) -> Result<(), crate::style::DuplicateStyle> {
        self.shared.styles.set_replacements(looks.freeze()?);
        Ok(())
    }

    /// The window's style caches (section 15.7).
    pub(crate) fn styles(&self) -> &StyleCaches {
        &self.shared.styles
    }

    /// The window's command board, when the window root has one.
    pub(crate) fn board(&self) -> Option<CommandBoard> {
        self.shared.board.borrow().clone()
    }

    /// The runtime's session and saved memory.
    pub(crate) fn memory(&self) -> &RuntimeMemory {
        &self.shared.memory
    }

    /// Runs `f` on the window's service of type `T`, made with its default on first use. `f` must not
    /// reach another service of the same type.
    pub(crate) fn service<T: Default + 'static, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut services = self.shared.services.borrow_mut();
        let slot = services.entry(TypeId::of::<T>()).or_insert_with(|| Box::new(T::default()));
        f(slot.downcast_mut::<T>().expect("a service holds the type it was made with"))
    }

    /// The window's open-layer cell, made on first use.
    pub(crate) fn layers_cell(&self, store: &mut craft_ui_rt::Store) -> State<u64> {
        match self.shared.layers.get() {
            Some(cell) => cell,
            None => {
                let cell = store.create(0);
                self.shared.layers.set(Some(cell));
                cell
            }
        }
    }

    /// Whether two handles answer for the same window's context.
    pub(crate) fn same(&self, other: &Context) -> bool {
        Rc::ptr_eq(&self.shared, &other.shared)
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("window", &self.shared.window).finish_non_exhaustive()
    }
}

/// A craft `Id` for `window`'s root, the base the window's ids hash from.
pub(crate) fn window_id(window: WindowId) -> Id {
    Id::NULL.with(("craft-window", window))
}

/// One entry of the build log: a tracked read or an effect for the scope whose build is running.
pub(crate) enum CellRead {
    Scale(State<f32>),
    Focused(State<bool>),
    Viewport(State<Vec2>),
    Mods(State<craft_keymap::Mods>),
    Caps(State<bool>),
    Held(State<Vec<craft_ui_rt::Key>>),
    /// `Env::of`: every env cell an `Env` copies, plus the window's scale and focus.
    Env(EnvCells, State<f32>, State<bool>),
    /// The scope builds again after this long.
    Wake(Duration),
    Command(WindowCommand),
    /// A read of a command's state for an instance of its scope (`Commands::resolve_command`).
    CommandState(CommandBoard, CommandRef, Option<Id>),
    /// A command's state published for an instance of its scope (`Commands::publish`).
    Publish(CommandBoard, CommandRef, Option<Id>, CommandState),
    /// A read made with the node's `BuildCx` when its build ends, for a service whose cell may not exist yet (a
    /// texture entry read from a hook: `CxCore::texture_state`).
    Build(Box<dyn FnOnce(&mut BuildCx<'_>)>),
}

thread_local! {
    /// The build log of the scope builds running on this thread, innermost scope last.
    static LOG: RefCell<Vec<CellRead>> = const { RefCell::new(Vec::new()) };
    /// How many scope builds are open on this thread.
    static OPEN: Cell<usize> = const { Cell::new(0) };
}

fn log(read: CellRead) {
    if building() {
        LOG.with(|log| log.borrow_mut().push(read));
    }
}

fn building() -> bool {
    OPEN.with(Cell::get) > 0
}

/// Opens a scope's part of the build log; the window root (`root`) clears what a panicked build left.
pub(crate) fn begin_cell_reads(root: bool) -> usize {
    if root {
        OPEN.with(|open| open.set(0));
        crate::runtime::env::clear_scopes();
    }
    OPEN.with(|open| open.set(open.get() + 1));
    LOG.with(|log| {
        let mut log = log.borrow_mut();
        if root {
            log.clear();
        }
        log.len()
    })
}

/// Closes a scope's part of the build log: each read since `mark` becomes a build read of the scope's
/// node (`cx`), and each effect reaches `cx`.
pub(crate) fn end_cell_reads(mark: usize, cx: &mut BuildCx<'_>) {
    OPEN.with(|open| open.set(open.get().saturating_sub(1)));
    let entries: Vec<CellRead> = LOG.with(|log| {
        let mut log = log.borrow_mut();
        let mark = mark.min(log.len());
        log.split_off(mark)
    });
    let mut wake: Option<Duration> = None;
    for entry in entries {
        match entry {
            CellRead::Scale(cell) => {
                cell.get(cx);
            }
            CellRead::Focused(cell) => {
                cell.get(cx);
            }
            CellRead::Viewport(cell) => {
                cell.get(cx);
            }
            CellRead::Mods(cell) => {
                cell.get(cx);
            }
            CellRead::Caps(cell) => {
                cell.get(cx);
            }
            CellRead::Held(cell) => {
                cell.get(cx);
            }
            CellRead::Env(cells, scale, focused) => {
                focused.get(cx);
                cells.tokens.get(cx);
                cells.theme.get(cx);
                cells.direction.get(cx);
                cells.density.get(cx);
                cells.motion.get(cx);
                cells.frame.get(cx);
                cells.root.get(cx);
                scale.get(cx);
            }
            CellRead::Wake(after) => wake = Some(wake.map_or(after, |w| w.min(after))),
            CellRead::Command(command) => cx.window_command(command),
            CellRead::CommandState(board, command, instance) => {
                let cell = board.cell(cx.store_mut(), &command, instance);
                cell.get(cx);
            }
            CellRead::Publish(board, command, instance, state) => board.set_state_of(cx, &command, instance, state),
            CellRead::Build(read) => read(cx),
        }
    }
    if let Some(after) = wake {
        let at = cx.now() + after.as_secs_f64();
        cx.request_build_at(Some(at));
    }
}

/// Logs a read of `command`'s state for `instance` for the node whose build is running.
pub(crate) fn log_command_read(board: CommandBoard, command: CommandRef, instance: Option<Id>) {
    log(CellRead::CommandState(board, command, instance));
}

/// Logs a publish of `command`'s state for `instance`: it reaches the cell when the running node's build ends. Outside
/// a build there is no store to write, and nothing is published.
pub(crate) fn log_publish(board: CommandBoard, command: CommandRef, instance: Option<Id>, state: CommandState) {
    log(CellRead::Publish(board, command, instance, state));
}

/// Logs `read`, run with the `BuildCx` of the node whose build is running when that build ends: how a service read
/// from a hook (which has no `BuildCx`) registers its cell read for the node, creating the cell if it needs one.
pub(crate) fn log_build_read(read: impl FnOnce(&mut BuildCx<'_>) + 'static) {
    log(CellRead::Build(Box::new(read)));
}

/// Logs an `Env::of` read for the scope whose build is running.
pub(crate) fn log_env_read(cells: EnvCells, scale: State<f32>, focused: State<bool>) {
    log(CellRead::Env(cells, scale, focused));
}

impl crate::contract::CxCore<'_> {
    /// The state of `key`'s picture at `revision` in `store`, read from a hook. The answer is the cache's state now;
    /// the tracked read ([`TextureStore::state`](crate::primitives::textures::TextureStore::state)) goes through the
    /// run's read log and is made with the node's `BuildCx` when its build ends, so this node runs again when a job
    /// answers for `key` (or in the next frame, while a picture waits for the image store), and a leaf reads its own
    /// thumbnail with no prop.
    pub fn texture_state<K: crate::primitives::textures::TextureKey>(
        &self,
        store: &crate::primitives::textures::TextureStore<K>,
        key: &K,
        revision: crate::primitives::textures::Revision,
    ) -> crate::primitives::textures::TextureState {
        let now = self.memory.ctx().time();
        let state = crate::primitives::textures::TextureState::of(store.cache().look(key, revision), store.cache().waited(key, now));
        let (store, key) = (store.clone(), key.clone());
        log_build_read(move |cx| {
            store.state(cx, &key, revision);
        });
        state
    }
}
