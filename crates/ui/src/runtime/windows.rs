//! `WindowHost`: the one multi-window driver. It owns `ShellInfo`, refreshes each viewport's `WindowInfo`, releases
//! a press after an OS resize (H4), raises a modal's window for a blocked press (H5), opens viewport mounts, keeps
//! window placement in `craft_window::restore::Store`, routes a root close through the platform quit from the app's
//! `logic`, and maps pointers to desktop space for drags that cross windows.

use crate::runtime::cursor;
use crate::runtime::egui::input::{self as raw_input, RawInput};
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::input_filter::{self, KeyEquivalent};
use crate::runtime::egui::{injected, pass, scale, screenshot, viewports, widgets};
use crate::runtime::frame::{PassStep, PerViewport, Stamped};
use crate::runtime::{overlay, pointer};
use craft_behavior::{Cursor, CursorIcon, CursorPriority, PartRef, PointerButton};
use craft_host::target::{Caps, DoubleClick, EdgeInsets, Os, ShellInfo, WindowChrome, WindowInfo, WindowKind, WindowState};
#[cfg(feature = "testing")]
use craft_host::target::{MenuPlacement, Runtime};
use craft_keymap::{Key, Mods};
use craft_window::Button;
use craft_window::policy::{CaptionFamily, Captions, Decision, Frame, LayoutWatch, MacInset};
use craft_window::restore::{Placement, Store};
use craft_window::system::WindowSystem;
use egui::{Context, Id, Pos2, Rect, Ui, Vec2, ViewportId};
use std::sync::{Arc, Mutex};

pub use crate::runtime::egui::screenshot::{ScreenshotError, ScreenshotFacts, ScreenshotSlot};

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

/// What a part of the window does for the operating system: a press on it moves the window, resizes it from an edge,
/// or is a caption button. A component tags a part with it through `PartBuilder::exclusive`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowHit {
    /// Drags the window.
    Move,
    /// Resizes the window from the edge or corner.
    Resize(ResizeEdge),
    /// A caption button.
    Caption(Button),
}

/// Where the exclusive parts of one window lie, in points of its viewport: the window procedure answers a Windows
/// hit test from them. Read it with [`WindowHost::caption_regions`].
#[derive(Clone, Debug, PartialEq, Default)]
#[non_exhaustive]
pub struct CaptionRegions {
    /// The caption buttons.
    pub buttons: Vec<(Button, Rect)>,
    /// The edge and corner zones.
    pub edges: Vec<(ResizeEdge, Rect)>,
    /// The areas that drag the window.
    pub moves: Vec<Rect>,
    /// For each drag area, at the same index in `moves`, the parts where an in-app hit lies over its band: an
    /// interactive widget shown above the band's component, such as a bar's button or menu. A press there goes to
    /// that widget under the press-target rule (8.3), so the window does not move from that area there. A cover
    /// belongs to its own area only: a drag area shown above another (a modal's strip over the title bar) covers
    /// the lower one and still answers itself.
    pub covers: Vec<Vec<Rect>>,
}

impl CaptionRegions {
    /// What lies at `point`: a button wins over an edge, and an edge over a drag area. A drag area answers only where
    /// none of its covers lies, so its band is the top in-app hit there.
    pub fn hit(&self, point: Pos2) -> Option<WindowHit> {
        let moves =
            || self.moves.iter().zip(&self.covers).any(|(rect, covers)| rect.contains(point) && !covers.iter().any(|cover| cover.contains(point)));
        self.buttons
            .iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(button, _)| WindowHit::Caption(*button))
            .or_else(|| self.edges.iter().find(|(_, rect)| rect.contains(point)).map(|(edge, _)| WindowHit::Resize(*edge)))
            .or_else(|| moves().then_some(WindowHit::Move))
    }
}

/// What a window's last complete pass published for its hit test: the regions and the scale they are in.
#[derive(Clone, Debug)]
struct Published {
    regions: CaptionRegions,
    /// Physical pixels per point of the viewport, interface zoom included.
    pixels_per_point: f32,
}

