//! The browser engine on a kittest harness: clicks, keys through a grid, drag to reorder, 10,000
//! presets and AccessKit.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::view::State;
use super::*;
use crate::behavior::{DropPosition, Move};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

type Key32 = u32;

struct Host {
    rows: Vec<PresetRow<Key32>>,
    state: PresetState<Key32>,
    selected: Option<Key32>,
    events: Vec<PresetEvent<Key32>>,
    /// The thumbnails painted in the last frame.
    painted: usize,
    lead: Option<Key32>,
    time: f64,
}

impl Host {
    fn new(rows: Vec<PresetRow<Key32>>) -> Self {
        Host { rows, state: PresetState::new(), selected: None, events: Vec::new(), painted: 0, lead: None, time: 0.0 }
    }

    fn take(&mut self) -> Vec<PresetEvent<Key32>> {
        std::mem::take(&mut self.events)
    }
}

/// Group 100 with presets 1 to 7, group 200 with presets 8 and 9.
fn library() -> Vec<PresetRow<Key32>> {
    let mut rows = vec![PresetRow::group(100, "Round", 0)];
    rows.extend((1..=7).map(|n| PresetRow::item(n, format!("Brush {n}"), 1)));
    rows.push(PresetRow::group(200, "Square", 0));
    rows.extend((8..=9).map(|n| PresetRow::item(n, format!("Brush {n}"), 1)));
    rows
}

fn show(ui: &mut Ui, host: &mut Host) {
    let Host { rows, state, selected, events, painted, lead, .. } = host;
    let id = ui.make_persistent_id(egui::IdSalt::new("presets"));
    *painted = 0;
    let out = PresetBrowser::new("presets", "Presets", state, rows).selected(selected.as_ref()).reorder(PresetReorder::items_and_groups()).show(
        ui,
        |painter, rect, _| {
            *painted += 1;
            painter.rect_filled(rect, 0.0, egui::Color32::RED);
        },
    );
    events.extend(out.events);
    *lead = State::<Key32>::load(ui.ctx(), id).lead;
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([300.0, 300.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn cell<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let label = label.to_owned();
    harness.get_by(move |n| matches!(n.role(), Role::TreeItem | Role::ListBoxOption) && n.label().as_deref() == Some(label.as_str()))
}

fn center(harness: &Harness<'_, Host>, label: &str) -> Pos2 {
    cell(harness, label).rect().center()
}

/// Runs one frame a second after the last, with `events`: separate clicks are not double clicks.
fn frame(harness: &mut Harness<'_, Host>, gap: f64, events: Vec<Event>) {
    harness.state_mut().time += gap;
    let time = harness.state().time;
    harness.input_mut().time = Some(time);
    harness.input_mut().events.extend(events);
    harness.run_steps(1);
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// A press and release in one frame.
fn click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    frame(harness, 1.0, vec![Event::PointerMoved(pos), button(pos, true, Modifiers::NONE), button(pos, false, Modifiers::NONE)]);
    frame(harness, 0.016, vec![]);
}

fn double_click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    click(harness, pos);
    frame(harness, 0.1, vec![button(pos, true, Modifiers::NONE), button(pos, false, Modifiers::NONE)]);
    frame(harness, 0.016, vec![]);
}

fn press_key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    frame(harness, 0.5, vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    frame(harness, 0.016, vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
}

fn drag(harness: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    frame(harness, 1.0, vec![Event::PointerMoved(from), button(from, true, Modifiers::NONE)]);
    frame(harness, 0.016, vec![Event::PointerMoved(from + vec2(8.0, 0.0))]);
    frame(harness, 0.016, vec![Event::PointerMoved(to)]);
    frame(harness, 0.016, vec![Event::PointerMoved(to), button(to, false, Modifiers::NONE)]);
    frame(harness, 0.016, vec![]);
}

fn reorders(events: &[PresetEvent<Key32>]) -> Vec<&EditEvent<Move<Key32>>> {
    events.iter().filter_map(|e| if let PresetEvent::Reorder(r) = e { Some(r) } else { None }).collect()
}

#[test]
fn a_click_selects_and_a_double_click_activates() {
    let mut harness = harness(Host::new(library()));
    let pos = center(&harness, "Brush 3");
    click(&mut harness, pos);
    assert!(matches!(harness.state_mut().take()[..], [PresetEvent::Select { key: 3, .. }]), "one Select");
    assert_eq!(harness.state().lead, Some(3), "the click moves the lead");

    double_click(&mut harness, pos);
    let events = harness.state_mut().take();
    assert!(events.iter().any(|e| matches!(e, PresetEvent::Activate { key: 3, .. })), "{events:?}");
}

#[test]
fn arrows_move_the_lead_through_the_grid_and_across_a_header_without_selecting() {
    let mut harness = harness(Host::new(library()));
    let pos = center(&harness, "Brush 1");
    click(&mut harness, pos);
    harness.state_mut().take();
    let columns = (1..=7).take_while(|n| cell(&harness, &format!("Brush {n}")).rect().top() == cell(&harness, "Brush 1").rect().top()).count() as u32;
    assert!((2..7).contains(&columns), "the library wraps: {columns} columns");

    press_key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().lead, Some(2));
    press_key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state().lead, Some(2 + columns), "the same column of the next line");
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().lead, Some(2 + columns - 2));
    // From the last line of the first run, Down goes to the next header and on to its first cell.
    press_key(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(harness.state().lead, Some(9));
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().lead, Some(200), "Left from the first cell of a run reaches the header above it");
    press_key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(harness.state().lead, Some(6), "Up from a header lands in the same column of the last line above");
    assert!(harness.state_mut().take().is_empty(), "keys moved the lead and selected nothing");

    press_key(&mut harness, Modifiers::NONE, Key::Space);
    assert!(matches!(harness.state_mut().take()[..], [PresetEvent::Select { key: 6, .. }]), "Space selects the lead");
}

