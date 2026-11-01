//! Screen and AccessKit tests of the undo list (task ST3).

use egui::accesskit::Role;
use egui::{Event, Key, Pos2, Ui, UiBuilder, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{UndoList, UndoListEvent};
use crate::edit::steps::{HistoryStep, StepList};
use crate::testing::HarnessExt as _;
use crate::theme::{Base, install};

struct Host {
    steps: StepList,
    events: Vec<UndoListEvent>,
    closed: bool,
}

/// "State 0" to "State {len - 1}", the last current: `len - 1` actions can be undone.
fn host(len: usize) -> Host {
    let mut steps = StepList::new(HistoryStep::new("State 0"));
    steps.set_limit(len + 100);
    for n in 1..len {
        let _ = steps.record(HistoryStep::new(format!("State {n}")));
    }
    Host { steps, events: Vec::new(), closed: false }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let scope = ui.scope_builder(UiBuilder::new().closable(), |ui| {
        if let Some(event) = UndoList::new("undo", &host.steps).show(ui).event {
            host.events.push(event);
        }
    });
    host.closed |= scope.response.should_close();
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([300.0, 700.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn row<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let label = label.to_owned();
    harness.get_by(move |n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label.as_str()))
}

/// The labels of the marked rows, newest first.
fn marked(harness: &Harness<'_, Host>) -> Vec<String> {
    let mut rows: Vec<_> = harness
        .query_all_by_role(Role::ListBoxOption)
        .filter(|n| n.accesskit_node().is_selected() == Some(true))
        .map(|n| (n.rect().top(), n.accesskit_node().label().unwrap_or_default()))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, label)| label).collect()
}

fn hover(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos + vec2(0.0, 3.0)));
    harness.run();
    harness.event(Event::PointerMoved(pos));
    harness.run();
}

// Hover on row 2 fills rows 0 to 2 with `list.hover`.
#[test]
fn hovering_a_row_marks_it_and_every_newer_row() {
    let mut harness = harness(host(7));
    assert!(marked(&harness).is_empty());
    let list = harness.get_by_role(Role::ListBox);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("Undo History"));

    let pos = row(&harness, "State 4").rect().center();
    hover(&mut harness, pos);
    assert_eq!(marked(&harness), ["State 6", "State 5", "State 4"]);
    assert!(harness.query_by_label("Undo 3 Actions").is_some(), "the footer counts the marked rows");

    let pos = row(&harness, "State 6").rect().center();
    hover(&mut harness, pos);
    assert_eq!(marked(&harness), ["State 6"]);
    assert!(harness.query_by_label("Undo 1 Action").is_some(), "one row is singular");
}

// A click on row 2 reports `UndoThrough(3)`.
#[test]
fn a_click_reports_the_count_of_marked_rows() {
    let mut harness = harness(host(7));
    let pos = row(&harness, "State 4").rect().center();
    harness.click_at(pos);
    harness.run_steps(3);
    assert_eq!(harness.state().events, [UndoListEvent::UndoThrough(3)]);
    assert!(harness.state().closed, "the popup closes");
}

// With no row marked the footer is the Cancel row; clicking it reports nothing and closes the popup.
#[test]
fn the_footer_says_cancel_while_nothing_is_marked_and_the_cancel_row_closes() {
    let mut harness = harness(host(7));
    let cancel = harness.get_by_role_and_label(Role::Button, "Cancel");
    let pos = cancel.rect().center();
    assert!(marked(&harness).is_empty());
    assert!(!harness.state().closed);
    harness.click_at(pos);
    harness.run_steps(3);
    assert!(harness.state().events.is_empty());
    assert!(harness.state().closed, "the Cancel row closes the popup");
}

