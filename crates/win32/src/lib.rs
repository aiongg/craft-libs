//! The Win32 calls that need `unsafe`, behind a safe API: subclassing a window's procedure with a handler, and
//! making a window handle from its raw value. Windows only; the crate is empty elsewhere.
//!
//! This is the only crate (besides the benchmark's allocator) that allows `unsafe`; the design-lint rule
//! `contract/unsafe-allow` names it. Every block says why it is sound.
#![warn(missing_docs)]
#![cfg(windows)]
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex, PoisonError};
use winsafe::{self as w, co, msg};

/// Tells this crate's subclass from another one on the same window.
const SUBCLASS_ID: usize = 0x4352_5733;

/// Answers a message of a subclassed window: `Some(result)` ends it there, `None` passes it to the window's own
/// procedure. It runs on the thread that owns the window. A destroyed window sends it `WM_NCDESTROY` last.
pub type Handler = fn(window: &w::HWND, message: co::WM, wparam: usize, lparam: isize) -> Option<isize>;

static HANDLERS: LazyLock<Mutex<HashMap<isize, Handler>>> = LazyLock::new(Default::default);

/// The window handle with the raw value `raw`, from `HWND::ptr() as isize` or a platform handle. A value that names
/// no live window only makes calls on the handle fail.
pub fn handle(raw: isize) -> w::HWND {
    // SAFETY: `HWND` has no destructor and nothing here dereferences it; the system validates the handle in every
    // call made with it, as for any stale window handle.
    unsafe { w::HWND::from_ptr(raw as *mut c_void) }
}

/// The raw value of `window`.
pub fn raw(window: &w::HWND) -> isize {
    window.ptr() as isize
}

/// Puts a subclass on the window `raw` that offers every message to `handler` first. Returns whether the window is
/// subclassed (a window subclassed before keeps its first handler). The subclass ends with the window. The window
/// must belong to the calling thread.
pub fn subclass(raw: isize, handler: Handler) -> bool {
    if handlers(|handlers| handlers.contains_key(&raw)) {
        return true;
    }
    let window = handle(raw);
    // SAFETY: `procedure` is a plain function with the `SUBCLASSPROC` signature that outlives every window, and the
    // reference data is unused. Comctl32 calls it on the thread that owns the window; for another thread's window
    // `SetWindowSubclass` fails.
    let set = unsafe { window.SetWindowSubclass(procedure, SUBCLASS_ID, 0) };
    if set.is_err() {
        return false;
    }
    handlers(|handlers| handlers.insert(raw, handler));
    true
}

fn handlers<R>(f: impl FnOnce(&mut HashMap<isize, Handler>) -> R) -> R {
    f(&mut HANDLERS.lock().unwrap_or_else(PoisonError::into_inner))
}

extern "system" fn procedure(window: w::HWND, message: co::WM, wparam: usize, lparam: isize, _id: usize, _data: usize) -> isize {
    let key = raw(&window);
    let handler = handlers(|handlers| handlers.get(&key).copied());
    let diag = diag_on();
    if diag {
        diag_line(&window, message, wparam, lparam, None);
    }
    if let Some(answer) = handler.and_then(|handler| handler(&window, message, wparam, lparam)) {
        return answer;
    }
    // SAFETY: the message and its parameters are the ones the system delivered to this procedure, passed on
    // unchanged to the next procedure of the chain.
    let started = std::time::Instant::now();
    let result = unsafe { window.DefSubclassProc(msg::Wm::new(message, wparam, lparam)) };
    if diag && message == co::WM::PAINT {
        diag_line(&window, message, wparam, lparam, Some(started.elapsed().as_secs_f64() * 1000.0));
    }
    if message == co::WM::NCDESTROY {
        handlers(|handlers| handlers.remove(&key));
        // The window ends with this message, so a failed removal leaves nothing to clean up.
        let _ = window.RemoveWindowSubclass(procedure, SUBCLASS_ID);
    }
    result
}

/// Whether `PHOTOCRAFT_DIAG` is set (read once): the sizing-loop messages are then logged to stderr.
fn diag_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PHOTOCRAFT_DIAG").is_some())
}

/// One `DIAG w32` line for a message of the sizing loop, with the cursor and the window rectangle. `paint_ms` is the
/// time the next procedure took for `WM_PAINT`, whose line is written after it returns.
fn diag_line(window: &w::HWND, message: co::WM, wparam: usize, lparam: isize, paint_ms: Option<f64>) {
    static FIRST: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let extra = match message {
        // The hit-test code the press landed on (`HTBOTTOMRIGHT` = 17, `HTCAPTION` = 2).
        co::WM::NCLBUTTONDOWN => format!("ht={wparam}"),
        co::WM::ENTERSIZEMOVE => String::new(),
        co::WM::SIZING => {
            // SAFETY: for `WM_SIZING`, `lparam` points to the `RECT` the system owns for the duration of the message.
            let r = unsafe { &*(lparam as *const w::RECT) };
            format!("sizing=({},{},{},{})", r.left, r.top, r.right, r.bottom)
        }
        co::WM::WINDOWPOSCHANGING | co::WM::WINDOWPOSCHANGED => {
            // SAFETY: for these messages, `lparam` points to the `WINDOWPOS` the system owns for the duration of the
            // message.
            let p = unsafe { &*(lparam as *const w::WINDOWPOS) };
            format!("pos=({},{},{},{}) flags={:#x}", p.x, p.y, p.cx, p.cy, p.flags.raw())
        }
        co::WM::SIZE => format!("size=({},{})", lparam as i16 as i32 & 0xffff, (lparam >> 16) as i16 as i32 & 0xffff),
        co::WM::PAINT => match paint_ms {
            Some(ms) => format!("proc_ms={ms:.2}"),
            None => return,
        },
        co::WM::CAPTURECHANGED | co::WM::CANCELMODE | co::WM::LBUTTONUP | co::WM::NCLBUTTONUP | co::WM::EXITSIZEMOVE => String::new(),
        _ => return,
    };
    let name = match message {
        co::WM::NCLBUTTONDOWN => "NCLBUTTONDOWN",
        co::WM::ENTERSIZEMOVE => "ENTERSIZEMOVE",
        co::WM::SIZING => "SIZING",
        co::WM::WINDOWPOSCHANGING => "WINDOWPOSCHANGING",
        co::WM::WINDOWPOSCHANGED => "WINDOWPOSCHANGED",
        co::WM::SIZE => "SIZE",
        co::WM::PAINT => "PAINT",
        co::WM::CAPTURECHANGED => "CAPTURECHANGED",
        co::WM::CANCELMODE => "CANCELMODE",
        co::WM::LBUTTONUP => "LBUTTONUP",
        co::WM::NCLBUTTONUP => "NCLBUTTONUP",
        _ => "EXITSIZEMOVE",
    };
    let ms = FIRST.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0;
    let cursor = w::GetCursorPos().map(|p| (p.x, p.y)).unwrap_or((-1, -1));
    let rect = window.GetWindowRect().map(|r| (r.left, r.top, r.right, r.bottom)).unwrap_or((-1, -1, -1, -1));
    eprintln!("DIAG w32 {ms:.2} {name} cursor=({},{}) rect=({},{},{},{}) {extra}", cursor.0, cursor.1, rect.0, rect.1, rect.2, rect.3);
}
