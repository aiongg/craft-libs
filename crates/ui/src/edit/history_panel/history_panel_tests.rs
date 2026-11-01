//! Screen and AccessKit tests of the History panel (task ST3; they need the `history` style from TK1).

use egui::accesskit::{Role, Toggled};
use std::cell::RefCell;

use egui::{DragAndDrop, Event, Id, Key, Modifiers, PointerButton, Pos2, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::HistoryKey;
use super::stories::{BranchRows, Doc, HistoryStoryLabels};
use super::{HistoryEvent, HistoryFeatures, HistoryLabels, HistoryOrder, HistoryPanel, SnapshotFormLabels};
use crate::behavior::ListDrag;
use crate::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use crate::direction::{self, Direction};
use crate::edit::snapshots::SnapshotId;
use crate::edit::snapshots::{HistorySource, SnapshotList};
use crate::edit::steps::{HistoryStep, StepId, StepList, StepRows};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::menu::MenuEntry;
use crate::testing::HarnessExt as _;
use crate::theme::{Base, TokensOf as _, install};

/// The panel over a fake document; the host carries out the panel's events as an app would.
struct Host {
    doc: Doc,
    features: HistoryFeatures,
    order: HistoryOrder,
    /// Every event the panel reported, in order.
    events: Vec<HistoryEvent>,
    /// The drop hints a drag showed.
    hints: Vec<String>,
    /// The most events one pass reported.
    max_per_pass: usize,
}

impl Host {
    fn new(steps: StepList, snapshots: SnapshotList) -> Self {
        Host {
            doc: Doc::new(steps, snapshots),
            features: HistoryFeatures::default(),
            order: HistoryOrder::OldestFirst,
            events: Vec::new(),
            hints: Vec::new(),
            max_per_pass: 0,
        }
    }

    fn states(len: usize, current: usize) -> Self {
        Self::new(numbered(len, current), SnapshotList::new())
    }

    fn take_events(&mut self) -> Vec<HistoryEvent> {
        std::mem::take(&mut self.events)
    }
}

/// "State 0" (the oldest) to "State {len - 1}", the state at `current` current.
fn numbered(len: usize, current: usize) -> StepList {
    let mut steps = StepList::new(HistoryStep::new("State 0"));
    steps.set_limit(len + 100);
    for n in 1..len {
        let _ = steps.record(HistoryStep::new(format!("State {n}")));
    }
    if let Some(id) = steps.id_at(current) {
        let _ = steps.go_to(id);
    }
    steps
}

/// Snapshots "Snap A", "Snap B", ... .
fn snapshots(count: usize) -> SnapshotList {
    let labels = SnapshotFormLabels::english();
    let mut list = SnapshotList::new();
    for n in 0..count {
        list.add(Some(&format!("Snap {}", char::from(b'A' + n as u8))), &labels, &LocaleNumbers::from_tag("en"));
    }
    list
}

fn show(ui: &mut Ui, host: &mut Host) {
    let source = host.doc.source.resolve(&host.doc.steps, &host.doc.snapshots);
    let rows: &dyn StepRows = match &host.doc.rows {
        Some(rows) => rows,
        None => &host.doc.steps,
    };
    let out = HistoryPanel::new("history", rows, &mut host.doc.panel)
        .snapshots(&host.doc.snapshots)
        .source(source)
        .features(host.features)
        .order(host.order)
        .show(ui);
    if let Some(hint) = crate::behavior::drop_hint(ui.ctx()) {
        host.hints.push(hint);
    }
    host.max_per_pass = host.max_per_pass.max(out.events.len());
    for event in out.events {
        host.events.push(event.clone());
        host.doc.apply(event);
    }
}

fn build(host: Host, size: [f32; 2], rtl: bool) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size(size)
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            if rtl {
                direction::apply(ctx, Direction::Rtl);
            }
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn harness(host: Host) -> Harness<'static, Host> {
    build(host, [260.0, 420.0], false)
}

fn row<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let label = label.to_owned();
    harness.get_by(move |n| matches!(n.role(), Role::ListBoxOption | Role::TreeItem) && n.label().as_deref() == Some(label.as_str()))
}

fn source_cell<'h>(harness: &'h Harness<'_, Host>, row_label: &str) -> Node<'h> {
    let name = format!("History brush source: {row_label}");
    harness.get_by(move |n| n.label().as_deref() == Some(name.as_str()))
}

fn id(harness: &Harness<'_, Host>, at: usize) -> StepId {
    harness.state().doc.steps.id_at(at).unwrap_or(StepId::new(u64::MAX))
}

fn button_label(pick: impl Fn(&HistoryLabels) -> &String) -> String {
    pick(&HistoryLabels::english()).clone()
}

fn description(node: &Node<'_>) -> Option<String> {
    node.accesskit_node().data().description().map(str::to_owned)
}

