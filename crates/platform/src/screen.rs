//! Screen capture for the screen colour picker (F04.01): every monitor's pixels and ICC profile,
//! taken once when the picker opens. The UI side is `craft_color::ui::screen_picker`.
//!
//! Backends: X11 (xcb `GetImage` per output, `_ICC_PROFILE` atoms), Wayland (the ScreenCast /
//! Screenshot portal, which asks the user once), macOS (`CGDisplayCreateImage`, the display's
//! colour space), Windows (`BitBlt` per monitor, `GetICMProfile`). Web has no capture:
//! [`capture`] returns [`ScreenError::Unsupported`].

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// One monitor's capture.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenImage {
    /// Where the monitor sits in the virtual desktop, in physical pixels.
    pub origin: [i32; 2],
    /// Pixels per row and rows.
    pub size: [u32; 2],
    /// The monitor's scale factor (points to pixels).
    pub scale: f32,
    /// Straight RGBA, row-major, `size[0] * size[1] * 4` bytes.
    pub rgba: Arc<Vec<u8>>,
    /// The monitor's ICC profile bytes when the system has one; the picker converts samples to
    /// sRGB through it.
    pub icc_profile: Option<Arc<Vec<u8>>>,
    /// The monitor's name as the system gives it ("DP-1", "Built-in Retina Display").
    pub name: String,
}

/// Why a capture failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenError {
    /// No backend on this platform (web) or display session.
    Unsupported,
    /// The user declined the portal's request (Wayland) or the OS's screen-recording permission
    /// (macOS).
    Denied,
    /// [`CaptureHandle::cancel`] was called before the capture finished.
    Cancelled,
    /// Something else, with the backend's message.
    Failed(String),
}

/// A capture in flight on a worker thread; dropping it cancels.
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    result: std::sync::mpsc::Receiver<Result<Vec<ScreenImage>, ScreenError>>,
}

impl CaptureHandle {
    /// The capture, once it has finished; `None` while it is still running.
    // pc: apps/photocraft/src/screen_color.rs:10 `service` (the receiver half).
    pub fn try_take(&self) -> Option<Result<Vec<ScreenImage>, ScreenError>> {
        todo!()
    }

    /// Stops a capture that has not finished; a later `try_take` gives `Cancelled`.
    pub fn cancel(&self) {
        todo!()
    }
}

/// Starts capturing every monitor on a worker thread. The images come back in monitor order with
/// the primary first. Validates sizes against [`MAX_PIXELS`] per image.
// pc: apps/photocraft/src/screen_color.rs:119 `capture` (X11), :197 `portal` (Wayland), :248 and
// :288 (macOS, Windows), :491 (other: Unsupported); :63 `icc_profile` for the profile bytes.
pub fn capture() -> CaptureHandle {
    todo!()
}

/// The most pixels one monitor image may hold (64 M); larger captures fail with `Failed`.
// pc: crates/ui-egui/src/dialogs/screen_picker.rs:37 `MAX_PIXELS`.
pub const MAX_PIXELS: usize = 64 * 1024 * 1024;

#[cfg(test)]
mod tests {
    /// A premultiplied BGRA capture with a stride unpacks to straight RGBA in row order.
    #[test]
    #[ignore = "skeleton"]
    fn unpacks_strided_premultiplied_pixels() {
        todo!()
    }
}
