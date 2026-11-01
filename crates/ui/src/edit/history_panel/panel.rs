//! `HistoryPanel::show`: rows, selection, events, rename, follow-scroll, the source column, drag and thumbnails.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::labels::HistoryLabels;
use super::state::HistoryPanelState;
use crate::canvas::textures::{ThumbSource, Thumbnails};
use crate::edit::snapshots::{HistorySource, SnapshotId, SnapshotList};
use crate::edit::steps::{StepId, StepRows};

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
    RenameSnapshot { id: SnapshotId, name: String },
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
///   renames it in place (the list's rename field, whole name selected; Enter or a click away
///   reports `RenameSnapshot`, Esc cancels, a blank name keeps the old one); states cannot be
///   renamed. A snapshot's context menu has Rename… and Delete; state rows have none.
/// - **Drag to the trash.** A state (not the oldest; in linear history not a later one) or a
///   snapshot drags with the list's ghost; a drop on the footer's trash reports `DeleteStep` or
///   `DeleteSnapshot`; Esc or a release elsewhere does nothing. The list's `Reorder` refuses every
///   target, so the drop is the drag's only effect (the list's own `Begin` and `Cancel` are not
///   reported).
/// - **Scroll.** The current row scrolls into view only when `StepRows::revision` moved since the
///   last pass (a new state, a jump, another document), the row is hidden and no snapshot is
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
}

impl<'a> HistoryPanel<'a> {
    /// A panel over `steps` (a `StepList` or the app's own [`StepRows`]), keeping its snapshot
    /// selection and scroll memory in `state`.
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
    /// `menu::MenuFacts::store` keyed by `id_salt`.
    pub fn show(self, ui: &mut Ui) -> HistoryOutput {
        // pc: crates/ui-egui/src/panels/history.rs:5-160 (`history_panel`: rows :44-114, source column :196-209, snapshot rows :273-334, follow-scroll :31-41)
        todo!()
    }
}
