//! Windows on craft-ui-rt (docs/contract-rt.md sections 3.4 and 10, "Windows").
//!
//! The platform owns the OS windows (`Platform::create_window`, `WindowCommand`); the runtime owns
//! each window's nodes, input and the app-modal block. This module is what the app host adds:
//!
//! - **Open and close.** A view opens a window with [`Ui::open_window`] (queued; the driver opens it
//!   after the step and the host builds its root with `step_with`, which names the window). A
//!   window closes through one path: the close button, Alt+F4 and [`Runtime::quit`] all end in
//!   `Runtime::request_close`. A window with a close guard (a component whose
//!   `Component::close_guard` is true) asks it and stays open until the guard's view calls
//!   [`Ui::close_window`]; any other window closes at once. A window blocked by another window's
//!   app-modal dialog refuses and the dialog's window takes focus.
//! - **Placement.** [`WindowRegistry`] opens a keyed window at its saved placement (repaired against
//!   the monitors) and keeps the placement current from the geometry the runtime tracks.
//! - **Windows chrome.** Parts a component tags with `PartBuilder::exclusive` become window zones
//!   (ui-rt `PaintCx::window_zone`). The registry publishes them to the window's [`HitSink`]; the
//!   window procedure answers the OS hit test from it, and caption events it reports come back as
//!   pointer events on the button.

use std::collections::BTreeMap;

use craft_behavior::placement::{Defaults, Placement, monitor_of, place};
use craft_behavior::{Monitor, MonitorKey};
use craft_host::target::{WindowChrome, WindowKind};
use craft_ui_rt::{
    MonitorInfo, NativeWindow, PlatformEvent, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, Rect, Runtime, SurfaceHost, Vec2, Waker,
    WindowCommand, WindowEvent, WindowId, WindowLevel, WindowZone,
};
use craft_window::Button;
use craft_window::native::CaptionEvent;
use craft_window::policy::Frame;
use craft_window::restore::Store;

use super::ui::Ui;
use crate::runtime::windows::{CaptionRegions, HitSink, ResizeEdge, WindowHit, WindowSpec};

impl Ui<'_, '_> {
    /// Asks the platform to close this window now: a close guard's view calls it once the app
    /// agrees (no unsaved changes, or after the user's answer).
    pub fn close_window(&mut self) {
        self.cx.window_command(WindowCommand::Close);
    }

    /// Asks for a new window; the driver opens it after this step. The host's `roots` closure
    /// builds its content when `step_with` names its id.
    /// The window carries the app's id (`AppSpec::app_id`, from the root env), as the main window does, so the platform
    /// groups it with the app's other windows and finds its icon by it.
    pub fn open_window(&mut self, mut spec: craft_ui_rt::WindowSpec) {
        if spec.app_id.is_none()
            && let Some(cells) = super::EnvCells::of(self.cx)
        {
            spec.app_id = self.cx.peek_state(cells.root).app.as_ref().map(|app| app.as_str().to_owned());
        }
        self.cx.open_inline_window(spec);
    }
}

const TAG_MOVE: u64 = 1;
const TAG_EDGE: u64 = 0x10;
const TAG_CAPTION: u64 = 0x20;

const EDGES: [ResizeEdge; 8] = [
    ResizeEdge::Top,
    ResizeEdge::Bottom,
    ResizeEdge::Left,
    ResizeEdge::Right,
    ResizeEdge::TopLeft,
    ResizeEdge::TopRight,
    ResizeEdge::BottomLeft,
    ResizeEdge::BottomRight,
];
const BUTTONS: [Button; 3] = [Button::Minimize, Button::Maximize, Button::Close];

/// The ui-rt zone tag of `hit`: 0 for a hit this module does not know.
pub(crate) fn zone_tag(hit: WindowHit) -> u64 {
    match hit {
        WindowHit::Move => TAG_MOVE,
        WindowHit::Resize(edge) => EDGES.iter().position(|e| *e == edge).map_or(0, |i| TAG_EDGE + i as u64),
        WindowHit::Caption(button) => BUTTONS.iter().position(|b| *b == button).map_or(0, |i| TAG_CAPTION + i as u64),
    }
}

