//! The Win32 side of [`crate::native`]: a window procedure that answers `WM_NCHITTEST` from the app's frame parts and
//! reports the maximize button's pointer events, finding an app window among the thread's windows, and layered-window
//! opacity.
//!
//! This is the one module of the workspace that uses `unsafe`; the crate and every other crate deny it, and the
//! design-lint rule `contract/unsafe-allow` names this file as the only place that may allow it. Every block says
//! why it is sound. winsafe wraps the rest.
#![allow(unsafe_code)]

use crate::Button;
use crate::native::{CaptionEvent, Hits, hit_code};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use winsafe::{self as w, co, msg, prelude::*};

/// `HTMAXBUTTON`, the hit-test result and the `wParam` of the non-client messages over the maximize button.
const HT_MAX_BUTTON: usize = 9;
/// Tells this module's subclass from another one on the same window.
const SUBCLASS_ID: usize = 0x4352_4146;

/// A window of this process by its handle value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Hwnd(isize);

impl Hwnd {
    pub(crate) fn new(raw: isize) -> Self {
        Hwnd(raw)
    }

    fn of(window: &w::HWND) -> Self {
        Hwnd(window.ptr() as isize)
    }

    fn handle(self) -> w::HWND {
        // SAFETY: the value came from a window handle of this process (eframe's `Frame`, an enumeration of this
        // thread's windows or the procedure's own `hwnd`). A handle whose window has since been destroyed makes the
        // calls on it fail, as for any stale `HWND`; none of them dereferences it.
        unsafe { w::HWND::from_ptr(self.0 as *mut c_void) }
    }
}

/// A routed window: where the hit test goes, and what the pointer on the maximize button did so far.
struct Routed {
    hits: Arc<dyn Hits>,
    /// `TrackMouseEvent` is armed for the non-client leave.
    tracking: bool,
    /// The pointer is over the maximize button.
    over: bool,
    /// The primary button went down on the maximize button.
    pressed: bool,
}

static ROUTED: LazyLock<Mutex<HashMap<isize, Routed>>> = LazyLock::new(Default::default);

fn routed<R>(f: impl FnOnce(&mut HashMap<isize, Routed>) -> R) -> R {
    f(&mut ROUTED.lock().unwrap_or_else(PoisonError::into_inner))
}

/// The window of this thread titled `title` that is not routed yet. Of several, the one whose client area starts
/// nearest `client_origin`, in physical screen pixels.
pub(crate) fn find(title: &str, client_origin: Option<[i32; 2]>) -> Option<Hwnd> {
    let mut best: Option<(i64, Hwnd)> = None;
    let listed = w::EnumThreadWindows(w::GetCurrentThreadId(), |window: w::HWND| {
        let here = Hwnd::of(&window);
        if window.GetWindowText().is_ok_and(|text| text == title) && !routed(|routed| routed.contains_key(&here.0)) {
            let distance = client_origin.map_or(0, |[x, y]| {
                window.ClientToScreen(w::POINT::with(0, 0)).map_or(i64::MAX, |origin| {
                    let (dx, dy) = (i64::from(origin.x - x), i64::from(origin.y - y));
                    dx * dx + dy * dy
                })
            });
            if best.is_none_or(|(nearest, _)| distance < nearest) {
                best = Some((distance, here));
            }
        }
        true
    });
    listed.ok()?;
    best.map(|(_, hwnd)| hwnd)
}

/// Sets the window's opacity through a layered window. Fully opaque takes the layered style off again, so an opaque
/// window is drawn as any other.
pub(crate) fn set_opacity(hwnd: Hwnd, opacity: f32) -> bool {
    let window = hwnd.handle();
    let layered = window.style_ex().has(co::WS_EX::LAYERED);
    if opacity >= 1.0 {
        if layered {
            window.set_style_ex(window.style_ex() & !co::WS_EX::LAYERED);
        }
        return true;
    }
    if !layered {
        window.set_style_ex(window.style_ex() | co::WS_EX::LAYERED);
    }
    let alpha = (opacity.max(0.0) * 255.0).round() as u8;
    window.SetLayeredWindowAttributes(w::COLORREF::from_rgb(0, 0, 0), alpha, co::LWA::ALPHA).is_ok()
}

