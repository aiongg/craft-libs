//! Chapter 17: collection props, columns, row state and the events and response a collection reports.

use crate::runtime::env::Label;
use craft_behavior::{ColumnKey, ColumnWidth, EditEvent, SelectionChange};
use egui::{Pos2, Response, Vec2};
use std::ops::Range;

/// What a collection lets the user select.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SelectMode {
    /// Nothing; the lead still moves.
    #[default]
    None,
    /// One item.
    Single,
    /// Any set of items.
    Multi,
    /// Cells of a sheet.
    Cells,
}

/// How the user may reorder items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReorderRule {
    /// Alt turns a drag into a copy (8.4).
    pub copy_with_alt: bool,
}

impl ReorderRule {
    /// A rule that only moves.
    pub const fn new() -> Self {
        ReorderRule { copy_with_alt: false }
    }

    /// Alt copies.
    #[must_use]
    pub const fn copy_with_alt(mut self) -> Self {
        self.copy_with_alt = true;
        self
    }
}

/// A toggle column: its cells flip by click, and a drag across them sweeps one value over all of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ToggleColumn {
    /// A drag across the cells sets them all to the first cell's new value.
    pub sweepable: bool,
}

impl ToggleColumn {
    /// A sweepable toggle column (eye, lock, mute).
    pub const fn sweepable() -> Self {
        ToggleColumn { sweepable: true }
    }
}

/// What a column's cells are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnKind {
    /// Content the item paints.
    Content,
    /// A toggle the collection flips.
    Toggle(ToggleColumn),
}

/// One column of a collection.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ColumnSpec {
    /// The column's stable key.
    pub key: ColumnKey,
    /// The header text.
    pub header: Label,
    /// How wide the column is.
    pub width: ColumnWidth,
    /// The user can drag the header's edge.
    pub resizable: bool,
    /// Activating the header reports `Sort`.
    pub sortable: bool,
    /// The user can move the column.
    pub movable: bool,
    /// The column stays in view while the others scroll.
    pub frozen: bool,
    /// What the cells are.
    pub kind: ColumnKind,
}

impl ColumnSpec {
    /// A content column with a header and a width; the other settings off.
    pub fn new(key: ColumnKey, header: impl Into<Label>, width: ColumnWidth) -> Self {
        ColumnSpec { key, header: header.into(), width, resizable: false, sortable: false, movable: false, frozen: false, kind: ColumnKind::Content }
    }

    /// Activating the header sorts.
    #[must_use]
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    /// The user can resize the column.
    #[must_use]
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }

    /// The user can move the column.
    #[must_use]
    pub fn movable(mut self) -> Self {
        self.movable = true;
        self
    }

    /// The column stays in view.
    #[must_use]
    pub fn frozen(mut self) -> Self {
        self.frozen = true;
        self
    }

    /// The column's cell kind.
    #[must_use]
    pub fn kind(mut self, kind: ColumnKind) -> Self {
        self.kind = kind;
        self
    }
}

/// What a collection reads from the app: plain data.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CollectionProps {
    /// What can be selected.
    pub select: SelectMode,
    /// Whether and how items reorder.
    pub reorder: Option<ReorderRule>,
    /// The item step runs rename through `Collection::rename_editor`.
    pub rename: bool,
    /// Empty: no columns and no header row.
    pub columns: Vec<ColumnSpec>,
    /// The text of the empty state.
    pub empty: Option<Label>,
}

impl CollectionProps {
    /// Collection props with every setting at its default.
    pub const fn new() -> Self {
        CollectionProps { select: SelectMode::None, reorder: None, rename: false, columns: Vec::new(), empty: None }
    }

    /// What can be selected.
    #[must_use]
    pub fn select(mut self, mode: SelectMode) -> Self {
        self.select = mode;
        self
    }

    /// Items reorder by `rule`.
    #[must_use]
    pub fn reorder(mut self, rule: ReorderRule) -> Self {
        self.reorder = Some(rule);
        self
    }

    /// Rename runs through the collection's editor.
    #[must_use]
    pub fn rename(mut self, on: bool) -> Self {
        self.rename = on;
        self
    }