/// Rows whose AccessKit node is selected, with their descriptions.
fn selected(harness: &Harness<'_, Host>) -> Vec<(String, Option<String>)> {
    items(harness)
        .filter(|n| n.accesskit_node().is_selected() == Some(true))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), description(&n)))
        .collect()
}

/// Whether the row is drawn inside the list's rect.
fn shown(harness: &Harness<'_, Host>, label: &str) -> bool {
    let list = list(harness).rect().expand(0.5);
    let label = label.to_owned();
    harness
        .query_by(move |n| matches!(n.role(), Role::ListBoxOption | Role::TreeItem) && n.label().as_deref() == Some(label.as_str()))
        .is_some_and(|n| list.contains_rect(n.rect()))
}

/// Lets the double-click delay pass, so the next click is a single click.
fn pause(harness: &mut Harness<'_, Host>) {
    harness.run_steps(30);
}

fn pointer(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: Option<bool>) {
    harness.event(Event::PointerMoved(pos));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// Two quick clicks at `pos`.
fn double_click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
    }
    harness.run();
}

fn key(harness: &mut Harness<'_, Host>, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// The rows of the list, whatever the list's role (a tree when a row has a depth).
fn items<'h>(harness: &'h Harness<'_, Host>) -> impl Iterator<Item = Node<'h>> {
    harness.query_all_by(|n| matches!(n.role(), Role::ListBoxOption | Role::TreeItem))
}

fn list<'h>(harness: &'h Harness<'_, Host>) -> Node<'h> {
    harness.get_by(|n| matches!(n.role(), Role::ListBox | Role::Tree))
}

fn row_names(harness: &Harness<'_, Host>) -> Vec<String> {
    items(harness).filter_map(|n| n.accesskit_node().label()).collect()
}

// A click on a past state reports one `GoTo` with its id and nothing else.
#[test]
fn clicking_a_state_reports_go_to_once() {
    let mut harness = harness(Host::states(6, 3));
    let target = id(&harness, 1);
    let center = row(&harness, "State 1").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::GoTo(target)]);
    assert_eq!(selected(&harness).first().map(|(label, _)| label.as_str()), Some("State 1"));
}

// A click on the current state reports nothing; with a snapshot selected the state row highlights again.
#[test]
fn clicking_the_current_state_reports_nothing_and_deselects_a_snapshot() {
    let mut host = Host::states(6, 3);
    host.doc.snapshots = snapshots(2);
    let snapshot = host.doc.snapshots.iter().next().expect("a snapshot").id;
    host.doc.panel.snapshot_selected(snapshot, host.doc.steps.revision());
    let mut harness = harness(host);
    assert_eq!(selected(&harness), [("Snap A".to_owned(), Some("Snapshot".to_owned()))]);

    let center = row(&harness, "State 3").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
    assert_eq!(selected(&harness), [("State 3".to_owned(), Some("Current state".to_owned()))]);
    assert_eq!(harness.state().doc.panel.selected_snapshot(), None);

    pause(&mut harness);
    harness.click_at(center);
    harness.run_steps(5);
    assert!(harness.state().events.is_empty(), "a click on the current state with nothing selected does nothing");
}

// Later rows paint `list.dimmed` and a click reports `GoTo` forward.
#[test]
fn later_states_are_dimmed_and_clickable() {
    let mut harness = harness(Host::states(6, 2));
    for (label, expected) in
        [("State 1", None), ("State 2", Some("Current state")), ("State 3", Some("Later state")), ("State 5", Some("Later state"))]
    {
        assert_eq!(description(&row(&harness, label)).as_deref(), expected, "{label}");
    }
    let target = id(&harness, 4);
    let center = row(&harness, "State 4").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::GoTo(target)], "a later state is not disabled");
}

// A click on a snapshot row reports `SelectSnapshot`; after `snapshot_selected` it is the one highlighted row.
#[test]
fn clicking_a_snapshot_selects_it_and_reports_select_snapshot() {
    let mut host = Host::states(5, 2);
    host.doc.snapshots = snapshots(2);
    let second = host.doc.snapshots.iter().nth(1).expect("a snapshot").id;
    let mut harness = harness(host);
    let center = row(&harness, "Snap B").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::SelectSnapshot(second)]);
    assert_eq!(harness.state().doc.panel.selected_snapshot(), Some(second));
    assert_eq!(
        selected(&harness),
        [("Snap B".to_owned(), Some("Snapshot".to_owned()))],
        "one highlighted row: the snapshot, not the state it restored"
    );
}

