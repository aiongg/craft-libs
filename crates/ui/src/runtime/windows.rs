//! `WindowHost`: the one multi-window driver. It owns `ShellInfo`, refreshes each viewport's `WindowInfo`, releases
//! a press after an OS resize (H4), raises a modal's window for a blocked press (H5), opens viewport mounts, keeps
//! window placement in `craft_window::restore::Store`, routes a root close through the platform quit from the app's
//! `logic`, and maps pointers to desktop space for drags that cross windows.

use crate::runtime::egui::input::{self as raw_input, RawInput};
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::{pass, viewports};
use crate::runtime::frame::PerViewport;
use crate::runtime::overlay;
use craft_host::target::{Caps, EdgeInsets, MenuPlacement, Os, Runtime, ShellInfo, WindowChrome, WindowInfo, WindowKind, WindowState};
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

/// A window command `act` sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Service::<Shared>::of(ctx).with(f)
}

/// Called by `install`: keeps the shell, the OS services and the placement store.
pub(crate) fn install(ctx: &Context, shell: ShellInfo, system: Arc<dyn WindowSystem>, store: Option<Arc<Mutex<Store>>>) {
    shared(ctx, |shared| {
        shared.shell = Some(shell);
        shared.system = Some(system);
        shared.store = store;
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
    let (facts, kind, chrome, insets) =
        PerViewport::<Record>::with(ctx, viewport, |record| (record.facts, record.kind, record.chrome, record.insets));
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
        WindowAction::ToggleFullscreen | WindowAction::StartMove | WindowAction::StartResize(_) => commands,
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
    }
    true
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

    /// The desktop pointer in the coordinates of `viewport`, when it lies inside that window.
    pub fn pointer_in(&self, viewport: ViewportId) -> Option<Pos2> {
        let desktop = self.desktop_pointer()?;
        let inner = PerViewport::<Record>::with(&self.ctx, viewport, |record| record.facts.and_then(|facts| facts.inner))?;
        inner.contains(desktop).then(|| Pos2::ZERO + (desktop - inner.min))
    }
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
    /// What the window is for.
    pub kind: WindowKind,
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
    PerViewport::<Record>::with(ctx, viewport, |record| record.kind = Some(open.kind));
    let spec = viewports::Spec { title: open.title, size, pos, decorated: true };
    viewports::show_immediate(ctx, viewport, &spec, |ui| {
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
