//! The Icon and Freeform views' keys, marquee, drops and rename through egui_kittest (FX6-31).

use craft_ui::behavior::{ListDrag, Selection};
use craft_ui::list::TableColumn;
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{DragAndDrop, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::freeform::{Dir, neighbour};
use super::{BinEntry, BinEvent, BinItem, BinModel, BinState, BinView, FreeformCard, MediaBin};

const CMD: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };
const CMD_SHIFT: Modifiers = Modifiers { alt: false, ctrl: true, shift: true, mac_cmd: false, command: true };

/// `(key, name, parent, is a bin)`: a bin with one item, two items at the root.
const NODES: [(u32, &str, Option<u32>, bool); 4] =
    [(1, "Footage", None, true), (10, "Beach.mov", Some(1), false), (30, "Title.psd", None, false), (31, "Cover.psd", None, false)];

struct Project;

impl BinModel<u32> for Project {
    fn children(&self, parent: Option<&u32>, out: &mut Vec<BinEntry<u32>>) {
        for &(key, _, p, bin) in &NODES {
            if p == parent.copied() {
                out.push(if bin { BinEntry::Bin(key) } else { BinEntry::Item(key) });
            }
        }
    }

    fn item(&self, key: &u32, item: &mut BinItem) {
        if let Some(node) = NODES.iter().find(|n| n.0 == *key) {
            item.name = node.1.to_owned();
        }
    }

    fn cell(&self, _: &u32, _: &str) -> String {
        String::new()
    }

    fn is_bin(&self, key: &u32) -> bool {
        NODES.iter().any(|n| n.0 == *key && n.3)
    }

    fn parent(&self, key: &u32) -> Option<u32> {
        NODES.iter().find(|n| n.0 == *key).and_then(|n| n.2)
    }

    fn count(&self, bin: &u32) -> usize {
        NODES.iter().filter(|n| n.2 == Some(*bin)).count()
    }
}

struct Host {
    state: BinState<u32>,
    selection: Selection<u32>,
    freeform: Vec<FreeformCard<u32>>,
    events: Vec<BinEvent<u32>>,
    foreign: Option<ListDrag<u32>>,
    rename: Option<u32>,
    columns: Vec<TableColumn<String>>,
}