// Down twice then Enter reports `UndoThrough(2)`; the marked row stays in view past 18 rows.
#[test]
fn arrow_keys_move_the_mark_and_enter_commits() {
    let mut harness = harness(host(7));
    row(&harness, "State 6").focus();
    harness.run();
    for _ in 0..2 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(marked(&harness), ["State 6", "State 5"]);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(marked(&harness), ["State 6"], "Up unmarks the older row");
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().events, [UndoListEvent::UndoThrough(2)]);
    assert!(harness.state().closed);

    let mut harness = self::harness(host(31));
    row(&harness, "State 30").focus();
    harness.run();
    let cancel = harness.get_by_role_and_label(Role::Button, "Cancel").rect();
    let top = row(&harness, "State 30").rect().top();
    assert!(row(&harness, "State 9").rect().top() >= cancel.top(), "the 22nd row starts below the visible rows");
    for _ in 0..22 {
        harness.key_press(Key::ArrowDown);
        harness.run_steps(3);
    }
    let row = row(&harness, "State 9").rect();
    let footer = harness.get_by_label("Undo 22 Actions").rect();
    assert!(row.top() >= top - 0.5 && row.bottom() <= footer.top() + 0.5, "{row:?} below {top} and above footer {footer:?}");
}

// The mark, Enter and Esc belong to the list only while it holds focus (LC53).
#[test]
fn keys_are_the_lists_only_while_it_holds_focus() {
    let mut harness = harness(host(7));
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(marked(&harness).is_empty(), "an unfocused list leaves Down alone");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().closed, "an unfocused list leaves Esc alone");

    row(&harness, "State 6").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed, "Esc closes a focused list");
    assert!(harness.state().events.is_empty());
}

// Enter with nothing marked reports nothing and leaves the list open.
#[test]
fn enter_without_a_mark_does_nothing() {
    let mut harness = harness(host(7));
    row(&harness, "State 6").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().events.is_empty());
    assert!(!harness.state().closed);
}

// A list that closed with a mark shows unmarked, with the Cancel row, when it is shown again.
#[test]
fn a_reopened_list_has_no_mark() {
    let mut harness = harness(host(7));
    let pos = row(&harness, "State 4").rect().center();
    hover(&mut harness, pos);
    assert_eq!(marked(&harness).len(), 3);
    harness.click_at(pos);
    harness.run_steps(3);
    assert_eq!(harness.state().events, [UndoListEvent::UndoThrough(3)]);
    assert!(marked(&harness).is_empty(), "the mark does not outlive the popup");
    assert!(harness.query_by_role_and_label(Role::Button, "Cancel").is_some(), "the Cancel row is back");
}

// Esc closes a marked list, and the mark is gone with it.
#[test]
fn escape_closes_and_clears_the_mark() {
    let mut harness = harness(host(7));
    row(&harness, "State 6").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(marked(&harness), ["State 6"]);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    assert!(harness.state().events.is_empty());
    assert!(marked(&harness).is_empty());
}

// `marked` presets the mark on the first show only.
#[test]
fn the_marked_preset_applies_once() {
    let host = host(7);
    let mut harness =
        Harness::<Host>::builder().with_size([300.0, 700.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let scope = ui.scope_builder(UiBuilder::new().closable(), |ui| {
                    let _ = UndoList::new("undo", &host.steps).marked(Some(1)).show(ui);
                });
                host.closed |= scope.response.should_close();
            },
            host,
        );
    harness.run();
    assert_eq!(marked(&harness), ["State 6", "State 5"]);
}

// The footer count reads `UndoListLabels::undo_actions`, so an app's override shows.
#[test]
fn the_footer_count_uses_the_labels_override() {
    let host = host(7);
    let mut harness =
        Harness::<Host>::builder().with_size([300.0, 700.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let labels = super::UndoListLabels { undo_actions: "Revert {n} edits".into(), ..Default::default() };
                let _ = UndoList::new("undo", &host.steps).labels(labels).marked(Some(1)).show(ui);
            },
            host,
        );
    harness.run();
    assert!(harness.query_by_label("Revert 2 edits").is_some());
}