/// The Windows hit test of one window: the window procedure asks [`HitSink::answer`] where the OS asks for a hit
/// test, and the answer comes from the exclusive parts the window's last pass declared, so the caption buttons,
/// edges and drag areas the app draws are the ones the OS acts on (snap layouts, native move and resize). The sink
/// holds the scale with the regions, so the window procedure needs nothing from egui.
/// [`WindowHost::install_hit_sink`] makes the sink; the platform layer that owns the window procedure holds it.
#[derive(Clone, Debug, Default)]
pub struct HitSink {
    published: Arc<Mutex<Option<Published>>>,
}

impl HitSink {
    /// What lies at `pixel`, in physical pixels from the top left of the window's client area: the hit test's screen
    /// point after `ScreenToClient`. `None` where no exclusive part lies there: the window procedure leaves the hit
    /// test to the window's content.
    pub fn answer(&self, pixel: Pos2) -> Option<WindowHit> {
        let published = self.published.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Published { regions, pixels_per_point } = published.as_ref()?;
        regions.hit((pixel.to_vec2() / *pixels_per_point).to_pos2())
    }

    fn publish(&self, published: Option<Published>) {
        *self.published.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = published;
    }
}

/// The regions the parts `parts` of the running pass declare; `None` for none. A drag area's covers are read from
/// the pass's widgets, so this runs at the end of the pass.
fn regions_of(ctx: &Context, viewport: ViewportId, parts: &[ExclusivePart]) -> Option<CaptionRegions> {
    if parts.is_empty() {
        return None;
    }
    let mut regions = CaptionRegions::default();
    for ExclusivePart { owner, part, hit, rect } in parts {
        match hit {
            WindowHit::Move => {
                regions.moves.push(*rect);
                regions.covers.push(covers_of(ctx, viewport, *owner, *part, *rect));
            }
            WindowHit::Resize(edge) => regions.edges.push((*edge, *rect)),
            WindowHit::Caption(button) => regions.buttons.push((*button, *rect)),
        }
    }
    Some(regions)
}

/// The parts of the drag area `band`, the part `part` of the component `owner`, where an in-app hit lies over it,
/// clipped to the band: each interactive egui widget of the running pass that egui's hit test puts above the
/// component's widgets, and each of the component's own shapes that a press there goes to instead of the band (a
/// member over it, a higher part). The component's widgets resolve as the component (8.3), so they are not hits
/// over the band themselves.
fn covers_of(ctx: &Context, viewport: ViewportId, owner: Id, part: PartRef, band: Rect) -> Vec<Rect> {
    let over = pointer::over_part(ctx, viewport, owner, part);
    widgets::above(ctx, owner, &over.widgets)
        .into_iter()
        .chain(over.above)
        .map(|cover| cover.intersect(band))
        .filter(|cover| cover.is_positive())
        .collect()
}

/// Keeps the regions of the running pass's exclusive parts for [`WindowHost::caption_regions`] and gives them to
/// the viewport's hit-test sink.
fn publish_regions(ctx: &Context, viewport: ViewportId) {
    let parts = with_exclusive(ctx, viewport, |exclusive| exclusive.current.clone());
    let regions = regions_of(ctx, viewport, &parts);
    with_exclusive(ctx, viewport, |exclusive| exclusive.done = regions.clone());
    if let Some(sink) = PerViewport::<Record>::with(ctx, viewport, |record| record.sink.clone()) {
        let pixels_per_point = scale::pixels_per_point(ctx);
        sink.publish(regions.map(|regions| Published { regions, pixels_per_point }));
    }
}

/// Whether the platform answers the hit test of `viewport`'s window from the app's exclusive parts.
pub(crate) fn hit_sink_installed(ctx: &Context, viewport: ViewportId) -> bool {
    PerViewport::<Record>::with(ctx, viewport, |record| record.sink.is_some())
}

/// An exclusive part as its component declared it.
#[derive(Clone, Debug)]
struct ExclusivePart {
    owner: Id,
    part: PartRef,
    hit: WindowHit,
    rect: Rect,
}

/// A viewport's exclusive parts: those the running pass declared so far, and the regions of its last complete pass.
#[derive(Default)]
struct Exclusive {
    current: Vec<ExclusivePart>,
    done: Option<CaptionRegions>,
}

fn with_exclusive<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut Exclusive) -> R) -> R {
    PerViewport::<Stamped<Exclusive>>::with_pass(
        ctx,
        viewport,
        pass::pass_nr(ctx, viewport),
        |exclusive, step| {
            exclusive.current.clear();
            if matches!(step, PassStep::Skipped) {
                exclusive.done = None;
            }
        },
        f,
    )
}

