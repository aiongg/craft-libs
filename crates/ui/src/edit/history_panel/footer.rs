//! The History footer: Delete (with the trash drop), New Snapshot (Alt opens the dialog), New Document.

use egui::{DragAndDrop, Ui};

use super::labels::HistoryLabels;
use super::panel::{HistoryEvent, HistoryFeatures, HistoryKey};
use crate::behavior::ListDrag;
use crate::composites::PanelFooter;
use crate::edit::steps::StepId;
use crate::icons::Icon;

/// What the footer is drawn from; built by `HistoryPanel::show` each pass.
pub(super) struct FooterSpec<'a> {
    pub(super) features: &'a HistoryFeatures,
    pub(super) labels: &'a HistoryLabels,
    /// What a click on the trash deletes: the selected snapshot, else the current state unless it
    /// is the oldest; `None` disables the button (a drop can still be taken, see `accepts`).
    pub(super) trash: Option<HistoryKey>,
    /// Whether a dragged row may be dropped on the trash: not the oldest state; in linear history
    /// not a later state; every snapshot.
    pub(super) accepts: &'a dyn Fn(&HistoryKey) -> bool,
    /// The current state, for New Document.
    pub(super) current: StepId,
}

/// Draws the footer (`PanelFooter`) with the buttons whose feature is on, from the reading end:
/// Delete (`Icon::Trash`, named [`HistoryLabels::delete_button`]), New Snapshot (`Icon::Scan`,
/// [`HistoryLabels::new_snapshot_button`]), New Document (`Icon::NewFile`,
/// [`HistoryLabels::new_document_button`]); each is a Tab stop. With no button left the footer is
/// not drawn and nothing is reported.
///
/// - A click on the trash reports `DeleteSnapshot` or `DeleteStep` for `spec.trash`.
/// - A drop on the trash: right after the button,
///   `PanelFooter::drop_target::<ListDrag<HistoryKey>>(ui, &trash, |drag| drag.items.first().is_some_and(spec.accepts))`;
///   a payload reports `DeleteSnapshot` or `DeleteStep` for its first item (the panel's list is
///   single-selection, so a drag carries one row).
/// - The camera reports `NewSnapshot { dialog: false }`, or `dialog: true` with Alt held.
/// - New Document reports `NewDocument(spec.current)`.
pub(super) fn show_footer(ui: &mut Ui, spec: &FooterSpec<'_>) -> Vec<HistoryEvent> {
    // pc: crates/ui-egui/src/panels/history.rs:117-160 (trash, camera with Alt :149, file-plus; drop on the trash :138-146)
    let features = spec.features;
    if !(features.delete || features.snapshots || features.new_document) {
        return Vec::new();
    }
    let delete_event = |key: HistoryKey| match key {
        HistoryKey::Snapshot(id) => HistoryEvent::DeleteSnapshot(id),
        HistoryKey::Step(id) => HistoryEvent::DeleteStep(id),
    };
    let mut events = Vec::new();
    PanelFooter::show(ui, |ui| {
        if features.delete {
            let droppable = DragAndDrop::payload::<ListDrag<HistoryKey>>(ui.ctx()).is_some_and(|drag| drag.items.first().is_some_and(spec.accepts));
            let trash =
                ui.add_enabled_ui(spec.trash.is_some() || droppable, |ui| PanelFooter::button(ui, Icon::Trash, &spec.labels.delete_button)).inner;
            if trash.clicked()
                && let Some(key) = spec.trash
            {
                events.push(delete_event(key));
            }
            if let Some(drag) = PanelFooter::drop_target::<ListDrag<HistoryKey>>(ui, &trash, |drag| drag.items.first().is_some_and(spec.accepts))
                && let Some(&key) = drag.items.first()
            {
                events.push(delete_event(key));
            }
        }
        if features.snapshots && PanelFooter::button(ui, Icon::Scan, &spec.labels.new_snapshot_button).clicked() {
            let dialog = ui.input(|i| i.modifiers.alt);
            events.push(HistoryEvent::NewSnapshot { dialog });
        }
        if features.new_document && PanelFooter::button(ui, Icon::NewFile, &spec.labels.new_document_button).clicked() {
            events.push(HistoryEvent::NewDocument(spec.current));
        }
    });
    events
}

#[cfg(test)]
mod tests {
    use egui::{Context, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::*;
    use crate::localized::Labels as _;
    use crate::theme::{Base, install};

    fn run(features: HistoryFeatures, trash: Option<HistoryKey>, alt: bool, click: Option<&str>) -> Vec<HistoryEvent> {
        let events = std::cell::RefCell::new(Vec::new());
        let accepts = |_: &HistoryKey| true;
        let mut harness =
            Harness::builder().with_size([240.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
                let labels = HistoryLabels::of(ui);
                let spec = FooterSpec { features: &features, labels: &labels, trash, accepts: &accepts, current: StepId::new(7) };
                events.borrow_mut().extend(show_footer(ui, &spec));
            });
        harness.run();
        if let Some(label) = click {
            if alt {
                harness.event(egui::Event::ModifiersChanged(Modifiers::ALT));
                harness.run();
            }
            harness.get_by_label(label).click();
            harness.run();
        }
        drop(harness);
        events.into_inner()
    }

    // The trash deletes `spec.trash`; the camera takes the dialog with Alt held; New Document
    // reports the current state.
    #[test]
    fn the_buttons_report_their_events() {
        let labels = HistoryLabels::english();
        let key = HistoryKey::Step(StepId::new(2));
        let all = HistoryFeatures::default();
        assert_eq!(run(all, Some(key), false, Some(&labels.delete_button)), [HistoryEvent::DeleteStep(StepId::new(2))]);
        assert_eq!(run(all, None, false, Some(&labels.delete_button)), []);
        assert_eq!(run(all, None, false, Some(&labels.new_snapshot_button)), [HistoryEvent::NewSnapshot { dialog: false }]);
        assert_eq!(run(all, None, true, Some(&labels.new_snapshot_button)), [HistoryEvent::NewSnapshot { dialog: true }]);
        assert_eq!(run(all, None, false, Some(&labels.new_document_button)), [HistoryEvent::NewDocument(StepId::new(7))]);
    }

    // A button whose feature is off is not drawn; with none left nothing is drawn.
    #[test]
    fn the_footer_leaves_out_buttons_of_features_that_are_off() {
        let labels = HistoryLabels::english();
        let off = HistoryFeatures { snapshots: false, new_document: false, ..HistoryFeatures::default() };
        let harness = Harness::builder().with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
            let spec = FooterSpec { features: &off, labels: &labels, trash: None, accepts: &|_| true, current: StepId::new(0) };
            show_footer(ui, &spec);
        });
        assert!(harness.query_by_label(&labels.delete_button).is_some());
        assert!(harness.query_by_label(&labels.new_snapshot_button).is_none());
        assert!(harness.query_by_label(&labels.new_document_button).is_none());
    }
}