#[test]
fn dragging_a_preset_within_its_run_is_one_gesture_with_the_move() {
    let mut harness = harness(Host::new(library()));
    let target = cell(&harness, "Brush 3").rect();
    let to = Pos2::new(target.right() - 4.0, target.center().y);
    let from = center(&harness, "Brush 1");
    drag(&mut harness, from, to);
    let events = harness.state_mut().take();
    let gesture = reorders(&events);
    assert!(matches!(gesture[0], EditEvent::Begin { .. }), "{events:?}");
    let commits: Vec<_> = gesture.iter().filter_map(|e| if let EditEvent::Commit { value, .. } = e { Some(value) } else { None }).collect();
    assert_eq!(commits, [&Move { items: vec![1], target: 3, position: DropPosition::After, copy: false }]);
    assert_eq!(gesture.len(), 2, "Begin and Commit only");
    assert!(!events.iter().any(|e| matches!(e, PresetEvent::Select { .. })), "a drag does not select");
}

#[test]
fn dropping_a_preset_on_a_header_moves_it_into_the_group() {
    let mut harness = harness(Host::new(library()));
    let header = cell(&harness, "Square").rect();
    let from = center(&harness, "Brush 1");
    drag(&mut harness, from, header.center());
    let events = harness.state_mut().take();
    let moves: Vec<_> =
        reorders(&events).into_iter().filter_map(|e| if let EditEvent::Commit { value, .. } = e { Some(value) } else { None }).collect();
    assert_eq!(moves, [&Move { items: vec![1], target: 200, position: DropPosition::Into, copy: false }]);
}

#[test]
fn ten_thousand_presets_paint_only_the_cells_in_view() {
    let rows = (0..10_000).map(|n| PresetRow::item(n, format!("Preset {n}"), 0)).collect();
    let harness = harness(Host::new(rows));
    let painted = harness.state().painted;
    assert!((1..120).contains(&painted), "painted {painted} thumbnails");
    let list = harness.get_by_role(Role::ListBox);
    assert_eq!(list.accesskit_node().size_of_set(), Some(10_000));
    let nodes = harness.query_all_by_role(Role::ListBoxOption).count();
    assert!(nodes < 120, "{nodes} nodes in the tree");
}

#[test]
fn the_tree_names_groups_and_presets_with_their_state() {
    let mut host = Host::new(library());
    host.selected = Some(2);
    host.rows[8] = PresetRow::group(200, "Square", 0).count(2);
    let mut harness = harness(host);
    let tree = harness.get_by_role(Role::Tree);
    assert_eq!(tree.accesskit_node().size_of_set(), Some(9), "the presets shown");
    let header = cell(&harness, "Round");
    assert_eq!((header.accesskit_node().role(), header.accesskit_node().data().is_expanded()), (Role::TreeItem, Some(true)));
    let square = cell(&harness, "Square");
    assert_eq!(square.accesskit_node().value().as_deref(), Some("2 presets"), "a header's value is its count");
    let preset = cell(&harness, "Brush 2");
    assert_eq!(preset.accesskit_node().is_selected(), Some(true));
    assert_eq!(preset.accesskit_node().level(), Some(1));
    assert_eq!((preset.accesskit_node().position_in_set(), preset.accesskit_node().size_of_set()), (Some(1), Some(7)));

    // Closing the first group takes its presets out of the tree and out of the layout.
    let at = center(&harness, "Round");
    click(&mut harness, at);
    assert!(harness.query_by(|n| n.label().as_deref() == Some("Brush 2")).is_none());
    assert_eq!(harness.state().state.toggled.len(), 1);
}

#[test]
fn list_view_has_one_row_per_preset_and_its_arrows_follow_the_tree() {
    let mut host = Host::new(library());
    host.state.view = PresetView::List;
    let mut harness = harness(host);
    let (a, b) = (cell(&harness, "Brush 1").rect(), cell(&harness, "Brush 2").rect());
    assert!(b.top() > a.top() && (a.left() - b.left()).abs() < 1.0, "rows stack: {a:?} {b:?}");
    let pos = center(&harness, "Brush 2");
    click(&mut harness, pos);
    press_key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state().lead, Some(3));
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().lead, Some(100), "Left from a preset goes to its group");
    press_key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert!(harness.state().state.toggled.contains(&100), "Left on an open group closes it");
}