/// Records that `part` of the component `owner` is, over `rect`, what `hit` says for the running viewport's window.
/// A part declared again in the same pass replaces its entry.
pub(crate) fn note_exclusive(ctx: &Context, owner: Id, part: PartRef, hit: WindowHit, rect: Rect) {
    with_exclusive(ctx, pass::current_viewport(ctx), |exclusive| {
        let entry = ExclusivePart { owner, part, hit, rect };
        match exclusive.current.iter_mut().find(|held| held.owner == owner && held.part == part) {
            Some(held) => *held = entry,
            None => exclusive.current.push(entry),
        }
    });
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

/// How an OS window opens (21.2): what it is for, who draws its frame, where it stands and whether it has a taskbar
/// entry. Its size limits, position and resizability are the container's fallback `Overlay`'s (`size_limits`,
/// `placed`, `resizable`).
///
/// A chrome or caption layout the spec leaves unset is the installed decision's for the window's kind
/// (`InstallSpec::frame`); with no decision installed the window has the OS's frame.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WindowSpec {
    /// What the window is for.
    pub kind: WindowKind,
    /// Who draws the frame; unset takes the installed decision's.
    pub chrome: Option<WindowChrome>,
    /// The caption buttons the app draws when the chrome is `Integrated`. Unset takes the installed decision's when
    /// `chrome` is unset too, and none otherwise.
    pub captions: Option<Captions>,
    /// Where the window stands among the windows.
    pub level: WindowLevel,
    /// Whether the window has a taskbar or Dock entry.
    pub taskbar: bool,
    /// Whether the window opens hidden and appears once its first pass has been laid out and painted.
    pub shown_after_first_paint: bool,
}

impl WindowSpec {
    /// A tool window (a native dock window): the installed decision's frame without caption buttons, a palette level,
    /// no taskbar entry.
    pub fn tool() -> Self {
        WindowSpec { kind: WindowKind::Tool, level: WindowLevel::Palette, taskbar: false, ..Self::dialog() }
    }

    /// A document window with the installed decision's frame.
    pub fn document() -> Self {
        WindowSpec { kind: WindowKind::Document, ..Self::dialog() }
    }

    /// A dialog window with the installed decision's frame.
    pub fn dialog() -> Self {
        WindowSpec {
            kind: WindowKind::Dialog,
            chrome: None,
            captions: None,
            level: WindowLevel::Normal,
            taskbar: true,
            shown_after_first_paint: false,
        }
    }

    /// Sets what the window is for.
    #[must_use]
    pub fn kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets who draws the frame, whatever the installed decision says.
    #[must_use]
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = Some(chrome);
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
        self.chrome = Some(chrome_of(frame));
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

    /// Sets whether the window has a taskbar or Dock entry.
    #[must_use]
    pub fn taskbar(mut self, taskbar: bool) -> Self {
        self.taskbar = taskbar;
        self
    }

    /// Sets whether the window opens hidden and appears after its first paint, so a window that shows a capture or
    /// other content that must be ready does not flash an empty frame.
    #[must_use]
    pub fn shown_after_first_paint(mut self, hidden_until_painted: bool) -> Self {
        self.shown_after_first_paint = hidden_until_painted;
        self
    }
}

/// The frame the installed decision gives a window of `kind`: a tool window shows no caption buttons. `None` while
/// no decision is installed.
fn installed_frame(ctx: &Context, kind: WindowKind) -> Option<Frame> {
    let frame = shared(ctx, |shared| shared.root_frame.clone())?;
    Some(match frame {
        Frame::Integrated(captions) if kind == WindowKind::Tool => {
            Frame::Integrated(Captions { family: captions.family, layout: craft_window::Layout { left: Vec::new(), right: Vec::new() } })
        }
        frame => frame,
    })
}