// A record after `snapshot_selected` moves the revision and the current state highlights again.
#[test]
fn a_selected_snapshot_is_deselected_when_the_list_changes() {
    let mut host = Host::states(5, 2);
    host.doc.snapshots = snapshots(1);
    let snapshot = host.doc.snapshots.iter().next().expect("a snapshot").id;
    host.doc.panel.snapshot_selected(snapshot, host.doc.steps.revision());
    let mut harness = harness(host);
    assert_eq!(selected(&harness).len(), 1);
    assert_eq!(harness.state().doc.panel.selected_snapshot(), Some(snapshot));

    let _ = harness.state_mut().doc.steps.record(HistoryStep::new("Blur Tool"));
    harness.run();
    assert_eq!(selected(&harness), [("Blur Tool".to_owned(), Some("Current state".to_owned()))]);
    assert_eq!(harness.state().doc.panel.selected_snapshot(), None);
}

// A click on a state's source cell reports only `SetSource(Step(id))`; on the source row nothing.
#[test]
fn the_source_cell_sets_the_source_and_does_not_step() {
    let mut harness = harness(Host::states(6, 3));
    assert_eq!(source_cell(&harness, "State 0").accesskit_node().toggled(), Some(Toggled::True), "the oldest state is the default source");
    let target = id(&harness, 2);
    let center = source_cell(&harness, "State 2").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::SetSource(HistorySource::Step(target))]);
    assert_eq!(source_cell(&harness, "State 2").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(source_cell(&harness, "State 0").accesskit_node().toggled(), Some(Toggled::False), "one source at a time");

    pause(&mut harness);
    harness.state_mut().take_events();
    let center = source_cell(&harness, "State 2").rect().center();
    harness.click_at(center);
    harness.run_steps(5);
    assert!(harness.state().events.is_empty(), "a click on the source row's cell does nothing: {:?}", harness.state().events);
}

// A press and release on one source cell reports exactly one `SetSource`, for that row. Dragged over
// three rows and released on another row's cell, the press is no click and reports nothing (the
// exclusive column has no sweep).
#[test]
fn a_press_on_the_source_cell_does_not_sweep_other_rows() {
    let mut harness = harness(Host::states(8, 6));
    let pressed = id(&harness, 2);
    let start = source_cell(&harness, "State 2").rect().center();
    let end = source_cell(&harness, "State 5").rect().center();
    pointer(&mut harness, start, Some(true));
    pointer(&mut harness, start + vec2(1.0, 1.0), None);
    pointer(&mut harness, start, Some(false));
    let sets: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, HistoryEvent::SetSource(_))).cloned().collect();
    assert_eq!(sets, [HistoryEvent::SetSource(HistorySource::Step(pressed))]);

    let mut harness = self::harness(Host::states(8, 6));
    pointer(&mut harness, start, Some(true));
    for share in [0.25, 0.5, 0.75, 1.0] {
        pointer(&mut harness, start + (end - start) * share, None);
    }
    pointer(&mut harness, end, Some(false));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
}

// Double-click opens the rename field; Enter reports `RenameSnapshot`, Esc cancels, a blank name reports nothing.
#[test]
fn double_click_renames_a_snapshot_in_place() {
    let mut host = Host::states(4, 3);
    host.doc.snapshots = snapshots(2);
    let first = host.doc.snapshots.iter().next().expect("a snapshot").id;
    let mut harness = harness(host);
    let field = |harness: &Harness<'_, Host>| harness.query_by_role(Role::TextInput).and_then(|n| n.value());

    let center = row(&harness, "Snap A").rect().center();
    double_click(&mut harness, center);
    assert_eq!(field(&harness).as_deref(), Some("Snap A"), "the field opens with the name");
    harness.event(Event::Text("Sky".into()));
    key(&mut harness, Key::Enter);
    assert_eq!(field(&harness), None);
    let renames: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, HistoryEvent::RenameSnapshot { .. })).cloned().collect();
    assert_eq!(renames, [HistoryEvent::RenameSnapshot { id: first, text: "Sky".to_owned() }]);
    assert_eq!(harness.state().doc.snapshots.get(first).map(|s| s.name.as_str()), Some("Sky"));

    harness.state_mut().take_events();
    pause(&mut harness);
    let center = row(&harness, "Sky").rect().center();
    double_click(&mut harness, center);
    harness.event(Event::Text("Zed".into()));
    key(&mut harness, Key::Escape);
    assert_eq!(field(&harness), None);
    assert!(row_names(&harness).contains(&"Sky".to_owned()), "Esc keeps the old name");

    pause(&mut harness);
    double_click(&mut harness, center);
    key(&mut harness, Key::Backspace);
    key(&mut harness, Key::Enter);
    assert!(
        harness.state().events.iter().all(|e| !matches!(e, HistoryEvent::RenameSnapshot { .. })),
        "a blank name keeps the old one: {:?}",
        harness.state().events
    );
    assert!(row_names(&harness).contains(&"Sky".to_owned()));
}

