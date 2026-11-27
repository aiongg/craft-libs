//! The platform boundary: windows and their surfaces, input, IME, clipboard, cursors,
//! accessibility, frame scheduling and wakes from other threads. Nothing here assumes a mouse, a
//! fixed DPI, a desktop window manager or threads: a browser platform delivers DOM input through
//! `poll`, draws into canvas surfaces, runs IME through a hidden input element and schedules frames
//! with `requestAnimationFrame`.

use std::any::Any;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use craft_behavior::{CursorIcon, PointerButton, PointerKind, PointerSample, WheelPhase, WheelSource, WheelUnit};
use craft_keymap::chord::{Key, Mods};
use emath::{Pos2, Rect, Vec2};
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

use crate::display::{ImageKey, RgbaImage};
use crate::geom::{Margin, SizeU};
use crate::id::WindowId;

/// What a renderer draws into. `Native` holds the platform's object; platforms whose windows have
/// raw window and display handles (winit on desktop and web, Android) put a [`NativeWindow`] there,
/// which every renderer reads with [`SurfaceHandle::native`].
#[derive(Clone)]
pub enum SurfaceHandle {
    Offscreen,
    Native(Rc<dyn Any>),
}

impl SurfaceHandle {
    /// The window as raw handles, when the platform handed one over.
    pub fn native(&self) -> Option<&NativeWindow> {
        match self {
            SurfaceHandle::Native(any) => any.downcast_ref::<NativeWindow>(),
            SurfaceHandle::Offscreen => None,
        }
    }
}

/// What a platform's window type provides to renderers: raw handles, valid while anyone holds the
/// `Arc`. A winit window, a web canvas wrapper and an Android native window all qualify.
pub trait WindowHandles: HasWindowHandle + HasDisplayHandle + Send + Sync + Any {}

impl<T: HasWindowHandle + HasDisplayHandle + Send + Sync + Any> WindowHandles for T {}

/// A platform window as renderers see it. The platform returns it from
/// [`SurfaceHost::surface`] through [`NativeWindow::handle`].
#[derive(Clone)]
pub struct NativeWindow(pub Arc<dyn WindowHandles>);

impl NativeWindow {
    pub fn new<W: WindowHandles>(window: Arc<W>) -> NativeWindow {
        NativeWindow(window)
    }

    pub fn handle(&self) -> SurfaceHandle {
        SurfaceHandle::Native(Rc::new(self.clone()))
    }

    /// Whether both wrap the same window object.
    pub fn same(&self, other: &NativeWindow) -> bool {
        std::ptr::addr_eq(Arc::as_ptr(&self.0), Arc::as_ptr(&other.0))
    }
}

impl HasWindowHandle for NativeWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for NativeWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

/// The platform's windows as surfaces: what the driver attaches renderers to.
///
/// A window exists from [`Platform::create_window`] (or the platform's start) until its
/// [`WindowEvent::Closed`]; its surface comes and goes inside that span
/// ([`WindowEvent::SurfaceCreated`], [`WindowEvent::SurfaceLost`]).
pub trait SurfaceHost {
    /// The open windows, in creation order.
    fn windows(&self) -> Vec<WindowId>;
    /// The window's current surface; `None` while it is lost, not yet created or the app is
    /// suspended.
    fn surface(&self, window: WindowId) -> Option<SurfaceHandle>;
    fn size_px(&self, window: WindowId) -> SizeU;
    fn scale(&self, window: WindowId) -> f32;
}

/// The system clipboard. Reads are asynchronous (the web's `navigator.clipboard`), so content
/// arrives later as [`WindowEvent::Paste`] in the window that asked.
pub trait Clipboard {
    fn set_text(&mut self, text: &str);
    fn request_text(&mut self, window: WindowId);
    /// Puts an image on the clipboard. Platforms without image support ignore it.
    fn set_image(&mut self, image: &RgbaImage) {
        let _ = image;
    }
    /// Asks for the clipboard's image; it arrives as [`WindowEvent::PasteImage`] when there is one.
    fn request_image(&mut self, window: WindowId) {
        let _ = window;
    }
}

/// Wakes the event loop from any thread: a background job or engine thread calls [`Waker::wake`]
/// when it has a result, and the platform delivers [`PlatformEvent::Wake`] and asks for a step.
#[derive(Clone)]
pub struct Waker(Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Waker {
        Waker(Arc::new(wake))
    }

    pub fn wake(&self) {
        (self.0)();
    }
}

