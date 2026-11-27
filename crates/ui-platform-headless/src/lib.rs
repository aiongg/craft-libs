//! A [`Platform`] with no window system: windows are records, surfaces are offscreen, the clock is
//! a number the check sets, and input is injected. Checks and the bench drive the runtime through
//! it and read back what the runtime sent (access updates, IME requests, cursors, window
//! commands, frame and wake requests). A renderer attaches to a window's [`SurfaceHandle::Offscreen`]
//! surface and gives its pixels back through `read_back`.
//!
//! Window commands take effect as on a desktop platform: a size change comes back as a resize
//! event, focus moves emit focus events to both windows, close emits `Closed`.

mod input;

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_ui_rt::{
    Clipboard, Cursor, ImeRequest, Platform, PlatformEvent, Pos2, SizeU, SurfaceHandle, SurfaceHost, Vec2, Waker, WindowCommand, WindowEvent,
    WindowId, WindowSpec,
};

pub use input::{Contact, Input};

/// What the platform knows about one open window.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowState {
    pub title: String,
    /// Logical points.
    pub size: Vec2,
    pub min_size: Option<Vec2>,
    pub position: Option<Pos2>,
    pub scale: f32,
    pub visible: bool,
    pub decorations: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub focused: bool,
    pub owner: Option<WindowId>,
    pub cursor: Cursor,
    /// Whether the window's surface exists (not lost).
    pub surface: bool,
}

/// The windows as surfaces. Every surface is [`SurfaceHandle::Offscreen`].
#[derive(Default)]
pub struct HeadlessSurfaces {
    windows: BTreeMap<WindowId, WindowState>,
    suspended: bool,
}

impl SurfaceHost for HeadlessSurfaces {
    fn windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    fn surface(&self, window: WindowId) -> Option<SurfaceHandle> {
        self.windows.get(&window).filter(|w| w.surface && !self.suspended).map(|_| SurfaceHandle::Offscreen)
    }

    fn size_px(&self, window: WindowId) -> SizeU {
        self.windows.get(&window).map_or(SizeU::default(), |w| SizeU::from_logical(w.size, w.scale))
    }

    fn scale(&self, window: WindowId) -> f32 {
        self.windows.get(&window).map_or(1.0, |w| w.scale)
    }
}

/// Holds the text; a read request answers with a paste event in the asking window.
#[derive(Default)]
pub struct MemoryClipboard {
    pub text: Option<String>,
    requested: Vec<WindowId>,
}

impl Clipboard for MemoryClipboard {
    fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_owned());
    }

    fn request_text(&mut self, window: WindowId) {
        self.requested.push(window);
    }
}

pub struct HeadlessPlatform {
    surfaces: HeadlessSurfaces,
    next_window: u64,
    /// Back to front.
    stack: Vec<WindowId>,
    queue: VecDeque<PlatformEvent>,
    clipboard: MemoryClipboard,
    woken: Arc<AtomicBool>,
    now: f64,
    frame_requested: bool,
    wake_at: Option<f64>,
    access_active: bool,
    /// What the runtime sent, in order.
    pub access_updates: Vec<(WindowId, accesskit::TreeUpdate)>,
    pub ime_requests: Vec<(WindowId, ImeRequest)>,
    pub commands: Vec<(WindowId, WindowCommand)>,
}

impl HeadlessPlatform {
    /// The id of the first window of [`HeadlessPlatform::with_window`].
    pub const MAIN: WindowId = WindowId(1);

    /// No window. An assistive technology counts as active in every window from its creation, so
    /// the access tree is built and checked in every run (see [`HeadlessPlatform::set_access_active`]).
    pub fn new() -> HeadlessPlatform {
        HeadlessPlatform {
            surfaces: HeadlessSurfaces::default(),
            next_window: 1,
            stack: Vec::new(),
            queue: VecDeque::new(),
            clipboard: MemoryClipboard::default(),
            woken: Arc::default(),
            now: 0.0,
            frame_requested: false,
            wake_at: None,
            access_active: true,
            access_updates: Vec::new(),
            ime_requests: Vec::new(),
            commands: Vec::new(),
        }
    }

    /// One window, [`HeadlessPlatform::MAIN`], of `size` logical points at `scale`.
    pub fn with_window(size: Vec2, scale: f32) -> HeadlessPlatform {
        let mut p = HeadlessPlatform::new();
        let id = p.create_window(&WindowSpec { size: Some(size), ..WindowSpec::new("main") });
        debug_assert_eq!(id, HeadlessPlatform::MAIN);
        p.state_mut(id).scale = scale;
        p
    }

    /// Whether windows created from now on report an active assistive technology.
    pub fn set_access_active(&mut self, on: bool) {
        self.access_active = on;
    }

    // ---- the clock ----

    /// The frame time a check passes to `Driver::step`, in seconds.
    pub fn now(&self) -> f64 {
        self.now
    }

    pub fn set_now(&mut self, time: f64) {
        self.now = time;
    }

    /// Moves the clock on by `dt` seconds and returns the new time.
    pub fn advance(&mut self, dt: f64) -> f64 {
        self.now += dt;
        self.now
    }

