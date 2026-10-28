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
//!   [`ListSelection`](crate::behavior::ListSelection), the list keys, rename in place, AccessKit
//!   `ListBox`, and a hook for the host's context menu. It reports [`ListEvent`]s.
//!
//! Rows draw with the `list.*` component tokens; [`ListRow::style`] is a pure function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod list_tests;
mod rename;
mod row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod row_tests;
mod spans;
mod toggle;
mod view;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ListRow", "List"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    row::stories().into_iter().chain(view::stories()).collect()
}

pub use row::{DropMark, ListRow, ListRowStyle, RenameEnd, RowEvent, RowResponse, ThumbSize, TrailingItem};
pub use toggle::{ToggleColumn, ToggleEdit};
pub use view::{ContextMenuRequest, List, ListEvent, ListOutput};