/// The chrome and caption buttons `spec` opens with: its own, else the installed decision's for its kind, else the
/// OS's frame.
fn frame_of_spec(ctx: &Context, spec: &WindowSpec) -> (WindowChrome, Option<Captions>) {
    let decided = installed_frame(ctx, spec.kind);
    let chrome = spec.chrome.or_else(|| decided.as_ref().map(chrome_of)).unwrap_or(WindowChrome::Native);
    let captions = spec.captions.clone().or_else(|| match decided {
        Some(Frame::Integrated(captions)) if spec.chrome.is_none() && !captions.is_empty() => Some(captions),
        _ => None,
    });
    (chrome, captions)
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
    /// Enters full screen (`true`) or leaves it. Unlike [`WindowAction::ToggleMaximize`] it is not refused while the
    /// window is in full screen, so a sequence of actions can be sent in one pass.
    SetFullscreen(bool),
    /// Maximizes (`true`) or restores the window, whatever state it reports.
    SetMaximized(bool),
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

/// What a double click on a window's free move band (its title bar, or the strip a modal leaves over it) asks of the
/// window in `frame`: an integrated frame maximizes or restores; a macOS inset frame follows the system setting
/// `setting`; a native frame is the OS's.
pub fn double_click_action(frame: &Frame, setting: DoubleClick, maximized: bool) -> Option<WindowAction> {
    match (frame, setting) {
        (Frame::Native, _) => None,
        (Frame::Integrated(_), _) | (Frame::Inset(_), DoubleClick::Zoom) => Some(WindowAction::SetMaximized(!maximized)),
        (Frame::Inset(_), DoubleClick::Minimize) => Some(WindowAction::Minimize),
        _ => None,
    }
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

/// A connected monitor, in physical pixels of the desktop.
///
/// Window facts are in points of the scale of the monitor their window is on, so the points of two windows on
/// monitors of different scales are different spaces and a monitor has no single rect in points. A caller converts
/// at the use site with [`MonitorInfo::points_at`], passing the `scale` of the window it compares with.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MonitorInfo {
    /// The name the platform reports.
    pub name: String,
    /// The monitor's area in physical pixels.
    pub pixels: Rect,
    /// Physical pixels per point of this monitor.
    pub scale: f32,
}

impl MonitorInfo {
    /// The monitor's area in points of a window whose scale is `window_scale` ([`WindowFacts::scale`]). Monitors
    /// that touch in pixels touch in these points, with no gap or overlap.
    pub fn points_at(&self, window_scale: f32) -> Rect {
        let scale = window_scale.max(f32::EPSILON);
        Rect::from_min_max(self.pixels.min / scale, self.pixels.max / scale)
    }

    /// The monitor as the placement owner's type, in the points of a window whose scale is `window_scale`.
    pub fn placed_at(&self, window_scale: f32) -> craft_behavior::Monitor {
        craft_behavior::Monitor::new(Some(self.name.clone()), self.points_at(window_scale), self.scale)
    }
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
    /// Whether no window of the app had focus when the last frame's windows had all reported.
    away: bool,
    /// The app's windows from the one focused longest ago to the one focused last: the stacking order of
    /// overlapping windows.
    stacking: Vec<ViewportId>,
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
    /// Whether the user can resize the window; unknown for a window not opened by `show_viewport`.
    resizable: Option<bool>,
    /// Whether the window was opened hidden and its first pass has not run yet.
    awaiting_first_paint: bool,
    /// The last OS move or resize the window was handed.
    handoff: Option<HandOff>,
    /// The geometry last written to the placement store, so an unchanged window writes nothing.
    placed: Option<Geometry>,
    last_pos: Option<Pos2>,
    last_inner_size: Option<Vec2>,
    /// The window procedure's hit test, once the platform layer installed it.
    sink: Option<HitSink>,
}

/// An OS move or resize started in the pass `pass`: the OS keeps the press and sends no release. `released` is the
/// pass whose input the runtime ended the press in.
#[derive(Clone, Copy)]
struct HandOff {
    pass: u64,
    released: Option<u64>,
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
        let root_chrome = shared.root_frame.as_ref().map(chrome_of);
        let shell = shared.shell.get_or_insert_with(ShellInfo::default);
        shell.os = os;
        if shell.runtime == Runtime::Native {
            shell.caps = Caps::native(os);
            shell.menu = menu_placement(shell.caps, root_chrome.unwrap_or(shell.window.chrome));
        }
    });
}

