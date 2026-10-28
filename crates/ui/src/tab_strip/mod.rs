//! The tab strip: document and panel tabs in one strip.
//!
//! - [`layout`]: the pure rules of the strip, with no painting. [`TabKind`] sets the per-kind
//!   defaults, [`StripPlan`] places the tabs, the overflow button and the host's areas,
//!   [`StripPlan::slot_at`] and [`StripPlan::tab_at`] resolve a position, [`drag_zone`] tells where
//!   a tab drag stands, [`reorder_key`] reads the reorder keys and [`step_focus`] moves the keyboard
//!   focus over the shown tabs and the overflow button.

pub mod layout;

pub use layout::{
    CloseSide, DragZone, FitMode, FocusMove, FocusSpot, OverflowMenu, OverflowPlace, PlacedTab, StripPlan, StripPlanInput, TabKind, close_side,
    drag_zone, focus_move, focus_spot, reorder_key, step_focus, strip_key,
};

#[cfg(test)]
mod layout_tests;
