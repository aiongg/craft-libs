//! `WindowHost`: the one multi-window driver. It owns `ShellInfo`, refreshes each viewport's `WindowInfo`, releases
//! a press after an OS resize (H4), raises a modal's window for a blocked press (H5), opens viewport mounts, keeps
//! window placement in `craft_window::restore::Store`, routes a root close through the platform quit from the app's
//! `logic`, and maps pointers to desktop space for drags that cross windows.

use crate::runtime::egui::input::{self as raw_input, RawInput};
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::{pass, viewports};
use crate::runtime::frame::PerViewport;
use crate::runtime::overlay;
use craft_host::target::{Caps, EdgeInsets, Os, ShellInfo, WindowChrome, WindowInfo, WindowKind, WindowState};
#[cfg(feature = "testing")]
use craft_host::target::{MenuPlacement, Runtime};
use craft_window::policy::{CaptionFamily, Captions, Decision, Frame, LayoutWatch, MacInset};
use craft_window::restore::{Placement, Store};
use craft_window::system::WindowSystem;
use egui::{Context, Id, Pos2, Rect, Ui, Vec2, ViewportId};
use std::sync::{Arc, Mutex};

/// Seconds between writes of a changed placement store.
const FLUSH_SECONDS: f64 = 1.0;

/// An edge or corner a window resize starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResizeEdge {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bottom right corner.
    BottomRight,
}

/// Where a window stands in the stack of windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowLevel {
    /// An ordinary window.
    #[default]
    Normal,
    /// A palette: above the app's other windows while the app is active, and an ordinary window (behind other
    /// apps' windows on Windows) once none of the app's windows has focus, so it stays shown without covering
    /// other apps.
    Palette,
    /// Above every other window.
    AlwaysOnTop,
}

/// How long none of the app's windows has focus before the app counts as inactive, so that focus moving between
/// its own windows does not drop a palette.
const AWAY_SECONDS: f64 = 0.25;

impl WindowLevel {
    /// The level egui is given while the app is `active` or not, on `os`. An inactive app's palette goes behind
    /// other apps' windows: on Windows `Normal` puts a window in front of every window that is not topmost, the
    /// activated app's included, so the palette goes to the bottom.
    fn for_egui(self, active: bool, os: Os) -> egui::WindowLevel {
        match self {
            WindowLevel::Normal => egui::WindowLevel::Normal,
            WindowLevel::AlwaysOnTop => egui::WindowLevel::AlwaysOnTop,
            WindowLevel::Palette if active => egui::WindowLevel::AlwaysOnTop,
            WindowLevel::Palette if os == Os::Windows => egui::WindowLevel::AlwaysOnBottom,
            WindowLevel::Palette => egui::WindowLevel::Normal,
        }
    }
}

/// How an OS window opens (21.2): what it is for, who draws its frame, and its limits.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WindowSpec {
    /// What the window is for.
    pub kind: WindowKind,
    /// Who draws the frame.
    pub chrome: WindowChrome,
    /// The caption buttons the app draws when `chrome` is `Integrated`; none draws none.
    pub captions: Option<Captions>,
    /// Where the window stands among the windows.
    pub level: WindowLevel,
    /// The smallest inner size.
    pub min_size: Option<Vec2>,
    /// The largest inner size.
    pub max_size: Option<Vec2>,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// The outer position in desktop points when nothing is saved.
    pub position: Option<Pos2>,
    /// Whether the window has a taskbar or Dock entry.
    pub taskbar: bool,
}

impl WindowSpec {
    /// A tool window (a native dock window): the app's own bar without caption buttons, a palette level, no
    /// taskbar entry.
    pub fn tool() -> Self {
        WindowSpec { kind: WindowKind::Tool, chrome: WindowChrome::Integrated, level: WindowLevel::Palette, taskbar: false, ..Self::dialog() }
    }

    /// A document window with the OS's frame; [`WindowSpec::frame`] sets the decided one.
    pub fn document() -> Self {
        WindowSpec { kind: WindowKind::Document, ..Self::dialog() }
    }

    /// A dialog window with the OS's frame.
    pub fn dialog() -> Self {
        WindowSpec {
            kind: WindowKind::Dialog,
            chrome: WindowChrome::Native,
            captions: None,
            level: WindowLevel::Normal,
            min_size: None,
            max_size: None,
            resizable: true,
            position: None,
            taskbar: true,
        }
    }