/// Makes the root window's frame `chrome`, as a host with that frame reports: a native build's menu placement
/// follows it. The installed decision, when there is one, decides the chrome instead.
#[cfg(feature = "testing")]
pub(crate) fn set_chrome(ctx: &Context, chrome: WindowChrome) {
    shared(ctx, |shared| {
        let shell = shared.shell.get_or_insert_with(ShellInfo::default);
        std::sync::Arc::make_mut(&mut shell.window).chrome = chrome;
        if shell.runtime == Runtime::Native {
            shell.menu = menu_placement(shell.caps, chrome);
        }
    });
}

/// Where a native build's menu bar lives: at the top of the screen when the OS has one and the OS draws the window's
/// frame around it, else in the window (an app-drawn frame carries the bar itself).
#[cfg(feature = "testing")]
fn menu_placement(caps: Caps, chrome: WindowChrome) -> MenuPlacement {
    if caps.contains(Caps::NATIVE_MENU_BAR) && chrome != WindowChrome::Integrated { MenuPlacement::ScreenTop } else { MenuPlacement::InWindow }
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

/// H4, in the input hook. The OS takes the button release of a move or resize it runs, so the press stays down in
/// egui and every later press is lost. Two cases end it with a release of the primary button, in any chrome:
/// the first input after [`act`] handed the OS a move or resize, when the input brings no release of its own (where
/// the OS sends one, a second would turn into a click); and, where the OS resizes undecorated windows from their
/// edges ([`Caps::OS_RESIZES_UNDECORATED`]), a size change under a held primary press. Reads only the raw input and
/// the viewport's record. Returns whether it added an event.
pub(crate) fn input_hook(ctx: &Context, input: &mut RawInput) -> bool {
    let viewport = input.viewport_id;
    let base = base_shell(ctx);
    let undecorated = base.caps.contains(Caps::OS_RESIZES_UNDECORATED) && window_info(ctx, viewport, &base).chrome == WindowChrome::Integrated;
    let size = viewports::inner_size_in(input, viewport);
    let pass = pass::pass_nr(ctx, viewport);
    let os_released =
        input.events.iter().any(|event| matches!(event, egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: false, .. }));
    PerViewport::<Record>::with(ctx, viewport, |record| {
        let resized = matches!((record.last_inner_size, size), (Some(before), Some(now)) if before != now);
        record.last_inner_size = size.or(record.last_inner_size);
        let handed_off = record.handoff.is_some_and(|handoff| handoff.released.is_none() && handoff.pass < pass);
        if handed_off {
            record.handoff = record.handoff.map(|handoff| HandOff { released: Some(pass), ..handoff });
        }
        let release = record.primary_down && ((undecorated && resized) || (handed_off && !os_released));
        if release {
            raw_input::push_primary_release(input, record.last_pos.unwrap_or(Pos2::ZERO));
        }
        raw_input::follow_primary(&input.events, &mut record.primary_down, &mut record.last_pos);
        release
    })
}

/// The begin hook (H5), where the viewport is current: refreshes its window info, and raises the modal's window
/// when the input hook dropped a press under a modal in another viewport.
pub(crate) fn begin(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    refresh(ctx, viewport, viewports::current(ctx));
    reshow_cursor(ctx, viewport);
    if overlay::take_dropped_press(ctx, viewport)
        && let Some(modal) = overlay::modal_viewport(ctx, viewport)
    {
        act(ctx, modal, WindowAction::Focus);
    }
}

/// The compositor owns the pointer during an OS move or resize and sets its own cursor, and egui-winit only sends a
/// cursor that changed, so afterwards the window keeps the compositor's cursor. On the first pointer motion after the
/// hand-off's press was ended, asks for the default cursor for one pass, so the cursor the pointer is over is sent
/// again on the next.
fn reshow_cursor(ctx: &Context, viewport: ViewportId) {
    if !ctx.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::PointerMoved(_)))) {
        return;
    }
    let pass = pass::pass_nr(ctx, viewport);
    let due = PerViewport::<Record>::with(ctx, viewport, |record| match record.handoff {
        Some(HandOff { released: Some(released), .. }) if released < pass => record.handoff.take().is_some(),
        _ => false,
    });
    if due {
        cursor::request(ctx, Cursor::System(CursorIcon::Default), CursorPriority::Drag);
        ctx.request_repaint_of(viewport);
    }
}

