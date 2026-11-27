//! A windowless platform for the trials: events are pushed into it, and what the runtime sent
//! (access updates, IME requests, frame and wake requests, window commands) is read back.
#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_theme::{Appearance, Contrast, Mode, TextSize, builtins, resolve};
use craft_ui_rt::{
    Clipboard, Cursor, ImeRequest, Mods, Platform, PlatformEvent, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, SizeU, SurfaceHandle,
    SurfaceHost, Tokens, Vec2, Waker, WindowCommand, WindowEvent, WindowId, WindowSpec,
};

pub fn tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

struct Surface {
    size_px: SizeU,
    scale: f32,
    alive: bool,
}

#[derive(Default)]
pub struct Surfaces {
    windows: BTreeMap<WindowId, Surface>,
}

impl SurfaceHost for Surfaces {
    fn windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    fn surface(&self, window: WindowId) -> Option<SurfaceHandle> {
        self.windows.get(&window).filter(|s| s.alive).map(|_| SurfaceHandle::Offscreen)
    }

    fn size_px(&self, window: WindowId) -> SizeU {
        self.windows.get(&window).map_or(SizeU::default(), |s| s.size_px)
    }

    fn scale(&self, window: WindowId) -> f32 {
        self.windows.get(&window).map_or(1.0, |s| s.scale)
    }
}

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

pub struct TestPlatform {
    surfaces: Surfaces,
    next_window: u64,
    queue: VecDeque<PlatformEvent>,
    clipboard: MemoryClipboard,
    woken: Arc<AtomicBool>,
    pub access_updates: Vec<(WindowId, accesskit::TreeUpdate)>,
    pub ime_requests: Vec<(WindowId, ImeRequest)>,
    pub commands: Vec<(WindowId, WindowCommand)>,
    pub cursors: Vec<(WindowId, Cursor)>,
    frame_requested: bool,
    wake_at: Option<f64>,
}

pub const MAIN: WindowId = WindowId(1);

impl TestPlatform {
    /// One window, [`MAIN`], of `size` logical points at `scale`, with an assistive technology
    /// active from the start.
    pub fn new(size: Vec2, scale: f32) -> TestPlatform {
        let mut p = TestPlatform {
            surfaces: Surfaces::default(),
            next_window: 1,
            queue: VecDeque::new(),
            clipboard: MemoryClipboard::default(),
            woken: Arc::default(),
            access_updates: Vec::new(),
            ime_requests: Vec::new(),
            commands: Vec::new(),
            cursors: Vec::new(),
            frame_requested: false,
            wake_at: None,
        };
        p.create_window(&WindowSpec { size: Some(size), ..WindowSpec::new("main") });
        p.surfaces.windows.get_mut(&MAIN).expect("main window").scale = scale;
        p.surfaces.windows.get_mut(&MAIN).expect("main window").size_px = SizeU::from_logical(size, scale);
        p
    }

    pub fn push(&mut self, window: WindowId, ev: WindowEvent) {
        self.queue.push_back(PlatformEvent::Window(window, ev));
    }

    pub fn lose_surface(&mut self, window: WindowId) {
        if let Some(s) = self.surfaces.windows.get_mut(&window) {
            s.alive = false;
        }
        self.push(window, WindowEvent::SurfaceLost);
    }

    pub fn restore_surface(&mut self, window: WindowId) {
        if let Some(s) = self.surfaces.windows.get_mut(&window) {
            s.alive = true;
        }
        self.push(window, WindowEvent::SurfaceCreated);
    }

    pub fn take_frame_request(&mut self) -> bool {
        std::mem::take(&mut self.frame_requested)
    }

    pub fn take_wake(&mut self) -> Option<f64> {
        self.wake_at.take()
    }

    pub fn updates_for(&self, window: WindowId) -> impl Iterator<Item = &accesskit::TreeUpdate> {
        self.access_updates.iter().filter(move |(w, _)| *w == window).map(|(_, u)| u)
    }
}

impl Platform for TestPlatform {
    fn surfaces(&mut self) -> &mut dyn SurfaceHost {
        &mut self.surfaces
    }

    fn poll(&mut self, out: &mut Vec<PlatformEvent>) {
        for window in std::mem::take(&mut self.clipboard.requested) {
            if let Some(text) = self.clipboard.text.clone() {
                self.queue.push_back(PlatformEvent::Window(window, WindowEvent::Paste(text)));
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
        let size = spec.size.unwrap_or(Vec2::new(400.0, 300.0));
        self.surfaces.windows.insert(id, Surface { size_px: SizeU::from_logical(size, 1.0), scale: 1.0, alive: true });
        self.queue.push_back(PlatformEvent::Window(id, WindowEvent::AccessActivated));
        id
    }

    fn window(&mut self, window: WindowId, cmd: WindowCommand) {
        if cmd == WindowCommand::Close && self.surfaces.windows.remove(&window).is_some() {
            self.queue.push_back(PlatformEvent::Window(window, WindowEvent::Closed));
        }
        self.commands.push((window, cmd));
    }

    fn ime(&mut self, window: WindowId, req: ImeRequest) {
        self.ime_requests.push((window, req));
    }

    fn set_cursor(&mut self, window: WindowId, cursor: Cursor) {
        self.cursors.push((window, cursor));
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

pub fn pointer(kind: PointerKind, phase: PointerPhase, pos: Pos2) -> WindowEvent {
    WindowEvent::Pointer(PointerEvent {
        id: if kind == PointerKind::Touch { 7 } else { 1 },
        kind,
        phase,
        button: PointerButton::Primary,
        pos,
        mods: Mods::NONE,
        sample: None,
    })
}

pub fn click(platform: &mut TestPlatform, window: WindowId, pos: Pos2) {
    platform.push(window, pointer(PointerKind::Mouse, PointerPhase::Down, pos));
    platform.push(window, pointer(PointerKind::Mouse, PointerPhase::Up, pos));
}
