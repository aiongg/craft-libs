//! The Check Spelling dialog: what each button asks the app to do and how the dialog ends.

use craft_text::ui::spelling::{CheckSpellingDialog, SpellingAction, SpellingItem};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    host: Rc<RefCell<DialogHost>>,
    actions: Rc<RefCell<Vec<SpellingAction>>>,
    results: Rc<RefCell<usize>>,
}

fn item(word: &str, suggestions: &[&str]) -> SpellingItem {
    SpellingItem::new(1, 0..word.len(), word, suggestions.iter().map(|s| (*s).to_owned()).collect())
}

fn items() -> Vec<SpellingItem> {
    vec![item("teh", &["the", "tea"]), item("recieve", &["receive", "relieve"]), item("wrold", &["world", "wold"])]
}

/// The word and replacement of a Change.
fn changed(action: &SpellingAction) -> Option<(&str, &str)> {
    match action {
        SpellingAction::Change { item, to, .. } => Some((item.word.as_str(), to.as_str())),
        _ => None,
    }
}

impl Rig {
    fn new(items: Vec<SpellingItem>) -> Rig {
        let mut host = DialogHost::new();
        host.open(CheckSpellingDialog::new(items));
        let host = Rc::new(RefCell::new(host));
        let (actions, results) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(0)));
        let (shared, sink, done) = (host.clone(), actions.clone(), results.clone());
        let mut fixture = Fixture::new(move |ui| {
            shared.borrow_mut().run(ui);
            if let Some(dialog) = shared.borrow_mut().get_mut::<CheckSpellingDialog>()
                && let Some(action) = dialog.take_action()
            {
                sink.borrow_mut().push(action);
            }
            if let Some(DialogResult::Ok(())) = shared.borrow_mut().take::<CheckSpellingDialog>() {
                *done.borrow_mut() += 1;
            }
        });
        fixture.settle();
        Rig { fixture, host, actions, results }
    }

    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.click(&target);
    }
}

/// Ignore moves to the next misspelling without an action; Ignore All asks to list again and remembers the word.
#[trial]
fn ignore_moves_on_and_ignore_all_relists() {
    let mut rig = Rig::new(items());
    rig.click("Ignore");
    assert!(rig.actions.borrow().is_empty());
    rig.click("Change");
    assert_eq!(rig.actions.borrow().last().and_then(changed), Some(("recieve", "receive")));

    rig.actions.borrow_mut().clear();
    rig.click("Ignore All");
    assert_eq!(*rig.actions.borrow(), [SpellingAction::Relist]);
    assert_eq!(rig.host.borrow().get::<CheckSpellingDialog>().map(|dialog| dialog.ignored().to_vec()), Some(vec!["recieve".to_owned()]));
}

/// Change uses the Change To text; Enter on a suggestion changes with that suggestion.
#[trial]
fn change_uses_change_to_and_enter_on_a_suggestion_changes() {
    let mut rig = Rig::new(items());
    rig.click("Change");
    assert_eq!(rig.actions.borrow().last().and_then(changed), Some(("teh", "the")));

    rig.actions.borrow_mut().clear();
    rig.click("tea");
    assert!(rig.actions.borrow().is_empty(), "a click only fills Change To");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.actions.borrow().last().and_then(changed), Some(("teh", "tea")));
}

/// An empty list says the check is complete and disables every button but Done; Done and Escape end the dialog.
#[trial]
fn empty_list_shows_complete_and_done_closes() {
    let mut rig = Rig::new(Vec::new());
    assert!(rig.fixture.by_label("Spell check complete.").is_some());
    rig.click("Ignore");
    rig.click("Change");
    assert!(rig.actions.borrow().is_empty(), "the buttons are disabled");
    rig.click("Done");
    assert_eq!(*rig.results.borrow(), 1);

    let mut rig = Rig::new(Vec::new());
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(*rig.results.borrow(), 1);
}

/// A double click on a suggestion fills Change To and changes, as Enter on it does.
#[trial]
fn double_click_on_a_suggestion_changes() {
    let mut rig = Rig::new(items());
    rig.click("tea");
    rig.click("tea");
    assert_eq!(rig.actions.borrow().last().and_then(changed), Some(("teh", "tea")));
}
