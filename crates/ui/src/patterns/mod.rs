//! Patterns: whole regions of an app window built from the library's controls and composites.
//!
//! - [`OptionsBar`]: the bar of the active tool's options, with a leading slot, a scrolling body of
//!   groups and cells, and a trailing slot pinned at the end.
//!
//! Patterns take semantic properties only and draw with the component tokens of what they hold
//! (`options_bar.*` for [`OptionsBar`]); each has a pure `style` function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

pub(crate) mod options_bar;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod options_bar_tests;
mod options_cell;

pub use options_bar::{CommitCancel, OptionsBar, OptionsBarSlots, OptionsBarStyle};
pub use options_cell::CellWidth;
