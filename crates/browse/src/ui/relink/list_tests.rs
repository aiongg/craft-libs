//! `LinkList` through egui_kittest.

use std::collections::HashMap;

use craft_docpath::links::{Link, LinkStatus};
use craft_ui::behavior::{Selection, SelectionPick};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Modifiers, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{LinkFilter, LinkList, LinkListEvent, LinkListState};

struct App {
    links: Vec<Link<u32>>,
    statuses: HashMap<u32, LinkStatus>,
    state: LinkListState<u32>,
    settings_changed: usize,
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
    App { links, statuses, state: LinkListState::default(), settings_changed: 0, events: Vec::new() }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness =
        Harness::<App>::builder().with_size(vec2(360.0, 320.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, app: &mut App| {
                let App { links, statuses, state, settings_changed, events } = app;
                let out = LinkList::new("links", links, statuses, state).show(ui);
                *settings_changed += usize::from(out.settings_changed);
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
    harness.state_mut().state.selection = selection;
    harness.run();
}

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !harness.get_by_label(label).accesskit_node().is_disabled()
}

// The list follows the library's selection rule: Shift ranges from the anchor, Cmd (also with Shift) toggles, and each change is reported with its pick.
#[test]
fn link_list_follows_the_library_selection_rule() {
    let mut harness = harness(app());
    row(&harness, "b.png").click();
    harness.run();
    row(&harness, "d.png").click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(harness.state().state.selection.items, [1, 2, 3]);
    assert_eq!(harness.state().state.selection.anchor, Some(1));

    row(&harness, "a.png").click_modifiers(Modifiers { shift: true, command: true, ..Modifiers::NONE });
    harness.run();
    assert_eq!(harness.state().state.selection.items, [1, 2, 3, 0], "Cmd wins over Shift: the row is added");

    row(&harness, "c.png").click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(harness.state().state.selection.items, [1, 3, 0], "Cmd-click on a selected row removes it");

    let picks: Vec<SelectionPick> = harness
        .state()
        .events
        .iter()
        .filter_map(|event| if let LinkListEvent::Selection(change) = event { Some(change.pick) } else { None })
        .collect();
    assert_eq!(picks, [SelectionPick::Replace, SelectionPick::Range, SelectionPick::Toggle, SelectionPick::Toggle]);

    // Filtered to Modified, the anchor and lead rows are hidden: a Shift-click selects the clicked row alone.
    harness.state_mut().state.selection = Selection { items: vec![0], lead: Some(0), anchor: Some(0) };
    harness.state_mut().state.settings.filter = LinkFilter::Modified;
    harness.run();
    row(&harness, "b.png").click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(harness.state().state.selection.items, [1]);
}

// A secondary click on an unselected row selects it first and reports the change.
#[test]
fn a_secondary_click_on_an_unselected_row_selects_it() {
    let mut harness = harness(app());
    select(&mut harness, &[0]);
    harness.state_mut().events.clear();
    row(&harness, "c.png").click_secondary();
    harness.run();
    assert_eq!(harness.state().state.selection.items, [2]);
    assert!(harness.state().events.iter().any(|e| matches!(e, LinkListEvent::Selection(change) if change.row == Some(2))));
}

// Choosing Show Missing in the menu changes the settings, reports `settings_changed` once and hides the other rows.
#[test]
fn a_menu_choice_is_a_settings_change() {
    let mut harness = harness(app());
    assert_eq!(harness.state().settings_changed, 0);
    harness.get_by_label("Links Menu").click();
    harness.run_steps(8);
    harness.get_by_label("Show Missing").click();
    harness.run_steps(8);
    assert_eq!(harness.state().state.settings.filter, LinkFilter::Missing);
    assert_eq!(harness.state().settings_changed, 1);
    assert_eq!(harness.query_all_by(|n| n.role() == Role::ListBoxOption).count(), 1, "only the missing link is listed");
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
