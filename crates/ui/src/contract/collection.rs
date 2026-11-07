//! Chapter 17: collection props, columns, row state and the events and response a collection reports.

use crate::runtime::drag::OsFile;
use crate::runtime::env::Label;
use craft_behavior::{ColumnKey, ColumnWidth, DropOperation, EditEvent, SelectionChange};
use egui::{Id, Pos2, Response, Vec2};
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
    /// Turning a cell on turns every other cell of the column off (solo).
    pub exclusive: bool,
}

impl ToggleColumn {
    /// A sweepable toggle column (eye, lock, mute).
    pub const fn sweepable() -> Self {
        ToggleColumn { sweepable: true, exclusive: false }
    }

    /// An exclusive toggle column: one cell is on at a time, and a drag does not sweep.
    pub const fn exclusive() -> Self {
        ToggleColumn { sweepable: false, exclusive: true }
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

/// The pointer gesture that activates an item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActivateOn {
    /// A double click.
    #[default]
    DoubleClick,
    /// A single click: pickers and popup lists.
    Click,
}

/// Where the lead starts when the collection first shows rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeadOnOpen {
    /// On the selection's lead, and nowhere when nothing is selected.
    #[default]
    Selected,
    /// On the selection's lead, else on the first row that can be picked: menus and popup lists, where Enter
    /// picks the active row.
    First,
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
    /// What activates an item with the pointer; Enter always activates the lead.
    pub activate: ActivateOn,
    /// Odd rows carry `RowState.striped`.
    pub striped: bool,
    /// A press on the empty space below the rows empties the selection. Off: only a key or a press on an item
    /// changes it, so a history list keeps its mark.
    pub allow_empty: bool,
    /// Where the lead starts.
    pub lead_on_open: LeadOnOpen,
    /// The lead, and the selection with it as for the arrow keys, moves to the row under the pointer when the pointer
    /// moves: menus, popup lists and the undo list.
    pub lead_follows_hover: bool,
    /// A primary press moves key focus to the collection. Off: a press picks and activates and focus stays where it
    /// is, for a popup list whose opener keeps focus.
    pub focus_on_press: bool,
}

impl CollectionProps {
    /// Collection props with every setting at its default.
    pub const fn new() -> Self {
        CollectionProps {
            select: SelectMode::None,
            reorder: None,
            rename: false,
            columns: Vec::new(),
            empty: None,
            activate: ActivateOn::DoubleClick,
            striped: false,
            allow_empty: false,
            lead_on_open: LeadOnOpen::Selected,
            lead_follows_hover: false,
            focus_on_press: true,
        }
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

    /// What activates an item with the pointer.
    #[must_use]
    pub const fn activate(mut self, on: ActivateOn) -> Self {
        self.activate = on;
        self
    }

    /// Odd rows carry `RowState.striped`.
    #[must_use]
    pub const fn striped(mut self, on: bool) -> Self {
        self.striped = on;
        self
    }

    /// A press on the empty space below the rows empties the selection.
    #[must_use]
    pub const fn allow_empty(mut self, on: bool) -> Self {
        self.allow_empty = on;
        self
    }

    /// Where the lead starts.
    #[must_use]
    pub const fn lead_on_open(mut self, at: LeadOnOpen) -> Self {
        self.lead_on_open = at;
        self
    }

    /// The lead follows the pointer over the rows.
    #[must_use]
    pub const fn lead_follows_hover(mut self, on: bool) -> Self {
        self.lead_follows_hover = on;
        self
    }

    /// A primary press moves key focus to the collection; off, it leaves focus where it is.
    #[must_use]
    pub const fn focus_on_press(mut self, on: bool) -> Self {
        self.focus_on_press = on;
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

/// A reorder: the items `keys` move together, and the first of them lands at position `to`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReorderMove<K> {
    /// The item the pointer or the key grabbed.
    pub key: K,
    /// Every moved item in display order, `key` among them. They end up next to each other.
    pub keys: Vec<K>,
    /// The new position of the first of `keys`, counted in the order that remains once all of them are taken out.
    pub to: usize,
    /// The drag was a copy (Alt held, `ReorderRule::copy_with_alt`): the item stays and a copy goes to `to`.
    pub copy: bool,
}

/// What a row drag carries through the drag session (8.4).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListDrag<K> {
    /// The collection the drag started in.
    pub list: Id,
    /// The dragged items in display order: the selection when the grabbed row was selected, else the row.
    pub keys: Vec<K>,
    /// The row the pointer grabbed.
    pub grabbed: K,
}

impl<K> ListDrag<K> {
    /// A drag of `keys` from `list`, grabbed at `grabbed`.
    pub fn new(list: Id, keys: Vec<K>, grabbed: K) -> Self {
        ListDrag { list, keys, grabbed }
    }
}

/// A request for the context menu of a column header: the app lists `ColumnCommands` in it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HeaderMenuRequest {
    /// The column whose header was pressed or focused.
    pub column: ColumnKey,
    /// Where the menu opens.
    pub at: Pos2,
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
    /// The change reaches every descendant too (Alt on the disclosure).
    pub all: bool,
}

impl<K> OpenChange<K> {
    /// `key` opens or closes.
    pub const fn new(key: K, open: bool) -> Self {
        OpenChange { key, open, all: false }
    }

    /// The change reaches every descendant.
    #[must_use]
    pub const fn all(mut self) -> Self {
        self.all = true;
        self
    }
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
        /// What the drop does: move or copy.
        op: DropOperation,
        /// The dragged items in display order, from the drag session's payload.
        keys: Vec<K>,
    },
    /// Files from the OS dropped on a collection that takes them (`Collection::takes_files`).
    #[non_exhaustive]
    FilesDropped {
        /// The position of the display order the files land before; the source's length drops them after the last item.
        insert: usize,
        /// The files, in the order the OS lists them.
        files: Vec<OsFile>,
    },
    /// A rename started, committed or was cancelled.
    Rename(EditEvent<Rename<K>>),
    /// A toggle column's cell flipped.
    Toggle(EditEvent<ToggleEdit<K>>),
    /// A column changed.
    Column(ColumnEvent),
    /// The user asked for the context menu of a column header.
    HeaderMenu(HeaderMenuRequest),
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
    /// What the collection lets the user select.
    pub select: SelectMode,
    /// The collection renames items.
    pub rename: bool,
    /// The collection reorders items.
    pub reorder: bool,
}

impl<K, E> CollectionResponse<K, E> {
    pub(crate) fn new(response: Response, events: Vec<CollectionEvent<K, E>>, visible: Range<usize>, gestures: Gestures) -> Self {
        CollectionResponse { response, events, visible, select: gestures.select, rename: gestures.rename, reorder: gestures.reorder }
    }
}

/// The gestures a collection's props allow, as its response reports them.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gestures {
    pub(crate) select: SelectMode,
    pub(crate) rename: bool,
    pub(crate) reorder: bool,
}

impl Gestures {
    pub(crate) fn of(props: &CollectionProps) -> Self {
        Gestures { select: props.select, rename: props.rename, reorder: props.reorder.is_some() }
    }
}

impl<K, E> super::Outcome for CollectionResponse<K, E> {
    type Value = super::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        crate::testing::StoryOutput::of_collection(&self.response, &self.events, &self.visible)
    }
}