// A state dragged onto the trash reports `DeleteStep` once; a snapshot reports `DeleteSnapshot`.
#[test]
fn dragging_a_state_to_the_trash_reports_delete() {
    let trash = button_label(|labels| &labels.delete_button);
    let mut harness = harness(Host::states(6, 3));
    let target = id(&harness, 2);
    let (from, to) = (row(&harness, "State 2").rect().center(), harness.get_by_label(&trash).rect().center());
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::DeleteStep(target)]);

    let mut host = Host::states(4, 3);
    host.doc.snapshots = snapshots(2);
    let second = host.doc.snapshots.iter().nth(1).expect("a snapshot").id;
    let mut harness = self::harness(host);
    let (from, to) = (row(&harness, "Snap B").rect().center(), harness.get_by_label(&trash).rect().center());
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::DeleteSnapshot(second)]);
}

// A drag from the oldest row or a later row (linear) never outlines the trash and reports nothing.
#[test]
fn the_oldest_state_and_later_states_in_linear_history_cannot_be_dragged() {
    let trash = button_label(|labels| &labels.delete_button);
    for label in ["State 0", "State 4"] {
        let mut harness = harness(Host::states(6, 3));
        let (from, to) = (row(&harness, label).rect().center(), harness.get_by_label(&trash).rect().center());
        harness.drag(from, to, Modifiers::NONE);
        harness.run_steps(5);
        assert!(harness.state().events.is_empty(), "{label}: {:?}", harness.state().events);
        assert!(harness.state().hints.is_empty(), "{label} never outlined the trash: {:?}", harness.state().hints);
    }
    let mut harness = harness(Host::states(6, 3));
    let (from, to) = (row(&harness, "State 2").rect().center(), harness.get_by_label(&trash).rect().center());
    pointer(&mut harness, from, Some(true));
    pointer(&mut harness, from + vec2(0.0, 8.0), None);
    pointer(&mut harness, to, None);
    assert_eq!(harness.state().hints.last().map(String::as_str), Some(trash.as_str()), "a past state outlines the trash");
}

// Trash click with a snapshot selected reports `DeleteSnapshot`, else `DeleteStep(current)`; disabled at the oldest.
#[test]
fn the_trash_deletes_the_selected_snapshot_else_the_current_state() {
    let trash = button_label(|labels| &labels.delete_button);
    let mut harness = harness(Host::states(6, 3));
    let current = id(&harness, 3);
    harness.get_by_label(&trash).click();
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::DeleteStep(current)]);

    let mut host = Host::states(6, 3);
    host.doc.snapshots = snapshots(1);
    let snapshot = host.doc.snapshots.iter().next().expect("a snapshot").id;
    host.doc.panel.snapshot_selected(snapshot, host.doc.steps.revision());
    let mut harness = self::harness(host);
    harness.get_by_label(&trash).click();
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::DeleteSnapshot(snapshot)]);

    let mut harness = self::harness(Host::states(6, 0));
    assert!(harness.get_by_label(&trash).accesskit_node().is_disabled(), "the oldest state cannot be deleted");
    harness.get_by_label(&trash).click();
    harness.run_steps(5);
    assert!(harness.state().events.is_empty());
}

// The camera reports `NewSnapshot { dialog: false }`, with Alt held `dialog: true`.
#[test]
fn alt_click_on_the_camera_asks_for_the_dialog() {
    let camera = button_label(|labels| &labels.new_snapshot_button);
    let mut harness = harness(Host::states(4, 3));
    harness.get_by_label(&camera).click();
    harness.run_steps(5);
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label(&camera).click();
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::NewSnapshot { dialog: false }, HistoryEvent::NewSnapshot { dialog: true }]);
}

// A record whose row is below the viewport scrolls it into view; a visible one keeps the offset.
#[test]
fn a_new_state_scrolls_into_view_only_when_hidden() {
    let mut harness = build(Host::states(30, 29), [260.0, 220.0], false);
    assert!(shown(&harness, "State 29"), "the first pass shows the current state");
    let center = list(&harness).rect().center();
    harness.wheel_at(center, vec2(0.0, 1e5));
    harness.run_steps(20);
    assert!(shown(&harness, "State 0") && !shown(&harness, "State 29"), "the user scrolled to the top");

    let _ = harness.state_mut().doc.steps.record(HistoryStep::new("State 30"));
    harness.run_steps(5);
    assert!(shown(&harness, "State 30"), "the new current state is revealed");

    let mut harness = build(Host::states(12, 3), [260.0, 420.0], false);
    assert!(shown(&harness, "State 0"));
    let _ = harness.state_mut().doc.steps.record(HistoryStep::new("State 4b"));
    harness.run_steps(5);
    assert!(shown(&harness, "State 0") && shown(&harness, "State 4b"), "a visible new row leaves the offset");
}

