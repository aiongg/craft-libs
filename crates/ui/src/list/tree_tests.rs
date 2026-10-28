//! Tree tests: the tree keys, the disclosure triangle, the lead moving to a group that closes
//! over it, AccessKit, the indent squeeze and `TreeState`.

use egui::accesskit::{Action, Role};
use egui::{Event, Key, Modifiers, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::tree::{NODES, display, is_group, show_tree};
use super::*;
use crate::behavior::Selection;
use crate::direction::{self, Direction};
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

/// A host whose open state is a `TreeState`, and the events its list reported.
struct Host {
    open: TreeState,
    selection: Selection<usize>,
    events: Vec<ListEvent<usize>>,
}

fn show(ui: &mut Ui, host: &mut Host) {
    let rows = display(NODES, &host.open);
    let out = List::new("tree", "Layers", rows.as_slice())
        .tree(true)
        .selection(&mut host.selection)
        .max_height(240.0)
        .show(ui, |_, &key| ListRow::new(key, NODES[key].0));
    for event in out.events {
        if let ListEvent::Open(change) = &event {
            let groups: Vec<usize> = (0..NODES.len()).filter(|&at| is_group(NODES, at)).collect();
            host.open.apply(change, groups.iter());
        }
        host.events.push(event);
    }
}

fn harness(open: &[usize], selected: usize) -> Harness<'static, Host> {
    harness_in(open, selected, Direction::Ltr)
}

fn harness_in(open: &[usize], selected: usize, dir: Direction) -> Harness<'static, Host> {
    let mut state = TreeState::load(&egui::Context::default(), "tree-tests");
    for key in open {
        state.set(key, true);
    }
    let host = Host { open: state, selection: Selection::only(selected), events: Vec::new() };
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show, host);
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    harness
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn events(harness: &mut Harness<'_, Host>) -> Vec<ListEvent<usize>> {
    std::mem::take(&mut harness.state_mut().events)
}

fn item<'h>(harness: &'h Harness<'_, Host>, name: &str) -> Node<'h> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::TreeItem && n.label().as_deref() == Some(name.as_str()))
}

fn button<'h>(harness: &'h Harness<'_, Host>, name: &str) -> Node<'h> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(name.as_str()))
}

fn open_event(row: usize, open: bool, all: bool) -> ListEvent<usize> {
    ListEvent::Open(OpenChange { row, open, all })
}

#[test]
fn the_tree_keys_open_enter_leave_and_close_groups() {
    let mut harness = harness(&[0], 0);
    harness.state_mut().events.clear();
    // Poster is open, Background a leaf, Photos closed.
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().selection, Selection::only(1), "→ on an open group enters its first child");
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().selection, Selection::only(1), "→ on a leaf does nothing");
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    events(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(events(&mut harness), [open_event(2, true, false)], "→ on a closed group opens it");
    assert!(harness.state().open.is_open(&2));
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(events(&mut harness), [open_event(2, false, false)], "← on an open group closes it");
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().selection, Selection::only(0), "← on a closed row goes to its parent");
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(events(&mut harness).last(), Some(&open_event(0, false, false)));
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert!(events(&mut harness).is_empty(), "← on a closed top-level row does nothing");
}

