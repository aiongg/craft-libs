//! The List view: a tree `Table` (`Table::tree`) whose first column holds the bins.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// Draws the List view (see `MediaBin`'s doc and risk 7): a `Table` with `.tree(|key| name)` over
/// the rows in display order (a `DisplayOrder` of bins before items, sorted by the layout's sort,
/// open bins' children under them, with depth, `has_children` for bins and `open` from
/// `BinState::expanded`); the first cell's chip, thumbnail, icon and name after the table's
/// indent and triangle, the slow-click rename and the bin drop targets. `TableEvent::Open`
/// becomes `BinEvent::Expanded` (one per bin under the shown bin for `all`); the other table
/// events pass through as `BinEvent::Table`.
pub(super) fn list_view() {
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:497 (`list_view`), :687 (`list_bin`), :457 (`slow_click_rename`).
    todo!()
}