    /// The time the runtime last asked to be woken at, not taken.
    pub fn pending_wake(&self) -> Option<f64> {
        self.wake_at
    }

    /// The frame time the runtime last asked to be woken at, taken.
    pub fn take_wake(&mut self) -> Option<f64> {
        self.wake_at.take()
    }

    /// Whether the runtime asked for another frame since the last call.
    pub fn take_frame_request(&mut self) -> bool {
        std::mem::take(&mut self.frame_requested)
    }

    /// Sets the clock to the pending wake time, exactly, and takes the wake. `None` (clock
    /// unchanged) when no wake is pending. A wake in the past leaves the clock where it is.
    pub fn advance_to_wake(&mut self) -> Option<f64> {
        let at = self.wake_at.take()?;
        self.now = self.now.max(at);
        Some(at)
    }

    // ---- windows ----

    pub fn window_state(&self, window: WindowId) -> Option<&WindowState> {
        self.surfaces.windows.get(&window)
    }

    /// The open windows, back to front.
    pub fn stacking(&self) -> &[WindowId] {
        &self.stack
    }

    /// The window with key focus.
    pub fn focused(&self) -> Option<WindowId> {
        self.surfaces.windows.iter().find(|(_, w)| w.focused).map(|(id, _)| *id)
    }

    fn state_mut(&mut self, window: WindowId) -> &mut WindowState {
        self.surfaces.windows.get_mut(&window).expect("an open window")
    }

    fn push(&mut self, window: WindowId, ev: WindowEvent) {
        self.queue.push_back(PlatformEvent::Window(window, ev));
    }

    /// Gives `window` key focus, taking it from the window that had it.
    fn focus(&mut self, window: WindowId) {
        let previous = self.focused();
        if previous == Some(window) {
            return;
        }
        if let Some(prev) = previous {
            self.state_mut(prev).focused = false;
            self.push(prev, WindowEvent::Focus(false));
        }
        self.state_mut(window).focused = true;
        self.push(window, WindowEvent::Focus(true));
    }

    fn raise(&mut self, window: WindowId) {
        self.stack.retain(|w| *w != window);
        self.stack.push(window);
    }

    // ---- injection ----

    /// Queues `input` for the next poll in `window` and applies the lifecycle changes it implies.
    /// Ignored for a window that is not open.
    pub fn inject(&mut self, window: WindowId, input: Input) {
        let Some(state) = self.surfaces.windows.get_mut(&window) else {
            return;
        };
        match &input {
            Input::Resize(size) => state.size = *size,
            Input::Scale(scale) => state.scale = *scale,
            Input::SurfaceLost => state.surface = false,
            Input::SurfaceCreated => state.surface = true,
            Input::Focus(true) => return self.focus(window),
            Input::Focus(false) => state.focused = false,
            _ => {}
        }
        self.push(window, input.event());
    }

    pub fn inject_all(&mut self, window: WindowId, inputs: impl IntoIterator<Item = Input>) {
        for input in inputs {
            self.inject(window, input);
        }
    }

    pub fn pointer(&mut self, window: WindowId, contact: Contact, phase: craft_ui_rt::PointerPhase, pos: Pos2) {
        self.inject(window, Input::pointer(contact, phase, pos));
    }

    pub fn mouse_move(&mut self, window: WindowId, pos: Pos2) {
        self.inject(window, Input::mouse_move(pos));
    }

    /// The mouse presses and releases at `pos` (a move first, so hover is set as for a real mouse).
    pub fn click(&mut self, window: WindowId, pos: Pos2) {
        self.inject_all(window, [Input::mouse_move(pos), Input::mouse_down(pos), Input::mouse_up(pos)]);
    }

    /// A finger touches and lifts at `pos`.
    pub fn tap(&mut self, window: WindowId, pos: Pos2) {
        let finger = Contact::touch(0);
        self.pointer(window, finger, craft_ui_rt::PointerPhase::Down, pos);
        self.pointer(window, finger, craft_ui_rt::PointerPhase::Up, pos);
    }

    pub fn wheel(&mut self, window: WindowId, pos: Pos2, delta: Vec2) {
        self.inject(window, Input::Wheel { pos, delta, mods: craft_ui_rt::Mods::NONE });
    }

    pub fn key_press(&mut self, window: WindowId, key: craft_ui_rt::Key, mods: craft_ui_rt::Mods) {
        self.inject_all(window, Input::key_press(key, mods));
    }

    pub fn text(&mut self, window: WindowId, text: &str) {
        self.inject(window, Input::Text(text.to_owned()));
    }

    pub fn file_drop(&mut self, window: WindowId, files: Vec<std::path::PathBuf>) {
        let hovered: Vec<Input> = files.iter().cloned().map(Input::FileHovered).collect();
        let dropped: Vec<Input> = files.into_iter().map(Input::FileDropped).collect();
        self.inject_all(window, hovered.into_iter().chain(dropped));
    }

