//! craft-ui-rt's [`Platform`] on craft's winit fork: one code path for the desktop (Windows, macOS,
//! X11, Wayland) and the browser canvas (winit's web backend), several windows, with the Android
//! lifecycle left as stubs.
//!
//! winit calls the application; the runtime wants to call the platform. [`run`] bridges the two.
//! winit callbacks translate their events into `PlatformEvent`s in a shared queue; a redraw of any
//! window runs `Driver::step`, so every step sees the events of the winit turn that asked for it.
//! Windows are made by the loop, not by `create_window` (winit needs the active event loop): the
//! call returns the id at once, and the window's `Scale`, `Resize` and `SurfaceCreated` events
//! arrive after the loop made it.
//!
//! Each window has its own AccessKit adapter; its activation arrives as `AccessActivated`.
//! Raw window and display handles come from [`WinitPlatform::window_handle`] and
//! [`WinitPlatform::display_handle`] (PhotoCraft's tablet crate, renderers).

mod app;
mod clipboard;
mod cursor;
mod input;
mod shared;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_ui_rt::{
    Clipboard, Cursor, ImageKey, ImePurpose, ImeRequest, MonitorInfo, NativeWindow, Platform, PlatformEvent, Pos2, RgbaImage, SizeU, SurfaceHandle,
    SurfaceHost, Vec2, Waker, WindowCommand, WindowEvent, WindowId, WindowLevel, WindowSpec,
};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::{Fullscreen, Window};

pub use app::{run, run_with};
pub use clipboard::WinitClipboard;
pub use winit;

use shared::{Cell, Entry};

pub(crate) enum UserEvent {
    Access(accesskit_winit::Event),
    Wake,
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(e: accesskit_winit::Event) -> Self {
        UserEvent::Access(e)
    }
}

struct Host(Cell);

impl SurfaceHost for Host {
    fn windows(&self) -> Vec<WindowId> {
        self.0.borrow().entries.iter().map(|e| e.id).collect()
    }

    fn surface(&self, window: WindowId) -> Option<SurfaceHandle> {
        let s = self.0.borrow();
        let e = s.entry(window)?;
        let native = e.window.clone().filter(|_| e.surface_live)?;
        Some(NativeWindow::new(native).handle())
    }

    fn size_px(&self, window: WindowId) -> SizeU {
        self.0.borrow().entry(window).map_or_else(SizeU::default, |e| e.size_px)
    }

    fn scale(&self, window: WindowId) -> f32 {
        self.0.borrow().entry(window).map_or(1.0, |e| e.scale)
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        self.0.borrow().monitors.clone()
    }
}

pub struct WinitPlatform {
    shared: Cell,
    host: Host,
    clipboard: WinitClipboard,
    proxy: EventLoopProxy<UserEvent>,
    woken: Arc<AtomicBool>,
}

impl WinitPlatform {
    pub(crate) fn new(shared: Cell, proxy: EventLoopProxy<UserEvent>, woken: Arc<AtomicBool>) -> WinitPlatform {
        WinitPlatform { host: Host(shared.clone()), clipboard: WinitClipboard::new(shared.clone()), shared, proxy, woken }
    }

    /// The native window, for renderers and the tablet crate: it implements `HasWindowHandle` and
    /// `HasDisplayHandle`. `None` before the loop made it, after it closed, and on Android while
    /// suspended.
    pub fn window_handle(&self, window: WindowId) -> Option<Arc<Window>> {
        self.shared.borrow().native(window)
    }

    /// The display the window lives on, as a raw handle (`wgpu` surfaces, X11 and Wayland clients).
    pub fn display_handle(&self, window: WindowId) -> Option<winit::raw_window_handle::RawDisplayHandle> {
        use winit::raw_window_handle::HasDisplayHandle;
        self.window_handle(window)?.display_handle().ok().map(|h| h.as_raw())
    }

