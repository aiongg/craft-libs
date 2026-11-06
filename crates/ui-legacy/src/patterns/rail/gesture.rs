//! The pointer rules of a flyout button as pure functions, so their edges can be tested without a
//! frame loop. Releasing over a row to choose it is the menu's own rule
//! ([`Menu::pressing`](crate::menu::Menu::pressing)).

use std::time::Duration;

use egui::{Pos2, Rect};

/// How long the primary button is held on a button to open its flyout, in seconds.
pub(crate) const LONG_PRESS: f64 = 0.35;

/// A primary-button gesture that started on the button and has left it: opens the flyout. The
/// press origin, not egui's drag state, so that it also works on a click-only button.
pub(crate) fn drag_open(button: Rect, origin: Option<Pos2>, pointer: Option<Pos2>, primary_down: bool) -> bool {
    primary_down && origin.is_some_and(|p| button.contains(p)) && pointer.is_some_and(|p| !button.contains(p))
}

/// Whether the button has been held for [`LONG_PRESS`]; `held_for` is the time since the press
/// began, `None` while no press is on the button.
pub(crate) fn long_press(held_for: Option<f64>) -> bool {
    held_for.is_some_and(|seconds| seconds >= LONG_PRESS)
}

/// The rest of the hold, to schedule the repaint that sees it complete.
pub(crate) fn long_press_wait(held_for: Option<f64>) -> Option<Duration> {
    held_for.filter(|seconds| *seconds < LONG_PRESS).map(|seconds| Duration::from_secs_f64(LONG_PRESS - seconds))
}

/// Whether a click on the button closes the flyout it opened: a click or secondary click while it
/// is open does, but not the release that ends the press that opened it (`opening_press`).
pub(crate) fn reclick_closes(open: bool, secondary_clicked: bool, clicked: bool, opening_press: bool) -> bool {
    open && (secondary_clicked || (clicked && !opening_press))
}