/// Puts the procedure on `hwnd`, answering the hit test and the maximize button's pointer from `hits`. Returns whether
/// the window is routed (a window routed before keeps its first `hits`).
pub(crate) fn route(hwnd: Hwnd, hits: Arc<dyn Hits>) -> bool {
    if routed(|routed| routed.contains_key(&hwnd.0)) {
        return true;
    }
    let window = hwnd.handle();
    // SAFETY: `procedure` is a plain function with the `SUBCLASSPROC` signature that outlives the window, and the
    // reference data is unused. Comctl32 calls it on the thread that owns the window, which is this one: subclassing
    // another thread's window fails.
    let set = unsafe { window.SetWindowSubclass(procedure, SUBCLASS_ID, 0) };
    if set.is_err() {
        return false;
    }
    routed(|routed| routed.insert(hwnd.0, Routed { hits, tracking: false, over: false, pressed: false }));
    true
}

/// The subclass procedure: the messages this module answers, else the window's own procedure.
extern "system" fn procedure(window: w::HWND, message: co::WM, wparam: usize, lparam: isize, _id: usize, _data: usize) -> isize {
    if let Some(answer) = answer(&window, message, wparam, lparam) {
        return answer;
    }
    let destroyed = message == co::WM::NCDESTROY;
    // SAFETY: the message and its parameters are the ones the system delivered to this procedure, passed on
    // unchanged to the next procedure of the chain.
    let result = unsafe { window.DefSubclassProc(msg::Wm::new(message, wparam, lparam)) };
    if destroyed {
        routed(|routed| routed.remove(&Hwnd::of(&window).0));
        // The window ends with this message, so a failed removal leaves nothing to clean up.
        let _ = window.RemoveWindowSubclass(procedure, SUBCLASS_ID);
    }
    result
}

/// The result of a message this module handles, `None` to leave it to the window.
fn answer(window: &w::HWND, message: co::WM, wparam: usize, lparam: isize) -> Option<isize> {
    let key = Hwnd::of(window).0;
    let hits = routed(|routed| routed.get(&key).map(|held| held.hits.clone()))?;
    match message {
        co::WM::NCHITTEST => {
            // The low and high words are signed screen coordinates.
            let screen = w::POINT::with(i32::from(lparam as i16), i32::from((lparam >> 16) as i16));
            let client = window.ScreenToClient(screen).ok()?;
            let zone = hits.zone_at([client.x as f32, client.y as f32]);
            hit_code(zone).map(isize::from)
        }
        co::WM::NCMOUSEMOVE if wparam == HT_MAX_BUTTON => {
            let (arm, entered) = routed(|routed| {
                let held = routed.get_mut(&key)?;
                let arm = !std::mem::replace(&mut held.tracking, true);
                Some((arm, !std::mem::replace(&mut held.over, true)))
            })?;
            if arm {
                track_leave(window);
            }
            if entered {
                hits.caption(CaptionEvent::Hover(Button::Maximize));
            }
            Some(0)
        }
        co::WM::NCMOUSEMOVE => {
            left(&key, &hits);
            None
        }
        co::WM::NCMOUSELEAVE => {
            routed(|routed| routed.get_mut(&key).map(|held| held.tracking = false));
            left(&key, &hits);
            None
        }
        co::WM::NCLBUTTONDOWN | co::WM::NCLBUTTONDBLCLK if wparam == HT_MAX_BUTTON => {
            routed(|routed| routed.get_mut(&key).map(|held| held.pressed = true));
            hits.caption(CaptionEvent::Press(Button::Maximize));
            Some(0)
        }
        // The window's own handling of this release would maximize the window; the app's button does it.
        co::WM::NCLBUTTONUP if wparam == HT_MAX_BUTTON => {
            if routed(|routed| routed.get_mut(&key).is_some_and(|held| std::mem::take(&mut held.pressed))) {
                hits.caption(CaptionEvent::Release(Button::Maximize));
            }
            Some(0)
        }
        _ => None,
    }
}

/// The pointer is no longer on the maximize button: its hover ends, and a press that began there is cancelled.
fn left(key: &isize, hits: &Arc<dyn Hits>) {
    let ended = routed(|routed| {
        let held = routed.get_mut(key)?;
        let was = std::mem::take(&mut held.over);
        was.then(|| std::mem::take(&mut held.pressed))
    });
    match ended {
        Some(true) => hits.caption(CaptionEvent::Cancel),
        Some(false) => hits.caption(CaptionEvent::Leave),
        None => {}
    }
}

/// Asks for a `WM_NCMOUSELEAVE` when the pointer leaves the window's non-client area.
fn track_leave(window: &w::HWND) {
    let mut track = w::TRACKMOUSEEVENT::default();
    track.dwFlags = co::TME::LEAVE | co::TME::NONCLIENT;
    track.hwndTrack = Hwnd::of(window).handle();
    // A failed request only means the leave is noticed at the next move over another zone.
    let _ = w::TrackMouseEvent(&mut track);
}
