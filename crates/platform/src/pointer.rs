//! Moving the system pointer, for a scrub that wraps at the screen's edge. egui-free: the app
//! installs [`warp`] as craft-ui's `behavior::Scrub::install_warp` service, converting from
//! viewport points to global screen points itself.
//!
//! macOS: `CGWarpMouseCursorPosition` followed by `CGAssociateMouseAndMouseCursorPosition(true)`,
//! so the pointer moves at once and the next mouse delta is not swallowed (egui's
//! `ViewportCommand::CursorPosition` arrives a frame late there). Other platforms report `false`
//! and the app leaves the move to egui.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// Move the pointer to `(x, y)` in global screen points (macOS: the top-left of the main display
/// is the origin, y down). Returns whether the platform moved it.
// pc: crates/ui-egui/src/input/scrub_wrap.rs:15 `wrap_target` (where to), number_field.rs ViewportCommand fallback.
pub fn warp(x: f64, y: f64) -> bool {
    todo!()
}

/// Whether [`warp`] can move the pointer on this platform (macOS: yes; elsewhere: no).
pub fn can_warp() -> bool {
    cfg!(target_os = "macos")
}
