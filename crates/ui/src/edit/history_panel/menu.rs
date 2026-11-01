//! `HistoryPanel::{panel_menu, run_command}`: the panel menu rows in Photoshop's order.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context, Id};

use super::panel::{HistoryEvent, HistoryFeatures, HistoryKey, HistoryPanel};
use super::state::HistoryPanelState;
use crate::docking::panel_menu::PanelCommand;
use crate::edit::steps::StepId;
use crate::menu::MenuEntry;

/// The menu's commands.
pub(super) const CMD_STEP_FORWARD: &str = "history.stepForward";
pub(super) const CMD_STEP_BACKWARD: &str = "history.stepBackward";
pub(super) const CMD_NEW_SNAPSHOT: &str = "history.newSnapshot";
pub(super) const CMD_DELETE: &str = "history.delete";
pub(super) const CMD_DELETE_LATER: &str = "history.deleteLater";
pub(super) const CMD_CLEAR: &str = "history.clear";
pub(super) const CMD_NEW_DOCUMENT: &str = "history.newDocument";
pub(super) const CMD_OPTIONS: &str = "history.options";

/// What the menu's enabled rules need, stored in egui memory by `HistoryPanel::show` each pass
/// under an id made from the panel's `id_salt` alone (as `TextStylesPanel`'s facts), so the dock
/// can build the menu without the panel's `Ui`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MenuFacts {
    pub(super) features: HistoryFeatures,
    /// A later state exists.
    pub(super) can_forward: bool,
    /// A past state exists.
    pub(super) can_backward: bool,
    /// What Delete deletes: the selected snapshot, else the current state unless it is the
    /// oldest, else (non-linear history) none, since a later state is deleted only by a drop.
    pub(super) delete: Option<HistoryKey>,
    /// The rows hold more than one state.
    pub(super) can_clear: bool,
    /// The current state, for New Document.
    pub(super) current: StepId,
}

impl MenuFacts {
    /// The id the facts of the panel `id_salt` are kept under.
    pub(super) fn id(id_salt: impl AsIdSalt) -> Id {
        // craft-libs: crates/text/src/ui/style_panel.rs:333-340 (`MenuFacts::of(ctx, IdSalt::new(id_salt))`)
        todo!()
    }

    /// Stores the facts for this pass.
    pub(super) fn store(self, ctx: &Context, id: Id) {
        // new: `ctx.data_mut(|d| d.insert_temp(id, self))`.
        todo!()
    }

    /// The facts of the last pass, if the panel was drawn.
    pub(super) fn load(ctx: &Context, id: Id) -> Option<MenuFacts> {
        // new: `ctx.data(|d| d.get_temp(id))`.
        todo!()
    }
}

impl HistoryPanel<'_> {
    /// The panel menu's rows, for the dock's `panel_menu_entries` hook, in Photoshop's order:
    /// Step Forward, Step Backward, a separator, New Snapshot…, Delete, (Delete Redoable Actions
    /// with `delete_later`), Clear History, New Document, a separator, History Options…. Rows for a
    /// feature that is off are left out. The app appends its own rows (VectorCraft's Set History
    /// Limit…), then the dock adds Float, Move Panel To and Close.
    ///
    /// Enabled: Step Forward when a later state exists; Step Backward when a past state exists;
    /// Delete when something can be deleted (the selected snapshot, else the current state unless
    /// it is the oldest); Clear History with more than one state; New Snapshot, New Document,
    /// Delete Redoable Actions and History Options always. Before the panel's first pass every row
    /// but New Snapshot, New Document and History Options is disabled. `state` is the document's
    /// panel state (its selected snapshot decides Delete). Words from [`HistoryLabels`](super::HistoryLabels)
    /// of the installed words.
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, state: &HistoryPanelState) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/history_menu_ui.rs:9-25 (`ITEMS`), :41-48 (enabled)
        todo!()
    }

    /// Carries out a row of [`panel_menu`](Self::panel_menu): the event the app runs, as a
    /// footer button's. New Snapshot… asks for the dialog (`NewSnapshot { dialog: true }`, Lead
    /// call 15); Delete reports `DeleteSnapshot` (and clears `state`'s snapshot selection) or
    /// `DeleteStep`; New Document reports `NewDocument` of the current state. `None` for a command
    /// that is not the panel's or a row that is disabled.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, state: &mut HistoryPanelState, command: &PanelCommand) -> Option<HistoryEvent> {
        // pc: crates/ui-egui/src/panels/history_menu_ui.rs:27-60 (`params`, run)
        todo!()
    }
}
