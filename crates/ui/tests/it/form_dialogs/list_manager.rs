//! `ListManager`: Delete runs the `delete` action from the list and edits text in the name box.

use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Bind, Role};
use craft_ui::patterns::form_dialogs::{ListManager, ListManagerAction, ListManagerEntry, ListManagerWords};
use craft_ui::runtime::env::Env;
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2};
use std::cell::RefCell;
use std::rc::Rc;

/// A list manager over three entries, with the actions it picked in the order they ran.
fn rig() -> (Fixture, Rc<RefCell<Vec<String>>>) {
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let sink = chosen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<ListManagerWords>();
        let actions = vec![ListManagerAction::run(&words), ListManagerAction::delete(&words)];
        let entries =
            vec![ListManagerEntry::new("a", "Align Layers"), ListManagerEntry::new("b", "Batch Export"), ListManagerEntry::new("c", "Flatten Image")];
        let mut open = true;
        let out = ListManager::new("Macros", "Macro name:", entries, actions).name("Batch Export").opened(Bind::bound(&mut open)).show(ui);
        if let Some(choice) = out.choice {
            sink.borrow_mut().push(choice.action);
        }
    });
    fixture.settle();
    (fixture, chosen)
}

/// Delete typed in the name box edits the text; Delete on a focused row of the list runs the `delete` action.
#[trial]
fn delete_runs_the_delete_action_from_the_list_and_not_from_the_name_box() {
    let (mut fixture, chosen) = rig();
    let name_box = fixture.by_label("Macro name:").expect("the name box is shown");
    fixture.click(&name_box);
    fixture.idle(2);
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    assert!(chosen.borrow().is_empty(), "Delete in the name box edits its text and runs no action");
    let row = fixture.by_label("Align Layers").expect("the row is shown");
    fixture.click(&row);
    fixture.idle(2);
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*chosen.borrow(), ["delete"]);
}

/// The tooltip a hover over the button named `label` shows, and the pointer moved away again.
fn tip_over(fixture: &mut Fixture, label: &str) -> Option<String> {
    let button = fixture.by_label(label).expect("the button is shown");
    fixture.frame(vec![moved(button.center())]);
    fixture.advance(DELAY);
    fixture.idle(3);
    let tip = fixture.by_role(Role::TOOLTIP).first().and_then(|tip| fixture.tree().name_of(tip.node)).map(str::to_owned);
    fixture.frame(vec![moved(Pos2::ZERO)]);
    fixture.idle(3);
    tip
}

/// An enabled action shows its tooltip; a disabled one shows its reason instead of the tooltip.
#[trial]
fn an_action_button_tells_its_tooltip_when_enabled_and_its_reason_when_disabled() {
    let mut fixture = Fixture::new(|ui| {
        let words = Env::of(ui).words::<ListManagerWords>();
        let actions = vec![
            ListManagerAction::run(&words).tooltip("Runs the macro"),
            ListManagerAction::create(&words).tooltip("Adds a macro").reason("Type a new name"),
        ];
        let entries = vec![ListManagerEntry::new("a", "Align Layers"), ListManagerEntry::new("b", "Batch Export")];
        let mut open = true;
        ListManager::new("Macros", "Macro name:", entries, actions).name("Batch Export").opened(Bind::bound(&mut open)).show(ui);
    });
    fixture.settle();
    assert_eq!(tip_over(&mut fixture, "Run").as_deref(), Some("Runs the macro"));
    assert_eq!(tip_over(&mut fixture, "Create").as_deref(), Some("Type a new name"), "the name exists, so Create is disabled and tells why");
}