    /// Sets what the window is for.
    #[must_use]
    pub fn kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets who draws the frame.
    #[must_use]
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// Sets the caption buttons.
    #[must_use]
    pub fn captions(mut self, captions: Option<Captions>) -> Self {
        self.captions = captions;
        self
    }

    /// Takes the chrome and the captions from a decided `frame`; a frame without buttons has no captions.
    #[must_use]
    pub fn frame(mut self, frame: &Frame) -> Self {
        self.chrome = chrome_of(frame);
        self.captions = match frame {
            Frame::Integrated(captions) if !captions.is_empty() => Some(captions.clone()),
            _ => None,
        };
        self
    }

    /// Sets the level.
    #[must_use]
    pub fn level(mut self, level: WindowLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets the smallest inner size.
    #[must_use]
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Sets the largest inner size.
    #[must_use]
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Sets whether the user can resize the window.
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets the outer position the window opens at when no placement is saved.
    #[must_use]
    pub fn position(mut self, position: Pos2) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets whether the window has a taskbar or Dock entry.
    #[must_use]
    pub fn taskbar(mut self, taskbar: bool) -> Self {
        self.taskbar = taskbar;
        self
    }
}

/// Who draws the frame `frame` describes.
fn chrome_of(frame: &Frame) -> WindowChrome {
    match frame {
        Frame::Native => WindowChrome::Native,
        Frame::Inset(_) => WindowChrome::Inset,
        Frame::Integrated(_) => WindowChrome::Integrated,
    }
}

/// The frame of a window drawn as `chrome` with `captions`; an integrated frame without captions has no buttons.
fn frame_of(chrome: WindowChrome, captions: Option<Captions>) -> Frame {
    match chrome {
        WindowChrome::Inset => Frame::Inset(MacInset::default()),
        WindowChrome::Integrated => Frame::Integrated(
            captions
                .unwrap_or_else(|| Captions { family: CaptionFamily::Generic, layout: craft_window::Layout { left: Vec::new(), right: Vec::new() } }),
        ),
        _ => Frame::Native,
    }
}

/// A window command `act` sends.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowAction {
    /// Closes the window; the root window's close goes through the close guard.
    Close,
    /// Minimizes.
    Minimize,
    /// Maximizes, or restores a maximized window.
    ToggleMaximize,
    /// Enters or leaves full screen.
    ToggleFullscreen,
    /// Raises the window and gives it OS focus.
    Focus,
    /// The OS starts moving the window with the pointer.
    StartMove,
    /// The OS starts resizing the window from `edge`.
    StartResize(ResizeEdge),
    /// Moves the window's outer top-left corner to a desktop position in points.
    SetOuterPosition(Pos2),
    /// Sizes the window's inner area in points.
    SetInnerSize(Vec2),
    /// Changes the window's level.
    SetLevel(WindowLevel),
    /// Changes the window's title.
    SetTitle(String),
}

/// What the OS tells about a window, in desktop points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WindowFacts {
    /// The inner rect.
    pub inner: Rect,
    /// The outer rect, where the platform tells.
    pub outer: Option<Rect>,
    /// Physical pixels per point of the monitor the window is on.
    pub scale: f32,
    /// Normal, minimized, maximized or full screen.
    pub state: WindowState,
    /// Whether the window has OS focus.
    pub focused: bool,
}

/// A connected monitor, in desktop points.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MonitorInfo {
    /// The name the platform reports.
    pub name: String,
    /// The monitor's area.
    pub rect: Rect,
    /// Physical pixels per point.
    pub scale: f32,
}

/// Install-time inputs and the cross-window records of one context.
#[derive(Default)]
struct Shared {
    shell: Option<ShellInfo>,
    system: Option<Arc<dyn WindowSystem>>,
    store: Option<Arc<Mutex<Store>>>,
    /// The system pointer read in `set_input`.
    system_pointer: Option<Pos2>,
    /// The pointer of the viewport a drag started in, in desktop points, for platforms that tell no system pointer.
    source_pointer: Option<Pos2>,
    /// Containers whose viewport the OS asked to close in this root frame.
    close_requests: Vec<Id>,
    last_flush: f64,
    /// The root window's frame and its caption layout watch, from `InstallSpec::frame`.
    root_frame: Option<Frame>,
    layouts: Option<LayoutWatch>,
    /// Whether the root window had focus in the last root pass.
    root_focused: bool,
    /// The time some window of the app last had focus.
    last_focused: f64,
    /// Whether no window of the app has had focus for [`AWAY_SECONDS`].
    away: bool,
}

