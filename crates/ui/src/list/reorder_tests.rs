//! Reorder in a list: drags with the marker's target, the payload and the ghost's copy cursor,
//! every way a drag ends, a drop on another widget, edge scrolling, `Into` a group, and the
//! stacking keys.

use egui::accesskit::Role;
use egui::{CursorIcon, DragAndDrop, Event, Key, Modifiers, PointerButton, Pos2, Sense, Ui, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, Node};

use super::drag::{show_copy, show_flat, show_refused, show_tree};
use super::*;
use crate::behavior::{Axis, DropPosition, ListDrag, Move, Reorder, RowInfo, Selection};
use crate::controls::NoArgs;
use crate::edit::{CancelReason, EditEvent};
use crate::testing::a11y;
use crate::theme::{Base, install};

const LIST_HEIGHT: f32 = 200.0;

type Reordered = EditEvent<Move<usize>>;

/// How a drag ended: the move dropped, or why it was cancelled.
type Ending = Result<Move<usize>, CancelReason>;

/// A host whose rows move with the stacking keys; a drop zone under the list takes dragged rows.
struct Host {
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
    refuse: Option<usize>,
    copy: bool,
    tree: bool,
    reorders: Vec<Reordered>,
    /// The drop targets a drag showed, each change once.
    targets: Vec<Option<Move<usize>>>,
    opens: Vec<OpenChange<usize>>,
    dropped: Option<Vec<usize>>,
}

impl Host {
    fn new(count: usize) -> Self {
        Self::with_rows((0..count).map(RowInfo::leaf).collect())
    }

    fn with_rows(rows: Vec<RowInfo<usize>>) -> Self {
        Host {
            rows,
            selection: Selection::only(0),
            refuse: None,
            copy: false,
            tree: false,
            reorders: Vec::new(),
            targets: Vec::new(),
            opens: Vec::new(),
            dropped: None,
        }
    }

    fn take(&mut self) -> Vec<Reordered> {
        std::mem::take(&mut self.reorders)
    }

    fn take_targets(&mut self) -> Vec<Option<Move<usize>>> {
        std::mem::take(&mut self.targets)
    }
}

/// Moves the rows of a flat list.
fn apply(rows: &mut Vec<RowInfo<usize>>, mv: &Move<usize>) {
    let moved: Vec<RowInfo<usize>> = mv.items.iter().map(|&k| RowInfo::leaf(k)).collect();
    rows.retain(|row| !mv.items.contains(&row.key));
    let Some(target) = rows.iter().position(|row| row.key == mv.target) else { return };
    let at = if mv.position == DropPosition::After { target + 1 } else { target };
    rows.splice(at..at, moved);
}

