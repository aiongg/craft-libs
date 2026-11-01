//! `HistoryPanel::show`: rows, selection, events, rename, follow-scroll, the source column, drag and thumbnails.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::footer::{FooterSpec, show_footer};
use super::labels::HistoryLabels;
use super::menu::MenuFacts;
use super::state::HistoryPanelState;
use crate::behavior::{Axis, Reorder, RowInfo, Selection, SelectionMode};
use crate::canvas::textures::{Priority, ThumbPaint, ThumbRequest, ThumbSource, Thumbnails};
use crate::composites::PanelFooter;
use crate::edit::EditEvent;
use crate::edit::snapshots::{HistorySource, Snapshot, SnapshotId, SnapshotList};
use crate::edit::steps::{StepId, StepRows};
use crate::icons::Icon;
use crate::list::{List, ListEvent, ListRow, RowEvent, ThumbSize, ToggleColumn};
use crate::localized::Labels as _;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt, facts_id};
use crate::theme::{Tokens, TokensOf as _};

/// Which parts of the panel an app shows. The default is PhotoCraft's full set: everything but
/// [`delete_later`](Self::delete_later). VectorCraft turns off `snapshots`, `source`,
/// `new_document` and `options`; FilmCraft turns on `delete_later`; LightCraft turns off every
/// footer part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HistoryFeatures {
    /// Snapshot rows, the New Snapshot button and menu row.
    pub snapshots: bool,
    /// The history brush source column.
    pub source: bool,
    /// The New Document button and menu row.
    pub new_document: bool,
    /// The Delete button (with its trash drop) and menu row.
    pub delete: bool,
    /// FilmCraft's Delete Redoable Actions menu row, after Delete; the app confirms it.
    pub delete_later: bool,
    /// The History Options… menu row.
    pub options: bool,
}

impl Default for HistoryFeatures {
    fn default() -> Self {
        Self { snapshots: true, source: true, new_document: true, delete: true, delete_later: false, options: true }
    }
}

/// The order of the state rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HistoryOrder {
    /// Oldest first, later states at the bottom (Photoshop, PhotoCraft).
    #[default]
    OldestFirst,
    /// Newest first (LightCraft keeps this); snapshots stay on top.
    NewestFirst,
}

/// A row of the panel's list: the key of its `ListDrag` payload while a row is dragged to the
/// trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HistoryKey {
    /// A snapshot row.
    Snapshot(SnapshotId),
    /// A state row.
    Step(StepId),
}

/// What the user did in the History panel or chose in its menu, in the order it happened. The
/// app does the work with its own commands; the panel changes only its [`HistoryPanelState`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HistoryEvent {
    /// Go to this state: a click, an arrow key or a screen reader's click on a state row. One
    /// event per jump; the app cancels an active gesture first (`cancel_all` with
    /// `UndoDuringGesture`) and calls `StepList::go_to` once.
    GoTo(StepId),
    /// A snapshot row was chosen: the app restores the snapshot as one new state named after it,
    /// then calls [`HistoryPanelState::snapshot_selected`] with the list's new revision.
    SelectSnapshot(SnapshotId),
    /// The source column was clicked on a row that is not the source.
    SetSource(HistorySource),
    /// A snapshot was renamed in place; `name` is trimmed and not blank.
    RenameSnapshot { id: SnapshotId, text: String },
    /// Delete this state: the trash with no snapshot selected, a drop of a state on the trash, or
    /// the menu's Delete.
    DeleteStep(StepId),
    /// Delete this snapshot: the trash or the menu's Delete with it selected, a drop on the trash,
    /// or its context menu's Delete.
    DeleteSnapshot(SnapshotId),
    /// Take a snapshot: the camera button (`dialog: false`, at once), Alt with it or the menu's
    /// New Snapshot… (`dialog: true`, the app opens `new_snapshot_form`).
    NewSnapshot { dialog: bool },
    /// A new document from this state (the New Document button or menu row: the current state).
    NewDocument(StepId),
    /// The menu's Step Forward.
    StepForward,
    /// The menu's Step Backward.
    StepBackward,
    /// The menu's Clear History.
    ClearHistory,
    /// FilmCraft's Delete Redoable Actions; the app asks first (`dialog::StandardPrompt`).
    DeleteLater,
    /// The menu's History Options…; the app opens `history_options_form`.
    Options,
}