fn zone_hit(tag: u64) -> Option<WindowHit> {
    match tag {
        TAG_MOVE => Some(WindowHit::Move),
        t if (TAG_EDGE..TAG_EDGE + 8).contains(&t) => Some(WindowHit::Resize(EDGES[(t - TAG_EDGE) as usize])),
        t if (TAG_CAPTION..TAG_CAPTION + 3).contains(&t) => Some(WindowHit::Caption(BUTTONS[(t - TAG_CAPTION) as usize])),
        _ => None,
    }
}

/// The regions the exclusive parts of `window` declare, as the window procedure answers from;
/// `None` when it declares none.
pub fn caption_regions(rt: &Runtime, window: WindowId) -> Option<CaptionRegions> {
    regions_of(&rt.window_zones(window))
}

fn regions_of(zones: &[WindowZone]) -> Option<CaptionRegions> {
    let mut regions = CaptionRegions::default();
    let mut any = false;
    for zone in zones {
        let Some(hit) = zone_hit(zone.tag) else { continue };
        any = true;
        match hit {
            WindowHit::Move => {
                regions.moves.push(zone.rect);
                regions.covers.push(zone.covers.clone());
            }
            WindowHit::Resize(edge) => regions.edges.push((edge, zone.rect)),
            WindowHit::Caption(button) => regions.buttons.push((button, zone.rect)),
        }
    }
    any.then_some(regions)
}

/// The ui-rt spec that opens `spec` as `title`. A window opened with a key is placed where it
/// was saved; `frame_of` is the installed frame decision for a kind of window (`None`: the OS's).
pub fn rt_spec(spec: &WindowSpec, title: &str, key: Option<&str>, frame_of: &dyn Fn(WindowKind) -> Option<Frame>) -> craft_ui_rt::WindowSpec {
    let chrome = spec.chrome.or_else(|| frame_of(spec.kind).map(|f| crate::runtime::windows::chrome_of_frame(&f))).unwrap_or(WindowChrome::Native);
    craft_ui_rt::WindowSpec {
        decorations: chrome == WindowChrome::Native,
        visible: !spec.shown_after_first_paint,
        key: key.map(str::to_owned),
        // A palette starts above the others; lowering it while the app is away or modal is the
        // host's policy and goes through `WindowCommand::Level`.
        level: match spec.level {
            crate::runtime::windows::WindowLevel::Normal => WindowLevel::Normal,
            _ => WindowLevel::AboveOthers,
        },
        taskbar: spec.taskbar,
        ..craft_ui_rt::WindowSpec::new(title)
    }
}

/// What the registry last recorded of a window.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seen {
    size: Vec2,
    scale: f32,
    position_px: Option<Pos2>,
    maximized: bool,
    fullscreen: bool,
    /// Surface top left to outer top left, physical pixels.
    inset: Vec2,
}

/// The app host's window state: saved placements, the Windows hit-test sinks and the captions the
/// window procedure reported. One per app; the host calls [`WindowRegistry::before_step`] and
/// [`WindowRegistry::after_step`] around each `Driver::step`.
pub struct WindowRegistry {
    store: Store,
    seen: BTreeMap<WindowId, Seen>,
    sinks: BTreeMap<WindowId, HitSink>,
    last_save: f64,
}

/// A saved placement is written at most this often, in seconds.
const SAVE_EVERY: f64 = 0.5;

impl WindowRegistry {
    /// A registry that saves placements in `store`.
    pub fn new(store: Store) -> WindowRegistry {
        WindowRegistry { store, seen: BTreeMap::new(), sinks: BTreeMap::new(), last_save: f64::NEG_INFINITY }
    }

    /// The placement saved under `key`.
    pub fn saved(&self, key: &str) -> Option<Placement> {
        self.store.get(key)
    }

    /// `spec` placed for opening: the saved placement under its key, repaired and fitted to the
    /// monitors, decides the inner size and the outer position; otherwise `defaults` do.
    pub fn place(&self, spec: craft_ui_rt::WindowSpec, defaults: &Defaults, surfaces: &dyn SurfaceHost) -> craft_ui_rt::WindowSpec {
        let saved = spec.key.as_deref().and_then(|key| self.store.get(key)).and_then(|p| p.repaired());
        let placed = place(saved.as_ref(), &monitors_of(surfaces), None, defaults);
        craft_ui_rt::WindowSpec {
            size: Some(Vec2::new(placed.normal.size[0], placed.normal.size[1])),
            position: placed.position().filter(|_| !defaults.no_position).map(|p| Pos2::new(p[0], p[1])),
            min_size: spec.min_size.or(Some(Vec2::new(defaults.min[0], defaults.min[1]))),
            ..spec
        }
    }