/// One viewport's window record.
#[derive(Default)]
struct Record {
    facts: Option<viewports::Facts>,
    /// The measured traffic-lights inset, kept until the scale changes or full screen ends.
    lights: craft_window::mac::Cache,
    /// What the OS reserves at the window's edges, once measured.
    insets: Option<EdgeInsets>,
    kind: Option<WindowKind>,
    chrome: Option<WindowChrome>,
    /// The caption buttons the app draws in this window.
    captions: Option<Captions>,
    /// The level the window was opened with or last set to; a palette's egui level follows the app's activity.
    level: Option<WindowLevel>,
    /// H4: whether the primary button is down as the viewport's events left the input hook.
    primary_down: bool,
    /// The geometry last written to the placement store, so an unchanged window writes nothing.
    placed: Option<Geometry>,
    last_pos: Option<Pos2>,
    last_inner_size: Option<Vec2>,
}

/// The window facts a placement keeps.
#[derive(Clone, Copy, PartialEq)]
struct Geometry {
    inner: Rect,
    outer: Option<Rect>,
    scale: Option<f32>,
    maximized: bool,
    fullscreen: bool,
}

fn shared<R>(ctx: &Context, f: impl FnOnce(&mut Shared) -> R) -> R {
    Service::<Shared>::run(ctx, f)
}

/// Called by `install`: keeps the shell, the OS services and the placement store.
pub(crate) fn install(ctx: &Context, shell: ShellInfo, system: Arc<dyn WindowSystem>, store: Option<Arc<Mutex<Store>>>, frame: Option<Decision>) {
    let layouts = frame.as_ref().map(LayoutWatch::new);
    shared(ctx, |shared| {
        shared.shell = Some(shell);
        shared.system = Some(system);
        shared.store = store;
        shared.root_frame = frame.map(|decision| decision.frame);
        shared.layouts = layouts;
    });
}

fn base_shell(ctx: &Context) -> ShellInfo {
    shared(ctx, |shared| shared.shell.clone()).unwrap_or_default()
}

/// The operating system of the shell: the one source of the key table, the chords shown, the drop operation and
/// the chrome. It does not differ between viewports.
pub(crate) fn os_of(ctx: &Context) -> Os {
    shared(ctx, |shared| shared.shell.as_ref().map(|shell| shell.os)).unwrap_or_else(|| ShellInfo::default().os)
}

/// Makes the shell run on `os`, as a test of the OS matrix does: the capabilities and the menu placement of a
/// native build follow it. egui's own OS is the caller's to set.
#[cfg(feature = "testing")]
pub(crate) fn set_os(ctx: &Context, os: Os) {
    shared(ctx, |shared| {
        let shell = shared.shell.get_or_insert_with(ShellInfo::default);
        shell.os = os;
        if shell.runtime == Runtime::Native {
            shell.caps = Caps::native(os);
            shell.menu = if shell.caps.contains(Caps::NATIVE_MENU_BAR) { MenuPlacement::ScreenTop } else { MenuPlacement::InWindow };
        }
    });
}

fn state_of(facts: &viewports::Facts) -> WindowState {
    if facts.fullscreen {
        WindowState::Fullscreen
    } else if facts.minimized {
        WindowState::Minimized
    } else if facts.maximized {
        WindowState::Maximized
    } else {
        WindowState::Normal
    }
}

/// `viewport`'s window facts.
fn window_info(ctx: &Context, viewport: ViewportId, base: &ShellInfo) -> WindowInfo {
    let (facts, kind, mut chrome, insets) =
        PerViewport::<Record>::with(ctx, viewport, |record| (record.facts, record.kind, record.chrome, record.insets));
    if viewport == ViewportId::ROOT {
        chrome = shared(ctx, |shared| shared.root_frame.as_ref().map(chrome_of)).or(chrome);
    }
    let mut info = if viewport == ViewportId::ROOT { (*base.window).clone() } else { WindowInfo::new(WindowKind::Tool) };
    if let Some(kind) = kind {
        info.kind = kind;
    }
    if let Some(chrome) = chrome {
        info.chrome = chrome;
    }
    if let Some(insets) = insets {
        info.insets = insets;
    }
    if let Some(facts) = facts {
        info.active = facts.focused;
        info.state = state_of(&facts);
    }
    info
}

