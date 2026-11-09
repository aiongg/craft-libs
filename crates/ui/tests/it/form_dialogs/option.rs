//! The option dialogs' own logic: password strength and secrecy, the report text and the list manager's action rules.

use craft_theme::variant::Tone;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    ListManager, ListManagerAction, ListManagerChoice, ListManagerEntry, ListManagerWords, ReportDialog, ReportNote, Secret,
};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

/// Pressing Run with an entry's name in the box answers with that entry and the trimmed name.
#[trial]
fn run_answers_with_the_entry_the_name_box_names() {
    let choice: Rc<RefCell<Option<ListManagerChoice>>> = Rc::new(RefCell::new(None));
    let held = choice.clone();
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<ListManagerWords>();
        let entries = vec![ListManagerEntry::new("a", "Align Layers"), ListManagerEntry::new("b", "Batch Export")];
        let actions = vec![ListManagerAction::run(&words), ListManagerAction::create(&words)];
        let mut open = true;
        let shown = ListManager::new("Macros", "Macro name:", entries, actions).name("  batch export ").opened(Bind::bound(&mut open)).show(ui);
        if shown.choice.is_some() {
            *held.borrow_mut() = shown.choice;
        }
    });
    fixture.settle();
    let run = fixture.by_label("Run").expect("the Run button shows");
    fixture.click(&run);
    fixture.idle(2);
    let answer = choice.borrow().clone().expect("Run answered");
    assert_eq!((answer.action.as_str(), answer.entry.as_deref(), answer.name.as_str()), ("run", Some("b"), "batch export"));
}
