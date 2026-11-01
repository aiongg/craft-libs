//! `HistoryPanel::{panel_menu, run_command}`: the panel menu rows in Photoshop's order.

use egui::{AsIdSalt, Context, Id, IdSalt};

use super::labels::HistoryLabels;
use super::panel::{HistoryEvent, HistoryFeatures, HistoryKey, HistoryPanel};
use super::state::HistoryPanelState;
use crate::docking::panel_menu::PanelCommand;
use crate::edit::steps::StepId;
use crate::localized::Labels;
use crate::menu::{MenuEntry, MenuItem};

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
        Id::NULL.with(IdSalt::new(id_salt))
    }

    /// Stores the facts for this pass.
    pub(super) fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }

    /// The facts of the last pass, if the panel was drawn.
    pub(super) fn load(ctx: &Context, id: Id) -> Option<MenuFacts> {
        ctx.data(|d| d.get_temp(id))
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
        let labels = HistoryLabels::of_ctx(ctx);
        let facts = MenuFacts::load(ctx, MenuFacts::id(id_salt));
        let features = facts.map_or_else(HistoryFeatures::default, |facts| facts.features);
        let enabled = |command: &str| match facts {
            Some(facts) => facts.action(state, command).is_some(),
            None => matches!(command, CMD_NEW_SNAPSHOT | CMD_NEW_DOCUMENT | CMD_OPTIONS),
        };
        let item = |label: &str, command: &str| MenuEntry::from(MenuItem::new(label, command.to_owned()).enabled(enabled(command)));
        let mut groups = [Vec::new(), Vec::new(), Vec::new()];
        groups[0].push(item(&labels.step_forward, CMD_STEP_FORWARD));
        groups[0].push(item(&labels.step_backward, CMD_STEP_BACKWARD));
        if features.snapshots {
            groups[1].push(item(&labels.new_snapshot, CMD_NEW_SNAPSHOT));
        }
        if features.delete {
            groups[1].push(item(&labels.delete, CMD_DELETE));
        }
        if features.delete_later {
            groups[1].push(item(&labels.delete_later, CMD_DELETE_LATER));
        }
        groups[1].push(item(&labels.clear, CMD_CLEAR));
        if features.new_document {
            groups[1].push(item(&labels.new_document, CMD_NEW_DOCUMENT));
        }
        if features.options {
            groups[2].push(item(&labels.options, CMD_OPTIONS));
        }
        let mut entries = Vec::new();
        for group in groups.into_iter().filter(|group| !group.is_empty()) {
            if !entries.is_empty() {
                entries.push(MenuEntry::Separator);
            }
            entries.extend(group);
        }
        entries
    }

    /// Carries out a row of [`panel_menu`](Self::panel_menu): the event the app runs, as a
    /// footer button's. New Snapshot… asks for the dialog (`NewSnapshot { dialog: true }`, Lead
    /// call 15); Delete reports `DeleteSnapshot` (and clears `state`'s snapshot selection) or
    /// `DeleteStep`; New Document reports `NewDocument` of the current state. `None` for a command
    /// that is not the panel's or a row that is disabled.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, state: &mut HistoryPanelState, command: &PanelCommand) -> Option<HistoryEvent> {
        // pc: crates/ui-egui/src/panels/history_menu_ui.rs:27-60 (`params`, run)
        let facts = MenuFacts::load(ctx, MenuFacts::id(id_salt))?;
        let event = facts.action(state, command)?;
        if matches!(event, HistoryEvent::DeleteSnapshot(_)) {
            state.clear_snapshot();
        }
        Some(event)
    }
}