/// The shell as `viewport` sees it: the OS-level fields from install, the window facts of that viewport.
pub(crate) fn shell_of(ctx: &Context, viewport: ViewportId) -> ShellInfo {
    let mut shell = base_shell(ctx);
    shell.window = Arc::new(window_info(ctx, viewport, &shell));
    shell
}

/// Stores `facts` as the viewport's window facts and, on macOS where the app draws the title bar, measures the
/// traffic lights for `WindowInfo.insets` (21.2). The measurement is AppKit's, so it runs here, outside every egui lock.
fn refresh(ctx: &Context, viewport: ViewportId, facts: viewports::Facts) {
    let mut lights = PerViewport::<Record>::with(ctx, viewport, |record| {
        record.facts = Some(facts);
        std::mem::take(&mut record.lights)
    });
    let lights_over_content = os_of(ctx) == Os::Mac && window_info(ctx, viewport, &base_shell(ctx)).chrome != WindowChrome::Native;
    let insets = lights_over_content
        .then(|| {
            let scale = facts.native_scale.unwrap_or(1.0);
            lights.inset(scale, facts.fullscreen, craft_window::mac::os_default, craft_window::mac::measure)
        })
        .map(|inset| {
            let mut insets = EdgeInsets::all(0.0);
            insets.left = inset.map_or(0.0, |inset| inset.leading);
            insets
        });
    PerViewport::<Record>::with(ctx, viewport, |record| {
        record.lights = lights;
        record.insets = insets;
    });
}

/// Before the pass, from `set_input`, outside every egui lock: reads the OS services and refreshes the window
/// info of every viewport the last root input reported.
pub(crate) fn set_input(ctx: &Context) {
    let system = shared(ctx, |shared| shared.system.clone());
    let pointer = system.and_then(|system| system.desktop_pointer()).map(|[x, y]| Pos2::new(x, y));
    for (viewport, facts) in viewports::reported(ctx) {
        refresh(ctx, viewport, facts);
    }
    shared(ctx, |shared| shared.system_pointer = pointer);
}

/// H4, in the input hook: where the OS resizes undecorated windows from their edges ([`Caps::OS_RESIZES_UNDECORATED`]),
/// an undecorated window resized that way never delivers the button release, so a size change under a held primary
/// press releases it. Reads only the raw input and the viewport's record. Returns whether it added an event.
pub(crate) fn input_hook(ctx: &Context, input: &mut RawInput) -> bool {
    let viewport = input.viewport_id;
    let base = base_shell(ctx);
    let undecorated = base.caps.contains(Caps::OS_RESIZES_UNDECORATED) && window_info(ctx, viewport, &base).chrome == WindowChrome::Integrated;
    let size = viewports::inner_size_in(input, viewport);
    PerViewport::<Record>::with(ctx, viewport, |record| {
        let resized = matches!((record.last_inner_size, size), (Some(before), Some(now)) if before != now);
        record.last_inner_size = size.or(record.last_inner_size);
        let mut added = false;
        if undecorated && resized && record.primary_down {
            raw_input::push_primary_release(input, record.last_pos.unwrap_or(Pos2::ZERO));
            added = true;
        }
        raw_input::follow_primary(&input.events, &mut record.primary_down, &mut record.last_pos);
        added
    })
}

/// The begin hook (H5), where the viewport is current: refreshes its window info, and raises the modal's window
/// when the input hook dropped a press under a modal in another viewport.
pub(crate) fn begin(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    refresh(ctx, viewport, viewports::current(ctx));
    if overlay::take_dropped_press(ctx, viewport)
        && let Some(modal) = overlay::modal_viewport(ctx, viewport)
    {
        act(ctx, modal, WindowAction::Focus);
    }
}

/// The end hook: keeps the root window's placement, writes a changed placement store once a second, and drops the
/// close requests the viewport mounts did not take in this frame.
pub(crate) fn end(ctx: &Context) {
    if pass::current_viewport(ctx) != ViewportId::ROOT {
        return;
    }
    shared(ctx, |shared| shared.close_requests.clear());
    follow_activity(ctx);
    follow_layout(ctx, viewports::current(ctx).focused);
    // The root window's placement is the app's `main` entry.
    if shared(ctx, |shared| shared.store.is_some()) {
        record_placement(ctx, ViewportId::ROOT, craft_window::restore::MAIN, &viewports::current(ctx));
    }
    flush_store(ctx);
}

