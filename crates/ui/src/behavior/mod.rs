//! Behaviour that several controls share: keyboard, focus and selection rules, kept apart from how
//! each control looks. They follow the suite's apps (PhotoCraft first, then DesignCraft), and the
//! WAI-ARIA patterns where the apps have no convention.
//!
//! - [`CommandSearch`]: the ranking of commands against a typed query (the command palette's and the
//!   menu bar's Help search), over [`PaletteEntry`] values.
//! - [`EditMenu`]: the Undo, Cut, Copy, Paste, Delete and Select All menu of text controls,
//!   with the app's [`EditMenuLabels`].
//! - [`FieldTab`]: a dialog's focus scope, in which Tab visits only the text and number fields.
//! - [`RadioSelection`]: a radio group's Tab stops, focus and click selection, used by
//!   [`RadioGroup`](crate::controls::RadioGroup), [`RadioSet`](crate::controls::RadioSet) and
//!   [`Segmented`](crate::controls::Segmented).
//! - [`Listbox`]: the active option of a listbox whose focus stays on its owner (arrows, Page
//!   Up and Down, Home, End, type-ahead), used by [`Select`](crate::controls::Select) and
//!   [`ComboBox`](crate::controls::ComboBox).
//! - [`KeyClaim`]: the keys a focused control edits with (a curve editor's Delete), which the
//!   app's shortcut dispatch leaves to it.
//! - [`ListSelection`]: the selection rules of a list over its display order (click, Shift, Cmd/Ctrl,
//!   keys), kept in a host-owned [`Selection`].
//! - [`Reorder`]: drag-to-reorder targets, keyboard moves and edge scrolling over a list or a strip,
//!   without drawing, reported as an [`EditEvent`](crate::edit::EditEvent) sequence of [`Move`]s.
//! - [`GridNav`]: the lead's moves over a wrapped grid or rows of unequal length (arrows, Home,
//!   End, Page Up and Down, mirrored in right-to-left), and the Tab stop of a grid, used by
//!   craft-color's `SwatchPalette`.
//! - [`grid_drop`]: the cell and side a drag over a wrapped grid drops at.
//! - [`Typing`]: a line of text typed into egui's `TextEdit` and reported as one edit gesture per
//!   typing run, used by [`TextField`](crate::controls::TextField),
//!   [`SearchField`](crate::controls::SearchField) and [`ComboBox`](crate::controls::ComboBox).
//! - [`scrub`]: a scrub's pointer travel and the screen-edge wrap's pointer warp ([`Scrub`]).
//! - [`DigitKeys`]: Photoshop's number keys for opacity (1 = 10 %, 0 = 100 %, two digits exact).
//! - [`ControlClick`], [`drop_touch_twins`], [`SingleLineDrag`]: input fixes run before a pass
//!   (macOS Control-click, the browser's doubled taps, a single-line field's drag selection).
//! - [`DragBadge`], [`set_drop_hint`] and [`DropVeil`]: what a drag shows: the badge at the
//!   pointer, a drop target's hint in it, and the veil while files from outside hover.
//! - [`cells`]: the selected cells of a sheet or a document table ([`cells::CellSelection`] over a
//!   [`cells::CellSpace`]), with Excel's rules for clicks, Shift, Cmd/Ctrl, arrows, Enter and Tab
//!   and merged cells; egui-free, used by craft-table's grid and craft-pages' tables.
//! - [`nudge`]: an arrow key press as a move along the document axes (1 unit, Shift 10), for
//!   canvas overlays.

pub mod cells;
#[cfg(test)]
mod cells_tests;
mod command_search;
mod digit_keys;
#[cfg(test)]
mod digit_keys_tests;
mod drag_feedback;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod drag_feedback_tests;
pub(crate) mod edit_menu;
mod field_tab;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod field_tab_tests;
mod grid_drop;
mod grid_nav;
mod key_claim;
mod list_selection;
#[cfg(test)]
mod list_selection_tests;
mod listbox;
pub mod nudge;
mod quirks;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod quirks_tests;
mod radio_selection;
mod reorder;
#[cfg(test)]
mod reorder_tests;
pub mod scrub;
#[cfg(test)]
mod scrub_tests;
#[cfg(test)]
mod tests;
mod typing;

pub use command_search::{CommandSearch, Match, PaletteEntry};
pub use digit_keys::{DigitEntry, DigitKeys};
pub use drag_feedback::{DragBadge, DragFeedbackLabels, DropVeil, HoveredFiles, drop_hint, set_drop_hint};
pub use edit_menu::{EditMenu, EditMenuLabels};
pub use field_tab::FieldTab;
pub use grid_drop::{GridDrop, GridGeometry, grid_drop};
pub use grid_nav::{GridKey, GridNav, GridStep};
pub use key_claim::{ClaimedKey, KeyClaim};
pub use list_selection::{
    DisplayOrder, ListKey, ListSelection, RowInfo, Selection, SelectionChange, SelectionMode, SelectionPick, Step, TreeMove, first_child, first_item,
    last_item, nearest_item, parent, tree_key,
};
pub(crate) use listbox::take_key;
pub use listbox::{ListOptions, Listbox, TYPE_AHEAD_PAUSE};
pub use quirks::{ControlClick, SingleLineDrag, TWIN_WINDOW, drop_touch_twins};
pub use radio_selection::RadioSelection;
pub use reorder::{
    Axis, DragEnd, DropPosition, ListDrag, Move, Reorder, ReorderGesture, Resolved, RowSpans, StackKey, UniformSpans, edge_scroll_speed,
};
pub use scrub::Scrub;
pub use typing::{Codec, Escape, NotAValue, PlainText, Typed, Typing, TypingLook};
pub(crate) use typing::{take_composing_keys, take_enter};
