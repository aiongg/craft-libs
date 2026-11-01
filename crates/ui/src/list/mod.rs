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
//! - [`FindingsList`]: problems a check found, with a summary line; activating one reports it so
//!   the app shows its item.
//!
//! - [`Table`]: rows of cells under a header of columns the user sorts, resizes, hides and
//!   moves, arranged by a [`TableLayout`] the app keeps in its settings.
//! - [`MetadataTable`]: name and value rows in groups, with a filter field and a column picker.
//! - [`EventLog`]: warnings and errors newest first, with a level filter, counts and details.
//!
//! Rows draw with the `list.*` component tokens; [`ListRow::style`] is a pure function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod context_menu_tests;
pub(crate) mod drag;
mod events;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod events_tests;
mod findings;
mod findings_stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod findings_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod list_tests;
mod metadata;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod metadata_tests;
mod rename;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod reorder_tests;
mod row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod row_tests;
pub(crate) mod spans;
mod table;
pub(crate) mod table_stories;
#[cfg(test)]
mod table_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod table_ui_tests;
mod toggle;
mod tree;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tree_tests;
mod view;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ListRow", "List", "TreeRow", "FindingsList", "Table", "MetadataTable", "EventLog"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    row::stories()
        .into_iter()
        .chain(view::stories())
        .chain(tree::stories())
        .chain(drag::stories())
        .chain(findings_stories::stories())
        .chain(table_stories::stories())
        .collect()
}

pub use drag::{GhostThumb, paint_ghost};
pub use events::{EventLevel, EventLog, EventLogLabels, EventLogOutput, EventLogState, LogEvent};
pub use findings::{Finding, FindingLevel, FindingsLabels, FindingsList, FindingsListResponse, findings_summary};
pub use metadata::{MetadataGroup, MetadataLabels, MetadataRow, MetadataTable, MetadataTableOutput, filter_metadata};
pub use row::{
    DropMark, ListRow, ListRowStyle, RenameEnd, RowEvent, RowResponse, ThumbSize, ThumbSizeLabels, ThumbTarget, TrailingItem, drop_line,
    paint_drop_mark,
};
pub use table::{CellContext, SortOrder, Table, TableColumn, TableEvent, TableLabels, TableLayout, TableOutput, TableSort, TableStyle};
pub use toggle::{ToggleColumn, ToggleEdit};
pub use tree::{OpenChange, TreeRow, TreeState, TreeText};
pub use view::{ContextMenuRequest, List, ListEvent, ListOutput};