fn show(ui: &mut Ui, host: &mut Host) {
    let Host { rows, selection, refuse, copy, tree, reorders, targets, opens, dropped } = host;
    let refuse = *refuse;
    let reorder = Reorder::new(Axis::Vertical).copy_with_alt(*copy).accepts(move |mv: &Move<usize>, _| Some(mv.target) != refuse);
    let out = List::new("list", "Layers", rows.as_slice())
        .selection(selection)
        .max_height(LIST_HEIGHT)
        .reorder(reorder)
        .tree(*tree)
        .show(ui, |_, &key| ListRow::new(key, format!("Row {key}")));
    let dragging = DragAndDrop::payload::<ListDrag<usize>>(ui.ctx()).is_some();
    if (dragging || out.drop_target.is_some()) && targets.last() != Some(&out.drop_target) {
        targets.push(out.drop_target.clone());
    }
    for event in out.events {
        match event {
            ListEvent::Reorder(event) => {
                if let EditEvent::Set { value: mv, .. } = &event {
                    apply(rows, mv);
                }
                reorders.push(event);
            }
            ListEvent::Open(change) => opens.push(change),
            _ => {}
        }
    }
    let (_, zone) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    if let Some(drag) = zone.dnd_release_payload::<ListDrag<usize>>() {
        *dropped = Some(drag.items.clone());
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn list<'h>(harness: &'h Harness<'_, Host>) -> Node<'h> {
    harness.get_by_role(Role::ListBox)
}

fn row<'h>(harness: &'h Harness<'_, Host>, key: usize) -> Node<'h> {
    let label = format!("Row {key}");
    harness.get_by(move |n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label.as_str()))
}

/// A point in the row `key`, `share` of the way down.
fn at(harness: &Harness<'_, Host>, key: usize, share: f32) -> Pos2 {
    let rect = row(harness, key).rect();
    Pos2::new(rect.center().x, rect.top() + rect.height() * share)
}

fn button(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    harness.run();
}

/// Presses on the row `key` and moves past egui's drag threshold.
fn grab(harness: &mut Harness<'_, Host>, key: usize) {
    let start = at(harness, key, 0.5);
    button(harness, start, true);
    harness.event(Event::PointerMoved(start + vec2(0.0, 8.0)));
    harness.run();
}

fn hover(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
}

fn hover_row(harness: &mut Harness<'_, Host>, key: usize, share: f32) {
    let pos = at(harness, key, share);
    hover(harness, pos);
}

fn focus_window(harness: &mut Harness<'_, Host>, focused: bool) {
    harness.input_mut().focused = focused;
    harness.event(Event::WindowFocused(focused));
    harness.run();
}

fn payload(harness: &Harness<'_, Host>) -> Option<ListDrag<usize>> {
    DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).map(|p| (*p).clone())
}

fn mv(items: &[usize], target: usize, position: DropPosition) -> Option<Move<usize>> {
    Some(Move { items: items.to_vec(), target, position, copy: false })
}

/// The drop targets the drag showed since the last call (without the empty start), and how it
/// ended.
fn summary(harness: &mut Harness<'_, Host>) -> (Vec<Option<Move<usize>>>, Option<Ending>) {
    let mut targets = harness.state_mut().take_targets();
    if targets.first() == Some(&None) {
        targets.remove(0);
    }
    let end = harness.state_mut().take().iter().find_map(|e| match e {
        EditEvent::Commit { value: mv, .. } => Some(Ok(mv.clone())),
        EditEvent::Cancel { reason, .. } => Some(Err(*reason)),
        _ => None,
    });
    (targets, end)
}

#[test]
fn a_drag_moves_the_selection_and_reports_its_targets() {
    let mut host = Host::new(10);
    host.selection = Selection { items: vec![2, 1], lead: Some(2), anchor: Some(1) };
    let mut harness = harness(host);
    grab(&mut harness, 1);
    assert!(matches!(harness.state_mut().take()[..], [EditEvent::Begin { .. }]));
    let drag = payload(&harness).expect("the payload");
    assert_eq!((drag.items, drag.label.as_str()), (vec![1, 2], "Row 1"), "the selection, in display order, and the grabbed label");

    hover_row(&mut harness, 2, 0.75);
    hover_row(&mut harness, 5, 0.75);
    hover_row(&mut harness, 6, 0.25);
    let pos = at(&harness, 6, 0.25);
    button(&mut harness, pos, false);
    let (updates, end) = summary(&mut harness);
    assert_eq!(updates, [mv(&[1, 2], 5, DropPosition::After), mv(&[1, 2], 6, DropPosition::Before)], "no target over the dragged rows");
    assert_eq!(end, Some(Ok(mv(&[1, 2], 6, DropPosition::Before).unwrap())));
    assert_eq!(harness.state().selection.items, [2, 1], "a drag doesn't change the selection");
    assert!(payload(&harness).is_none(), "a drop on the list takes the payload");
    assert_eq!(harness.state().dropped, None);

    harness.run_steps(30);
    grab(&mut harness, 7);
    assert_eq!(payload(&harness).map(|d| d.items), Some(vec![7]), "a row outside the selection drags alone");
    let pos = at(&harness, 7, 0.5);
    button(&mut harness, pos, false);

    let mut harness = self::harness(Host::new(4));
    grab(&mut harness, 1);
    let below = row(&harness, 3).rect().center_bottom() + vec2(0.0, 30.0);
    assert!(list(&harness).rect().contains(below + vec2(0.0, 40.0)), "empty space below the rows");
    hover(&mut harness, below);
    button(&mut harness, below, false);
    let (updates, _) = summary(&mut harness);
    assert_eq!(updates.last(), Some(&mv(&[1], 3, DropPosition::After)), "the space below the last row drops after it");
}

