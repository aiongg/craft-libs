//! `LinkList` through egui_kittest (task B9).

use std::collections::HashMap;

use craft_docpath::links::{Link, LinkStatus};
use craft_ui::behavior::Selection;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Modifiers, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{LinkList, LinkListEvent, LinkListState};

struct App {
    links: Vec<Link<u32>>,
    statuses: HashMap<u32, LinkStatus>,
    selection: Selection<u32>,
    state: LinkListState,
    events: Vec<LinkListEvent<u32>>,
}

fn link(key: u32, name: &str, page: Option<u32>, embedded: bool) -> Link<u32> {
    Link { key, name: name.into(), stored: name.into(), identity: None, page, embedded }
}

fn app() -> App {
    let links = vec![
        link(0, "a.png", Some(2), false),
        link(1, "b.png", None, false),
        link(2, "c.png", None, false),
        link(3, "d.png", None, false),
        link(4, "e.png", None, true),
    ];
    let statuses =
        HashMap::from([(0, LinkStatus::Missing), (1, LinkStatus::Modified), (2, LinkStatus::Ok), (3, LinkStatus::Ok), (4, LinkStatus::Embedded)]);
    App { links, statuses, selection: Selection::new(), state: LinkListState::default(), events: Vec::new() }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness =
        Harness::<App>::builder().with_size(vec2(360.0, 320.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, app: &mut App| {
                let App { links, statuses, selection, state, events } = app;
                let out = LinkList::new("links", links, statuses).selection(selection).state(state).show(ui);
                events.extend(out.events);
            },
            app,
        );
    harness.run();
    harness
}

fn row<'h>(harness: &'h Harness<'_, App>, name: &str) -> Node<'h> {
    let prefix = name.to_owned();
    harness.get_by(move |n| n.role() == Role::ListBoxOption && n.label().is_some_and(|l| l.starts_with(&prefix)))
}

fn select(harness: &mut Harness<'_, App>, keys: &[u32]) {
    let selection = Selection { items: keys.to_vec(), lead: keys.last().copied(), anchor: keys.last().copied() };
    harness.state_mut().selection = selection;
    harness.run();
}

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !harness.get_by_label(label).accesskit_node().is_disabled()
}

// VectorCraft's `click`: Shift ranges from the last plain click; with Cmd it adds; a hidden anchor makes it a plain click.
#[test]
fn link_list_shift_click_ranges_from_the_last_plain_click() {
    let mut harness = harness(app());
    row(&harness, "b.png").click();
    harness.run();
    row(&harness, "d.png").click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(harness.state().selection.items, [1, 2, 3]);
    assert_eq!(harness.state().selection.anchor, Some(1));

    row(&harness, "a.png").click_modifiers(Modifiers { shift: true, command: true, ..Modifiers::NONE });
    harness.run();
    assert_eq!(harness.state().selection.items, [1, 2, 3, 0]);

    // Filtered to Modified, the anchor row `a.png`... is hidden: a Shift-click acts as a plain click.
    harness.state_mut().selection = Selection { items: vec![0], lead: Some(0), anchor: Some(0) };
    harness.state_mut().state.filter = super::LinkFilter::Modified;
    harness.run();
    row(&harness, "b.png").click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(harness.state().selection.items, [1]);
    assert!(harness.state().events.iter().any(|e| matches!(e, LinkListEvent::Selected)));
}

// Update only with a Modified link selected; Edit Original not for a Missing one; Relink needs a selection.
#[test]
fn link_list_buttons_follow_the_statuses_of_the_selection() {
    let mut harness = harness(app());
    assert!(!enabled(&harness, "Relink…") && !enabled(&harness, "Update Link"));

    select(&mut harness, &[0]);
    assert!(enabled(&harness, "Relink…"));
    assert!(!enabled(&harness, "Update Link"));
    assert!(!enabled(&harness, "Edit Original"));
    assert!(enabled(&harness, "Break Link"));

    select(&mut harness, &[1]);
    assert!(enabled(&harness, "Update Link"));
    assert!(enabled(&harness, "Edit Original"));

    select(&mut harness, &[4]);
    assert!(!enabled(&harness, "Break Link"));
    assert!(!enabled(&harness, "Edit Original"));

    select(&mut harness, &[1]);
    harness.get_by_label("Update Link").click();
    harness.run();
    assert_eq!(harness.state().events.last(), Some(&LinkListEvent::Update(vec![1])));
}

// Each row's accessible name has the status in words ("a.png, Page 2, Missing").
#[test]
fn a_rows_status_is_in_its_name() {
    let harness = harness(app());
    assert_eq!(row(&harness, "a.png").accesskit_node().label().as_deref(), Some("a.png, Page 2, Missing"));
    assert_eq!(row(&harness, "b.png").accesskit_node().label().as_deref(), Some("b.png, Modified"));
    assert_eq!(row(&harness, "c.png").accesskit_node().label().as_deref(), Some("c.png"));
    assert_eq!(row(&harness, "e.png").accesskit_node().label().as_deref(), Some("e.png, Embedded"));
}