/// What [`HistoryPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app carries out the panel's events"]
pub struct HistoryOutput {
    /// The panel's list: its focus and rect.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<HistoryEvent>,
}

/// The History panel: snapshot rows, a hairline, then the state rows, in one scroll area above a
/// footer. It shows the app's states through [`StepRows`] and reports [`HistoryEvent`]s.
///
/// ```ignore
/// let out = HistoryPanel::new("history", &doc.steps, &mut doc.panel)
///     .snapshots(&doc.snapshots)
///     .source(doc.source.resolve(&doc.steps, &doc.snapshots))
///     .thumbnails(&app.snapshot_thumbs, &SnapshotThumbs { doc: &doc })
///     .show(ui);
/// for event in out.events {
///     match event {
///         HistoryEvent::GoTo(id) => { cancel_gestures(app); app.go_to_state(id); }
///         HistoryEvent::SelectSnapshot(id) => { app.restore_snapshot(id); doc.panel.snapshot_selected(id, doc.steps.revision()); }
///         HistoryEvent::SetSource(source) => doc.source.set(source, &mut doc.steps),
///         other => app.run_history_command(other),
///     }
/// }
/// ```
///
/// - **Rows.** One [`List`](crate::list::List): one focus stop, `SelectionMode::Single`, mixed
///   heights (state rows `list.row_height`, snapshot rows the snapshot thumbnail plus
///   `list.thumb_pad`), tree mode only when a row has `depth > 0` (indented by depth, EffectCraft's
///   branch lines dropped). The last snapshot row has `ListRow::rule`. Later states are dimmed
///   (`list.dimmed`) and clickable. One row is highlighted: the selected snapshot while one is
///   selected, else the current state.
/// - **State row**, at the reading start: the source column (`list.toggle_column` wide, an
///   exclusive `ToggleColumn` with `Icon::Clock` on the source row, faint under the pointer on
///   others), the step's icon (`list.icon_size`; none leaves the slot empty), the label.
/// - **Snapshot row**: the source column, a square thumbnail `history.snapshot_thumb` wide drawn
///   by [`Thumbnails::paint`] from the app's `ThumbSource<SnapshotId>` (the list's thumbnail
///   frame; checkerboard and failure look from the painter), the name. Without
///   [`thumbnails`](Self::thumbnails) the frame is empty.
/// - **Pointer.** A click on a state reports `GoTo` once; on the current state nothing, or, with
///   a snapshot selected, the panel deselects it. A click on a snapshot reports
///   `SelectSnapshot`. A click on the source cell reports `SetSource` and does not step; on the
///   current source nothing; a press does not sweep other rows. Double-click or F2 on a snapshot
///   renames it in place (the list's rename field named [`HistoryLabels::name`], whole name selected; Enter or a click away
///   reports `RenameSnapshot`, Esc cancels, a blank name keeps the old one); states cannot be
///   renamed. A snapshot's context menu has Rename… and Delete; state rows have none. Rename… is kept in
///   the [`HistoryPanelState`] and starts in the next pass; [`start_rename`](Self::start_rename)
///   starts a rename in the pass it is given.
/// - **Drag to the trash.** A state (not the oldest; in linear history not a later one) or a
///   snapshot drags with the list's ghost (`List::can_drag`: a row History cannot delete starts no
///   drag); a drop on the footer's trash reports `DeleteStep` or
///   `DeleteSnapshot`; Esc or a release elsewhere does nothing. The list's `Reorder` refuses every
///   target, so the drop is the drag's only effect (the list's own `Begin` and `Cancel` are not
///   reported).
/// - **Scroll.** The current row scrolls into view only when `StepRows::revision` moved since the
///   last pass of this panel (a new state, a jump, or another document: the state's serial changed), the row is hidden and no snapshot is
///   selected; no animation. Other changes leave the scroll where the user put it.
/// - **Keys** (list focused): Up and Down move the selection by one row and report `GoTo` or
///   `SelectSnapshot` for it (one event per pass for the last target under key repeat); Home and
///   End go to the first and last row; Enter and Space activate the lead row as a click; F2
///   renames a snapshot; Tab leaves into the footer buttons. No Delete key binding (Photoshop and
///   PhotoCraft have none).
/// - **Right to left.** The source column, icon and label mirror (the list and `ListRow` do it);
///   the thumbnail paints into the rect the list has mirrored; the footer buttons start at the
///   reading end.
/// - **AccessKit.** A `ListBox` named [`HistoryLabels::list`]. A state row's name is its label,
///   its description "Current state", "Later state" or nothing; a snapshot row's description
///   "Snapshot"; `selected` on the highlighted row. The source cell is a `Button`, checked on the
///   source, named by [`HistoryLabels::source`], with the source tooltip.
///
/// The panel menu is [`HistoryPanel::panel_menu`] and [`HistoryPanel::run_command`]
/// (`menu.rs`); `show` stores what the menu's enabled rules need.
#[must_use = "show the panel with `show`"]
pub struct HistoryPanel<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) steps: &'a dyn StepRows,
    pub(super) state: &'a mut HistoryPanelState,
    pub(super) snapshots: Option<&'a SnapshotList>,
    pub(super) source: Option<HistorySource>,
    pub(super) thumbnails: Option<(&'a Thumbnails<SnapshotId>, &'a dyn ThumbSource<SnapshotId>)>,
    pub(super) features: HistoryFeatures,
    pub(super) order: HistoryOrder,
    pub(super) labels: Option<HistoryLabels>,
    pub(super) start_rename: Option<SnapshotId>,
}

