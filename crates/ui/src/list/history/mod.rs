//! `HistoryPanel`: the states of a document's history with snapshots above them, the history brush source column,
//! rename in place, drag to the trash, a footer of actions and the panel menu.

mod menu;
mod rows;
#[cfg(feature = "testing")]
mod stories;
mod words;

pub use menu::{HistoryMenu, show_history_menu};
pub use rows::{HistorySource, Snapshot, SnapshotId, SnapshotList, StepId, StepRows};
pub use words::HistoryWords;

use super::{List, ListRow, RowInk, ThumbSize, ToggleCell, ToggleFace, ToggleState};
use crate::contract::overlay::Anchor;
use crate::contract::{
    ActivateOn, Bind, CollectionEvent, ColumnKind, ColumnSpec, Entry, ListDrag, NoValue, Outcome, Pattern, Props as _, SelectMode,
    ShowCollection as _, ShowContainer as _, ToggleColumn, Trigger, request_rename,
};
use crate::layout_components::Stack;
use crate::menu::Menu;
use crate::primitives::{IconSpec, Priority, ThumbRequest, ThumbSource, Thumbnails};
use crate::runtime::env::Env;
use crate::sections::{FooterAction, FooterBar};
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, IconId, ItemSource, Selection};
use egui::{AsIdSalt, IdSalt, Pos2, Response, Ui};
use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE: ColumnKey = ColumnKey::new("source");
const NAME: ColumnKey = ColumnKey::new("name");

/// Which parts of the panel an app shows. The default is PhotoCraft's full set: everything but
/// [`delete_later`](Self::delete_later).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HistoryFeatures {
    /// Snapshot rows, the New Snapshot button and menu row.
    pub snapshots: bool,
    /// The history brush source column.
    pub source: bool,
    /// The New Document button and menu row.
    pub new_document: bool,
    /// The Delete button (with its trash drop) and menu row.
    pub delete: bool,
    /// The Delete Redoable Actions menu row, after Delete; the app confirms it.
    pub delete_later: bool,
    /// The History Options… menu row.
    pub options: bool,
}

impl Default for HistoryFeatures {
    fn default() -> Self {
        Self { snapshots: true, source: true, new_document: true, delete: true, delete_later: false, options: true }
    }
}

impl HistoryFeatures {
    /// No footer button and no menu row but the history's own.
    pub const NONE: HistoryFeatures =
        HistoryFeatures { snapshots: false, source: false, new_document: false, delete: false, delete_later: false, options: false };
}

/// The order of the state rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HistoryOrder {
    /// Oldest first, later states at the bottom.
    #[default]
    OldestFirst,
    /// Newest first; snapshots stay on top.
    NewestFirst,
}

/// A row of the panel's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HistoryKey {
    /// A snapshot row.
    Snapshot(SnapshotId),
    /// A state row.
    Step(StepId),
}

/// What the user did in the History panel or chose in its menu, in the order it happened. The app does the work with
/// its own commands; the panel changes only its [`HistoryPanelState`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HistoryEvent {
    /// Go to this state: a click or an arrow key on a state row.
    GoTo(StepId),
    /// A snapshot row was chosen: the app restores the snapshot as one new state named after it, then calls
    /// [`HistoryPanelState::snapshot_selected`] with the rows' new revision.
    SelectSnapshot(SnapshotId),
    /// The source column was clicked on a row that is not the source.
    SetSource(HistorySource),
    /// A snapshot was renamed in place; `text` is trimmed and not blank.
    #[non_exhaustive]
    RenameSnapshot {
        /// The snapshot renamed.
        id: SnapshotId,
        /// The new name.
        text: String,
    },
    /// Delete this state: the trash with no snapshot selected, a drop of a state on the trash, or the menu's Delete.
    DeleteStep(StepId),
    /// Delete this snapshot: the trash or the menu's Delete with it selected, a drop on the trash, or its context
    /// menu's Delete.
    DeleteSnapshot(SnapshotId),
    /// Take a snapshot: the camera button (`dialog: false`), Alt with it or the menu's New Snapshot… (`dialog: true`,
    /// the app opens its form).
    #[non_exhaustive]
    NewSnapshot {
        /// Whether the app opens the new-snapshot dialog instead of taking the snapshot at once.
        dialog: bool,
    },
    /// A new document from this state.
    NewDocument(StepId),
    /// The menu's Step Forward.
    StepForward,
    /// The menu's Step Backward.
    StepBackward,
    /// The menu's Clear History.
    ClearHistory,
    /// The menu's Delete Redoable Actions; the app asks first.
    DeleteLater,
    /// The menu's History Options…; the app opens its form.
    Options,
}