#[test]
fn alt_arrows_open_or_close_every_group() {
    let mut harness = harness(&[], 0);
    harness.state_mut().events.clear();
    key(&mut harness, Modifiers::ALT, Key::ArrowRight);
    assert_eq!(events(&mut harness), [open_event(0, true, true)]);
    assert!((0..NODES.len()).filter(|&at| is_group(NODES, at)).all(|at| harness.state().open.is_open(&at)));
    key(&mut harness, Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(events(&mut harness), [open_event(0, false, true)]);
    assert!(!harness.state().open.is_open(&0) && !harness.state().open.is_open(&2));
}

#[test]
fn closing_a_group_moves_the_lead_to_it_and_keeps_the_selection() {
    // Select two rows inside Frames (5), whose children are 6 and 7; lead on 7.
    let mut harness = harness(&[0, 2, 5], 6);
    harness.state_mut().selection = Selection { items: vec![6, 7], lead: Some(7), anchor: Some(6) };
    harness.run();
    harness.state_mut().events.clear();
    harness.get_by_label("Collapse Frames").click();
    harness.run();
    let events = events(&mut harness);
    assert_eq!(events, [ListEvent::LeadMoved(5), open_event(5, false, false)]);
    assert_eq!((harness.state().selection.lead, &harness.state().selection.items[..]), (Some(5), &[6, 7][..]));
}

#[test]
fn a_click_on_the_triangle_opens_without_selecting_and_alt_asks_for_all() {
    let mut harness = harness(&[], 1);
    harness.run();
    button(&harness, "Expand Poster").click_modifiers(Modifiers::ALT);
    harness.run();
    assert_eq!(events(&mut harness), [open_event(0, true, true)]);
    assert_eq!(harness.state().selection, Selection::only(1), "the click doesn't select the row");
}

/// Sends the screen reader's `action` to `node`.
fn request(harness: &Harness<'_, Host>, node: &Node<'_>, action: Action) {
    let (target_node, target_tree) = node.accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest { action, target_tree, target_node, data: None }));
}

#[test]
fn accesskit_reports_a_tree_with_levels_and_expansion_and_takes_expand_and_collapse() {
    let mut harness = harness(&[0, 2], 0);
    assert_eq!(harness.get_by_role(Role::Tree).accesskit_node().label().as_deref(), Some("Layers"));
    let levels: Vec<_> = harness
        .query_all_by_role(Role::TreeItem)
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.accesskit_node().level(), n.accesskit_node().data().is_expanded()))
        .collect();
    assert_eq!(levels[0], ("Poster".to_owned(), Some(0), Some(true)));
    assert_eq!(levels[1], ("Background".to_owned(), Some(1), None));
    assert_eq!(levels[2], ("Photos".to_owned(), Some(1), Some(true)));
    assert_eq!(levels[3].1, Some(2));
    // Each item's set is its siblings: Poster, Guides and Notes at the top; Background, Photos and
    // Text in Poster; three rows in Photos. The tree itself has no set size.
    let sets: Vec<_> =
        harness.query_all_by_role(Role::TreeItem).map(|n| (n.accesskit_node().position_in_set(), n.accesskit_node().size_of_set())).collect();
    let set = |position| (Some(position), Some(3));
    assert_eq!(sets, [set(0), set(0), set(1), set(0), set(1), set(2), set(2), set(1), set(2)]);
    assert_eq!(harness.get_by_role(Role::Tree).accesskit_node().size_of_set(), None);
    let frames = item(&harness, "Frames");
    assert_eq!(frames.accesskit_node().data().is_expanded(), Some(false));
    assert!(frames.accesskit_node().data().supports_action(Action::Expand));
    assert!(item(&harness, "Photos").accesskit_node().data().supports_action(Action::Collapse));
    harness.state_mut().events.clear();
    request(&harness, &item(&harness, "Frames"), Action::Expand);
    harness.run();
    assert_eq!(events(&mut harness), [open_event(5, true, false)]);
    request(&harness, &item(&harness, "Photos"), Action::Collapse);
    harness.run();
    assert_eq!(events(&mut harness), [open_event(2, false, false)]);
}

