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

use craft_ui_rt::{BuildCx, Margin, Os, Rect, State, Vec2, Waker, WindowCommand, WindowId};

use super::rt::EnvCells;
use crate::primitives::Id;
use crate::style::StyleCaches;

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
}

/// The ui-rt cells the facts come from, for read registration.
#[derive(Clone, Copy)]
struct WindowCells {
    scale: State<f32>,
    focused: State<bool>,
    viewport: State<Vec2>,
}

struct Shared {
    window: WindowId,
    waker: RefCell<Option<Waker>>,
    facts: Cell<Facts>,
    cells: Cell<Option<WindowCells>>,
    /// The window's style caches (section 15.7).
    styles: StyleCaches,
    /// The window root's env cells, whose tokens and theme-base revisions the style caches are filled at.
    root_cells: Cell<Option<EnvCells>>,
    /// Crate-private services of the window, one value per type: caches and readers that egui kept
    /// in its context memory.
    services: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    /// Window commands asked for outside a build, for the host.
    commands: RefCell<Vec<WindowCommand>>,
    platform: RefCell<Vec<PlatformRequest>>,
}

impl Context {
    /// The context of `window`, with no facts read yet.
    pub(crate) fn new(window: WindowId, waker: Option<Waker>) -> Context {
        let facts = Facts { time: 0.0, scale: 1.0, focused: true, viewport: Vec2::ZERO, safe_area: Margin::default(), os: Os::default() };
        Context {
            shared: Rc::new(Shared {
                window,
                waker: RefCell::new(waker),
                facts: Cell::new(facts),
                cells: Cell::new(None),
                styles: StyleCaches::default(),
                root_cells: Cell::new(None),
                services: RefCell::new(HashMap::new()),
                commands: RefCell::new(Vec::new()),
                platform: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The context `cx`'s window root keeps: found where the window root provided it, else made,
    /// kept in a slot of `cx`'s node and provided to everything under it. Its facts are brought up
    /// to date from the window's cells (read without recording).
    pub(crate) fn of_build(cx: &mut BuildCx<'_>) -> Context {
        let ctx = match cx.provided::<Context>() {
            Some(ctx) => ctx,
            None => {
                let window = cx.window();
                let waker = cx.provided::<super::rt::JobBoard>().and_then(|jobs| jobs.waker());
                let kept = cx.slot("craft-context", move || Context::new(window, waker));
                let ctx = cx.peek_state(kept).clone();
                ctx.shared.root_cells.set(EnvCells::of(cx));
                cx.provide(ctx.clone());
                cx.provide(ctx.styles().clone());
                ctx
            }
        };
        ctx.refresh(cx);
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
        };
        self.shared.facts.set(facts);
        self.shared.cells.set(Some(WindowCells { scale: env.scale, focused: env.window_focused, viewport: env.viewport }));
        // A theme, contrast, density or scale write moves one of these: the styles resolved before it go before any
        // child of this build resolves one.
        let store = cx.runtime().store();
        let (tokens, theme) = match self.shared.root_cells.get() {
            Some(cells) => (cells.tokens.version(store), cells.theme.version(store)),
            None => (env.tokens.version(store), 0),
        };
        self.shared.styles.clear_if_moved((tokens, theme, env.scale.version(store)));
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

    /// The modifiers held now.
    ///
    /// ui-rt keeps no modifier cell yet, so this answers `Mods::NONE` and records no read; a
    /// component reads the modifiers of its own input from its run's queue.
    pub fn modifiers(&self) -> craft_keymap::Mods {
        craft_keymap::Mods::NONE
    }

    /// Whether the window has the platform's key focus. Tracked inside a build.
    pub fn window_focused(&self) -> bool {
        self.track(|cells| CellRead::Focused(cells.focused));
        self.facts().focused
    }

    /// Whether caps lock is on. ui-rt reports no lock-key state yet: always false.
    pub fn caps_lock(&self) -> bool {
        false
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

    /// Runs `f` on the window's service of type `T`, made with its default on first use. `f` must not
    /// reach another service of the same type.
    pub(crate) fn service<T: Default + 'static, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut services = self.shared.services.borrow_mut();
        let slot = services.entry(TypeId::of::<T>()).or_insert_with(|| Box::new(T::default()));
        f(slot.downcast_mut::<T>().expect("a service holds the type it was made with"))
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
#[derive(Clone, Debug)]
pub(crate) enum CellRead {
    Scale(State<f32>),
    Focused(State<bool>),
    Viewport(State<Vec2>),
    /// `Env::of`: every env cell an `Env` copies, plus the window's scale and focus.
    Env(EnvCells, State<f32>, State<bool>),
    /// The scope builds again after this long.
    Wake(Duration),
    Command(WindowCommand),
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
            CellRead::Env(cells, scale, focused) => {
                focused.get(cx);
                cells.tokens.get(cx);
                cells.theme.get(cx);
                cells.direction.get(cx);
                cells.density.get(cx);
                cells.motion.get(cx);
                cells.frame.get(cx);
                scale.get(cx);
            }
            CellRead::Wake(after) => wake = Some(wake.map_or(after, |w| w.min(after))),
            CellRead::Command(command) => cx.window_command(command),
        }
    }
    if let Some(after) = wake {
        let at = cx.now() + after.as_secs_f64();
        cx.request_build_at(Some(at));
    }
}

/// Logs an `Env::of` read for the scope whose build is running.
pub(crate) fn log_env_read(cells: EnvCells, scale: State<f32>, focused: State<bool>) {
    log(CellRead::Env(cells, scale, focused));
}
