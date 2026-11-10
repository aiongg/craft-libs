//! The panel menu: the rows of the History menu in Photoshop's order, enabled from what the panel reports.

use super::words::HistoryWords;
use super::{HistoryEvent, HistoryFeatures, HistoryKey, StepId};
use crate::contract::{Props as _, Show as _};
use crate::display::Divider;
use crate::menu::MenuItem;
use crate::runtime::env::{Env, Label};
use egui::Ui;

/// What the panel menu's enabled rules need, reported by every pass of the panel in [`HistoryOutput::menu`].
///
/// [`HistoryOutput::menu`]: super::HistoryOutput::menu
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HistoryMenu {
    pub(super) features: HistoryFeatures,
    pub(super) can_forward: bool,
    pub(super) can_backward: bool,
    pub(super) delete: Option<HistoryKey>,
    pub(super) can_clear: bool,
    pub(super) current: StepId,
}

impl HistoryMenu {
    /// A menu before the panel's first pass: only New Snapshot, New Document and History Options are enabled.
    pub fn new(features: HistoryFeatures, current: StepId) -> Self {
        HistoryMenu { features, can_forward: false, can_backward: false, delete: None, can_clear: false, current }
    }
}

/// Shows the rows in the body of a menu and returns the event of the row run: Step Forward, Step Backward, a
/// rule, New Snapshot…, Delete, Delete Redoable Actions (with `delete_later`), Clear History, New Document,
/// a rule, History Options…. Rows of a feature that is off are left out, with their rules. New Snapshot… asks
/// for the dialog (`NewSnapshot { dialog: true }`).
pub fn show_history_menu(menu: &HistoryMenu, ui: &mut Ui) -> Option<HistoryEvent> {
    let words = Env::of(ui).words::<HistoryWords>();
    let features = &menu.features;
    let mut run = None;
    let mut row = |ui: &mut Ui, label: &Label, event: Option<HistoryEvent>| {
        if MenuItem::new(label.clone()).enabled(event.is_some()).show(ui).ran {
            run = event;
        }
    };
    row(ui, &words.step_forward, menu.can_forward.then_some(HistoryEvent::StepForward));
    row(ui, &words.step_backward, menu.can_backward.then_some(HistoryEvent::StepBackward));
    Divider::horizontal().show(ui);
    if features.snapshots {
        row(ui, &words.new_snapshot, Some(HistoryEvent::NewSnapshot { dialog: true }));
    }
    if features.delete {
        let event = menu.delete.map(|key| match key {
            HistoryKey::Snapshot(id) => HistoryEvent::DeleteSnapshot(id),
            HistoryKey::Step(id) => HistoryEvent::DeleteStep(id),
        });
        row(ui, &words.delete, event);
    }
    if features.delete_later {
        row(ui, &words.delete_later, Some(HistoryEvent::DeleteLater));
    }
    row(ui, &words.clear, menu.can_clear.then_some(HistoryEvent::ClearHistory));
    if features.new_document {
        row(ui, &words.new_document, Some(HistoryEvent::NewDocument(menu.current)));
    }
    if features.options {
        Divider::horizontal().show(ui);
        row(ui, &words.options, Some(HistoryEvent::Options));
    }
    run
}
