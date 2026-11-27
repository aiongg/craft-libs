//! The platform boundary: windows and their surfaces, input, IME, clipboard, cursors,
//! accessibility, frame scheduling and wakes from other threads. Nothing here assumes a mouse, a
//! fixed DPI, a desktop window manager or threads: a browser platform delivers DOM input through
//! `poll`, draws into canvas surfaces, runs IME through a hidden input element and schedules frames
//! with `requestAnimationFrame`.

use std::any::Any;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use craft_behavior::{CursorIcon, PointerButton, PointerKind};
use craft_keymap::chord::{Key, Mods};
use emath::{Pos2, Rect, Vec2};

use crate::display::ImageKey;
use crate::geom::{Margin, SizeU};
use crate::id::WindowId;

/// What a renderer draws into. `Native` holds the platform's object (a winit window, a web canvas
/// element, an Android native window); the renderer for that platform downcasts it.
#[derive(Clone)]
pub enum SurfaceHandle {
    Offscreen,
    Native(Rc<dyn Any>),
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

/// One pointer sample in logical window points. Touch contacts and pens carry their own ids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub id: u64,
    pub kind: PointerKind,
    pub phase: PointerPhase,
    pub button: PointerButton,
    pub pos: Pos2,
    pub mods: Mods,
}

/// An event of one window.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    Pointer(PointerEvent),
    /// Scroll delta in logical points.
    Wheel {
        pos: Pos2,
        delta: Vec2,
        mods: Mods,
    },
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
    Access(accesskit::ActionRequest),
    /// An assistive technology asked for the window's tree (accesskit's initial tree request).
    /// The next frame sends the whole tree; updates before it are not sent. Sent again on a later
    /// activation.
    AccessActivated,
    Scale(f32),
    /// The surface's logical size.
    Resize(Vec2),
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