/// What the panel keeps per document between passes: the selected snapshot and the snapshot a context menu is open
/// on. Not persisted; keep one per document.
///
/// Each state has its own instance serial, which the panel's list id includes: snapshot ids restart in every
/// document, so two documents' panels never share a rename draft or a scroll offset.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryPanelState {
    serial: u64,
    selected: Option<(SnapshotId, u64)>,
    menu: Option<(SnapshotId, Pos2)>,
    shown: Option<u64>,
}

impl Default for HistoryPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryPanelState {
    /// A state with nothing selected and a new instance serial.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self { serial: NEXT.fetch_add(1, Ordering::Relaxed), selected: None, menu: None, shown: None }
    }

    /// Selects snapshot `id` after the app restored it, with the rows' revision after its record. Any later change to
    /// the rows deselects it: one row is highlighted.
    pub fn snapshot_selected(&mut self, id: SnapshotId, revision: u64) {
        self.selected = Some((id, revision));
    }

    /// The selected snapshot, as of the last pass.
    pub fn selected_snapshot(&self) -> Option<SnapshotId> {
        self.selected.map(|(id, _)| id)
    }

    /// Deselects the snapshot.
    pub fn clear_snapshot(&mut self) {
        self.selected = None;
    }

    fn keeps(&self, revision: u64) -> bool {
        self.selected.is_some_and(|(_, at)| at == revision)
    }
}

/// What [`HistoryPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app carries out the panel's events"]
pub struct HistoryOutput {
    /// The panel's response.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<HistoryEvent>,
    /// What the panel menu needs to enable its rows, as of this pass.
    pub menu: HistoryMenu,
}

impl Outcome for HistoryOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

type Pictures<'a> = (&'a Thumbnails<SnapshotId>, &'a dyn ThumbSource<SnapshotId>);

/// The History panel: snapshot rows, a hairline, then the state rows, in one list above a footer. It shows the app's
/// states through [`StepRows`] and reports [`HistoryEvent`]s.
///
/// - Rows: one highlighted, the selected snapshot while one is selected, else the current state. Later states are
///   dimmed and clickable. Rows are indented by depth when any state has one (a tree of branches).
/// - State row: the source column, the step's icon (none leaves the slot empty), the label.
/// - Snapshot row: the source column, a thumbnail from the app's [`ThumbSource`], the name.
/// - Pointer: a click on a state reports `GoTo` once; on the current state nothing, or, with a snapshot selected, the
///   panel deselects it. A click on a snapshot reports `SelectSnapshot`. A click on the source cell reports
///   `SetSource`. Double-click or F2 on a snapshot renames it in place; its context menu has Rename… and Delete.
/// - Drag to the trash: a state that can be deleted, or a snapshot, drags to the footer's trash, which reports
///   `DeleteStep` or `DeleteSnapshot`.
/// - Scroll: the current row scrolls into view when the rows' revision moved, it is hidden and no snapshot is selected.
///
/// The panel menu is [`HistoryOutput::menu`] shown in a `Menu` with [`show_history_menu`].
#[derive(Pattern)]
pub struct HistoryPanel<'a> {
    id_salt: IdSalt,
    steps: &'a dyn StepRows,
    state: &'a mut HistoryPanelState,
    snapshots: Option<&'a SnapshotList>,
    source: Option<HistorySource>,
    pictures: Option<Pictures<'a>>,
    features: HistoryFeatures,
    order: HistoryOrder,
    rename: Option<SnapshotId>,
}

