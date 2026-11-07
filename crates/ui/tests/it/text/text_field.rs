//! `TextField`: a click on a completion candidate picks it without moving focus, and the search form's clear part
//! empties the text.

use craft_behavior::PartKey;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::primitives::{Completer, Completion};
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
