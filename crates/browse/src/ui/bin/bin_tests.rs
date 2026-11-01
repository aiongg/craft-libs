//! `MediaBin` behaviour through egui_kittest (tasks B11, B12).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::Selection;
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::icons::Icon;
use craft_ui::list::TableColumn;
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, Modifiers, PointerButton, Pos2, Ui, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{BinEntry, BinEvent, BinFooterAction, BinItem, BinModel, BinState, BinView, MediaBin, OpenHow};

/// Cmd on a Mac and Ctrl elsewhere, as a platform reports it: `ctrl` with the abstract `command`.
const CMD: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// A bin tree of `(key, name, parent, is a bin)`.
struct Project {
    nodes: Vec<(u32, &'static str, Option<u32>, bool)>,
}

impl Project {
    fn new() -> Self {
        Self {
            nodes: vec![
                (1, "Footage", None, true),
                (2, "Archive", None, true),
                (10, "Beach.mov", Some(1), false),
                (11, "Clip.mov", Some(1), false),
                (20, "Old beach.mov", Some(2), false),
                (30, "Title.psd", None, false),
            ],
        }
    }
}

impl BinModel<u32> for Project {
    fn children(&self, parent: Option<&u32>, out: &mut Vec<BinEntry<u32>>) {
        for &(key, _, p, bin) in &self.nodes {
            if p == parent.copied() {
                out.push(if bin { BinEntry::Bin(key) } else { BinEntry::Item(key) });
            }
        }
    }

    fn item(&self, key: &u32, item: &mut BinItem) {
        if let Some(node) = self.nodes.iter().find(|n| n.0 == *key) {
            item.name = node.1.to_owned();
        }
    }

    fn cell(&self, key: &u32, column: &str) -> String {
        format!("{column} {key}")
    }

    fn is_bin(&self, key: &u32) -> bool {
        self.nodes.iter().any(|n| n.0 == *key && n.3)
    }

    fn parent(&self, key: &u32) -> Option<u32> {
        self.nodes.iter().find(|n| n.0 == *key).and_then(|n| n.2)
    }

    fn count(&self, bin: &u32) -> usize {
        self.nodes.iter().filter(|n| n.2 == Some(*bin)).map(|n| if n.3 { self.count(&n.0) } else { 1 }).sum()
    }
}

struct Host {
    project: Project,
    state: BinState<u32>,
    selection: Selection<u32>,
    columns: Vec<TableColumn<String>>,
    actions: Vec<BinFooterAction>,
    events: Vec<BinEvent<u32>>,
}

impl Host {
    fn new() -> Self {
        let mut state = BinState::default();
        state.expanded.insert(1);
        Self {
            project: Project::new(),
            state,
            selection: Selection::new(),
            columns: vec![TableColumn::new("name".to_owned(), "Name"), TableColumn::new("duration".to_owned(), "Duration")],
            actions: vec![BinFooterAction::new("newItem", Icon::Plus, "New Item").accepts_drops(true)],
            events: Vec::new(),
        }
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let Host { project, state, selection, columns, actions, events } = host;
    let out = MediaBin::new("bin", state, &*project).selection(selection).columns(columns).footer_actions(actions).show(ui);
    for event in &out.events {
        state.apply(event);
    }
    events.extend(out.events);
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([640.0, 480.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn row<'h>(harness: &'h Harness<'_, Host>, name: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::TreeItem && n.label().as_deref() == Some(name))
}

/// A point on the row's name column, clear of the indent and the triangle.
fn on_row(harness: &Harness<'_, Host>, name: &str) -> Pos2 {
    let rect = row(harness, name).rect();
    pos2(rect.left() + 120.0, rect.center().y)
}

fn play(harness: &mut Harness<'_, Host>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
}

fn rows(harness: &Harness<'_, Host>) -> Vec<String> {
    let mut names: Vec<_> = harness.query_all_by_role(Role::TreeItem).filter_map(|n| n.accesskit_node().label()).collect();
    names.sort();
    names
}

// A search lists matching items of sub-bins, flat; clearing it shows the bin again (B11).
#[test]
fn a_search_shows_matches_from_sub_bins() {
    let mut harness = harness(Host::new());
    assert_eq!(rows(&harness), ["Archive", "Beach.mov", "Clip.mov", "Footage", "Title.psd"]);
    harness.state_mut().state.search = "BEACH".to_owned();
    harness.run();
    assert_eq!(rows(&harness), ["Beach.mov", "Old beach.mov"], "matches from every sub-bin, flat, bins left out");
    harness.state_mut().state.search.clear();
    harness.run();
    assert_eq!(rows(&harness), ["Archive", "Beach.mov", "Clip.mov", "Footage", "Title.psd"]);
}

// A click on the selected item's name, then nothing for the double-click time: the rename editor opens (B11).
#[test]
fn a_click_on_the_selected_name_renames_after_the_double_click_time() {
    let mut harness = harness(Host::new());
    harness.click_at(on_row(&harness, "Clip.mov"));
    harness.run_steps(10);
    assert_eq!(harness.state().selection.items, [11]);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "the first click only selects");
    let name = row(&harness, "Clip.mov").rect();
    harness.click_at(pos2(name.left() + 70.0, name.center().y));
    harness.run_steps(2);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "the editor waits out the double-click time");
    harness.run_steps(12);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the editor opened");
    harness.type_text("Sunset.mov");
    harness.key_press(Key::Enter);
    harness.run_steps(4);
    assert!(harness.state().events.contains(&BinEvent::Rename { key: 11, name: "Sunset.mov".to_owned() }));
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

// Double click plain, with Shift, with Cmd: OpenBin InPlace, NewTab, NewWindow (B11).
#[test]
fn a_double_click_on_a_bin_reports_how_to_open_it() {
    for (modifiers, how) in [(Modifiers::NONE, OpenHow::InPlace), (Modifiers::SHIFT, OpenHow::NewTab), (CMD, OpenHow::NewWindow)] {
        let mut harness = harness(Host::new());
        let at = on_row(&harness, "Archive");
        play(&mut harness, events::click(at, PointerButton::Primary, 2, modifiers));
        harness.run_steps(4);
        let opened: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, BinEvent::OpenBin { .. })).cloned().collect();
        assert_eq!(opened, [BinEvent::OpenBin { key: 2, how }], "{modifiers:?}");
    }
}

// A drag released over a bin row reports Drop into it; over the background, into None (B11).
#[test]
fn dropping_on_a_bin_row_reports_the_target() {
    let mut harness = harness(Host::new());
    let (from, bin) = (on_row(&harness, "Beach.mov"), on_row(&harness, "Archive"));
    harness.drag(from, bin, Modifiers::NONE);
    harness.run_steps(4);
    let drops: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, BinEvent::Drop { .. })).cloned().collect();
    assert_eq!(drops, [BinEvent::Drop { items: vec![10], into: Some(2) }]);

    harness.state_mut().events.clear();
    let (from, empty) = (on_row(&harness, "Clip.mov"), pos2(200.0, 380.0));
    harness.drag(from, empty, Modifiers::NONE);
    harness.run_steps(4);
    let drops: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, BinEvent::Drop { .. })).cloned().collect();
    assert_eq!(drops, [BinEvent::Drop { items: vec![11], into: None }]);
}