/// Whether the root window was asked to close and nothing has cancelled it yet. Valid in the app's `logic`, which
/// eframe runs while the window is minimized or hidden too.
pub(crate) fn root_close_requested(ctx: &Context) -> bool {
    pass::current_viewport(ctx) == ViewportId::ROOT && viewports::current(ctx).close_requested && !viewports::close_cancelled(ctx)
}

/// Keeps the root window open and brings it forward, so the app can ask before it closes.
#[cfg_attr(not(feature = "app"), expect(dead_code, reason = "used by the app shell"))]
pub(crate) fn hold_close(ctx: &Context) {
    viewports::cancel_close(ctx);
    viewports::raise(ctx, ViewportId::ROOT);
}

/// The close path of the root window, for the app's `logic`: a close request goes to the platform quit where the
/// platform has one (the window stays for the quit to close); otherwise egui goes on as it would. Call it last in
/// `logic`, after the app's own close decision, so it sees the decision's `CancelClose`. It belongs in `logic`
/// because eframe runs no UI pass for a minimized or hidden window and closes it itself; on macOS that close is the
/// Touch Bar crash `craft_quit` exists to avoid. `app::run` calls it; apps that call `install` themselves do too.
pub fn logic(ctx: &Context) {
    if root_close_requested(ctx) && craft_quit::quit() {
        viewports::cancel_close(ctx);
    }
}

fn flush_store(ctx: &Context) {
    let now = pass::now(ctx);
    let store = shared(ctx, |shared| {
        if now - shared.last_flush < FLUSH_SECONDS {
            return None;
        }
        shared.last_flush = now;
        shared.store.clone()
    });
    if let Some(store) = store {
        let mut store = store.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if store.is_dirty()
            && let Err(error) = store.save()
        {
            log::warn!("window placement not saved: {error}");
        }
    }
}

/// Sends `action` to `viewport`. It is the only sender of window commands: it checks the capabilities and the
/// window's state; a root `Close` reaches the app's close decision and the platform quit in [`logic`]. Returns
/// whether a command went out.
pub fn act(ctx: &Context, viewport: ViewportId, action: WindowAction) -> bool {
    let base = base_shell(ctx);
    let commands = base.caps.contains(Caps::WINDOW_COMMANDS);
    let window = window_info(ctx, viewport, &base);
    let allowed = match action {
        WindowAction::Close => true,
        WindowAction::Focus => commands || base.caps.contains(Caps::VIEWPORTS),
        WindowAction::Minimize => commands && window.state != WindowState::Minimized,
        WindowAction::ToggleMaximize => commands && window.state != WindowState::Fullscreen,
        WindowAction::ToggleFullscreen
        | WindowAction::StartMove
        | WindowAction::StartResize(_)
        | WindowAction::SetOuterPosition(_)
        | WindowAction::SetInnerSize(_)
        | WindowAction::SetLevel(_)
        | WindowAction::SetTitle(_) => commands,
    };
    if !allowed {
        return false;
    }
    match action {
        WindowAction::Close => viewports::close(ctx, viewport),
        WindowAction::Focus => viewports::raise(ctx, viewport),
        WindowAction::Minimize => viewports::command(ctx, viewport, egui::ViewportCommand::Minimized(true)),
        WindowAction::ToggleMaximize => viewports::command(ctx, viewport, egui::ViewportCommand::Maximized(window.state != WindowState::Maximized)),
        WindowAction::ToggleFullscreen => {
            viewports::command(ctx, viewport, egui::ViewportCommand::Fullscreen(window.state != WindowState::Fullscreen))
        }
        WindowAction::StartMove => viewports::command(ctx, viewport, egui::ViewportCommand::StartDrag),
        WindowAction::StartResize(edge) => viewports::command(ctx, viewport, egui::ViewportCommand::BeginResize(viewports::resize_direction(edge))),
        WindowAction::SetOuterPosition(pos) => viewports::command(ctx, viewport, egui::ViewportCommand::OuterPosition(pos)),
        WindowAction::SetInnerSize(size) => viewports::command(ctx, viewport, egui::ViewportCommand::InnerSize(size)),
        WindowAction::SetLevel(level) => {
            PerViewport::<Record>::with(ctx, viewport, |record| record.level = Some(level));
            ctx.request_repaint_of(viewport);
        }
        WindowAction::SetTitle(title) => viewports::command(ctx, viewport, egui::ViewportCommand::Title(title)),
    }
    true
}