impl<'a> HistoryPanel<'a> {
    /// A panel over `steps`, keeping its snapshot selection in `state`.
    pub fn new(id_salt: impl AsIdSalt, steps: &'a dyn StepRows, state: &'a mut HistoryPanelState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            steps,
            state,
            snapshots: None,
            source: None,
            pictures: None,
            features: HistoryFeatures::default(),
            order: HistoryOrder::default(),
            rename: None,
        }
    }

    /// The snapshot rows above the states.
    #[must_use]
    pub fn snapshots(mut self, snapshots: &'a SnapshotList) -> Self {
        self.snapshots = Some(snapshots);
        self
    }

    /// The history brush source the column shows; `None` shows no source.
    #[must_use]
    pub fn source(mut self, source: Option<HistorySource>) -> Self {
        self.source = source;
        self
    }

    /// The snapshot thumbnails: the cache the app keeps and its source of snapshot pictures.
    #[must_use]
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<SnapshotId>, source: &'a dyn ThumbSource<SnapshotId>) -> Self {
        self.pictures = Some((thumbnails, source));
        self
    }

    /// Which parts the panel shows.
    #[must_use]
    pub fn features(mut self, features: HistoryFeatures) -> Self {
        self.features = features;
        self
    }

    /// The order of the state rows.
    #[must_use]
    pub fn order(mut self, order: HistoryOrder) -> Self {
        self.order = order;
        self
    }

    /// Starts the in-place rename of snapshot `id` in this pass. Ignored when the snapshot is not shown.
    #[must_use]
    pub fn start_rename(mut self, id: SnapshotId) -> Self {
        self.rename = Some(id);
        self
    }

    /// Shows the panel in the room offered: the rows scroll above the footer.
    pub fn show(self, ui: &mut Ui) -> HistoryOutput {
        let HistoryPanel { id_salt, steps, state, snapshots, source, pictures, features, order, rename } = self;
        let env = Env::of(ui);
        let words = env.words::<HistoryWords>();
        let ctx = ui.ctx().clone();
        let list_id = ui.scope_id().with((id_salt.value(), state.serial, "history-list"));

        let revision = steps.revision();
        let len = steps.len();
        let current = steps.current();
        let current_id = steps.id(current);
        let snaps: Vec<&Snapshot> = snapshots.filter(|_| features.snapshots).map(|list| list.iter().collect()).unwrap_or_default();
        if let Some(id) = state.selected_snapshot()
            && (!state.keeps(revision) || !snaps.iter().any(|snap| snap.id == id))
        {
            state.clear_snapshot();
        }
        let selected_snapshot = state.selected_snapshot();

        let state_rows: Vec<usize> = match order {
            HistoryOrder::OldestFirst => (0..len).collect(),
            HistoryOrder::NewestFirst => (0..len).rev().collect(),
        };
        let mut keys: Vec<HistoryKey> = snaps.iter().map(|snap| HistoryKey::Snapshot(snap.id)).collect();
        keys.extend(state_rows.iter().map(|&at| HistoryKey::Step(steps.id(at))));
        let first_state = snaps.len();
        let tree = state_rows.iter().any(|&at| steps.depth(at) > 0);

        let row_of = |id: StepId| (0..len).find(|&at| steps.id(at) == id);
        let deletable = |key: &HistoryKey| match *key {
            HistoryKey::Snapshot(_) => true,
            HistoryKey::Step(id) => row_of(id).is_some_and(|at| steps.can_delete(at) && (steps.is_non_linear() || !steps.is_later(at))),
        };
        let droppable: Vec<HistoryKey> = keys.iter().copied().filter(|key| deletable(key)).collect();
        let trash = match selected_snapshot {
            Some(id) => Some(HistoryKey::Snapshot(id)),
            None => steps.can_delete(current).then_some(HistoryKey::Step(current_id)),
        };

        // The current row scrolls into view when the rows changed since this panel's last pass.
        let moved = state.shown.replace(revision) != Some(revision);

        let highlight = selected_snapshot.map_or(HistoryKey::Step(current_id), HistoryKey::Snapshot);
        let mut selection = Selection::single(highlight);
        let rows = Rows { keys: &keys, steps, snaps: &snaps, state_rows: &state_rows, first_state, source };
        let mut columns = Vec::new();
        if features.source {
            columns.push(
                ColumnSpec::new(SOURCE, words.source_column.clone(), ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::exclusive())),
            );
        }
        columns.push(ColumnSpec::new(NAME, words.name_column.clone(), ColumnWidth::fill(1.0)));

        if let Some(id) = rename.filter(|id| snaps.iter().any(|snap| snap.id == *id)) {
            request_rename(&ctx, list_id, &HistoryKey::Snapshot(id));
        }

        let mut events = Vec::new();
        let mut target = None;
        let side = env.tokens.size.thumb.sm;
        let has_footer = features.delete || features.snapshots || features.new_document;
        let footer_words = words.clone();
        let mut footer_events: Vec<HistoryEvent> = Vec::new();
        let mut response = None;

        let mut stack = Stack::new().fill();
        if has_footer {
            stack = stack.footer(|ui, _| {
                let mut kinds = Vec::new();
                let mut actions = Vec::new();
                if features.delete {
                    let droppable = droppable.clone();
                    actions.push(
                        FooterAction::new(IconId::new("trash"), footer_words.delete_button.clone())
                            .enabled(trash.is_some())
                            .accepts_drops(move |drag: &ListDrag<HistoryKey>| drag.list == list_id && droppable.contains(&drag.grabbed)),
                    );
                    kinds.push(0);
                }
                if features.snapshots {
                    actions.push(FooterAction::new(IconId::new("scan"), footer_words.new_snapshot_button.clone()));
                    kinds.push(1);
                }
                if features.new_document {
                    actions.push(FooterAction::new(IconId::new("new_file"), footer_words.new_document_button.clone()));
                    kinds.push(2);
                }
                let footer = FooterBar::actions(actions).show_footer(ui);
                let delete_event = |key: HistoryKey| match key {
                    HistoryKey::Snapshot(id) => HistoryEvent::DeleteSnapshot(id),
                    HistoryKey::Step(id) => HistoryEvent::DeleteStep(id),
                };
                if let Some(drop) = &footer.dropped {
                    if let Some(drag) = drop.payload::<ListDrag<HistoryKey>>() {
                        footer_events.push(delete_event(drag.grabbed));
                    }
                } else if let Some(at) = footer.picked {
                    match kinds.get(at) {
                        Some(0) => footer_events.extend(trash.map(delete_event)),
                        Some(1) => footer_events.push(HistoryEvent::NewSnapshot { dialog: footer.mods.alt }),
                        Some(2) => footer_events.push(HistoryEvent::NewDocument(current_id)),
                        _ => {}
                    }
                }
            });
        }
        let shown = stack.show(ui, |ui, _| {
            let mut list = List::<HistoryKey>::new(words.list.clone())
                .id(list_id)
                .select(SelectMode::Single)
                .activate(ActivateOn::Click)
                .columns(columns.clone())
                .header(false)
                .rename(true)
                .rename_column(NAME)
                .tree(tree)
                .selection(Bind::bound(&mut selection))
                .can_rename(|key| matches!(key, HistoryKey::Snapshot(_)))
                .can_drag(|key| features.delete && deletable(key))
                .drag_payload(|drag: &ListDrag<HistoryKey>| Some(drag.clone()));
            if moved && selected_snapshot.is_none() {
                list = list.reveal(HistoryKey::Step(current_id));
            }
            let out = list.show_items(ui, &rows, &mut |key, icx| match *key {
                HistoryKey::Snapshot(id) => {
                    let snap = snaps.iter().find(|snap| snap.id == id);
                    let mut row = ListRow::new(snap.map_or(String::new(), |snap| snap.name.clone())).cells(icx.cells());
                    row = row.rule(snaps.last().is_some_and(|last| last.id == id));
                    if features.source {
                        row = row.toggle(source_cell(&words, source == Some(HistorySource::Snapshot(id))));
                    }
                    match pictures {
                        Some((thumbs, picture_source)) => {
                            let request = ThumbRequest::for_cell(side, ctx.pixels_per_point());
                            let texture = thumbs.state(&ctx, picture_source, &id, &request, Priority::VISIBLE);
                            row.thumb(ThumbSize::Small, move |cx, rect| {
                                thumbs.paint(cx, rect, &texture, &crate::primitives::ThumbPaint::default());
                            })
                        }
                        None => row.thumb(ThumbSize::Small, |_, _| {}),
                    }
                }
                HistoryKey::Step(id) => {
                    let at = row_of(id).unwrap_or(0);
                    let later = steps.is_later(at);
                    let mut row = ListRow::new(steps.label(at).into_owned()).cells(icx.cells());
                    if later {
                        row = row.ink(RowInk::Dimmed);
                    }
                    if features.source {
                        row = row.toggle(source_cell(&words, source == Some(HistorySource::Step(id))));
                    }
                    match steps.icon(at) {
                        Some(icon) => row.icon(IconSpec::svg(icon)),
                        None => row.lead_painter(|_, _| {}),
                    }
                }
            });
            let mut menu_at = None;
            for event in out.events {
                match event {
                    CollectionEvent::Activate(key) => target = Some(key),
                    CollectionEvent::Toggle(edit) => {
                        if let Some(edit) = edit.value().filter(|edit| edit.on && edit.column == SOURCE) {
                            events.push(HistoryEvent::SetSource(match edit.key {
                                HistoryKey::Snapshot(id) => HistorySource::Snapshot(id),
                                HistoryKey::Step(id) => HistorySource::Step(id),
                            }));
                        }
                    }
                    CollectionEvent::Rename(edit) => {
                        if let Some(rename) = edit.value()
                            && let HistoryKey::Snapshot(id) = rename.key
                        {
                            let name = rename.name.trim();
                            if !name.is_empty() && snaps.iter().any(|snap| snap.id == id && snap.name != name) {
                                events.push(HistoryEvent::RenameSnapshot { id, text: name.to_owned() });
                            }
                        }
                    }
                    CollectionEvent::ContextMenu(request) => {
                        if let HistoryKey::Snapshot(id) = request.key {
                            menu_at = Some((id, request.at));
                        }
                    }
                    _ => {}
                }
            }
            if menu_at.is_some() {
                state.menu = menu_at;
            }
            response = Some(out.response);
        });
        let response = response.unwrap_or(shown.output);
        // A selection the list changed (a click or an arrow key) names the target as an activation does.
        if target.is_none() && selection.items.last() != Some(&highlight) {
            target = selection.items.last().copied();
        }
        match target {
            Some(HistoryKey::Snapshot(id)) => events.push(HistoryEvent::SelectSnapshot(id)),
            Some(HistoryKey::Step(id)) if id != current_id => events.push(HistoryEvent::GoTo(id)),
            Some(HistoryKey::Step(_)) if selected_snapshot.is_some() => state.clear_snapshot(),
            _ => {}
        }
        events.extend(footer_events);

        if let Some((id, at)) = state.menu {
            let mut open = true;
            let entries = [Entry::new(0u8, words.rename.clone()), Entry::new(1, words.delete.clone())];
            let picked = Menu::new(words.list.clone())
                .opened_by(&response, Trigger::Manual)
                .anchor(Anchor::Point(at))
                .open(Bind::bound(&mut open))
                .pick(ui, &entries)
                .picked;
            match picked {
                Some(0) => request_rename(&ctx, list_id, &HistoryKey::Snapshot(id)),
                Some(1) => events.push(HistoryEvent::DeleteSnapshot(id)),
                _ => {}
            }
            if !open || picked.is_some() {
                state.menu = None;
            }
        }

        let menu = HistoryMenu {
            features,
            can_forward: current + 1 < len,
            can_backward: current > 0,
            delete: trash,
            can_clear: len > 1,
            current: current_id,
        };
        HistoryOutput { response, events, menu }
    }
}

