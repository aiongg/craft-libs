//! The title bar's window side: which window the app's bar lets the OS move, and on macOS the `NSWindow.movable`
//! flag that follows it.

use crate::runtime::egui::viewports;
use egui::Context;

/// Whether the OS may move the running viewport's window from the title-bar band: the last bar drawn in this
/// pass or the one before said so, else true.
pub fn window_movable(ctx: &Context) -> bool {
    match viewports::movable_record(ctx) {
        Some((pass, movable)) if viewports::pass_number(ctx).saturating_sub(pass) <= 1 => movable,
        _ => true,
    }
}

/// Records that this pass's title bar of the running viewport lets the OS move the window (`true`) or not. A pass
/// that draws no bar writes it too, or AppKit drags the window from the transparent strip. The record holds for
/// this pass and the next.
pub fn set_window_movable(ctx: &Context, movable: bool) {
    viewports::write_movable(ctx, movable);
}

/// Sets the `movable` flag of the context's window from this pass's [`window_movable`]. Call it once per frame per
/// window, after that window's UI has run; `applied` holds the value last set for the window, so AppKit is only
/// called when it changes.
///
/// A window is told apart by its viewport: the key window is the one whose viewport has focus, so an unfocused
/// viewport is left as it is. Only a window with a full-size content view and visible window buttons is touched,
/// because floating panels in their own windows keep their own `movable`, which AppKit also needs to resize them.
/// macOS only; Apple builds do not run on the dev box, so this compiles by inspection until the Mac builds it.
#[cfg(target_os = "macos")]
pub fn sync_movable(ctx: &Context, applied: &mut Option<bool>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowButton, NSWindowStyleMask};

    if !viewports::current(ctx).focus_reported {
        return;
    }
    let movable = window_movable(ctx);
    if *applied == Some(movable) {
        return;
    }
    // eframe runs the app on the main thread; anywhere else AppKit is out of reach.
    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some(window) = NSApplication::sharedApplication(mtm).keyWindow() else { return };
    let buttons = window.standardWindowButton(NSWindowButton::CloseButton).is_some_and(|close| !close.isHidden());
    if window.styleMask().contains(NSWindowStyleMask::FullSizeContentView) && buttons {
        window.setMovable(movable);
        *applied = Some(movable);
    }
}
