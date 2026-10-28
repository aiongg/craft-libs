//! Behaviour that several controls share: keyboard, focus and selection rules from the WAI-ARIA
//! patterns, kept apart from how each control looks.
//!
//! - [`RadioSelection`]: a radio group's one Tab stop, arrow keys and click selection, used by
//!   [`RadioGroup`](crate::controls::RadioGroup) and [`Segmented`](crate::controls::Segmented).

mod radio_selection;

pub use radio_selection::RadioSelection;