    /// The window's native handle as a raw handle (`NSView`, `HWND`, X11 window, Wayland surface).
    pub fn raw_window_handle(&self, window: WindowId) -> Option<winit::raw_window_handle::RawWindowHandle> {
        use winit::raw_window_handle::HasWindowHandle;
        self.window_handle(window)?.window_handle().ok().map(|h| h.as_raw())
    }

    /// The monitors as of the last loop turn that saw them change (window moved, scale changed,
    /// resume).
    pub fn monitors(&self) -> Vec<MonitorInfo> {
        self.shared.borrow().monitors.clone()
    }

    /// Registers the pixels `Cursor::Image { key, .. }` shows. The cursor takes effect on the loop
    /// turn after the first `set_cursor` that names it.
    pub fn register_cursor_image(&self, key: ImageKey, image: Arc<RgbaImage>) {
        let mut s = self.shared.borrow_mut();
        s.cursor_cache.retain(|(k, _), _| *k != key);
        s.cursor_images.insert(key, image);
    }

    pub fn unregister_cursor_image(&self, key: ImageKey) {
        let mut s = self.shared.borrow_mut();
        s.cursor_cache.retain(|(k, _), _| *k != key);
        s.cursor_images.remove(&key);
    }

    /// Like [`Platform::create_window`], drawing into the page's `<canvas id="canvas_id">`.
    #[cfg(target_arch = "wasm32")]
    pub fn create_window_on_canvas(&mut self, spec: &WindowSpec, canvas_id: &str) -> WindowId {
        let id = self.create_window(spec);
        if let Some(e) = self.shared.borrow_mut().entry_mut(id) {
            e.canvas = Some(canvas_id.to_owned());
        }
        id
    }

    /// Reports the system bars' insets in logical points. winit has no safe-area query on the
    /// desktop; the Android entry reads the window insets and calls this.
    pub fn set_safe_area(&self, window: WindowId, insets: craft_ui_rt::Margin) {
        shared::push(&self.shared, window, WindowEvent::SafeArea(insets));
    }
}

fn level_of(level: WindowLevel) -> winit::window::WindowLevel {
    match level {
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AboveOthers => winit::window::WindowLevel::AlwaysOnTop,
    }
}

/// Windows only: the taskbar entry of an open window. X11 fixes it at creation (window type), the
/// other platforms leave it to the system.
fn set_taskbar(window: &Window, shown: bool) {
    #[cfg(windows)]
    {
        use winit::platform::windows::WindowExtWindows;
        window.set_skip_taskbar(!shown);
    }
    #[cfg(not(windows))]
    let _ = (window, shown);
}

fn apply(window: &Window, cmd: &WindowCommand) {
    let logical = |v: &Vec2| LogicalSize::new(f64::from(v.x), f64::from(v.y));
    match cmd {
        WindowCommand::Title(t) => window.set_title(t),
        WindowCommand::Size(v) => {
            let _ = window.request_inner_size(logical(v));
        }
        WindowCommand::MinSize(v) => window.set_min_inner_size(v.as_ref().map(logical)),
        WindowCommand::Position(p) => window.set_outer_position(LogicalPosition::new(f64::from(p.x), f64::from(p.y))),
        WindowCommand::Visible(v) => window.set_visible(*v),
        WindowCommand::Decorations(d) => window.set_decorations(*d),
        WindowCommand::Level(l) => window.set_window_level(level_of(*l)),
        WindowCommand::Taskbar(t) => set_taskbar(window, *t),
        WindowCommand::Minimized(m) => window.set_minimized(*m),
        WindowCommand::Maximized(m) => window.set_maximized(*m),
        WindowCommand::Fullscreen(f) => window.set_fullscreen(f.then_some(Fullscreen::Borderless(None))),
        WindowCommand::Focus => window.focus_window(),
        // winit has no raise without focus; this is the closest it offers.
        WindowCommand::Raise => {
            window.set_minimized(false);
            window.focus_window();
        }
        WindowCommand::DragMove => {
            if let Err(e) = window.drag_window() {
                log::debug!("window move refused: {e}");
            }
        }
        WindowCommand::Close => {}
    }
}

impl Platform for WinitPlatform {
    fn surfaces(&mut self) -> &mut dyn SurfaceHost {
        &mut self.host
    }