#[test]
fn a_drag_ends_on_escape_focus_loss_refusal_removal_a_drop_elsewhere_and_a_cleared_payload() {
    let mut host = Host::new(10);
    host.refuse = Some(5);
    let mut harness = harness(host);
    focus_window(&mut harness, true);
    grab(&mut harness, 1);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Escape)));
    assert!(payload(&harness).is_none());
    let pos = at(&harness, 1, 0.5);
    button(&mut harness, pos, false);
    assert!(harness.state_mut().take().is_empty(), "the rest of a cancelled drag does nothing");

    harness.run_steps(30);
    grab(&mut harness, 1);
    focus_window(&mut harness, false);
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::FocusLost)));
    focus_window(&mut harness, true);
    let pos = at(&harness, 1, 0.5);
    button(&mut harness, pos, false);

    harness.run_steps(30);
    grab(&mut harness, 1);
    hover_row(&mut harness, 3, 0.75);
    hover_row(&mut harness, 5, 0.75);
    let pos = at(&harness, 5, 0.75);
    button(&mut harness, pos, false);
    let (updates, end) = summary(&mut harness);
    assert_eq!(updates, [mv(&[1], 3, DropPosition::After), None], "a refused target has no marker");
    assert_eq!(end, Some(Err(CancelReason::Dismissed)));

    harness.run_steps(30);
    grab(&mut harness, 1);
    harness.state_mut().rows.retain(|row| row.key != 1);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Removed)), "the dragged row left the list");
    let pos = at(&harness, 2, 0.5);
    button(&mut harness, pos, false);

    harness.run_steps(30);
    grab(&mut harness, 2);
    let zone = list(&harness).rect().center_bottom() + vec2(0.0, 20.0);
    hover(&mut harness, zone);
    button(&mut harness, zone, false);
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Dismissed)));
    assert_eq!(harness.state().dropped, Some(vec![2]), "another widget takes the rows from the payload");

    harness.run_steps(30);
    grab(&mut harness, 3);
    egui::DragAndDrop::clear_payload(&harness.ctx);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Dismissed)), "the app cleared the payload: not an Escape");
    let pos = at(&harness, 3, 0.5);
    button(&mut harness, pos, false);
}

#[test]
fn alt_copies_with_the_copy_cursor_also_onto_the_dragged_rows_edges() {
    let mut host = Host::new(10);
    host.copy = true;
    let mut harness = harness(host);
    grab(&mut harness, 3);
    hover_row(&mut harness, 3, 0.75);
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    assert_eq!(harness.output().platform_output.cursor_icon, CursorIcon::Copy);
    let pos = at(&harness, 3, 0.75);
    button(&mut harness, pos, false);
    let copy = Move { items: vec![3], target: 3, position: DropPosition::After, copy: true };
    let (updates, end) = summary(&mut harness);
    assert_eq!(updates, [Some(copy.clone())], "no target on its own edge without Alt");
    assert_eq!(end, Some(Ok(copy)));
}