    /// The columns.
    #[must_use]
    pub fn columns(mut self, columns: Vec<ColumnSpec>) -> Self {
        self.columns = columns;
        self
    }

    /// The empty-state text.
    #[must_use]
    pub fn empty(mut self, text: impl Into<Label>) -> Self {
        self.empty = Some(text.into());
        self
    }
}

/// What an item reads of its place in the collection; the item step joins it to the item's flags at step 12.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowState {
    /// The row is selected.
    pub selected: bool,
    /// The row holds the lead.
    pub lead: bool,
    /// A drag would drop on the row.
    pub drop_target: bool,
    /// The row is being dragged.
    pub dragging: bool,
    /// The row's name is being edited.
    pub renaming: bool,
    /// The app marks the row.
    pub emphasized: bool,
    /// An odd row of a striped list.
    pub striped: bool,
    /// A tree row whose children show.
    pub open: bool,
}

/// A committed rename.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rename<K> {
    /// The renamed item.
    pub key: K,
    /// Its new name.
    pub name: String,
}

/// A reorder: `key` moves to position `to`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReorderMove<K> {
    /// The moved item.
    pub key: K,
    /// Its new position.
    pub to: usize,
}

/// A toggle column's cell flipped.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ToggleEdit<K> {
    /// The item.
    pub key: K,
    /// The column.
    pub column: ColumnKey,
    /// The new value.
    pub on: bool,
}

/// A tree row opened or closed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenChange<K> {
    /// The row.
    pub key: K,
    /// Whether its children show now.
    pub open: bool,
}

/// A request for the context menu of an item. Collections never open menus: the app does.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ContextMenuRequest<K> {
    /// The item.
    pub key: K,
    /// Where the menu opens.
    pub at: Pos2,
}

/// A change to the columns.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ColumnEvent {
    /// The header was activated.
    #[non_exhaustive]
    Sort {
        /// The column.
        key: ColumnKey,
        /// Whether the order is now descending.
        descending: bool,
    },
    /// The user dragged the column's edge.
    #[non_exhaustive]
    Resized {
        /// The column.
        key: ColumnKey,
        /// Its new width.
        width: f32,
    },
    /// The user moved the column.
    #[non_exhaustive]
    Moved {
        /// The column.
        key: ColumnKey,
        /// Its new position.
        to: usize,
    },
}

/// What a collection reports, in drawing order.
#[derive(Debug)]
#[non_exhaustive]
pub enum CollectionEvent<K, E> {
    /// The selection changed.
    Selection(SelectionChange<K>),
    /// The lead moved.
    LeadMoved(K),
    /// Enter or a double click on an item.
    Activate(K),
    /// The user asked for an item's context menu.
    ContextMenu(ContextMenuRequest<K>),
    /// A tree row opened or closed.
    Open(OpenChange<K>),
    /// An item moved to another place.
    Reorder(EditEvent<ReorderMove<K>>),
    /// The selected items of a free layout moved by an offset.
    Move(EditEvent<Vec2>),
    /// A drop onto an item.
    #[non_exhaustive]
    Dropped {
        /// The item dropped on.
        key: K,
    },
    /// A rename started, committed or was cancelled.
    Rename(EditEvent<Rename<K>>),
    /// A toggle column's cell flipped.
    Toggle(EditEvent<ToggleEdit<K>>),
    /// A column changed.
    Column(ColumnEvent),
    /// An item reported an event.
    Item(K, E),
}

/// The output of a collection.
#[derive(Debug)]
#[non_exhaustive]
pub struct CollectionResponse<K, E> {
    /// The collection's own response.
    pub response: Response,
    /// Every event, in drawing order.
    pub events: Vec<CollectionEvent<K, E>>,
    /// The positions the collection showed.
    pub visible: Range<usize>,
}

impl<K, E> CollectionResponse<K, E> {
    pub(crate) fn new(response: Response, events: Vec<CollectionEvent<K, E>>, visible: Range<usize>) -> Self {
        CollectionResponse { response, events, visible }
    }
}

impl<K, E> super::Outcome for CollectionResponse<K, E> {
    type Value = super::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}
