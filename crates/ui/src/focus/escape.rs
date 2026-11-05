//! Escape on a focused control.

use egui::{Event, Key, Response};

/// Whether Escape was pressed this pass on a control that had the keyboard focus when the pass
/// began.
///
/// egui drops the focus on a plain Escape before any widget runs, so by the time a control is
/// drawn it reports `lost_focus()` for Escape, for a click elsewhere and for Tab alike, and
/// `has_focus()` is already false. This reads the focus egui kept from the previous pass
/// instead. Only a plain Escape counts (egui keeps the focus on a modified one), and a control that consumed the key itself reports false.
pub fn escaped(response: &Response) -> bool {
    let ctx = &response.ctx;
    let pressed =
        ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { key: Key::Escape, pressed: true, modifiers, .. } if !modifiers.any())));
    pressed && ctx.memory(|m| m.had_focus_last_frame(response.id))
}
