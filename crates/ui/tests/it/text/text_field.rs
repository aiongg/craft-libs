//! `TextField`: a click on a completion candidate picks it without moving focus, and the search form's clear part
//! and Escape empty the text.

use craft_behavior::{EditEvent, Mixed, PartKey};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::primitives::{Completer, Completion};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::text::run_node;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Id, Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

const CLEAR: PartKey = PartKey::new("clear");

struct Prefixes;

impl Completer for Prefixes {
    fn complete(&self, text: &str, caret: usize, out: &mut Vec<Completion>) -> Option<Range<usize>> {
        let before = text.get(..caret)?;
        let hits: Vec<_> = ["alpha", "alps", "beta"].into_iter().filter(|word| !before.is_empty() && word.starts_with(before)).collect();
        out.extend(hits.iter().map(|word| Completion::new(*word, *word)));
        (!hits.is_empty()).then_some(0..caret)
    }
}

/// A click on a candidate inserts it, and the field keeps key focus.
#[trial]
fn a_click_on_a_candidate_inserts_it_and_the_field_keeps_focus() {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = text.clone();
    let mut fixture = Fixture::new(move |ui| {
        TextField::new("Name", Bind::bound(&mut *shown.borrow_mut())).complete(Prefixes).show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("al");
    fixture.idle(4);
    let focused = fixture.focused();
    assert!(focused.is_some(), "Tab focused the field");
    let option = fixture.by_label("alps").expect("the list shows the candidate");
    fixture.click(&option);
    fixture.idle(3);
    assert_eq!(*text.borrow(), "alps", "the candidate replaced the typed prefix");
    assert_eq!(fixture.focused(), focused, "the press on the row left focus in the field");
}

/// The clear part empties a search field.
#[trial]
fn the_clear_part_empties_a_search_field() {
    let text = Rc::new(RefCell::new("poster".to_owned()));
    let id = Rc::new(Cell::new(Id::NULL));
    let (shown, field) = (text.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        field.set(TextField::search("Search", Bind::bound(&mut *shown.borrow_mut())).show(ui).response.id);
    });
    fixture.settle();
    let clear = fixture.part(id.get(), CLEAR).expect("a search field with text shows the clear part");
    fixture.click(&clear);
    fixture.idle(3);
    assert_eq!(*text.borrow(), "", "the clear part emptied the text");
    assert!(fixture.part(id.get(), CLEAR).is_none(), "an empty search field shows no clear part");
}

/// Clearing a search field that has an open typing run empties it for good: the clear part goes, and leaving the
/// field commits nothing back.
#[trial]
fn clearing_a_search_field_with_a_draft_leaves_it_empty_after_blur() {
    let (text, other) = (Rc::new(RefCell::new(String::new())), Rc::new(RefCell::new(String::new())));
    let id = Rc::new(Cell::new(Id::NULL));
    let (shown, beside, field) = (text.clone(), other.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        field.set(TextField::search("Search", Bind::bound(&mut *shown.borrow_mut())).show(ui).response.id);
        TextField::new("Other", Bind::bound(&mut *beside.borrow_mut())).id_salt("other").show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("abc");
    fixture.idle(2);
    assert_eq!(*text.borrow(), "abc");
    let clear = fixture.part(id.get(), CLEAR).expect("a search field with text shows the clear part");
    fixture.click(&clear);
    fixture.idle(3);
    assert_eq!(*text.borrow(), "");
    assert!(fixture.part(id.get(), CLEAR).is_none(), "an empty search field shows no clear part");
    assert!(fixture.texts().iter().all(|run| !run.text.contains("abc")), "the field shows the cleared text");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(*text.borrow(), "", "leaving the field commits nothing back");
}

/// Escape in a search field that shows committed text clears and commits the empty text and stays with the field;
/// the next Escape, on the empty field, goes on.
#[trial]
fn escape_in_a_search_field_clears_the_committed_text_then_passes_on() {
    let text = Rc::new(RefCell::new("abc".to_owned()));
    let escaped = Rc::new(Cell::new(false));
    let (shown, took) = (text.clone(), escaped.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = TextField::search("Search", Bind::bound(&mut *shown.borrow_mut())).show(ui);
        took.set(took.get() || out.escaped);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    escaped.set(false);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*text.borrow(), "", "Escape cleared and committed the empty text");
    assert!(escaped.get(), "the field took the Escape");
    escaped.set(false);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert!(!escaped.get(), "Escape on the empty field goes on to the host");
}

/// Typing into a mixed field and pressing Enter commits the typed text over the mixed value, as one edit.
#[trial]
fn typing_into_a_mixed_field_commits_over_the_mixed_value() {
    let value = Rc::new(RefCell::new(Mixed::<String>::Mixed));
    let sets = Rc::new(Cell::new(0));
    let (shown, counted) = (value.clone(), sets.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = TextField::new("Name", Bind::bound_mixed(&mut *shown.borrow_mut())).show(ui);
        counted.set(counted.get() + usize::from(matches!(out.event, Some(EditEvent::Set { .. } | EditEvent::Commit { .. }))));
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("abc");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(*value.borrow(), Mixed::Same("abc".to_owned()), "the typed text replaced the mixed value");
    assert_eq!(sets.get(), 1, "the commit reached the app as one edit");
}

/// A field that asks for selection on focus, shown in response to an action and given the focus by a request, holds
/// the focus with all its text selected on the next pass.
#[trial]
fn a_select_on_focus_field_takes_a_focus_request_with_all_its_text_selected() {
    let shown = Rc::new(Cell::new(false));
    let seen = Rc::new(Cell::new(None));
    let (show, id_seen) = (shown.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        if show.get() {
            let mut name = String::from("Preset 1");
            id_seen.set(Some(TextField::new("Name", Bind::bound(&mut name)).select_on_focus().show(ui).response.id));
        }
    });
    fixture.settle();
    shown.set(true);
    fixture.idle(2);
    let id = seen.get().expect("the field showed");
    request(fixture.ctx(), FocusTarget::Id(id));
    fixture.idle(2);
    let run = NodeKey::of(run_node(id, 0, 0));
    assert_eq!(fixture.tree().text_selection(id), Some(((run, 0), (run, 8))), "the field's whole text is selected");
}
