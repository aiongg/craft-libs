//! Nudging canvas objects with the arrow keys: a selection outline, a guide, a transform box or
//! an anchor (PhotoCraft: 1 pixel, Shift 10, along the document axes).
//!
//! Each press is one [`EditEvent::Set`](crate::edit::EditEvent::Set); the host bundles a run of
//! presses into one undo step (PhotoCraft's `nudge_bundle`).

use egui::{Event, Key, Modifiers, Ui};
use kurbo::Vec2;

use crate::popup_list;

/// The move of one press, in document units (PhotoCraft's).
pub const STEP: f64 = 1.0;
/// The move of one press with Shift held.
pub const BIG_STEP: f64 = 10.0;

/// Consumes one arrow key press, plain or with Shift only, and returns the move in document units
/// along the document axes: Right +x, Down +y, `step` or with Shift `big_step`. Right to left does
/// not swap Left and Right: the document doesn't mirror.
///
/// Reads nothing while a text field has focus, a popup is open or a modal layer is above `ui`.
pub fn read(ui: &Ui, step: f64, big_step: f64) -> Option<Vec2> {
    if !reads(ui) {
        return None;
    }
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| arrow(e).is_some())?;
        let (dir, shift) = arrow(&i.events[at])?;
        i.events.remove(at);
        Some(dir * if shift { big_step } else { step })
    })
}

/// [`read`] that also takes Alt and Alt+Shift, and reports whether Alt was held: an arrow with
/// Alt moves a path's direction handle alone, without its partner (PhotoCraft's ⌥-drag).
pub fn read_with_alt(ui: &Ui, step: f64, big_step: f64) -> Option<(Vec2, bool)> {
    if !reads(ui) {
        return None;
    }
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| arrow_with_alt(e).is_some())?;
        let (dir, shift, alt) = arrow_with_alt(&i.events[at])?;
        i.events.remove(at);
        Some((dir * if shift { big_step } else { step }, alt))
    })
}

/// Whether the canvas's keys are free: no text field has focus, no popup is open and no modal layer
/// is above `ui`.
pub fn reads(ui: &Ui) -> bool {
    let ctx = ui.ctx();
    !(ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())))
}

/// An arrow key press plain or with Shift: its unit direction and whether Shift is held.
fn arrow(event: &Event) -> Option<(Vec2, bool)> {
    match arrow_with_alt(event)? {
        (dir, shift, false) => Some((dir, shift)),
        _ => None,
    }
}

/// An arrow key press with no modifier but Shift and Alt: its unit direction, whether Shift is
/// held and whether Alt is.
fn arrow_with_alt(event: &Event) -> Option<(Vec2, bool, bool)> {
    let Event::Key { key, pressed: true, modifiers, .. } = event else { return None };
    let dir = match key {
        Key::ArrowLeft => Vec2::new(-1.0, 0.0),
        Key::ArrowRight => Vec2::new(1.0, 0.0),
        Key::ArrowUp => Vec2::new(0.0, -1.0),
        Key::ArrowDown => Vec2::new(0.0, 1.0),
        _ => return None,
    };
    [Modifiers::NONE, Modifiers::SHIFT, Modifiers::ALT, Modifiers::ALT | Modifiers::SHIFT]
        .into_iter()
        .find(|&m| modifiers.matches_exact(m))
        .map(|m| (dir, m.shift, m.alt))
}
