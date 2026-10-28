//! Behaviour that several controls share: keyboard, focus and selection rules from the WAI-ARIA
//! patterns, kept apart from how each control looks.
//!
//! - [`RadioSelection`]: a radio group's one Tab stop, arrow keys and click selection, used by
//!   [`RadioGroup`](crate::controls::RadioGroup), [`RadioSet`](crate::controls::RadioSet) and
//!   [`Segmented`](crate::controls::Segmented).
//! - [`Typing`]: a line of text typed into egui's `TextEdit` and reported as one edit gesture per
//!   typing run, used by [`TextField`](crate::controls::TextField) and
//!   [`SearchField`](crate::controls::SearchField).

mod radio_selection;
#[cfg(test)]
mod tests;
mod typing;

pub use radio_selection::RadioSelection;
pub use typing::{Escape, Typed, Typing, TypingLook};