/// Keeps the app's activity current from the windows' focus. A palette's window level follows it: the mounts build
/// their viewports with the level of the moment, and a change asks for the next frame. Runs in the root pass, which
/// every frame of the app runs.
fn follow_activity(ctx: &Context) {
    let windows = viewports::reported(ctx);
    let now = pass::now(ctx);
    let focused = windows.iter().any(|(_, facts)| facts.focused);
    let (changed, waiting) = shared(ctx, |shared| {
        if focused {
            shared.last_focused = now;
        }
        let away = !focused && now - shared.last_focused >= AWAY_SECONDS;
        let waiting = !focused && !away;
        (std::mem::replace(&mut shared.away, away) != away, waiting)
    });
    if waiting {
        crate::runtime::egui::repaint::request_after(ctx, ViewportId::ROOT, AWAY_SECONDS);
    }
    if changed {
        ctx.request_repaint();
    }
}

/// On the root window's focus gain, re-reads the desktop's caption layout and gives the root frame the change.
fn follow_layout(ctx: &Context, focused: bool) {
    shared(ctx, |shared| {
        let gained = focused && !std::mem::replace(&mut shared.root_focused, focused);
        if !gained {
            return;
        }
        if let Some(layouts) = &mut shared.layouts
            && layouts.on_focus()
            && let Some(Frame::Integrated(captions)) = &mut shared.root_frame
        {
            captions.layout = layouts.layout().clone();
        }
    });
}

/// The multi-window driver of one context.
#[derive(Clone)]
pub struct WindowHost {
    ctx: Context,
}

impl WindowHost {
    /// The window host of `ctx`.
    pub fn of(ctx: &Context) -> Self {
        WindowHost { ctx: ctx.clone() }
    }

    /// The shell as the running viewport sees it.
    pub fn shell(&self) -> ShellInfo {
        shell_of(&self.ctx, pass::current_viewport(&self.ctx))
    }

    /// Whether OS viewports can open: the capability is on and the integration does not embed them.
    pub fn viewports_available(&self) -> bool {
        base_shell(&self.ctx).caps.contains(Caps::VIEWPORTS) && viewports::available(&self.ctx)
    }

    /// Whether the running viewport's window is drawn; a minimized or covered window runs no UI pass, so this reads
    /// false only from the app's `logic`.
    pub fn visible(&self) -> bool {
        viewports::current(&self.ctx).visible
    }

    /// Runs and paints the next pass even while the window is hidden; holds for one frame.
    pub fn paint_while_hidden(&self) {
        viewports::paint_while_hidden(&self.ctx);
    }

    /// Asks for one pass of `viewport` after `seconds`; a negative time means now, a non-finite one asks for
    /// nothing. An idle window draws no frames, so a timer that must fire asks for its own.
    pub fn repaint_in(&self, viewport: ViewportId, seconds: f64) {
        if seconds.is_finite() {
            crate::runtime::egui::repaint::request_after(&self.ctx, viewport, seconds);
        }
    }

    /// Sends `action` to `viewport`; see [`act`].
    pub fn act(&self, viewport: ViewportId, action: WindowAction) -> bool {
        act(&self.ctx, viewport, action)
    }

    /// The system pointer in desktop points, as the platform tells it or, failing that, as the window a drag
    /// started in saw it.
    pub fn desktop_pointer(&self) -> Option<Pos2> {
        shared(&self.ctx, |shared| shared.system_pointer.or(shared.source_pointer))
    }

    /// Moves the system pointer to `to` in desktop points; false where the platform cannot.
    pub fn warp_pointer(&self, to: Pos2) -> bool {
        let system = shared(&self.ctx, |shared| shared.system.clone());
        system.is_some_and(|system| system.warp_pointer([to.x, to.y]))
    }

    /// `viewport`'s window facts, once it has reported its inner rect.
    pub fn facts(&self, viewport: ViewportId) -> Option<WindowFacts> {
        let facts = PerViewport::<Record>::with(&self.ctx, viewport, |record| record.facts)?;
        let info = window_info(&self.ctx, viewport, &base_shell(&self.ctx));
        Some(WindowFacts {
            inner: facts.inner?,
            outer: facts.outer,
            scale: facts.native_scale.unwrap_or(1.0),
            state: info.state,
            focused: facts.focused,
        })
    }

    /// The connected monitors in desktop points; none where the platform does not list them.
    pub fn monitors(&self) -> Vec<MonitorInfo> {
        let system = shared(&self.ctx, |shared| shared.system.clone());
        system.map(|system| system.monitors()).unwrap_or_default().into_iter().map(monitor_info).collect()
    }

