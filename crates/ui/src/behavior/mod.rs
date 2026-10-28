//! Behaviour that several controls share: keyboard, focus and selection rules, kept apart from how
//! each control looks. They follow the suite's apps (PhotoCraft first, then DesignCraft), and the
//! WAI-ARIA patterns where the apps have no convention.
//!
//! - [`FieldTab`]: a dialog's focus scope, in which Tab visits only the text and number fields.
//! - [`RadioSelection`]: a radio group's Tab stops, focus and click selection, used by
//!   [`RadioGroup`](crate::controls::RadioGroup), [`RadioSet`](crate::controls::RadioSet) and
//!   [`Segmented`](crate::controls::Segmented).
//! - [`Listbox`]: the active option of a listbox whose focus stays on its owner (arrows, Page
//!   Up and Down, Home, End, type-ahead), used by [`Select`](crate::controls::Select) and
//!   [`ComboBox`](crate::controls::ComboBox).
//! - [`Typing`]: a line of text typed into egui's `TextEdit` and reported as one edit gesture per
//!   typing run, used by [`TextField`](crate::controls::TextField),
//!   [`SearchField`](crate::controls::SearchField) and [`ComboBox`](crate::controls::ComboBox).

mod field_tab;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod field_tab_tests;
mod listbox;
mod radio_selection;
#[cfg(test)]
mod tests;
mod typing;

pub use field_tab::FieldTab;
pub(crate) use listbox::take_key;
pub use listbox::{ListOptions, Listbox, TYPE_AHEAD_PAUSE};
pub use radio_selection::RadioSelection;
pub use typing::{Codec, Escape, NotAValue, PlainText, Typed, Typing, TypingLook};
pub(crate) use typing::{take_composing_keys, take_enter};