impl std::fmt::Debug for Waker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Waker")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImePurpose {
    #[default]
    Normal,
    Password,
}

/// Rects are logical points in window coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum ImeRequest {
    Start { purpose: ImePurpose, cursor: Rect },
    Cursor(Rect),
    Stop,
}

/// The pointer's look over a window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Cursor {
    #[default]
    Default,
    Icon(CursorIcon),
    /// No pointer drawn (a canvas tool painting its own).
    Hidden,
    /// An image from the image store with its hot spot in image pixels.
    Image {
        key: ImageKey,
        hotspot: (u32, u32),
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Move,
    Down,
    Up,
    Cancel,
    /// The pointer left the surface (mouse and pen only).
    Leave,
}

/// One pointer sample in logical window points. Touch contacts and pens carry their own ids, so
/// several contacts can be down at once (a pinch). During a press the platform keeps delivering
/// the pointer to the window it went down in, with positions outside it while it is over other
/// windows (pointer capture).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub id: u64,
    pub kind: PointerKind,
    pub phase: PointerPhase,
    pub button: PointerButton,
    pub pos: Pos2,
    pub mods: Mods,
    /// A pen's reading (pressure, tilt, barrel rotation, eraser end) or a touch contact's force,
    /// for this sample. Every sample the device reported comes as its own event, so a stroke sees
    /// them all.
    pub sample: Option<PointerSample>,
}

impl PointerEvent {
    /// A sample with no pen reading.
    pub const fn new(id: u64, kind: PointerKind, phase: PointerPhase, button: PointerButton, pos: Pos2, mods: Mods) -> PointerEvent {
        PointerEvent { id, kind, phase, button, pos, mods, sample: None }
    }

    #[must_use]
    pub const fn with_sample(mut self, sample: PointerSample) -> PointerEvent {
        self.sample = Some(sample);
        self
    }
}

/// A file the OS drags over a window or drops on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OsFile {
    /// The file's name; empty while the OS tells only a hovered file's type.
    pub name: String,
    /// Where the file is, on platforms that give a path.
    pub path: Option<PathBuf>,
    /// The file's media type, as far as the OS knows it.
    pub mime: String,
    /// The file's content, on platforms that give no path (the web); set only on a drop.
    pub bytes: Option<Arc<[u8]>>,
}

/// The files of one OS drag, the payload ([`crate::DragData`]) drop targets take them as.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OsFiles {
    /// In the order the OS lists them.
    pub files: Vec<OsFile>,
}

/// An OS drag of files over one window. Positions are logical window points; desktop platforms
/// often give none while the OS runs its drag loop, and the runtime then uses the last pointer
/// position it saw in the window, or waits for the next one.
#[derive(Clone, Debug, PartialEq)]
pub enum FileDrag {
    /// Files entered the window (one event for every file the OS listed in the same poll).
    Enter {
        files: Vec<OsFile>,
        pos: Option<Pos2>,
    },
    Over(Pos2),
    /// The files left the window or the OS cancelled the drag.
    Leave,
    Drop {
        files: Vec<OsFile>,
        pos: Option<Pos2>,
    },
}

/// The operating system's light or dark setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemTheme {
    Light,
    Dark,
}