    pub fn access_action(&mut self, window: WindowId, action: accesskit::Action, target: accesskit::NodeId) {
        self.inject(window, Input::action(action, target));
    }

    /// An application-wide suspend (mobile backgrounding): every surface goes until `resume`.
    pub fn suspend(&mut self) {
        self.surfaces.suspended = true;
        self.queue.push_back(PlatformEvent::Suspend);
    }

    pub fn resume(&mut self) {
        self.surfaces.suspended = false;
        self.queue.push_back(PlatformEvent::Resume);
    }

    // ---- what the runtime sent ----

    pub fn clipboard_text(&self) -> Option<&str> {
        self.clipboard.text.as_deref()
    }

    pub fn set_clipboard_text(&mut self, text: &str) {
        self.clipboard.text = Some(text.to_owned());
    }

    pub fn cursor(&self, window: WindowId) -> Cursor {
        self.window_state(window).map_or(Cursor::Default, |w| w.cursor)
    }

    pub fn updates_for(&self, window: WindowId) -> impl Iterator<Item = &accesskit::TreeUpdate> {
        self.access_updates.iter().filter(move |(w, _)| *w == window).map(|(_, u)| u)
    }

    pub fn ime_for(&self, window: WindowId) -> impl Iterator<Item = &ImeRequest> {
        self.ime_requests.iter().filter(move |(w, _)| *w == window).map(|(_, r)| r)
    }
}

impl Default for HeadlessPlatform {
    fn default() -> Self {
        HeadlessPlatform::new()
    }
}

impl Platform for HeadlessPlatform {
    fn surfaces(&mut self) -> &mut dyn SurfaceHost {
        &mut self.surfaces
    }

    fn poll(&mut self, out: &mut Vec<PlatformEvent>) {
        for window in std::mem::take(&mut self.clipboard.requested) {
            if let Some(text) = self.clipboard.text.clone() {
                self.push(window, WindowEvent::Paste(text));
            }
        }
        if self.woken.swap(false, Ordering::AcqRel) {
            self.queue.push_back(PlatformEvent::Wake);
        }
        out.extend(self.queue.drain(..));
    }

    fn create_window(&mut self, spec: &WindowSpec) -> WindowId {
        let id = WindowId(self.next_window);
        self.next_window += 1;
        self.surfaces.windows.insert(
            id,
            WindowState {
                title: spec.title.clone(),
                size: spec.size.unwrap_or(Vec2::new(800.0, 600.0)),
                min_size: spec.min_size,
                position: spec.position,
                scale: 1.0,
                visible: spec.visible,
                decorations: spec.decorations,
                minimized: false,
                maximized: false,
                fullscreen: false,
                focused: false,
                owner: spec.owner,
                cursor: Cursor::Default,
                surface: true,
            },
        );
        self.stack.push(id);
        if self.access_active {
            self.push(id, WindowEvent::AccessActivated);
        }
        id
    }

    fn window(&mut self, window: WindowId, cmd: WindowCommand) {
        self.commands.push((window, cmd.clone()));
        if !self.surfaces.windows.contains_key(&window) {
            return;
        }
        match cmd {
            WindowCommand::Title(t) => self.state_mut(window).title = t,
            WindowCommand::Size(size) => {
                self.state_mut(window).size = size;
                self.push(window, WindowEvent::Resize(size));
            }
            WindowCommand::MinSize(m) => self.state_mut(window).min_size = m,
            WindowCommand::Position(p) => self.state_mut(window).position = Some(p),
            WindowCommand::Visible(v) => self.state_mut(window).visible = v,
            WindowCommand::Decorations(d) => self.state_mut(window).decorations = d,
            WindowCommand::Minimized(m) => self.state_mut(window).minimized = m,
            WindowCommand::Maximized(m) => self.state_mut(window).maximized = m,
            WindowCommand::Fullscreen(f) => self.state_mut(window).fullscreen = f,
            WindowCommand::Focus => {
                self.raise(window);
                self.focus(window);
            }
            WindowCommand::Raise => self.raise(window),
            WindowCommand::DragMove => {}
            WindowCommand::Close => {
                self.surfaces.windows.remove(&window);
                self.stack.retain(|w| *w != window);
                self.push(window, WindowEvent::Closed);
            }
        }
    }

    fn ime(&mut self, window: WindowId, req: ImeRequest) {
        self.ime_requests.push((window, req));
    }

    fn set_cursor(&mut self, window: WindowId, cursor: Cursor) {
        if let Some(w) = self.surfaces.windows.get_mut(&window) {
            w.cursor = cursor;
        }
    }

    fn clipboard(&mut self) -> &mut dyn Clipboard {
        &mut self.clipboard
    }

    fn access(&mut self, window: WindowId, update: accesskit::TreeUpdate) {
        self.access_updates.push((window, update));
    }

    fn request_frame(&mut self) {
        self.frame_requested = true;
    }

    fn request_frame_at(&mut self, time: f64) {
        self.wake_at = Some(time);
    }

    fn waker(&self) -> Waker {
        let woken = self.woken.clone();
        Waker::new(move || woken.store(true, Ordering::Release))
    }
}