// A drag released over a footer action that accepts drops reports FooterDrop with the items (B11).
#[test]
fn a_drop_on_a_footer_action_reports_it() {
    let mut harness = harness(Host::new());
    let from = on_row(&harness, "Beach.mov");
    let to = harness.get_by_label("New Item").rect().center();
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(4);
    let drops: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, BinEvent::FooterDrop { .. })).cloned().collect();
    assert_eq!(drops, [BinEvent::FooterDrop { action: "newItem", items: vec![10] }]);
    assert!(harness.state().events.iter().all(|e| !matches!(e, BinEvent::Drop { .. })));
}

// FilmCraft #580: Esc during the zoom slider drag gives Cancel and the size it started from (B11).
#[test]
fn escape_during_the_zoom_drag_restores_the_size() {
    let mut harness = harness(Host::new());
    harness.state_mut().state.settings.view = BinView::Icon;
    harness.run();
    let before = harness.state().state.settings.icon_size;
    let slider = harness.get_by_role_and_label(Role::Slider, "Thumbnail Size").rect();
    let (start, away) = (pos2(slider.left() + 4.0, slider.center().y), pos2(slider.right() - 4.0, slider.center().y));
    play(
        &mut harness,
        vec![
            vec![egui::Event::PointerMoved(start)],
            vec![egui::Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }],
            vec![egui::Event::PointerMoved(away)],
        ],
    );
    harness.run_steps(2);
    assert_ne!(harness.state().state.settings.icon_size, before, "the drag resizes");
    play(&mut harness, events::key_press(Key::Escape, Modifiers::NONE));
    play(
        &mut harness,
        vec![vec![egui::Event::PointerButton { pos: away, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]],
    );
    harness.run_steps(4);
    let cancelled = harness.state().events.iter().any(|e| matches!(e, BinEvent::Zoom(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert!(cancelled, "{:?}", harness.state().events);
    assert_eq!(harness.state().state.settings.icon_size, before);
}

// A Freeform drag moves the selection and the dragged card's stack; Esc puts them back; release commits once (B12).
#[test]
#[ignore = "skeleton"]
fn freeform_drag_moves_the_stack_and_escape_puts_it_back() {
    todo!()
}

// Icon view: hover scrub reports the fraction and draws the playhead; bins are cards before items (B12).
#[test]
#[ignore = "skeleton"]
fn the_icon_view_shows_bins_first_and_scrubs_items() {
    todo!()
}
