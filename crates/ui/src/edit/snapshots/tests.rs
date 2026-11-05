//! Behaviour tests of snapshots, the source choice and the forms.

use egui::vec2;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::{HistoryOptions, HistorySource, HistorySourceChoice, SnapshotId, SnapshotList, SnapshotOptions, history_options_form, new_snapshot_form};
use crate::dialog::{CommandForm, DialogFields, DialogHost, DialogResult, Param, ParamKind, ParamResult};
use crate::edit::history_panel::{HistoryOptionsLabels, SnapshotFormLabels};
use crate::edit::steps::{HistoryStep, StepList};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<ParamResult>>,
}

/// Opens `form`, runs it, presses OK and returns the answer's fields.
fn answer(form: CommandForm) -> DialogFields {
    let mut host = DialogHost::default();
    host.open(form);
    let mut harness: Harness<'_, App> =
        HarnessBuilder::default().with_size(vec2(700.0, 500.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<CommandForm>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        );
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    harness.run();
    let [DialogResult::Ok(ParamResult { fields, .. })] = &harness.state().answers[..] else { panic!("one OK, got {:?}", harness.state().answers) };
    fields.clone()
}

fn en() -> LocaleNumbers {
    LocaleNumbers::from_tag("en")
}

fn names(list: &SnapshotList) -> Vec<&str> {
    list.iter().map(|snapshot| snapshot.name.as_str()).collect()
}

// Add, remove, add: the second id differs from the first, and the suggestion counter keeps counting.
#[test]
fn snapshot_ids_are_never_reused() {
    let labels = SnapshotFormLabels::english();
    let mut list = SnapshotList::new();
    let first = list.add(None, &labels, &en());
    assert!(list.remove(first));
    let second = list.add(None, &labels, &en());
    assert_ne!(first, second);
    assert_eq!(names(&list), ["Snapshot 2"]);
    assert_eq!(list.take_dropped(), [first]);
    assert!(list.take_dropped().is_empty());
}

// `add(None)` and `add(Some("  "))` give "Snapshot 1" and "Snapshot 2"; `add(Some(" A "))` gives "A".
#[test]
fn a_blank_name_takes_the_suggestion_and_names_are_trimmed() {
    let labels = SnapshotFormLabels::english();
    let mut list = SnapshotList::new();
    list.add(None, &labels, &en());
    list.add(Some("  "), &labels, &en());
    list.add(Some(" A "), &labels, &en());
    assert_eq!(names(&list), ["Snapshot 1", "Snapshot 2", "A"]);
}

// `rename(id, "  ")` returns false and keeps the name; a real name is trimmed.
#[test]
fn rename_refuses_a_blank_name() {
    let labels = SnapshotFormLabels::english();
    let mut list = SnapshotList::new();
    let id = list.add(Some("One"), &labels, &en());
    let revision = list.revision();
    assert!(!list.rename(id, "  "));
    assert!(!list.rename(SnapshotId::new(99), "X"));
    assert_eq!((names(&list), list.revision()), (vec!["One"], revision));
    assert!(list.rename(id, " Two "));
    assert_eq!(names(&list), ["Two"]);
    assert_ne!(list.revision(), revision);
}

// After `set(Snapshot(id))`, `resolve` is that snapshot; after removing it, `None`.
#[test]
fn the_source_resolves_to_a_snapshot_when_one_is_chosen() {
    let mut steps = StepList::new(HistoryStep::new("Open"));
    let mut list = SnapshotList::new();
    let id = list.add(None, &SnapshotFormLabels::english(), &en());
    let mut choice = HistorySourceChoice::default();
    choice.set(HistorySource::Snapshot(id), &mut steps);
    assert_eq!(choice.resolve(&steps, &list), Some(HistorySource::Snapshot(id)));
    list.remove(id);
    assert_eq!(choice.resolve(&steps, &list), None);
}

// Choosing a snapshot unpins the state; choosing a state pins it in the step list.
#[test]
fn a_snapshot_source_and_a_state_source_are_separate_choices() {
    let mut steps = StepList::new(HistoryStep::new("Open"));
    let open = steps.current_id();
    steps.record(HistoryStep::new("Blur"));
    let blur = steps.current_id();
    let mut list = SnapshotList::new();
    let snapshot = list.add(None, &SnapshotFormLabels::english(), &en());
    let mut choice = HistorySourceChoice::default();
    choice.set(HistorySource::Step(blur), &mut steps);
    assert_eq!((steps.pinned(), choice.resolve(&steps, &list)), (Some(blur), Some(HistorySource::Step(blur))));
    choice.set(HistorySource::Snapshot(snapshot), &mut steps);
    assert_eq!((steps.pinned(), choice.resolve(&steps, &list)), (None, Some(HistorySource::Snapshot(snapshot))));
    assert_eq!(steps.source(), Some(open));
}

// After `reset`, `resolve` is the oldest state and nothing is pinned.
#[test]
fn reset_returns_to_the_oldest_state() {
    let mut steps = StepList::new(HistoryStep::new("Open"));
    let open = steps.current_id();
    steps.record(HistoryStep::new("Blur"));
    let blur = steps.current_id();
    let list = SnapshotList::new();
    let mut choice = HistorySourceChoice::default();
    choice.set(HistorySource::Step(blur), &mut steps);
    choice.reset(&mut steps);
    assert_eq!((steps.pinned(), choice.resolve(&steps, &list)), (None, Some(HistorySource::Step(open))));
}

// The form's fields read back through `SnapshotOptions::from_fields`; no From row without options.
#[test]
fn new_snapshot_form_returns_name_and_from_by_key() {
    let labels = SnapshotFormLabels::english();
    let from = vec![("fullDocument".to_owned(), "Full Document".to_owned()), ("mergedLayers".to_owned(), "Merged Layers".to_owned())];
    let fields = answer(new_snapshot_form("snapshot", &labels, "Snapshot 3", from));
    assert_eq!(SnapshotOptions::from_fields(&fields), SnapshotOptions { name: "Snapshot 3".into(), from: Some("fullDocument".into()) });
    let fields = answer(new_snapshot_form("snapshot", &labels, " Mine ", Vec::new()));
    assert_eq!(SnapshotOptions::from_fields(&fields), SnapshotOptions { name: "Mine".into(), from: None });
}

// `nonLinear` and an extra row's key are both in the fields; `HistoryOptions::from_fields` reads the check.
#[test]
fn history_options_form_returns_the_check_and_the_extra_rows() {
    let labels = HistoryOptionsLabels::english();
    let extra = vec![Param::new("undoableVisibility", "Make Layer Visibility Changes Undoable", ParamKind::Bool { default: true })];
    let fields = answer(history_options_form("historyOptions", &labels, &HistoryOptions { non_linear: true }, extra));
    assert_eq!(HistoryOptions::from_fields(&fields), HistoryOptions { non_linear: true });
    assert_eq!(fields.bool("undoableVisibility"), Some(true));
    assert_eq!(HistoryOptions::from_fields(&DialogFields::new()), HistoryOptions { non_linear: false });
}

// A `Default` list suggests "Snapshot 1" like a new one; the number takes the locale's digits.
#[test]
fn the_suggested_number_starts_at_one_and_uses_locale_digits() {
    let labels = SnapshotFormLabels::english();
    assert_eq!(SnapshotList::default(), SnapshotList::new());
    assert_eq!(SnapshotList::default().suggested_name(&labels, &en()), "Snapshot 1");
    let mut list = SnapshotList::new();
    list.add(None, &labels, &en());
    let arabic = LocaleNumbers::from_tag("ar-EG");
    assert_eq!(list.suggested_name(&labels, &arabic), "Snapshot \u{662}");
}