impl MenuFacts {
    /// The event a command runs, `None` for a command that is not the panel's, a row of a feature
    /// that is off, or a disabled row. The selected snapshot decides Delete.
    fn action(&self, state: &HistoryPanelState, command: &str) -> Option<HistoryEvent> {
        let features = &self.features;
        match command {
            CMD_STEP_FORWARD => self.can_forward.then_some(HistoryEvent::StepForward),
            CMD_STEP_BACKWARD => self.can_backward.then_some(HistoryEvent::StepBackward),
            CMD_NEW_SNAPSHOT => features.snapshots.then_some(HistoryEvent::NewSnapshot { dialog: true }),
            CMD_DELETE if features.delete => match state.selected.map(|(id, _)| id) {
                Some(id) => Some(HistoryEvent::DeleteSnapshot(id)),
                None => match self.delete? {
                    HistoryKey::Snapshot(id) => Some(HistoryEvent::DeleteSnapshot(id)),
                    HistoryKey::Step(id) => Some(HistoryEvent::DeleteStep(id)),
                },
            },
            CMD_DELETE_LATER => features.delete_later.then_some(HistoryEvent::DeleteLater),
            CMD_CLEAR => self.can_clear.then_some(HistoryEvent::ClearHistory),
            CMD_NEW_DOCUMENT => features.new_document.then_some(HistoryEvent::NewDocument(self.current)),
            CMD_OPTIONS => features.options.then_some(HistoryEvent::Options),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::Context;

    use super::*;

    fn facts(delete: Option<HistoryKey>, can_forward: bool, can_backward: bool, can_clear: bool) -> MenuFacts {
        MenuFacts { features: HistoryFeatures::default(), can_forward, can_backward, delete, can_clear, current: StepId::new(3) }
    }

    fn enabled(entries: &[MenuEntry<PanelCommand>]) -> Vec<(String, bool)> {
        entries.iter().filter_map(MenuEntry::item).filter_map(|item| Some((item.command()?.clone(), item.is_enabled()))).collect()
    }

    // Before the first pass only New Snapshot, New Document and History Options are enabled; after it
    // the rows follow the facts.
    #[test]
    fn the_menu_enables_rows_by_the_stored_facts() {
        let ctx = Context::default();
        let state = HistoryPanelState::new();
        let before = enabled(&HistoryPanel::panel_menu(&ctx, "h", &state));
        let on: Vec<_> = before.iter().filter(|(_, on)| *on).map(|(c, _)| c.as_str()).collect();
        assert_eq!(on, [CMD_NEW_SNAPSHOT, CMD_NEW_DOCUMENT, CMD_OPTIONS]);

        facts(Some(HistoryKey::Step(StepId::new(3))), false, true, true).store(&ctx, MenuFacts::id("h"));
        let after = enabled(&HistoryPanel::panel_menu(&ctx, "h", &state));
        let off: Vec<_> = after.iter().filter(|(_, on)| !*on).map(|(c, _)| c.as_str()).collect();
        assert_eq!(off, [CMD_STEP_FORWARD]);
    }

    // Rows of a feature that is off leave out their separators too.
    #[test]
    fn features_off_leave_out_rows_and_doubled_separators() {
        let ctx = Context::default();
        let mut stored = facts(None, false, false, false);
        stored.features = HistoryFeatures { snapshots: false, source: false, new_document: false, options: false, ..HistoryFeatures::default() };
        stored.store(&ctx, MenuFacts::id("h"));
        let entries = HistoryPanel::panel_menu(&ctx, "h", &HistoryPanelState::new());
        let commands: Vec<_> = enabled(&entries).into_iter().map(|(c, _)| c).collect();
        assert_eq!(commands, [CMD_STEP_FORWARD, CMD_STEP_BACKWARD, CMD_DELETE, CMD_CLEAR]);
        assert_eq!(entries.iter().filter(|e| matches!(e, MenuEntry::Separator)).count(), 1);
    }

    // The commands map to the footer's events; a disabled row or a foreign command gives none.
    #[test]
    fn run_command_reports_the_event_of_an_enabled_row() {
        let ctx = Context::default();
        let mut state = HistoryPanelState::new();
        let step = StepId::new(3);
        assert_eq!(HistoryPanel::run_command(&ctx, "h", &mut state, &CMD_CLEAR.to_owned()), None, "before the first pass");
        facts(Some(HistoryKey::Step(step)), true, false, false).store(&ctx, MenuFacts::id("h"));
        let mut run = |command: &str| HistoryPanel::run_command(&ctx, "h", &mut state, &command.to_owned());
        assert_eq!(run(CMD_STEP_FORWARD), Some(HistoryEvent::StepForward));
        assert_eq!(run(CMD_STEP_BACKWARD), None);
        assert_eq!(run(CMD_CLEAR), None);
        assert_eq!(run(CMD_DELETE), Some(HistoryEvent::DeleteStep(step)));
        assert_eq!(run(CMD_NEW_SNAPSHOT), Some(HistoryEvent::NewSnapshot { dialog: true }));
        assert_eq!(run(CMD_NEW_DOCUMENT), Some(HistoryEvent::NewDocument(step)));
        assert_eq!(run(CMD_DELETE_LATER), None, "the feature is off");
        assert_eq!(run("other.command"), None);
    }
}