fn host(view: BinView) -> Host {
    let mut state = BinState::default();
    state.settings.view = view;
    let at = |key, x, y| FreeformCard { key, pos: vec2(x, y), stack: None };
    Host {
        state,
        selection: Selection::new(),
        freeform: vec![at(1, 20.0, 20.0), at(30, 200.0, 20.0), at(31, 380.0, 20.0)],
        events: Vec::new(),
        foreign: None,
        rename: None,
        columns: vec![TableColumn::new("name".to_owned(), "Name")],
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    if let Some(drag) = host.foreign.take() {
        DragAndDrop::set_payload(ui.ctx(), drag);
    }
    let mut bin = MediaBin::new("bin", &mut host.state, &Project).selection(&mut host.selection).columns(&host.columns).freeform(&host.freeform);
    if let Some(key) = host.rename.take() {
        bin = bin.start_rename(key);
    }
    let out = bin.show(ui);
    for event in &out.events {
        host.state.apply(event);
    }
    host.events.extend(out.events);
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([700.0, 480.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn card(harness: &Harness<'_, Host>, name: &str) -> Rect {
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(name)).rect()
}

fn play(harness: &mut Harness<'_, Host>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
}

fn moves(harness: &Harness<'_, Host>) -> Vec<(Vec<u32>, Vec<Vec2>)> {
    harness
        .state()
        .events
        .iter()
        .filter_map(|e| if let BinEvent::FreeformMoved { keys, to } = e { Some((keys.clone(), to.clone())) } else { None })
        .collect()
}

fn drops(harness: &Harness<'_, Host>) -> Vec<BinEvent<u32>> {
    harness.state().events.iter().filter(|e| matches!(e, BinEvent::Drop { .. })).cloned().collect()
}

const EMPTY: Pos2 = pos2(300.0, 400.0);

// The nearest card in the key's direction, keeping to the row or column.
#[test]
fn the_arrow_keys_pick_the_nearest_card_in_their_direction() {
    let at = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(100.0, 100.0));
    let rects = [at(0.0, 0.0), at(150.0, 0.0), at(0.0, 150.0), at(150.0, 160.0)];
    assert_eq!(neighbour(&rects, 0, Dir::Right), Some(1));
    assert_eq!(neighbour(&rects, 0, Dir::Down), Some(2));
    assert_eq!(neighbour(&rects, 3, Dir::Left), Some(2));
    assert_eq!(neighbour(&rects, 3, Dir::Up), Some(1));
    assert_eq!(neighbour(&rects, 0, Dir::Left), None);
}

// The Freeform view is one list box with a single Tab stop; arrows move the lead and Cmd+arrows nudge the cards.
#[test]
fn freeform_arrows_move_the_lead_and_command_arrows_nudge_the_selection() {
    let mut harness = harness(host(BinView::Freeform));
    assert_eq!(harness.query_all_by_role(Role::ListBox).count(), 1, "one list box");
    harness.click_at(card(&harness, "Footage").center());
    harness.run_steps(2);
    assert_eq!(harness.state().selection.items, [1]);

    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(harness.state().selection.items, [30], "the lead moved to the card on its right");

    harness.state_mut().events.clear();
    play(&mut harness, events::key_press(Key::ArrowRight, CMD));
    harness.run_steps(2);
    assert_eq!(moves(&harness), [(vec![30], vec![vec2(201.0, 20.0)])]);

    harness.state_mut().events.clear();
    play(&mut harness, events::key_press(Key::ArrowDown, CMD_SHIFT));
    harness.run_steps(2);
    assert_eq!(moves(&harness), [(vec![30], vec![vec2(200.0, 30.0)])]);
}

// A drag from the background selects the cards it touches; Esc puts the selection back.
#[test]
fn a_background_drag_is_a_marquee() {
    let mut harness = harness(host(BinView::Freeform));
    let (a, b) = (card(&harness, "Footage"), card(&harness, "Title.psd"));
    let from = pos2(a.left() - 6.0, a.bottom() + 40.0);
    let to = pos2(b.center().x, b.center().y);
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(3);
    let mut picked = harness.state().selection.items.clone();
    picked.sort_unstable();
    assert_eq!(picked, [1, 30]);
}

// A drag from another list: onto the empty view it drops into the shown bin, except items already there (LC59).
#[test]
fn a_foreign_drag_released_in_freeform_drops_into_the_shown_bin_or_a_bin_card() {
    let foreign =
        |items: Vec<u32>| ListDrag { list: Id::NULL.with(egui::IdSalt::new("other-bin")), grabbed: items[0], items, label: "Other".to_owned() };
    let mut harness = harness(host(BinView::Freeform));
    harness.state_mut().foreign = Some(foreign(vec![10]));
    harness.run();
    harness.drag(pos2(5.0, 450.0), EMPTY, Modifiers::NONE);
    harness.run_steps(4);
    assert_eq!(drops(&harness), [BinEvent::Drop { items: vec![10], into: None }], "{:?}", harness.state().events);

    harness.state_mut().events.clear();
    harness.state_mut().foreign = Some(foreign(vec![30]));
    harness.run();
    harness.drag(pos2(5.0, 450.0), EMPTY, Modifiers::NONE);
    harness.run_steps(4);
    assert!(drops(&harness).is_empty(), "already in the shown bin: {:?}", harness.state().events);

    harness.state_mut().foreign = Some(foreign(vec![30]));
    harness.run();
    harness.drag(pos2(5.0, 450.0), card(&harness, "Footage").center(), Modifiers::NONE);
    harness.run_steps(4);
    assert_eq!(drops(&harness), [BinEvent::Drop { items: vec![30], into: Some(1) }], "{:?}", harness.state().events);
}

// The Icon view renames in place: F2, and `MediaBin::start_rename`; the List view opens its field too.
#[test]
fn the_icon_and_list_views_open_the_rename_field() {
    for view in [BinView::Icon, BinView::List] {
        let mut harness = harness(host(view));
        harness.state_mut().rename = Some(30);
        harness.run_steps(3);
        assert!(harness.query_by_role(Role::TextInput).is_some(), "{view:?}: start_rename opened the field");
    }

    let mut harness = harness(host(BinView::Icon));
    harness.click_at(card(&harness, "Title.psd").center());
    harness.run_steps(2);
    harness.key_press(Key::F2);
    harness.run_steps(3);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "F2 opened the field");
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.state().events.iter().all(|e| !matches!(e, BinEvent::Rename { .. })), "Esc renames nothing");
}