// Passes with an unchanged revision keep the user's scroll offset.
#[test]
fn unrelated_changes_leave_the_scroll_where_it_is() {
    let mut host = Host::states(30, 29);
    host.doc.snapshots = snapshots(1);
    let mut harness = build(host, [260.0, 220.0], false);
    let center = list(&harness).rect().center();
    harness.wheel_at(center, vec2(0.0, 1e5));
    harness.run_steps(20);
    assert!(shown(&harness, "State 0") && !shown(&harness, "State 29"));

    let snapshot = harness.state().doc.snapshots.iter().next().expect("a snapshot").id;
    harness.state_mut().doc.snapshots.rename(snapshot, "Renamed");
    harness.run_steps(10);
    assert!(shown(&harness, "State 0") && !shown(&harness, "State 29"), "still at the top after more passes");
}

// In a 160 pt dock with 6 snapshots and 20 states the footer stays inside the panel and the rows scroll.
#[test]
fn snapshots_scroll_with_the_states_above_the_footer() {
    let mut host = Host::states(20, 3);
    host.doc.snapshots = snapshots(6);
    let mut harness = build(host, [260.0, 160.0], false);
    let trash = button_label(|labels| &labels.delete_button);
    let footer = harness.get_by_label(&trash).rect();
    let list = list(&harness).rect();
    assert!(footer.bottom() <= 160.5, "the footer is inside the dock: {footer:?}");
    assert!(list.bottom() <= footer.top() + 0.5, "the rows end above the footer: list {list:?}, footer {footer:?}");

    assert!(shown(&harness, "State 3") && !shown(&harness, "State 19"), "the first pass reveals the current state");

    harness.wheel_at(list.center(), vec2(0.0, 1e5));
    harness.run_steps(20);
    assert!(shown(&harness, "Snap A") && !shown(&harness, "State 19"), "the snapshots are at the top");
    harness.wheel_at(list.center(), vec2(0.0, -1e5));
    harness.run_steps(20);
    assert!(shown(&harness, "State 19") && !shown(&harness, "Snap A"), "snapshots and states scroll together");
    assert_eq!(harness.get_by_label(&trash).rect(), footer, "the footer stays put");
}

// Down moves the selection one row and reports `GoTo`; held keys report one event per pass for the last row.
#[test]
fn arrow_keys_select_and_report_one_event_per_pass() {
    let mut harness = harness(Host::states(8, 2));
    key(&mut harness, Key::Tab);
    key(&mut harness, Key::ArrowDown);
    let next = id(&harness, 3);
    assert_eq!(harness.state_mut().take_events(), [HistoryEvent::GoTo(next)]);

    for _ in 0..2 {
        harness.event(Event::Key {
            key: Key::ArrowDown,
            physical_key: Some(Key::ArrowDown),
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    let last = id(&harness, 5);
    let events = harness.state_mut().take_events();
    assert_eq!(events.last(), Some(&HistoryEvent::GoTo(last)), "the last row is the one reported");
    assert!(harness.state().max_per_pass <= 1, "one event per pass: {events:?}");

    key(&mut harness, Key::ArrowUp);
    let up = id(&harness, 4);
    assert_eq!(harness.state_mut().take_events(), [HistoryEvent::GoTo(up)]);
}

/// The panel menu's rows after the last pass: `(command, enabled)`.
fn menu(harness: &Harness<'_, Host>) -> Vec<(String, bool)> {
    let entries = HistoryPanel::panel_menu(&harness.ctx, "history", &harness.state().doc.panel);
    entries.iter().filter_map(MenuEntry::item).filter_map(|item| Some((item.command()?.clone(), item.is_enabled()))).collect()
}

fn enabled(rows: &[(String, bool)], command: &str) -> bool {
    rows.iter().any(|(name, on)| name == command && *on)
}

// Step Forward, Step Backward, Delete and Clear History follow the rows; New Snapshot and New Document always enabled.
#[test]
fn the_menu_enables_rows_by_state() {
    let always = ["history.newSnapshot", "history.newDocument", "history.options"];
    let cases = [
        // (len, current, forward, backward, delete, clear)
        (5, 2, true, true, true, true),
        (5, 0, true, false, false, true),
        (5, 4, false, true, true, true),
        (1, 0, false, false, false, false),
    ];
    for (len, current, forward, backward, delete, clear) in cases {
        let harness = harness(Host::states(len, current));
        let rows = menu(&harness);
        let got = [
            enabled(&rows, "history.stepForward"),
            enabled(&rows, "history.stepBackward"),
            enabled(&rows, "history.delete"),
            enabled(&rows, "history.clear"),
        ];
        assert_eq!(got, [forward, backward, delete, clear], "{len} states, current {current}");
        assert!(always.iter().all(|command| enabled(&rows, command)), "{rows:?}");
    }
}

// `run_command(delete)` with a snapshot selected reports `DeleteSnapshot` and clears the selection.
#[test]
fn delete_in_the_menu_removes_the_selected_snapshot() {
    let mut host = Host::states(5, 2);
    host.doc.snapshots = snapshots(1);
    let snapshot = host.doc.snapshots.iter().next().expect("a snapshot").id;
    host.doc.panel.snapshot_selected(snapshot, host.doc.steps.revision());
    let mut harness = harness(host);
    let ctx = harness.ctx.clone();
    let command = "history.delete".to_owned();
    let event = HistoryPanel::run_command(&ctx, "history", &mut harness.state_mut().doc.panel, &command);
    assert_eq!(event, Some(HistoryEvent::DeleteSnapshot(snapshot)));
    assert_eq!(harness.state().doc.panel.selected_snapshot(), None);
    harness.run();

    let event = HistoryPanel::run_command(&ctx, "history", &mut harness.state_mut().doc.panel, &command);
    let current = id(&harness, 2);
    assert_eq!(event, Some(HistoryEvent::DeleteStep(current)), "with nothing selected Delete takes the current state");
}

// AccessKit: `ListBox` "History"; state rows "Current state" or "Later state"; snapshot rows "Snapshot"; the source cell a checked `Button`.
#[test]
fn rows_have_names_descriptions_and_selected_state() {
    let harness = harness(Host::states(5, 2));
    let list = list(&harness);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("History"));
    assert_eq!(list.accesskit_node().size_of_set(), Some(5));

    let mut host = Host::states(5, 2);
    host.doc.snapshots = snapshots(1);
    let harness = self::harness(host);
    let rows: Vec<_> =
        items(&harness).map(|n| (n.accesskit_node().label().unwrap_or_default(), description(&n), n.accesskit_node().is_selected())).collect();
    let expected = [
        ("Snap A", Some("Snapshot"), false),
        ("State 0", None, false),
        ("State 1", None, false),
        ("State 2", Some("Current state"), true),
        ("State 3", Some("Later state"), false),
        ("State 4", Some("Later state"), false),
    ];
    let expected: Vec<_> =
        expected.into_iter().map(|(label, description, on)| (label.to_owned(), description.map(str::to_owned), Some(on))).collect();
    assert_eq!(rows, expected);

    let cell = harness.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("History brush source: State 0"));
    assert_eq!(cell.accesskit_node().toggled(), Some(Toggled::True), "the source cell is checked on the source");
    let other = harness.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("History brush source: State 1"));
    assert_eq!(other.accesskit_node().toggled(), Some(Toggled::False));
}