/// The end hook: keeps the root window's placement, writes a changed placement store once a second, and drops the
/// close requests the viewport mounts did not take in this frame.
pub(crate) fn end(ctx: &Context) {
    publish_regions(ctx, pass::current_viewport(ctx));
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
    let resizable = PerViewport::<Record>::with(ctx, viewport, |record| record.resizable != Some(false));
    let allowed = match action {
        WindowAction::Close => true,
        WindowAction::Focus => commands || base.caps.contains(Caps::VIEWPORTS),
        WindowAction::Minimize => commands && window.state != WindowState::Minimized,
        WindowAction::ToggleMaximize => commands && window.state != WindowState::Fullscreen,
        WindowAction::StartResize(_) => commands && resizable,
        WindowAction::ToggleFullscreen
        | WindowAction::SetFullscreen(_)
        | WindowAction::SetMaximized(_)
        | WindowAction::StartMove
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
        WindowAction::SetFullscreen(on) => viewports::command(ctx, viewport, egui::ViewportCommand::Fullscreen(on)),
        WindowAction::SetMaximized(on) => viewports::command(ctx, viewport, egui::ViewportCommand::Maximized(on)),
        WindowAction::StartMove => {
            viewports::command(ctx, viewport, egui::ViewportCommand::StartDrag);
            hand_off(ctx, viewport);
        }
        WindowAction::StartResize(edge) => {
            viewports::command(ctx, viewport, egui::ViewportCommand::BeginResize(viewports::resize_direction(edge)));
            hand_off(ctx, viewport);
        }
        WindowAction::SetOuterPosition(pos) => viewports::command(ctx, viewport, egui::ViewportCommand::OuterPosition(pos)),
        WindowAction::SetInnerSize(size) => viewports::command(ctx, viewport, egui::ViewportCommand::InnerSize(size)),
        WindowAction::SetLevel(level) => {
            // A window opened by `show_viewport` gets its level with the next build of its viewport; any other (the
            // root) is told now.
            let opened = PerViewport::<Record>::with(ctx, viewport, |record| {
                record.level = Some(level);
                record.kind.is_some()
            });
            if opened {
                ctx.request_repaint_of(viewport);
            } else {
                let active = !shared(ctx, |shared| shared.away);
                viewports::command(ctx, viewport, egui::ViewportCommand::WindowLevel(level.for_egui(active, os_of(ctx))));
            }
        }
        WindowAction::SetTitle(title) => viewports::command(ctx, viewport, egui::ViewportCommand::Title(title)),
    }
    true
}

/// The OS took the press that started a move or resize of `viewport`'s window: [`input_hook`] ends it on the next
/// input.
fn hand_off(ctx: &Context, viewport: ViewportId) {
    let pass = pass::pass_nr(ctx, viewport);
    PerViewport::<Record>::with(ctx, viewport, |record| record.handoff = Some(HandOff { pass, released: None }));
}