#[test]
fn a_drag_near_the_bottom_edge_scrolls_and_keeps_the_grabbed_row() {
    let mut harness = harness(Host::new(1000));
    grab(&mut harness, 0);
    let edge = list(&harness).rect().center_bottom() - vec2(0.0, 2.0);
    harness.event(Event::PointerMoved(edge));
    harness.run_steps(20);
    let list_rect = list(&harness).rect();
    assert!(!list_rect.contains_rect(row(&harness, 0).rect()), "scrolled past the grabbed row");
    assert!(harness.query_by_label("Row 0").is_some(), "the grabbed row stays drawn");
    button(&mut harness, edge, false);
    let Some(Ok(drop)) = summary(&mut harness).1 else { panic!("a drop") };
    assert!(drop.target > 10, "dropped among the rows scrolled to: {drop:?}");
}

#[test]
fn a_group_row_takes_drops_into_its_middle() {
    let rows = vec![RowInfo::leaf(0), RowInfo::group(1, true), RowInfo::leaf(2).at_depth(1), RowInfo::leaf(3).at_depth(1), RowInfo::leaf(4)];
    let mut harness = harness(Host::with_rows(rows));
    grab(&mut harness, 4);
    for share in [0.5, 0.2, 0.8] {
        hover_row(&mut harness, 1, share);
    }
    hover_row(&mut harness, 1, 0.5);
    let pos = at(&harness, 1, 0.5);
    button(&mut harness, pos, false);
    let (updates, end) = summary(&mut harness);
    let (into, before, after) = (mv(&[4], 1, DropPosition::Into), mv(&[4], 1, DropPosition::Before), mv(&[4], 1, DropPosition::After));
    assert_eq!(updates, [into.clone(), before, after, into.clone()]);
    assert_eq!(end, into.map(Ok));
}

#[test]
fn stacking_keys_move_the_selection_and_reveal_the_lead() {
    let mut harness = harness(Host::new(30));
    harness.state_mut().selection = Selection::only(2);
    harness.key_press(Key::Tab);
    harness.run();
    let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
    /// Presses a key that moves rows, which the host moves.
    fn press(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) -> Move<usize> {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        let events = harness.state_mut().take();
        let [EditEvent::Set { value: mv, .. }] = &events[..] else { panic!("one move: {events:?}") };
        mv.clone()
    }
    assert_eq!(press(&mut harness, ctrl, Key::OpenBracket), mv(&[2], 3, DropPosition::After).unwrap(), "Ctrl+[ one row toward the end");
    assert_eq!(press(&mut harness, ctrl | Modifiers::SHIFT, Key::OpenBracket), mv(&[2], 29, DropPosition::After).unwrap());
    assert!(list(&harness).rect().expand(0.5).contains_rect(row(&harness, 2).rect()), "the lead is revealed at its new place");
    assert_eq!(press(&mut harness, ctrl, Key::CloseBracket), mv(&[2], 29, DropPosition::Before).unwrap(), "Ctrl+] one row toward the start");
    assert_eq!(harness.state().selection, Selection::only(2), "the selection stays");
}

#[test]
fn tree_keys_and_stacking_keys_work_together_in_a_tree() {
    let rows = vec![RowInfo::group(0, true), RowInfo::leaf(1).at_depth(1), RowInfo::leaf(2).at_depth(1), RowInfo::leaf(3)];
    let mut host = Host::with_rows(rows);
    host.tree = true;
    let mut harness = harness(host);
    harness.key_press(Key::Tab);
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    let close = OpenChange { row: 0, open: false, all: false };
    assert_eq!(std::mem::take(&mut harness.state_mut().opens), [close], "← closes the group");
    let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
    harness.key_press_modifiers(ctrl, Key::OpenBracket);
    harness.run();
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [EditEvent::Set { value: moved, .. }] if *moved == mv(&[0], 3, DropPosition::After).unwrap()), "{events:?}");
    assert!(harness.state().opens.is_empty());
}

#[test]
fn the_reorder_stories_name_every_control() {
    for show in [show_flat, show_tree, show_refused, show_copy] {
        let mut args = NoArgs {};
        let harness = Harness::builder()
            .with_size([400.0, 420.0])
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| show(ui, &mut args));
        assert_eq!(a11y::unlabelled(&harness), Vec::<String>::new());
    }
}