// In right to left the source column sits at the right edge and the label ends before it.
#[test]
fn columns_mirror_in_right_to_left() {
    let place = |rtl: bool| {
        let harness = build(Host::states(4, 2), [260.0, 420.0], rtl);
        (source_cell(&harness, "State 1").rect(), row(&harness, "State 1").rect())
    };
    let (cell, row) = place(false);
    assert!(cell.center().x < row.center().x && cell.left() - row.left() < row.right() - cell.right(), "left to right: {cell:?} in {row:?}");
    let (cell, row) = place(true);
    assert!(cell.center().x > row.center().x && row.right() - cell.right() < cell.left() - row.left(), "right to left: {cell:?} in {row:?}");
}

// A `StepRows` with depths indents tree rows and dims `is_later` rows.
#[test]
fn tree_rows_indent_by_depth_and_dim_off_line_rows() {
    let words = HistoryStoryLabels::english();
    let mut host = Host::states(1, 0);
    host.doc.rows = Some(BranchRows::new(&words));
    let harness = harness(host);
    let rows: Vec<_> =
        items(&harness).map(|n| (n.accesskit_node().label().unwrap_or_default(), n.accesskit_node().level(), description(&n))).collect();
    let expected = [
        ("Open", Some(0), None),
        ("Brush Tool", Some(0), None),
        ("Crop", Some(1), Some("Later state")),
        ("Blur Tool", Some(1), Some("Later state")),
        ("Gradient Tool", Some(0), None),
        ("Layer Opacity", Some(0), Some("Current state")),
        ("Fill Layer", Some(0), Some("Later state")),
    ];
    let expected: Vec<_> =
        expected.into_iter().map(|(label, level, description)| (label.to_owned(), level, description.map(str::to_owned))).collect();
    assert_eq!(rows, expected);
}

// `HistoryOrder::NewestFirst` lists states newest first, snapshots still on top, the current state highlighted.
#[test]
fn newest_first_reverses_the_rows_and_keeps_the_selection() {
    let mut host = Host::states(5, 2);
    host.doc.snapshots = snapshots(1);
    host.order = HistoryOrder::NewestFirst;
    let harness = harness(host);
    let mut rows: Vec<_> = items(&harness).map(|n| (n.rect().top(), n.accesskit_node().label().unwrap_or_default())).collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let labels: Vec<_> = rows.into_iter().map(|(_, label)| label).collect();
    assert_eq!(labels, ["Snap A", "State 4", "State 3", "State 2", "State 1", "State 0"]);
    assert_eq!(selected(&harness), [("State 2".to_owned(), Some("Current state".to_owned()))]);
}