    /// Places `viewport`'s outer rect at `outer`: its position, and the inner size that leaves the window's
    /// decoration as it is. False when a command could not go out.
    pub fn set_outer(&self, viewport: ViewportId, outer: Rect) -> bool {
        let decoration = self.facts(viewport).and_then(|facts| facts.outer.map(|current| current.size() - facts.inner.size())).unwrap_or(Vec2::ZERO);
        let moved = act(&self.ctx, viewport, WindowAction::SetOuterPosition(outer.min));
        let sized = act(&self.ctx, viewport, WindowAction::SetInnerSize((outer.size() - decoration).max(Vec2::ZERO)));
        moved && sized
    }

    /// The viewport of the container `id` while its window is open.
    pub fn viewport_of(&self, id: Id) -> Option<ViewportId> {
        let viewport = viewports::viewport_id(id);
        PerViewport::<Record>::with(&self.ctx, viewport, |record| record.kind.is_some()).then_some(viewport)
    }

    /// The window under the desktop point `at`: the focused one where windows overlap, else the smallest.
    pub fn viewport_at(&self, at: Pos2) -> Option<ViewportId> {
        let mut under: Vec<(ViewportId, WindowFacts)> = viewports::reported(&self.ctx)
            .into_iter()
            .filter_map(|(viewport, _)| self.facts(viewport).map(|facts| (viewport, facts)))
            .filter(|(_, facts)| facts.outer.unwrap_or(facts.inner).contains(at))
            .collect();
        under.sort_by(|(_, a), (_, b)| {
            let area = |facts: &WindowFacts| {
                let rect = facts.outer.unwrap_or(facts.inner);
                rect.width() * rect.height()
            };
            b.focused.cmp(&a.focused).then(area(a).total_cmp(&area(b)))
        });
        under.first().map(|(viewport, _)| *viewport)
    }

    /// Who draws the running viewport's frame, and the caption buttons it shows.
    pub fn frame(&self) -> Frame {
        let viewport = pass::current_viewport(&self.ctx);
        if viewport == ViewportId::ROOT
            && let Some(frame) = shared(&self.ctx, |shared| shared.root_frame.clone())
        {
            return frame;
        }
        let captions = PerViewport::<Record>::with(&self.ctx, viewport, |record| record.captions.clone());
        frame_of(window_info(&self.ctx, viewport, &base_shell(&self.ctx)).chrome, captions)
    }

    /// The desktop pointer in the coordinates of `viewport`, when it lies inside that window.
    pub fn pointer_in(&self, viewport: ViewportId) -> Option<Pos2> {
        let desktop = self.desktop_pointer()?;
        let inner = PerViewport::<Record>::with(&self.ctx, viewport, |record| record.facts.and_then(|facts| facts.inner))?;
        inner.contains(desktop).then(|| Pos2::ZERO + (desktop - inner.min))
    }
}

/// A monitor of the window system in desktop points.
fn monitor_info(monitor: craft_window::restore::Monitor) -> MonitorInfo {
    let scale = monitor.scale.max(f32::EPSILON);
    let min = Pos2::new(monitor.pos_px[0] as f32 / scale, monitor.pos_px[1] as f32 / scale);
    let size = Vec2::new(monitor.size_px[0] as f32 / scale, monitor.size_px[1] as f32 / scale);
    MonitorInfo { name: monitor.key.name, rect: Rect::from_min_size(min, size), scale: monitor.scale }
}

/// Records the pointer of the viewport a drag runs in, `local` in that viewport's coordinates, as a desktop
/// position: the fallback where the platform tells no system pointer. `None` keeps the last position.
pub(crate) fn note_source_pointer(ctx: &Context, viewport: ViewportId, local: Option<Pos2>) {
    let inner = PerViewport::<Record>::with(ctx, viewport, |record| record.facts.and_then(|facts| facts.inner));
    if let Some(desktop) = local.zip(inner).map(|(local, inner)| inner.min + local.to_vec2()) {
        shared(ctx, |shared| shared.source_pointer = Some(desktop));
    }
}

/// Forgets the drag's pointer once the session ends.
pub(crate) fn clear_source_pointer(ctx: &Context) {
    shared(ctx, |shared| shared.source_pointer = None);
}

/// The placement saved under `key`.
pub(crate) fn saved_placement(ctx: &Context, key: &str) -> Option<Placement> {
    let store = shared(ctx, |shared| shared.store.clone())?;
    let store = store.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    store.get(key)
}