impl<'a> HistoryPanel<'a> {
    /// A panel over `steps` (a `StepList` or the app's own [`StepRows`]), keeping its snapshot
    /// selection in `state`. Keep one [`HistoryPanelState`] per document: its instance serial
    /// separates the documents' scroll offsets and rename drafts under one `id_salt`.
    pub fn new(id_salt: impl AsIdSalt, steps: &'a dyn StepRows, state: &'a mut HistoryPanelState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            steps,
            state,
            snapshots: None,
            source: None,
            thumbnails: None,
            features: HistoryFeatures::default(),
            order: HistoryOrder::default(),
            labels: None,
            start_rename: None,
        }
    }

    /// The snapshot rows above the states (default none).
    pub fn snapshots(mut self, snapshots: &'a SnapshotList) -> Self {
        self.snapshots = Some(snapshots);
        self
    }

    /// The history brush source the column shows (`HistorySourceChoice::resolve`); `None` shows
    /// no source.
    pub fn source(mut self, source: Option<HistorySource>) -> Self {
        self.source = source;
        self
    }

    /// The snapshot thumbnails: the cache the app keeps (one `Thumbnails<SnapshotId>` per window)
    /// and the app's source of snapshot pictures.
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<SnapshotId>, source: &'a dyn ThumbSource<SnapshotId>) -> Self {
        self.thumbnails = Some((thumbnails, source));
        self
    }

    /// Which parts the panel shows (default [`HistoryFeatures::default`]).
    pub fn features(mut self, features: HistoryFeatures) -> Self {
        self.features = features;
        self
    }

    /// The order of the state rows (default oldest first).
    pub fn order(mut self, order: HistoryOrder) -> Self {
        self.order = order;
        self
    }

    /// Starts the in-place rename of snapshot `id` in this pass, as the list's rename field would
    /// open it (the whole name selected). The app calls it in the pass it handles its own command
    /// for it; the snapshot's context menu does the same through the panel's state. Ignored when
    /// the snapshot is not shown.
    pub fn start_rename(mut self, id: SnapshotId) -> Self {
        self.start_rename = Some(id);
        self
    }

    /// The panel's words (default English).
    pub fn labels(mut self, labels: HistoryLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the panel in the available rect: the rows scroll above the footer, so many snapshots
    /// never push the footer out of a short dock.
    ///
    /// Thumbnails: the list's row builder runs before its row draws, and only for rows in view.
    /// For a snapshot row it calls `thumbs.state(ctx, source, &id, &ThumbRequest::for_cell(side,
    /// ctx.pixels_per_point()), Priority::VISIBLE)` with a clone of the `Context`, then moves the
    /// `TextureState` into the row's `.thumb(ThumbSize::Px(side), move |painter, rect| { ..
    /// thumbs.paint(painter, rect, &state, &ThumbPaint::default()) .. })`; `side` is
    /// `history.snapshot_thumb` ([`HistoryPanel::style`]).
    ///
    /// The footer is `footer::show_footer`; the menu facts are stored with
    /// `menu::MenuFacts::store` under `facts_id(id_salt)`.
    pub fn show(self, ui: &mut Ui) -> HistoryOutput {
        // pc: crates/ui-egui/src/panels/history.rs:5-160 (`history_panel`: rows :44-114, source column :196-209, snapshot rows :273-334, follow-scroll :31-41)
        let HistoryPanel { id_salt, steps, state, snapshots, source, thumbnails, features, order, labels, start_rename } = self;
        let labels = labels.unwrap_or_else(|| HistoryLabels::of(ui));
        let tokens = Tokens::of(ui);
        let sizes = HistoryPanel::style(&tokens);
        let row_style = ListRow::<HistoryKey>::style(&tokens);
        let ctx = ui.ctx().clone();
        // Snapshot ids restart in every document, so the list is salted with the state's serial:
        // a rename draft and a scroll offset belong to one document.
        let list_salt = (id_salt.value(), state.serial);
        let list_id = List::id_in(ui, list_salt);
        let panel_id = ui.make_persistent_id(id_salt);

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
        let first_state = snaps.len();
        let mut order_rows: Vec<RowInfo<HistoryKey>> = snaps.iter().map(|snap| RowInfo::leaf(HistoryKey::Snapshot(snap.id))).collect();
        order_rows.extend(state_rows.iter().map(|&at| RowInfo { depth: steps.depth(at), ..RowInfo::leaf(HistoryKey::Step(steps.id(at))) }));
        let tree = state_rows.iter().any(|&at| steps.depth(at) > 0);

        let mut selection = Selection::only(selected_snapshot.map_or(HistoryKey::Step(current_id), HistoryKey::Snapshot));
        let side = sizes.snapshot_thumb;
        let thumb_size = ThumbSize::Px(side.round() as u16);
        let snapshot_height = row_style.height(thumb_size, false);
        let state_height = row_style.height(ThumbSize::None, false);

        let has_footer = features.delete || features.snapshots || features.new_document;
        let reserved = if has_footer {
            PanelFooter::reserved_height(&PanelFooter::style(&tokens, ui.pixels_per_point())) + ui.spacing().item_spacing.y
        } else {
            0.0
        };

        let row_of = |id: StepId| (0..len).find(|&at| steps.id(at) == id);
        let accepts = |key: &HistoryKey| match *key {
            HistoryKey::Snapshot(_) => true,
            HistoryKey::Step(id) => row_of(id).is_some_and(|at| steps.can_delete(at) && (steps.is_non_linear() || !steps.is_later(at))),
        };
        // The follow-scroll fires when the revision or the document (the state's serial) changed
        // since the last pass this panel drew.
        let shown_key = panel_id.with("shown");
        let shown = (state.serial, revision);
        let moved = ui.data_mut(|d| {
            let last = d.get_temp::<(u64, u64)>(shown_key);
            d.insert_temp(shown_key, shown);
            last != Some(shown)
        });
        let mut list = List::new(list_salt, &labels.list, order_rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .allow_empty(false)
            .tree(tree)
            .heights(move |at| if at < first_state { snapshot_height } else { state_height })
            .can_rename(|key| matches!(key, HistoryKey::Snapshot(_)))
            .can_drag(|key| features.delete && accepts(key))
            .reorder(Reorder::new(Axis::Vertical).accepts(|_, _| false))
            .max_height((ui.available_height() - reserved).max(0.0));
        if moved && selected_snapshot.is_none() {
            list = list.reveal(HistoryKey::Step(current_id));
        }
        if let Some(id) = start_rename.or_else(|| state.rename.take()).filter(|id| snaps.iter().any(|snap| snap.id == *id)) {
            list = list.start_rename(HistoryKey::Snapshot(id));
        }

        let out = list.show(ui, |at, key| match *key {
            HistoryKey::Snapshot(id) => {
                let snap = snaps.iter().find(|snap| snap.id == id);
                let mut row = ListRow::new(*key, snap.map_or("", |snap| snap.name.as_str()))
                    .description(&labels.snapshot)
                    .rename_field_name(&labels.name)
                    .rule(at + 1 == first_state);
                if features.source {
                    row = row.toggle(source_column(&labels, source == Some(HistorySource::Snapshot(id))));
                }
                match thumbnails {
                    Some((thumbs, picture_source)) => {
                        let request = ThumbRequest::for_cell(side, ctx.pixels_per_point());
                        let texture = thumbs.state(&ctx, picture_source, &id, &request, Priority::VISIBLE);
                        row.thumb(thumb_size, move |painter, rect| {
                            thumbs.paint(painter, rect, &texture, &ThumbPaint::default());
                        })
                    }
                    None => row.thumb(thumb_size, |_, _| {}),
                }
            }
            HistoryKey::Step(id) => {
                let at = state_rows[at - first_state];
                let later = steps.is_later(at);
                let description = if at == current {
                    labels.current_state.as_str()
                } else if later {
                    labels.later_state.as_str()
                } else {
                    ""
                };
                let mut row = ListRow::new(*key, steps.label(at)).dimmed(later).description(description);
                if features.source {
                    row = row.toggle(source_column(&labels, source == Some(HistorySource::Step(id))));
                }
                match steps.icon(at) {
                    Some(icon) => row.icon(icon),
                    None => row.glyph(|_, _| {}),
                }
            }
        });

        let mut events = Vec::new();
        let mut target = None;
        let mut open_menu = None;
        for event in out.events {
            match event {
                ListEvent::Selection(change) => target = change.row.or(target),
                ListEvent::Clicked(key) | ListEvent::Activate(key) => target = Some(key),
                ListEvent::Row(RowEvent::Toggle(EditEvent::Set { value, .. })) if value.on => {
                    events.push(HistoryEvent::SetSource(match value.row {
                        HistoryKey::Snapshot(id) => HistorySource::Snapshot(id),
                        HistoryKey::Step(id) => HistorySource::Step(id),
                    }));
                }
                ListEvent::Row(RowEvent::Renamed { row: HistoryKey::Snapshot(id), text }) => {
                    let name = text.trim();
                    if !name.is_empty() && snaps.iter().any(|snap| snap.id == id && snap.name != name) {
                        events.push(HistoryEvent::RenameSnapshot { id, text: name.to_owned() });
                    }
                }
                ListEvent::ContextMenu(request) => {
                    if let Some(HistoryKey::Snapshot(id)) = request.row {
                        state.menu_row = Some(id);
                        open_menu = Some(request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer));
                    }
                }
                _ => {}
            }
        }
        match target {
            Some(HistoryKey::Snapshot(id)) => events.push(HistoryEvent::SelectSnapshot(id)),
            Some(HistoryKey::Step(id)) if id != current_id => events.push(HistoryEvent::GoTo(id)),
            Some(HistoryKey::Step(_)) if selected_snapshot.is_some() => {
                state.clear_snapshot();
            }
            _ => {}
        }

        let entries = [
            MenuEntry::from(MenuItem::new(&labels.rename, SnapshotCommand::Rename)),
            MenuEntry::from(MenuItem::new(&labels.delete, SnapshotCommand::Delete)),
        ];
        let menu = ContextMenu::new(&entries).open_at(open_menu).own_gestures(false).show(ui, &out.response);
        if let Some(command) = menu.chosen
            && let Some(id) = state.menu_row
        {
            match command {
                SnapshotCommand::Rename => state.rename = Some(id),
                SnapshotCommand::Delete => events.push(HistoryEvent::DeleteSnapshot(id)),
            }
        }

        let trash = match selected_snapshot {
            Some(id) => Some(HistoryKey::Snapshot(id)),
            None => steps.can_delete(current).then_some(HistoryKey::Step(current_id)),
        };
        MenuFacts {
            features,
            can_forward: current + 1 < len,
            can_backward: current > 0,
            delete: trash,
            can_clear: len > 1,
            current: current_id,
            rows_revision: revision,
        }
        .store(&ctx, facts_id(id_salt));
        if has_footer {
            let spec = FooterSpec { features: &features, labels: &labels, trash, list: list_id, accepts: &accepts, current: current_id };
            events.extend(show_footer(ui, &spec));
        }
        HistoryOutput { response: out.response, events }
    }
}

/// The source column of a row: exclusive, the History Brush icon on the source row and under the
/// pointer elsewhere. `{row}` in the name is the row's label, filled by the list.
fn source_column(labels: &HistoryLabels, is_source: bool) -> ToggleColumn {
    ToggleColumn::new(&labels.source, Icon::Clock, Icon::Clock, is_source).exclusive(true).tooltip(&labels.source_tooltip)
}

/// A row of a snapshot's context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotCommand {
    Rename,
    Delete,
}
