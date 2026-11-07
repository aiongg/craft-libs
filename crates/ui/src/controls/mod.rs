//! Value controls: `Checkbox`, `RadioGroup` and `Radio`, `Disclosure`, `Select`.

pub mod check;
pub mod disclosure;
pub mod radio;
pub mod select;
#[cfg(feature = "testing")]
mod stories;
mod words;

use crate::style::{FocusLook, StyleCx};

/// The ring of a control that holds keyboard focus: the line's focus colour at the medium stroke, on the shape's edge.
pub(crate) fn focus_look(cx: &StyleCx<'_>) -> FocusLook {
    FocusLook::new(egui::Stroke::new(cx.tokens.stroke.medium, cx.color(cx.tokens.color.line.focus).egui()), 0.0)
}
