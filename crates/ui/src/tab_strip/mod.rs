//! Tabs and tab strips: document and panel tabs in one strip.
//!
//! - [`TabStrip`]: the strip. The host passes its tabs in its order each frame and applies the
//!   [`TabEvent`]s the strip reports; see [`TabStrip`] for its pointer, keyboard and AccessKit
//!   rules.
//! - [`Tab`]: one tab as data the host passes each frame (key, label, optional detail text, icon,
//!   modified mark, closable, progress). [`Tab::show_at`] draws it at a rect in the state a
//!   strip gives it ([`TabView`]) and reports its × part; `tab.*` are its tokens, dumped from
//!   [`Tab::style`] ([`TabStyle`]).
//! - [`layout`]: the pure rules of the strip, with no painting. [`TabKind`] sets the per-kind
//!   defaults, [`StripPlan`] places the tabs, the overflow button and the host's areas,
//!   [`StripPlan::slot_at`] and [`StripPlan::tab_at`] resolve a position, [`drag_zone`] tells where
//!   a tab drag stands, [`reorder_key`] reads the reorder keys and [`step_focus`] moves the keyboard
//!   focus over the shown tabs and the overflow button.

pub mod layout;
#[cfg(test)]
mod layout_tests;
mod stories;
mod strip;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod strip_tests;
mod tab;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tab_tests;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Tab", "TabStrip"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    let mut all = tab::stories();
    all.extend(stories::stories());
    all
}

pub use layout::{
    CloseSide, DragZone, FitMode, FocusMove, FocusSpot, OverflowMenu, OverflowPlace, PlacedTab, StripPlan, StripPlanInput, TabKind, close_side,
    drag_zone, focus_move, focus_spot, reorder_key, step_focus, strip_key,
};
pub use strip::{DragOut, TabEvent, TabMenuRequest, TabStrip, TabStripOutput};
pub use tab::{Tab, TabLayout, TabMetrics, TabParts, TabResponse, TabStripText, TabStyle, TabView};