// Features off remove their buttons; with all off no footer is drawn.
#[test]
fn the_footer_leaves_out_buttons_for_features_that_are_off() {
    let labels = HistoryLabels::english();
    let buttons = |features: HistoryFeatures| {
        let mut host = Host::states(4, 2);
        host.features = features;
        let harness = harness(host);
        [&labels.delete_button, &labels.new_snapshot_button, &labels.new_document_button].map(|name| harness.query_by_label(name).is_some())
    };
    assert_eq!(buttons(HistoryFeatures::default()), [true, true, true]);
    assert_eq!(buttons(HistoryFeatures { snapshots: false, new_document: false, ..HistoryFeatures::default() }), [true, false, false]);
    assert_eq!(buttons(HistoryFeatures { delete: false, ..HistoryFeatures::default() }), [false, true, true]);
    let none = HistoryFeatures { snapshots: false, source: false, new_document: false, delete: false, delete_later: false, options: false };
    assert_eq!(buttons(none), [false, false, false]);
}

// Rename… in a snapshot's context menu opens the rename field; Delete reports `DeleteSnapshot`.
#[test]
fn a_snapshots_context_menu_renames_and_deletes() {
    let mut host = Host::states(4, 3);
    host.doc.snapshots = snapshots(2);
    let second = host.doc.snapshots.iter().nth(1).expect("a snapshot").id;
    let mut harness = harness(host);
    row(&harness, "Snap B").click_secondary();
    harness.run();
    harness.get_by_label(&HistoryLabels::english().rename).click();
    harness.run();
    let field = harness.query_by_role(Role::TextInput).and_then(|n| n.value());
    assert_eq!(field.as_deref(), Some("Snap B"), "the menu's Rename… opens the row's field");
    key(&mut harness, Key::Escape);

    pause(&mut harness);
    row(&harness, "Snap B").click_secondary();
    harness.run();
    harness.get_by_label(&HistoryLabels::english().delete).click();
    harness.run();
    assert_eq!(harness.state().events, [HistoryEvent::DeleteSnapshot(second)]);
}