    fn poll(&mut self, out: &mut Vec<PlatformEvent>) {
        out.append(&mut self.shared.borrow_mut().events);
    }

    fn create_window(&mut self, spec: &WindowSpec) -> WindowId {
        let mut s = self.shared.borrow_mut();
        s.next_id += 1;
        let id = WindowId(s.next_id);
        let size = spec.size.unwrap_or(Vec2::new(800.0, 600.0));
        s.ever_opened = true;
        s.entries.push(Entry {
            id,
            spec: spec.clone(),
            canvas: None,
            window: None,
            adapter: None,
            queued: Vec::new(),
            size_px: SizeU::from_logical(size, 1.0),
            scale: 1.0,
            surface_live: false,
            mods: Default::default(),
            caps_lock: false,
            num_lock: false,
            cursor: Pos2::ZERO,
            reported_state: None,
            reported_inset: None,
        });
        s.frame_wanted = true;
        id
    }

    fn window(&mut self, window: WindowId, cmd: WindowCommand) {
        let native = {
            let mut s = self.shared.borrow_mut();
            if cmd == WindowCommand::Close {
                if let Some(at) = s.entries.iter().position(|e| e.id == window) {
                    s.entries.remove(at);
                    s.cursor_wanted.retain(|(w, ..)| *w != window);
                    s.events.push(PlatformEvent::Window(window, WindowEvent::Closed));
                    s.frame_wanted = true;
                }
                return;
            }
            let Some(entry) = s.entry_mut(window) else { return };
            match entry.window.clone() {
                Some(w) => w,
                None => {
                    entry.queued.push(cmd);
                    return;
                }
            }
        };
        apply(&native, &cmd);
    }

    fn ime(&mut self, window: WindowId, req: ImeRequest) {
        let Some(native) = self.window_handle(window) else { return };
        let area = |r: craft_ui_rt::Rect| {
            native.set_ime_cursor_area(
                LogicalPosition::new(f64::from(r.min.x), f64::from(r.min.y)),
                LogicalSize::new(f64::from(r.width()), f64::from(r.height())),
            );
        };
        match req {
            ImeRequest::Start { purpose, cursor } => {
                native.set_ime_allowed(true);
                native.set_ime_purpose(match purpose {
                    ImePurpose::Normal => winit::window::ImePurpose::Normal,
                    ImePurpose::Password => winit::window::ImePurpose::Password,
                });
                area(cursor);
            }
            ImeRequest::Cursor(r) => area(r),
            ImeRequest::Stop => native.set_ime_allowed(false),
        }
    }

    fn set_cursor(&mut self, window: WindowId, cursor: Cursor) {
        cursor::apply(&mut self.shared.borrow_mut(), window, cursor);
    }

    fn clipboard(&mut self) -> &mut dyn Clipboard {
        &mut self.clipboard
    }

    fn access(&mut self, window: WindowId, update: accesskit::TreeUpdate) {
        if let Some(adapter) = self.shared.borrow_mut().entry_mut(window).and_then(|e| e.adapter.as_mut()) {
            adapter.update_if_active(|| update);
        }
    }

    fn request_frame(&mut self) {
        self.shared.borrow_mut().frame_wanted = true;
    }

    fn request_frame_at(&mut self, time: f64) {
        self.shared.borrow_mut().wake_at = Some(time);
    }

    fn waker(&self) -> Waker {
        let (proxy, woken) = (self.proxy.clone(), self.woken.clone());
        Waker::new(move || {
            // One pending wake is enough: the platform reports several wakes as one event.
            if !woken.swap(true, Ordering::AcqRel) {
                let _ = proxy.send_event(UserEvent::Wake);
            }
        })
    }

    /// Through the system's handler on native targets (which picks the browser window or tab), and in a new tab on
    /// the web.
    fn open_url(&mut self, url: &str, _new_tab: bool) {
        if let Err(error) = craft_platform::open::open_external(url) {
            log::warn!("could not open {url}: {error}");
        }
    }
}
