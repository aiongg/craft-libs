//! Value controls: `Checkbox`, `Slider`, `RadioGroup` and `Radio`, `Disclosure`.

pub mod check;
pub mod disclosure;
pub mod radio;
pub mod slider;
#[cfg(feature = "testing")]
mod stories;

use crate::style::{FocusLook, StyleCx};

/// The ring of a control that holds keyboard focus: the line's focus colour at the medium stroke, on the shape's edge.
pub(crate) fn focus_look(cx: &StyleCx<'_>) -> FocusLook {
    FocusLook::new(egui::Stroke::new(cx.tokens.stroke.medium, cx.color(cx.tokens.color.line.focus).egui()), 0.0)
}