/// Keeps the app's activity and the stacking order of its windows current from the windows' focus. A palette's
/// window level follows the activity: the mounts build their viewports with the level of the moment, and a change
/// asks for the next frame. Runs at the end of the root pass, inside which every child window's pass of the frame
/// has run, so each window's focus is the one its own pass reported after its input batch. The root input's list of
/// windows holds each child as of its previous pass, so it is not read for focus. The app is inactive when no
/// window reports focus: the batch that took focus from one window gave it to no other.
fn follow_activity(ctx: &Context) {
    let windows = viewports::reported(ctx);
    let focused: Vec<ViewportId> = windows
        .iter()
        .map(|(viewport, _)| *viewport)
        .filter(|viewport| PerViewport::<Record>::with(ctx, *viewport, |record| record.facts.is_some_and(|facts| facts.focused)))
        .collect();
    let changed = shared(ctx, |shared| {
        shared.stacking.retain(|viewport| windows.iter().any(|(open, _)| open == viewport) && !focused.contains(viewport));
        shared.stacking.extend(&focused);
        let away = focused.is_empty();
        std::mem::replace(&mut shared.away, away) != away
    });
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

    /// The viewport the running pass is for.
    pub fn current(&self) -> ViewportId {
        pass::current_viewport(&self.ctx)
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
        viewports::paint_while_hidden(&self.ctx, crate::runtime::egui::pass::current_viewport(&self.ctx));
    }

    /// Issues one screenshot of `viewport`: a fresh slot bounded to `max_image_bytes` for its one image, and a pass of
    /// the viewport even while its window is hidden. The bound covers the single retained image; the application
    /// reserves its aggregate budget before asking.
    pub fn screenshot(&self, viewport: ViewportId, max_image_bytes: usize) -> ScreenshotSlot {
        screenshot::issue(&self.ctx, viewport, max_image_bytes)
    }

    /// What a screenshot of the running viewport measures this pass.
    pub fn screenshot_facts(&self) -> ScreenshotFacts {
        screenshot::facts(&self.ctx)
    }

    /// Queues a pointer move to `at` in `viewport`, delivered on a pass of its own.
    pub fn inject_move(&self, viewport: ViewportId, at: Pos2) {
        injected::inject_move(&self.ctx, viewport, at);
    }

    /// Queues a move to `at`, then `count` presses and releases of `button`, one pass each; a button egui has no
    /// counterpart for queues nothing.
    pub fn inject_click(&self, viewport: ViewportId, at: Pos2, button: PointerButton, count: u32) {
        injected::inject_click(&self.ctx, viewport, at, button, count);
    }

    /// Queues `key` pressed and released with `mods` held, as the platform reports them.
    pub fn inject_key(&self, viewport: ViewportId, key: Key, mods: Mods) {
        injected::inject_key(&self.ctx, viewport, key, mods);
    }

    /// Adds `equivalent` to the next input of `viewport`: a key equivalent the platform's menu took from the window
    /// reaches the application as the key press the user made.
    pub fn key_equivalent(&self, viewport: ViewportId, equivalent: KeyEquivalent) {
        input_filter::add_key_equivalent(&self.ctx, viewport, equivalent);
    }

    /// Queues `text` as typed text.
    pub fn inject_text(&self, viewport: ViewportId, text: &str) {
        injected::inject_text(&self.ctx, viewport, text);
    }

    /// Drops the injected queue of `viewport` (the document changed); a pending queue counts as withdrawn
    /// ([`take_injected_withdrawn`](Self::take_injected_withdrawn)).
    pub fn withdraw_injected(&self, viewport: ViewportId) {
        injected::withdraw(&self.ctx, viewport);
    }

    /// Drops every injected queue and step in flight; true when anything was pending.
    pub fn cancel_injected(&self) -> bool {
        injected::cancel(&self.ctx)
    }

    /// Whether no injected step is queued or awaits its pass: the application may answer its waiters.
    pub fn injected_idle(&self) -> bool {
        injected::idle(&self.ctx)
    }

    /// Whether an injected step was withdrawn since the last call.
    pub fn take_injected_withdrawn(&self) -> bool {
        injected::take_withdrawn(&self.ctx)
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

    /// The connected monitors in physical pixels; none where the platform does not list them.
    pub fn monitors(&self) -> Vec<MonitorInfo> {
        let system = shared(&self.ctx, |shared| shared.system.clone());
        system.map(|system| system.monitors()).unwrap_or_default().into_iter().map(monitor_info).collect()
    }

    /// The connected monitors as the placement owner's type, in the points of a window whose scale is
    /// `window_scale` ([`WindowFacts::scale`]).
    pub fn monitors_at(&self, window_scale: f32) -> Vec<craft_behavior::Monitor> {
        self.monitors().iter().map(|monitor| monitor.placed_at(window_scale)).collect()
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

    /// The window under the desktop point `at`: where windows overlap, the focused one, else the one focused last,
    /// else the smallest.
    pub fn viewport_at(&self, at: Pos2) -> Option<ViewportId> {
        self.viewports_at(at).first().copied()
    }

    /// The windows under the desktop point `at`, the topmost first (the order of [`Self::viewport_at`]).
    pub fn viewports_at(&self, at: Pos2) -> Vec<ViewportId> {
        let stacking = shared(&self.ctx, |shared| shared.stacking.clone());
        let mut under: Vec<(ViewportId, WindowFacts)> = viewports::reported(&self.ctx)
            .into_iter()
            .filter_map(|(viewport, _)| self.facts(viewport).map(|facts| (viewport, facts)))
            .filter(|(_, facts)| facts.outer.unwrap_or(facts.inner).contains(at))
            .collect();
        let area = |facts: &WindowFacts| {
            let rect = facts.outer.unwrap_or(facts.inner);
            rect.width() * rect.height()
        };
        let height = |viewport: &ViewportId| stacking.iter().position(|stacked| stacked == viewport).map_or(0, |at| at + 1);
        under.sort_by(|(a_id, a), (b_id, b)| b.focused.cmp(&a.focused).then(height(b_id).cmp(&height(a_id))).then(area(a).total_cmp(&area(b))));
        under.into_iter().map(|(viewport, _)| viewport).collect()
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

    /// Where the exclusive parts of `viewport`'s window lay in its last complete pass: the caption buttons, the edge
    /// zones and the drag areas. `None` while the window declared none.
    pub fn caption_regions(&self, viewport: ViewportId) -> Option<CaptionRegions> {
        with_exclusive(&self.ctx, viewport, |exclusive| exclusive.done.clone())
    }

    /// The hit-test sink of `viewport`'s window, made on the first call. The platform layer calls it once it has put
    /// its window procedure in place and holds the sink there; from then on the platform answers that window's move,
    /// resize and caption hits. The window's move and resize parts still register, as the exclusive parts the sink
    /// reads, and a press on them starts no app move.
    pub fn install_hit_sink(&self, viewport: ViewportId) -> HitSink {
        PerViewport::<Record>::with(&self.ctx, viewport, |record| record.sink.get_or_insert_with(HitSink::default).clone())
    }

    /// The desktop pointer in the coordinates of `viewport`, when it lies inside that window.
    pub fn pointer_in(&self, viewport: ViewportId) -> Option<Pos2> {
        let desktop = self.desktop_pointer()?;
        let inner = PerViewport::<Record>::with(&self.ctx, viewport, |record| record.facts.and_then(|facts| facts.inner))?;
        inner.contains(desktop).then(|| Pos2::ZERO + (desktop - inner.min))
    }
}

/// A monitor of the window system, in physical pixels.
fn monitor_info(monitor: craft_window::restore::Monitor) -> MonitorInfo {
    let min = Pos2::new(monitor.pos_px[0] as f32, monitor.pos_px[1] as f32);
    let size = Vec2::new(monitor.size_px[0] as f32, monitor.size_px[1] as f32);
    MonitorInfo { name: monitor.key.name, pixels: Rect::from_min_size(min, size), scale: monitor.scale }
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
    /// The smallest and largest inner size, from the container's overlay.
    pub limits: (Option<Vec2>, Option<Vec2>),
    /// The outer position in desktop points when nothing is saved, from the container's overlay.
    pub position: Option<Pos2>,
    /// Whether the user can resize the window, from the container's overlay.
    pub resizable: bool,
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
    let (chrome, captions) = frame_of_spec(ctx, spec);
    let level = PerViewport::<Record>::with(ctx, viewport, |record| {
        record.kind = Some(spec.kind);
        record.chrome = Some(chrome);
        record.captions = captions.clone();
        record.resizable = Some(open.resizable);
        record.awaiting_first_paint |= spec.shown_after_first_paint && record.facts.is_none();
        record.level.get_or_insert(spec.level).for_egui(active, os)
    });
    let opens = viewports::Spec {
        title: open.title,
        size,
        pos: pos.or(open.position),
        chrome,
        // A tool window has no OS caption buttons; its panel's title row carries a close action.
        os_buttons: !(spec.kind == WindowKind::Tool && captions.is_none() && chrome != WindowChrome::Native),
        min_size: open.limits.0,
        max_size: open.limits.1,
        level,
        resizable: open.resizable,
        taskbar: spec.taskbar,
        hidden: PerViewport::<Record>::with(ctx, viewport, |record| record.awaiting_first_paint),
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
    if PerViewport::<Record>::with(ctx, viewport, |record| std::mem::take(&mut record.awaiting_first_paint)) {
        viewports::show(ctx, viewport);
    }
}

/// Whether the OS asked the viewport of container `id` to close in this root frame; once.
pub(crate) fn take_close_request(ctx: &Context, id: Id) -> bool {
    shared(ctx, |shared| {
        let before = shared.close_requests.len();
        shared.close_requests.retain(|requested| *requested != id);
        shared.close_requests.len() != before
    })
}