#[test]
fn the_indent_squeezes_but_the_triangle_and_the_label_keep_their_room() {
    let width = 160.0;
    let mut harness = Harness::builder().with_size([width, 60.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|ui| {
        let _ = TreeRow::new(ListRow::new(0usize, "Deep group"), 8, true, true).show(ui);
    });
    harness.run();
    let style = {
        let mut style = None;
        harness.ctx.run_ui(egui::RawInput::default(), |ui| style = Some(ListRow::<usize>::style(&Tokens::of(ui)))).drop_without_applying_deltas();
        style.unwrap()
    };
    let triangle = harness.get_by_label("Collapse Deep group").rect();
    let row = harness.get_by_role(Role::TreeItem).rect();
    assert!(triangle.left() >= row.left() + style.pad_x - 0.5, "the triangle stays inside the row: {triangle:?}");
    assert!(
        triangle.right() <= row.right() - style.trailing_pad - style.min_label + 0.5,
        "the label keeps min_label to the right of the triangle: {triangle:?} in {row:?}"
    );
}

#[test]
fn tree_state_applies_one_group_or_all() {
    let ctx = egui::Context::default();
    let mut state = TreeState::load(&ctx, "state");
    state.apply(&OpenChange { row: "a", open: true, all: false }, ["a", "b"].iter());
    assert_eq!((state.is_open(&"a"), state.is_open(&"b")), (true, false));
    state.apply(&OpenChange { row: "a", open: true, all: true }, ["a", "b"].iter());
    assert!(state.is_open(&"b"));
    state.store(&ctx);
    assert!(TreeState::load(&ctx, "state").is_open(&"b"), "the open groups are kept in egui memory");
    state.apply(&OpenChange { row: "b", open: false, all: true }, ["a", "b"].iter());
    assert!(!state.is_open(&"a") && !state.is_open(&"b"));
}

#[test]
fn the_tree_story_names_every_control() {
    let mut args = Args::new().with_number("width", 320.0, 120.0, 480.0).with_number("height", 280.0, 120.0, 480.0);
    let harness = Harness::builder()
        .with_size([400.0, 320.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(move |ui| show_tree(ui, &mut args));
    assert_eq!(a11y::unlabelled(&harness), Vec::<String>::new());
    assert_eq!(harness.get_by_role(Role::Tree).accesskit_node().size_of_set(), None, "a tree's items carry their sets");
}

#[test]
fn right_to_left_swaps_the_arrow_keys() {
    let mut harness = harness_in(&[0], 0, Direction::Rtl);
    harness.state_mut().events.clear();
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().selection, Selection::only(1), "← enters the first child");
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    events(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(events(&mut harness), [open_event(2, true, false)], "← opens a closed group");
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(events(&mut harness), [open_event(2, false, false)], "→ closes an open group");
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().selection, Selection::only(0), "→ goes to the parent");
    key(&mut harness, Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(events(&mut harness).last(), Some(&open_event(0, true, true)), "Alt+← opens every group");
    key(&mut harness, Modifiers::ALT, Key::ArrowRight);
    assert_eq!(events(&mut harness).last(), Some(&open_event(0, false, true)), "Alt+→ closes every group");
}

#[test]
fn the_triangle_has_a_tooltip_with_the_alt_hint() {
    let mut harness = harness(&[], 0);
    button(&harness, "Expand Poster").hover();
    harness.run_steps(30);
    assert!(harness.query_by_label("Expand  (Alt-click: all groups)").is_some(), "tooltip shows the verb and the hint");
}

#[test]
fn names_and_tooltips_follow_the_hosts_patterns() {
    let text = TreeText {
        expand: "{row} aufklappen".to_owned(),
        collapse: "{row} zuklappen".to_owned(),
        expand_tip: "{row} aufklappen (Alt-Klick: alle Gruppen)".to_owned(),
        collapse_tip: "{row} zuklappen".to_owned(),
    };
    let mut harness =
        Harness::builder().with_size([320.0, 60.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(move |ui| {
            let row = ListRow::new(0usize, "Poster").toggle(ToggleColumn::eye("Sichtbarkeit von {row}", true));
            let _ = TreeRow::new(row, 0, true, false).text(text.clone()).show(ui);
        });
    harness.run();
    assert!(harness.query_by_label("Sichtbarkeit von Poster").is_some(), "the toggle column's name");
    harness.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Poster aufklappen")).hover();
    harness.run_steps(30);
    assert!(harness.query_by_label("Poster aufklappen (Alt-Klick: alle Gruppen)").is_some(), "the triangle's tooltip");
}
