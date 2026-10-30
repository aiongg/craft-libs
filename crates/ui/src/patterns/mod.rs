//! Patterns: whole regions of an app window built from the library's controls and composites.
//!
//! - [`OptionsBar`]: the bar of the active tool's options, with a leading slot, a scrolling body of
//!   groups and cells, and a trailing slot pinned at the end.
//! - [`Panel`]: the body of a docked panel, with padding, a scroll area and a footer.
//! - [`StatusBar`]: the bar at the bottom of the window, with a leading group and a trailing group.
//!
//! Patterns take semantic properties only and draw with the component tokens of what they hold
//! (`options_bar.*` for [`OptionsBar`]); each has a pure `style` function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

pub(crate) mod options_bar;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod options_bar_tests;
mod options_cell;
pub(crate) mod panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
pub(crate) mod rail;
pub(crate) mod status_bar;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod status_bar_tests;

pub use options_bar::{CommitCancel, OptionsBar, OptionsBarSlots, OptionsBarStyle};
pub use options_cell::CellWidth;
pub use panel::{Panel, PanelOutput, PanelStyle};
pub use status_bar::{StatusBar, StatusBarSlots, StatusBarStyle};