/// Saves `placement` under `key`; the end hook writes the file.
pub(crate) fn save_placement(ctx: &Context, key: &str, placement: &Placement) {
    let Some(store) = shared(ctx, |shared| shared.store.clone()) else { return };
    store.lock().unwrap_or_else(std::sync::PoisonError::into_inner).set(key, placement);
}

/// The outer position of a saved placement in desktop points.
fn saved_position(placement: &Placement) -> Option<Pos2> {
    let scale = placement.scale.max(f32::EPSILON);
    placement.position_px.map(|[x, y]| Pos2::new(x as f32 / scale, y as f32 / scale))
}

/// Keeps the window's placement in the store: its size and position unless it is maximized or full screen, which
/// keep the last normal size.
fn record_placement(ctx: &Context, viewport: ViewportId, key: &str, facts: &viewports::Facts) {
    let Some(inner) = facts.inner else { return };
    let geometry = Geometry { inner, outer: facts.outer, scale: facts.native_scale, maximized: facts.maximized, fullscreen: facts.fullscreen };
    let unchanged = PerViewport::<Record>::with(ctx, viewport, |record| record.placed.replace(geometry) == Some(geometry));
    if unchanged {
        return;
    }
    let scale = facts.native_scale.unwrap_or(1.0);
    let mut next = saved_placement(ctx, key).unwrap_or_else(|| Placement::new(inner.size().into(), scale));
    next.scale = scale;
    next.maximized = facts.maximized;
    next.fullscreen = facts.fullscreen;
    if !facts.maximized && !facts.fullscreen {
        next.normal.size = inner.size().into();
        if let Some(outer) = facts.outer {
            next.normal.pos = [outer.min.x, outer.min.y];
            next.position_px = Some([(outer.min.x * scale).round() as i32, (outer.min.y * scale).round() as i32]);
        }
    }
    save_placement(ctx, key, &next);
}

/// What a viewport mount asks for.
pub(crate) struct ViewportOpen<'a> {
    /// The container's id.
    pub id: Id,
    /// The window title.
    pub title: &'a str,
    /// The size to open with when nothing is saved.
    pub size: Option<Vec2>,
    /// The key the placement is saved under.
    pub save: Option<&'a str>,
    /// How the window opens.
    pub spec: &'a WindowSpec,
}

/// Opens the OS viewport of a mount and runs `body` once for each pass it gets: with the saved placement, the OS
/// close request turned into a request to the container, and the placement saved after the pass.
pub(crate) fn show_viewport(ctx: &Context, open: &ViewportOpen<'_>, mut body: impl FnMut(&mut Ui)) {
    let viewport = viewports::viewport_id(open.id);
    let saved = open.save.and_then(|key| saved_placement(ctx, key));
    let (size, pos) = match &saved {
        Some(placement) => (Some(Vec2::from(placement.normal.size)), saved_position(placement)),
        None => (open.size, None),
    };
    let spec = open.spec;
    let active = !shared(ctx, |shared| shared.away);
    let os = os_of(ctx);
    let level = PerViewport::<Record>::with(ctx, viewport, |record| {
        record.kind = Some(spec.kind);
        record.chrome = Some(spec.chrome);
        record.captions = spec.captions.clone();
        record.level.get_or_insert(spec.level).for_egui(active, os)
    });
    let opens = viewports::Spec {
        title: open.title,
        size,
        pos: pos.or(spec.position),
        chrome: spec.chrome,
        min_size: spec.min_size,
        max_size: spec.max_size,
        level,
        resizable: spec.resizable,
        taskbar: spec.taskbar,
    };
    viewports::show_immediate(ctx, viewport, &opens, |ui| {
        let ctx = ui.ctx().clone();
        refresh(&ctx, viewport, viewports::current(&ctx));
        if viewports::current(&ctx).close_requested {
            viewports::cancel_close(&ctx);
            shared(&ctx, |shared| {
                if !shared.close_requests.contains(&open.id) {
                    shared.close_requests.push(open.id);
                }
            });
        }
        body(ui);
        if let Some(key) = open.save {
            record_placement(&ctx, viewport, key, &viewports::current(&ctx));
        }
    });
}

/// Whether the OS asked the viewport of container `id` to close in this root frame; once.
pub(crate) fn take_close_request(ctx: &Context, id: Id) -> bool {
    shared(ctx, |shared| {
        let before = shared.close_requests.len();
        shared.close_requests.retain(|requested| *requested != id);
        shared.close_requests.len() != before
    })
}