    /// Before a step: turns what the window procedure reported since the last one into the
    /// pointer events the caption button would have had, so it looks and acts as on the other
    /// platforms.
    pub fn before_step(&mut self, rt: &mut Runtime) {
        for (&window, sink) in &self.sinks {
            let events = sink.take_events();
            if events.is_empty() {
                continue;
            }
            let Some(regions) = caption_regions(rt, window) else { continue };
            let Some(geometry) = rt.window_geometry(window) else { continue };
            let scale = geometry.scale.max(f32::EPSILON);
            let center = |button: Button| regions.buttons.iter().find(|(b, _)| *b == button).map(|(_, rect)| rect.center());
            let on_band = |pixel: [f32; 2]| {
                let at = Pos2::new(pixel[0] / scale, pixel[1] / scale);
                regions.moves.iter().any(|rect| rect.contains(at)).then_some(at)
            };
            let pointer = |phase, button, pos| {
                PlatformEvent::Window(window, WindowEvent::Pointer(PointerEvent::new(0, PointerKind::Mouse, phase, button, pos, Default::default())))
            };
            let moved = |pos| pointer(PointerPhase::Move, PointerButton::Primary, pos);
            let press = |pos, button, down| pointer(if down { PointerPhase::Down } else { PointerPhase::Up }, button, pos);
            let mut replay = Vec::new();
            for event in events {
                match event {
                    CaptionEvent::Hover(caption) => replay.extend(center(caption).map(moved)),
                    CaptionEvent::Press(caption) => {
                        if let Some(at) = center(caption) {
                            replay.extend([moved(at), press(at, PointerButton::Primary, true)]);
                        }
                    }
                    CaptionEvent::Release(caption) => replay.extend(center(caption).map(|at| press(at, PointerButton::Primary, false))),
                    CaptionEvent::Leave => replay.push(moved(OUTSIDE)),
                    CaptionEvent::Cancel => replay.extend([moved(OUTSIDE), press(OUTSIDE, PointerButton::Primary, false)]),
                    CaptionEvent::BandDouble(pixel) => {
                        if let Some(at) = on_band(pixel) {
                            replay.extend([
                                moved(at),
                                press(at, PointerButton::Primary, true),
                                press(at, PointerButton::Primary, false),
                                press(at, PointerButton::Primary, true),
                                press(at, PointerButton::Primary, false),
                            ]);
                        }
                    }
                    CaptionEvent::BandMenu(pixel) => {
                        if let Some(at) = on_band(pixel) {
                            replay.extend([moved(at), press(at, PointerButton::Secondary, true), press(at, PointerButton::Secondary, false)]);
                        }
                    }
                    _ => {}
                }
            }
            for ev in &replay {
                rt.handle_event(ev);
            }
        }
    }

    /// After a step: records the placement of every keyed window whose geometry changed, restores
    /// the saved maximized or full-screen state of a window seen for the first time, publishes the
    /// zones to the hit sinks when a frame ran, forgets closed windows and writes the store when
    /// it changed (at most every [`SAVE_EVERY`] seconds; [`WindowRegistry::flush`] writes at once).
    pub fn after_step(&mut self, rt: &mut Runtime, surfaces: &dyn SurfaceHost, ran: bool, now: f64) {
        let monitors = monitors_of(surfaces);
        let open: Vec<WindowId> = rt.windows().collect();
        self.seen.retain(|w, _| open.contains(w));
        self.sinks.retain(|w, _| open.contains(w));
        for window in open {
            let Some(key) = rt.window_key(window).map(str::to_owned) else { continue };
            let Some(geometry) = rt.window_geometry(window) else { continue };
            let seen = Seen {
                size: geometry.size,
                scale: geometry.scale,
                position_px: geometry.position_px,
                maximized: geometry.maximized,
                fullscreen: geometry.fullscreen,
                inset: geometry.frame_inset,
            };
            let first = !self.seen.contains_key(&window);
            if first && let Some(saved) = self.store.get(&key) {
                if saved.maximized {
                    rt.window_command(window, WindowCommand::Maximized(true));
                }
                if saved.fullscreen {
                    rt.window_command(window, WindowCommand::Fullscreen(true));
                }
            }
            if self.seen.insert(window, seen) == Some(seen) {
                continue;
            }
            if let Some(next) = placement_of(&monitors, seen, self.store.get(&key)) {
                self.store.set(&key, &next);
            }
        }
        if ran {
            for (&window, sink) in &self.sinks {
                let scale = rt.window_geometry(window).map_or(1.0, |g| g.scale);
                sink.publish_regions(caption_regions(rt, window), scale);
            }
        }
        if self.store.is_dirty() && now - self.last_save >= SAVE_EVERY {
            self.flush(now);
        }
    }

