//! A section row and an item row of a collection are nodes the screen reader reaches: the section is a heading named
//! by its text that Click, Enter and Expand/Collapse open and close, and Focus and Click on an item row move the
//! lead and pick it.

use super::kit::{List, Row, Words};
use craft_ui::contract::{CollectionEvent, Props as _, Role, SelectMode, Show as _, ShowCollection as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn region() -> Rect {
    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(300.0, 300.0))
}

/// The keys of the events a pass reported, as text, in order.
type Log = Rc<RefCell<Vec<String>>>;

fn rig(words: &'static Words, select: SelectMode) -> (Fixture, Log) {
    let log = Log::default();
    let seen = log.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = in_region(ui, region(), |ui| List::new(select).id_salt("list").tree().show_items(ui, words, &mut |_, _| Row::new()));
        for event in &out.events {
            match event {
                CollectionEvent::Activate(key) if words.sections.contains(key) => {
                    let mut open = words.open.borrow_mut();
                    open[*key] = !open[*key];
                    seen.borrow_mut().push(format!("activate {key}"));
                }
                CollectionEvent::Open(change) => {
                    words.open.borrow_mut()[change.key] = change.open;
                    seen.borrow_mut().push(format!("open {} {}", change.key, change.open));
                }
                CollectionEvent::LeadMoved { key, .. } => seen.borrow_mut().push(format!("lead {key}")),
                CollectionEvent::Selection(change) => seen.borrow_mut().push(format!("select {change:?}")),
                _ => {}
            }
        }
    });
    fixture.settle();
    (fixture, log)
}

fn sections() -> &'static Words {
    let mut words = Words::flat(&["File", "New", "Open", "Edit", "Undo"]);
    words.tree = vec![(0, true), (1, false), (1, false), (0, true), (1, false)];
    words.sections = vec![0, 3];
    words.open = RefCell::new(vec![true; 5]);
    Box::leak(Box::new(words))
}

fn headings(fixture: &Fixture) -> Vec<(String, Target)> {
    fixture.by_role(Role::HEADING).into_iter().map(|target| (fixture.tree().name_of(target.node).unwrap_or_default().to_owned(), target)).collect()
}

fn items(fixture: &Fixture) -> usize {
    fixture.by_role(Role::LIST_ITEM).len()
}

/// Each section header is a node named by its label; Click and Enter on it close and open the section.
#[trial]
fn a_section_header_is_a_named_node_that_click_and_enter_toggle() {
    let (mut fixture, _) = rig(sections(), SelectMode::Single);
    let names: Vec<String> = headings(&fixture).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["File", "Edit"], "each section is a heading named by its text");
    assert_eq!(items(&fixture), 3, "the open sections show their rows");
    let file = headings(&fixture).remove(0).1;
    fixture.access(AccessActionKind::Click, &file).settle();
    assert_eq!(items(&fixture), 1, "Click on the header closes its section");
    let file = headings(&fixture).remove(0).1;
    fixture.access(AccessActionKind::Focus, &file).settle();
    fixture.key(Key::Enter, Modifiers::NONE).settle();
    assert_eq!(items(&fixture), 3, "Enter on the header opens it again");
}

/// Expand and Collapse on the header open and close the section.
#[trial]
fn expand_and_collapse_on_a_section_header_open_and_close_it() {
    let (mut fixture, log) = rig(sections(), SelectMode::Single);
    let edit = headings(&fixture).remove(1).1;
    fixture.access(AccessActionKind::Collapse, &edit).settle();
    assert!(log.borrow().iter().any(|line| line == "open 3 false"), "Collapse reports the close: {:?}", log.borrow());
    assert_eq!(items(&fixture), 2, "the section's row is hidden");
    let edit = headings(&fixture).remove(1).1;
    fixture.access(AccessActionKind::Expand, &edit).settle();
    assert_eq!(items(&fixture), 3);
}

/// AccessKit Focus on an item row moves the lead to it; Click picks it.
#[trial]
fn access_focus_and_click_on_an_item_row_move_the_lead_and_pick() {
    let words: &'static Words = Box::leak(Box::new(Words::flat(&["one", "two", "three"])));
    let (mut fixture, log) = rig(words, SelectMode::Single);
    let rows = fixture.by_role(Role::LIST_ITEM);
    fixture.access(AccessActionKind::Focus, &rows[2]).settle();
    assert_eq!(log.borrow().last().map(String::as_str), Some("lead 2"), "Focus moves the lead: {:?}", log.borrow());
    assert!(!log.borrow().iter().any(|line| line.starts_with("select")), "Focus alone selects nothing");
    fixture.access(AccessActionKind::Click, &rows[1]).settle();
    assert!(log.borrow().iter().any(|line| line == "lead 1"), "Click moves the lead: {:?}", log.borrow());
    assert!(log.borrow().iter().any(|line| line.starts_with("select") && line.contains('1')), "Click picks the row: {:?}", log.borrow());
}
