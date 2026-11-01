//! The grid's keys: arrows, Home and End, page keys, rating keys, Space, Enter, Delete.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::GridNav;
use egui::{Key, Modifiers};

use super::{RatingKeys, SpaceAction, ZoomStep};

/// What one key press asks of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GridKeyAction {
    /// Move the lead to `to`; `extend` with Shift, `lead_only` with Cmd/Ctrl.
    Move {
        to: usize,
        extend: bool,
        lead_only: bool,
    },
    SelectAll,
    Activate,
    Preview,
    ToggleLead,
    Remove,
    Rate(u8),
    Zoom(ZoomStep),
    Menu,
}

/// What the key rules read of the grid.
pub(super) struct KeyGrid<'a> {
    pub mac: bool,
    pub nav: &'a GridNav,
    pub lead: Option<usize>,
    pub space: SpaceAction,
    pub check_mode: bool,
    pub rating: RatingKeys,
}

/// The action for `key` with `modifiers` on `grid` (its lead at `grid.lead`): arrows and Home,
/// End, Page Up, Page Down through `nav.step`; Cmd/Ctrl+A; Enter; Space per `space` (Ctrl+Space
/// always `ToggleLead`; check mode always toggles); Delete and Backspace; the rating keys per
/// `rating` (digits 1–5 rate, 0 clears); Cmd/Ctrl + `=`/`+`, `-`, `0` zoom (checked before the
/// rating digits, so Cmd/Ctrl+0 is Reset only when `rating` is not `CommandDigits`, where it
/// clears the rating: Bridge's rule); the Menu key and Shift+F10.
pub(super) fn action(key: Key, modifiers: Modifiers, grid: &KeyGrid<'_>) -> Option<GridKeyAction> {
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/view.rs:519 (`zoom_keys`); lightcraft: crates/ui-egui/src/panels/grid.rs:536 (`vertical_neighbor`).
    todo!()
}