    /// Writes the placements now (quit, or the last window closing).
    pub fn flush(&mut self, now: f64) {
        self.last_save = now;
        // A store that cannot be written keeps its changes and tries again at the next flush.
        let _ = self.store.save();
    }

    /// Routes the Windows hit test of `window` to a sink the registry keeps, when the OS window
    /// exists and the app draws its frame. Returns false where the platform has no routing (every
    /// platform but Windows): the app then moves and resizes the window itself.
    pub fn install_hit_sink(&mut self, window: WindowId, native: &NativeWindow, waker: Waker) -> bool {
        if self.sinks.contains_key(&window) {
            return true;
        }
        #[cfg(not(windows))]
        {
            let _ = (native, waker);
            false
        }
        #[cfg(windows)]
        {
            let Some(os_window) = craft_window::native::NativeWindow::of_handle(native) else { return false };
            let sink = HitSink::linked_rt(waker);
            if !os_window.route_hits(std::sync::Arc::new(sink.clone())) {
                return false;
            }
            self.sinks.insert(window, sink);
            true
        }
    }

    /// Keeps `sink` for `window` as [`WindowRegistry::install_hit_sink`] does, for a platform
    /// layer that routed it itself.
    pub fn attach_hit_sink(&mut self, window: WindowId, sink: HitSink) {
        self.sinks.insert(window, sink);
    }

    /// The sink of `window`, when its hit test is routed.
    pub fn hit_sink(&self, window: WindowId) -> Option<&HitSink> {
        self.sinks.get(&window)
    }
}

/// The platform's monitors as the placement code sees them: physical pixels of the desktop.
fn monitors_of(surfaces: &dyn SurfaceHost) -> Vec<Monitor> {
    surfaces
        .monitors()
        .iter()
        .map(|m: &MonitorInfo| {
            let size = [(m.size.x * m.scale).round() as u32, (m.size.y * m.scale).round() as u32];
            let key = MonitorKey::new(m.name.clone().unwrap_or_default(), size);
            Monitor::new(key, [(m.position.x * m.scale).round() as i32, (m.position.y * m.scale).round() as i32], size, m.scale)
        })
        .collect()
}

/// `previous` (or a new record) updated to where the window is: its scale, its state, its size
/// and outer position unless it is maximized or full screen (which keep the last normal ones),
/// and the monitor showing most of it. `None` before the window has a size.
fn placement_of(monitors: &[Monitor], seen: Seen, previous: Option<Placement>) -> Option<Placement> {
    if seen.size.x <= 0.0 || seen.size.y <= 0.0 {
        return None;
    }
    let mut next = previous.unwrap_or_else(|| Placement::new([seen.size.x, seen.size.y], seen.scale));
    next.scale = seen.scale;
    next.maximized = seen.maximized;
    next.fullscreen = seen.fullscreen;
    if !seen.maximized && !seen.fullscreen {
        next.normal.size = [seen.size.x, seen.size.y];
        if let Some(surface) = seen.position_px {
            let outer = surface - seen.inset;
            next.position_px = Some([outer.x.round() as i32, outer.y.round() as i32]);
            next.normal.pos = [outer.x / seen.scale, outer.y / seen.scale];
            let window_px = Rect::from_min_size(outer, seen.size * seen.scale);
            if let Some(monitor) = monitor_of(monitors, window_px) {
                next.monitor = Some(monitor.key.clone());
            }
        }
    }
    Some(next)
}

/// A position outside every window: a release there ends a press without a click.
const OUTSIDE: Pos2 = Pos2::new(-1.0, -1.0);
