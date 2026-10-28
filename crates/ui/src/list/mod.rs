//! Lists: rows for panels that list things (layers, history, channels).
//!
//! - [`ListRow`]: one row. Optional toggle columns (eye, lock) with a sweep gesture, an icon or
//!   a framed thumbnail, a label with an optional detail line, and a trailing slot whose optional
//!   items drop out before the label gets too narrow. A row is drawn from plain inputs (selected,
//!   focused, dimmed, highlighted, renaming, drop marker); it keeps no selection or order of its
//!   own, so a list drives it. It reports pointer input on its response and the edits of its toggle
//!   columns and rename field as [`RowEvent`]s.
//! - [`List`]: the container. One focus stop, a scroll area that lays out only the rows in view
//!   (10,000 rows and more, of one height or mixed heights), the selection rules of
//!   [`ListSelection`](crate::behavior::ListSelection), the list keys, rename in place, drag and
//!   stacking keys to reorder by [`Reorder`](crate::behavior::Reorder), AccessKit `ListBox`, and a
//!   hook for the host's context menu. It reports [`ListEvent`]s.
//!
//! - [`TreeRow`]: a row with an indent by depth, squeezed before the label gets narrow, and a
//!   disclosure triangle for groups; a [`List`] in [tree mode](List::tree) adds the tree keys
//!   (→ and ← open, close, enter and leave; Alt for every group), AccessKit `Tree` and
//!   `TreeItem` with `level` and `expanded`, and reports [`ListEvent::Open`]. [`TreeState`] keeps
//!   the open groups for a host without a model.
//!
//! Rows draw with the `list.*` component tokens; [`ListRow::style`] is a pure function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

mod drag;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod list_tests;
mod rename;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod reorder_tests;
mod row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod row_tests;
mod spans;
mod toggle;
mod tree;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tree_tests;
mod view;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ListRow", "List", "TreeRow"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    row::stories().into_iter().chain(view::stories()).chain(tree::stories()).chain(drag::stories()).collect()
}

pub use row::{DropMark, ListRow, ListRowStyle, RenameEnd, RowEvent, RowResponse, ThumbSize, TrailingItem};
pub use toggle::{ToggleColumn, ToggleEdit};
pub use tree::{OpenChange, TreeRow, TreeState, TreeText};
pub use view::{ContextMenuRequest, List, ListEvent, ListOutput};