/// The source column of a row: exclusive, the History Brush icon on the source row and under the pointer elsewhere.
fn source_cell(words: &HistoryWords, is_source: bool) -> ToggleCell {
    let clock = IconId::new("clock");
    ToggleCell::new(SOURCE, ToggleFace::icons(clock.clone(), clock), ToggleState::of(is_source), words.source.clone())
        .tooltip(words.source_tooltip.clone())
}

/// The panel's rows as the list's item source.
struct Rows<'a> {
    keys: &'a [HistoryKey],
    steps: &'a dyn StepRows,
    snaps: &'a [&'a Snapshot],
    state_rows: &'a [usize],
    first_state: usize,
    source: Option<HistorySource>,
}

impl DisplayOrder<HistoryKey> for Rows<'_> {
    fn len(&self) -> usize {
        self.keys.len()
    }

    fn key(&self, at: usize) -> HistoryKey {
        self.keys[at]
    }

    fn depth(&self, at: usize) -> usize {
        at.checked_sub(self.first_state).map_or(0, |state| self.steps.depth(self.state_rows[state]))
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        match at.checked_sub(self.first_state) {
            None => Some(Cow::Borrowed(self.snaps[at].name.as_str())),
            Some(state) => Some(self.steps.label(self.state_rows[state])),
        }
    }
}

impl ItemSource<HistoryKey> for Rows<'_> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == SOURCE
            && self.source
                == Some(match self.keys[at] {
                    HistoryKey::Snapshot(id) => HistorySource::Snapshot(id),
                    HistoryKey::Step(id) => HistorySource::Step(id),
                })
    }
}