// `HistoryPanel::start_rename` opens the field in the pass it is given.
#[test]
fn start_rename_opens_the_field_in_the_same_pass() {
    let mut host = Host::states(4, 3);
    host.doc.snapshots = snapshots(2);
    let first = host.doc.snapshots.iter().next().expect("a snapshot").id;
    let mut harness = Harness::builder().with_size([260.0, 420.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, host: &mut Host| {
            HistoryPanel::new("history", &host.doc.steps, &mut host.doc.panel)
                .snapshots(&host.doc.snapshots)
                .start_rename(first)
                .show(ui)
                .events
                .len();
        },
        host,
    );
    harness.step();
    let field = harness.query_by_role(Role::TextInput).and_then(|n| n.value());
    assert_eq!(field.as_deref(), Some("Snap A"));
}

// An open rename does not carry over to another document's snapshot with the same id.
#[test]
fn a_rename_draft_never_crosses_documents() {
    let mut host = Host::states(4, 3);
    host.doc.snapshots = snapshots(2);
    let mut harness = harness(host);
    let center = row(&harness, "Snap A").rect().center();
    double_click(&mut harness, center);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the field is open in the first document");

    let mut other = Doc::new(numbered(4, 3), snapshots(2));
    std::mem::swap(&mut harness.state_mut().doc, &mut other);
    harness.run();
    assert!(harness.query_by_role(Role::TextInput).is_none(), "the second document shows no field");
    key(&mut harness, Key::Enter);
    assert!(harness.state().events.iter().all(|e| !matches!(e, HistoryEvent::RenameSnapshot { .. })), "{:?}", harness.state().events);
}

// Showing another document reveals its current row, also when its state saw this revision before.
#[test]
fn another_document_reveals_its_current_row() {
    let mut harness = build(Host::states(30, 29), [260.0, 220.0], false);
    let center = list(&harness).rect().center();
    harness.wheel_at(center, vec2(0.0, 1e5));
    harness.run_steps(20);
    assert!(shown(&harness, "State 0") && !shown(&harness, "State 29"), "the user scrolled the first document to the top");

    let mut other = Doc::new(numbered(30, 29), SnapshotList::new());
    std::mem::swap(&mut harness.state_mut().doc, &mut other);
    harness.run_steps(5);
    assert!(shown(&harness, "State 29"), "the second document shows its current row");
    harness.wheel_at(center, vec2(0.0, 1e5));
    harness.run_steps(20);
    assert!(!shown(&harness, "State 29"));

    std::mem::swap(&mut harness.state_mut().doc, &mut other);
    harness.run_steps(5);
    assert!(shown(&harness, "State 29"), "back in the first document the current row is revealed again");
}

// The trash is enabled only by an accepted drag: a later state with non-linear history, current at the oldest.
#[test]
fn a_non_linear_drop_of_a_later_state_deletes_it_while_the_oldest_is_current() {
    let trash = button_label(|labels| &labels.delete_button);
    let mut steps = numbered(6, 0);
    steps.set_non_linear(true);
    let mut harness = harness(Host::new(steps, SnapshotList::new()));
    assert!(harness.get_by_label(&trash).accesskit_node().is_disabled());
    let target = id(&harness, 4);
    let (from, to) = (row(&harness, "State 4").rect().center(), harness.get_by_label(&trash).rect().center());
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(5);
    assert_eq!(harness.state().events, [HistoryEvent::DeleteStep(target)]);
}

// A drop acts on the grabbed row, and a payload from another list is refused.
#[test]
fn the_trash_refuses_a_drag_from_another_list() {
    let trash = button_label(|labels| &labels.delete_button);
    let mut harness = harness(Host::states(6, 0));
    let outside = list(&harness).rect().center() + vec2(0.0, 120.0);
    let on_trash = harness.get_by_label(&trash).rect().center();
    pointer(&mut harness, outside, Some(true));
    let payload = ListDrag {
        list: Id::unique("another list"),
        items: vec![HistoryKey::Step(id(&harness, 2))],
        grabbed: HistoryKey::Step(id(&harness, 2)),
        label: "State 2".to_owned(),
    };
    DragAndDrop::set_payload(&harness.ctx, payload);
    pointer(&mut harness, on_trash, None);
    assert!(harness.get_by_label(&trash).accesskit_node().is_disabled(), "a foreign payload does not enable the trash");
    pointer(&mut harness, on_trash, Some(false));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
    assert!(harness.state().hints.is_empty(), "{:?}", harness.state().hints);
}

// A row History cannot delete starts no drag: no payload and no ghost.
#[test]
fn a_row_that_cannot_be_deleted_starts_no_drag() {
    let mut harness = harness(Host::states(6, 3));
    let from = row(&harness, "State 0").rect().center();
    pointer(&mut harness, from, Some(true));
    pointer(&mut harness, from + vec2(0.0, 12.0), None);
    pointer(&mut harness, from + vec2(0.0, 40.0), None);
    assert!(!DragAndDrop::has_payload_of_type::<ListDrag<HistoryKey>>(&harness.ctx));
}

// The footer is a toolbar named by `HistoryLabels::footer`.
#[test]
fn the_footer_is_a_named_toolbar() {
    let harness = harness(Host::states(3, 1));
    let toolbar = harness.get_by_role(Role::Toolbar);
    assert_eq!(toolbar.accesskit_node().label().as_deref(), Some(HistoryLabels::english().footer.as_str()));
}

/// A source that records each key it is asked about.
struct Recorder {
    asked: RefCell<Vec<(SnapshotId, ThumbRequest)>>,
    inner: FakeThumbSource<SnapshotId>,
}

impl ThumbSource<SnapshotId> for Recorder {
    fn revision(&self, key: &SnapshotId, request: &ThumbRequest) -> Revision {
        self.asked.borrow_mut().push((*key, *request));
        self.inner.revision(key, request)
    }

    fn render(&self, key: &SnapshotId, request: &ThumbRequest) -> ThumbRender {
        self.inner.render(key, request)
    }
}

// Each visible snapshot row asks the app's source for a picture of the snapshot thumbnail side; state rows ask for none.
#[test]
fn snapshot_rows_request_their_thumbnails() {
    let source = Recorder { asked: RefCell::new(Vec::new()), inner: FakeThumbSource::new() };
    let host = {
        let mut host = Host::states(4, 3);
        host.doc.snapshots = snapshots(2);
        host
    };
    let side = RefCell::new(0.0_f32);
    let mut harness = Harness::builder().with_size([260.0, 420.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let thumbs = Thumbnails::<SnapshotId>::in_context(ui.ctx(), "history-test.thumbs", ThumbnailOptions::default());
            *side.borrow_mut() = HistoryPanel::style(&crate::theme::Tokens::of(ui)).snapshot_thumb;
            HistoryPanel::new("history", &host.doc.steps, &mut host.doc.panel)
                .snapshots(&host.doc.snapshots)
                .thumbnails(&thumbs, &source)
                .show(ui)
                .events
                .len();
        },
        host,
    );
    harness.run();
    drop(harness);
    let ids: Vec<SnapshotId> = {
        let list = snapshots(2);
        list.iter().map(|snap| snap.id).collect()
    };
    let asked = source.asked.into_inner();
    let expected = ThumbRequest::for_cell(side.into_inner(), 1.0).side;
    for id in ids {
        let request = asked.iter().find(|(key, _)| *key == id).map(|(_, request)| request.side);
        assert_eq!(request, Some(expected), "snapshot {id:?} is asked for at the thumbnail side");
    }
    assert!(asked.iter().all(|(key, _)| key.get() <= 2), "{asked:?}");
}