/// An event of one window.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    Pointer(PointerEvent),
    /// A wheel or touchpad scroll, `delta` in the device's `unit` as it reported it. Windows and
    /// Linux touchpad drivers that send a pinch as Ctrl+wheel arrive here too; the runtime folds
    /// them into a pinch ([`crate::WheelOptions::ctrl_wheel_pinch`]).
    Wheel {
        pos: Pos2,
        delta: Vec2,
        unit: WheelUnit,
        phase: WheelPhase,
        source: WheelSource,
        mods: Mods,
    },
    /// A touchpad pinch or rotation the system recognised (macOS magnify and rotate, Wayland and
    /// Windows precision touchpads): `zoom` is the step's factor (1 for none), `rotation` its
    /// radians, clockwise on screen.
    Pinch {
        pos: Pos2,
        zoom: f32,
        rotation: f32,
        mods: Mods,
    },
    /// An OS drag of files ([`FileDrag`]).
    Files(FileDrag),
    Key {
        key: Key,
        mods: Mods,
        down: bool,
        repeat: bool,
    },
    Text(String),
    ImePreedit {
        text: String,
        cursor: Option<Range<usize>>,
    },
    ImeCommit(String),
    Paste(String),
    PasteImage(Arc<RgbaImage>),
    /// The system theme changed, or was read when the window opened.
    Theme(SystemTheme),
    Access(accesskit::ActionRequest),
    /// An assistive technology asked for the window's tree (accesskit's initial tree request).
    /// The next frame sends the whole tree; updates before it are not sent. Sent again on a later
    /// activation.
    AccessActivated,
    Scale(f32),
    /// The surface's logical size.
    Resize(Vec2),
    /// The surface's top left on the virtual desktop, in physical pixels (the desktop has no
    /// single scale when monitors differ). Platforms that cannot tell (Wayland) never send it, and
    /// drags then find other windows only by the pointer events they deliver.
    Moved(Pos2),
    SafeArea(Margin),
    SurfaceCreated,
    SurfaceLost,
    /// The window gained (`true`) or lost key focus.
    Focus(bool),
    /// The user asked to close the window (title bar button, Alt+F4). The window stays until the
    /// app closes it.
    CloseRequested,
    /// The window is gone; its nodes go with it.
    Closed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformEvent {
    Window(WindowId, WindowEvent),
    Suspend,
    Resume,
    /// A [`Waker`] fired since the last poll (several wakes may arrive as one).
    Wake,
}

/// What a new window looks like. Sizes and positions are logical points; the platform picks
/// whatever is `None`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    pub size: Option<Vec2>,
    pub min_size: Option<Vec2>,
    /// The outer top left on the virtual desktop.
    pub position: Option<Pos2>,
    pub visible: bool,
    pub resizable: bool,
    /// System title bar and borders; false for windows that draw their own chrome.
    pub decorations: bool,
    /// Draws over the window it is attached to and moves with it (a floating panel window).
    pub owner: Option<WindowId>,
}

impl WindowSpec {
    /// A visible, resizable, decorated window with this title.
    pub fn new(title: impl Into<String>) -> WindowSpec {
        WindowSpec { title: title.into(), visible: true, resizable: true, decorations: true, ..WindowSpec::default() }
    }
}

/// A change to an open window.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowCommand {
    Title(String),
    /// Inner size in logical points.
    Size(Vec2),
    MinSize(Option<Vec2>),
    /// Outer top left on the virtual desktop, in logical points.
    Position(Pos2),
    Visible(bool),
    Decorations(bool),
    Minimized(bool),
    Maximized(bool),
    Fullscreen(bool),
    /// Gives the window key focus.
    Focus,
    /// Brings the window to the front without focus.
    Raise,
    /// Starts a system move of the window with the pressed pointer (a custom title bar).
    DragMove,
    Close,
}

/// The host side of the runtime: windows, input, IME, clipboard, cursors, accessibility and
/// frame scheduling. [`crate::Driver::step`] calls it once per turn; it never calls back into the
/// runtime. Implementations: winit (desktop and web), headless (tests and bench).
pub trait Platform {
    fn surfaces(&mut self) -> &mut dyn SurfaceHost;
    /// Moves the events received since the last call into `out`. Never blocks.
    fn poll(&mut self, out: &mut Vec<PlatformEvent>);
    /// Opens a window and returns its id at once; its surface arrives later as
    /// [`WindowEvent::SurfaceCreated`].
    fn create_window(&mut self, spec: &WindowSpec) -> WindowId;
    /// Applies a change to an open window; ignored for a closed one.
    fn window(&mut self, window: WindowId, cmd: WindowCommand);
    fn ime(&mut self, window: WindowId, req: ImeRequest);
    fn set_cursor(&mut self, window: WindowId, cursor: Cursor);
    fn clipboard(&mut self) -> &mut dyn Clipboard;
    /// The window's access tree update; dropped before the window's [`WindowEvent::AccessActivated`].
    fn access(&mut self, window: WindowId, update: accesskit::TreeUpdate);
    /// Asks for one more call of the driver's `step` (a redraw request, an animation frame).
    fn request_frame(&mut self);
    /// Asks for a call of `step` once the frame time reaches `time` (a timer: a caret's next
    /// blink). Replaces the time asked for before.
    fn request_frame_at(&mut self, time: f64);
    /// A waker for background threads; every call returns one that wakes this platform.
    fn waker(&self) -> Waker;
}
