//! The find bar and the Find and Replace dialog.

use craft_text::find::{FindError, FindMatches, FindOptions, FindPattern};
use craft_text::ui::find::{FindBar, FindBarState};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A find bar over two matches, opened as Cmd+F opens it, with what it reported kept.
struct Bar {
    fixture: Fixture,
    query: Rc<RefCell<String>>,
    matches: Rc<RefCell<FindMatches>>,
    searched: Rc<Cell<bool>>,
    stepped: Rc<Cell<bool>>,
    closed: Rc<Cell<bool>>,
}

fn bar(query: &str) -> Bar {
    let query = Rc::new(RefCell::new(query.to_owned()));
    let matches = Rc::new(RefCell::new({
        let mut found = FindMatches::new(0);
        found.set_page(0, vec![0..3, 5..8]);
        found
    }));
    let (options, state) = (Rc::new(RefCell::new(FindOptions::default())), Rc::new(RefCell::new(FindBarState::default())));
    state.borrow_mut().open();
    let (searched, stepped, closed) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));
    let (shown_query, shown_matches, flags) = (query.clone(), matches.clone(), (searched.clone(), stepped.clone(), closed.clone()));
    let mut fixture = Fixture::new(move |ui| {
        let (mut query, mut options, mut matches, mut state) =
            (shown_query.borrow_mut(), options.borrow_mut(), shown_matches.borrow_mut(), state.borrow_mut());
        let out = FindBar::new(&mut query, &mut options, &mut matches).state(&mut state).show(ui);
        flags.0.set(flags.0.get() || out.search);
        flags.1.set(flags.1.get() || out.stepped);
        flags.2.set(flags.2.get() || out.closed);
    });
    fixture.settle();
    Bar { fixture, query, matches, searched, stepped, closed }
}

/// Opening the bar puts the keyboard in its field: typing sets the query and asks the app to search.
#[trial]
fn typing_in_an_opened_bar_searches() {
    let mut bar = bar("");
    bar.fixture.type_text("ab").settle();
    assert_eq!(*bar.query.borrow(), "ab");
    assert!(bar.searched.get());
}

/// Esc clears the field; a second Esc closes the bar.
#[trial]
fn escape_clears_the_query_then_closes() {
    let mut bar = bar("abc");
    bar.fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(bar.query.borrow().is_empty(), "the first Esc clears the field");
    assert!(!bar.closed.get(), "and keeps the bar");
    bar.fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(bar.closed.get());
}

