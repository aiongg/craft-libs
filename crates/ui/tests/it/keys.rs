//! Which key roles take Delete: a command scope gets Delete from a focused button, while a deletable list and a text
//! field keep it, and a collection that leaves its arrows to its owner lets the owner's arrow command run.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::buttons::Button;
use craft_ui::contract::{
    Bind, CollectionEvent, CommandSet, Commands as _, ItemKeys, Props as _, SelectMode, Show as _, ShowCollection as _, ShowContainer as _,
};
use craft_ui::list::{List, ListRow};
use craft_ui::popover::Popover;
use craft_ui::runtime::commands::CommandScope;
use craft_ui::runtime::focus::Initial;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The commands of the scope: Delete and a bare Down, both kept from the window's shortcuts while focus is inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "keysTrial", scope = "trial.keys")]
#[non_exhaustive]
enum KeysCommand {
    #[command(label = "Remove", default = "Delete", modal = "always")]
    Remove,
    #[command(label = "Step", default = "Down", modal = "always")]
    Step,
}

const SCOPE: CommandScope = <KeysCommand as CommandSet>::SCOPE;

struct Three;

impl DisplayOrder<usize> for Three {
    fn len(&self) -> usize {
        3
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Three {}

/// How often each command of the scope ran.
#[derive(Default)]
struct Ran {
    remove: Cell<u32>,
    step: Cell<u32>,
}

/// Counts the scope's commands that `$cx` has this pass.
macro_rules! count {
    ($cx:expr, $ran:expr) => {
        while let Some((command, _)) = $cx.take_command::<KeysCommand>() {
            let counter = if command == KeysCommand::Remove { &$ran.remove } else { &$ran.step };
            counter.set(counter.get() + 1);
        }
    };
}

/// Delete with focus on a button inside a scope that has a Delete command runs that command.
#[trial]
fn delete_on_a_focused_button_runs_the_scope_command() {
    let ran = Rc::new(Ran::default());
    let seen = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(SCOPE).show(ui, |ui, cx| {
            count!(cx, seen);
            Button::new("Next").show(ui);
        });
    });
    fixture.settle();
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.settle();
    assert_eq!(ran.remove.get(), 1);
}

/// Delete in a deletable list deletes the row, and the scope's Delete command does not run on the same press.
#[trial]
fn delete_in_a_deletable_list_deletes_the_row_and_not_the_command() {
    let ran = Rc::new(Ran::default());
    let deleted = Rc::new(RefCell::new(Vec::new()));
    let (seen, sink) = (ran.clone(), deleted.clone());
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(SCOPE).show(ui, |ui, cx| {
            count!(cx, seen);
            let out = List::new("Items")
                .select(SelectMode::Single)
                .deletable(true)
                .show_items(ui, &Three, &mut |key, _| ListRow::new(format!("Row {key}")));
            for event in out.events {
                if let CollectionEvent::Delete(keys) = event {
                    sink.borrow_mut().push(keys);
                }
            }
        });
    });
    fixture.settle();
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.settle();
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.settle();
    assert_eq!(*deleted.borrow(), [vec![0]]);
    assert_eq!(ran.remove.get(), 0);
}

/// Backspace and Delete in a text field edit its text, and the scope's Delete command does not run.
#[trial]
fn backspace_and_delete_in_a_text_field_edit_the_text() {
    let ran = Rc::new(Ran::default());
    let text = Rc::new(RefCell::new(String::new()));
    let (seen, shown) = (ran.clone(), text.clone());
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(SCOPE).show(ui, |ui, cx| {
            count!(cx, seen);
            TextField::new("Name", Bind::bound(&mut *shown.borrow_mut())).show(ui);
        });
    });
    fixture.settle();
    fixture.type_text("abc");
    fixture.idle(2);
    fixture.key(Key::Backspace, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*text.borrow(), "ab");
    fixture.key(Key::Home, Modifiers::NONE);
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*text.borrow(), "b");
    assert_eq!(ran.remove.get(), 0);
}

/// A list steps its lead with Down and the scope's Down command stays unrun; a list that leaves its arrows to its
/// owner runs the command and keeps its lead.
#[trial]
fn a_collection_that_leaves_its_arrows_to_the_owner_lets_the_owner_command_run() {
    let run = |keys: ItemKeys| {
        let ran = Rc::new(Ran::default());
        let leads = Rc::new(RefCell::new(Vec::new()));
        let (seen, sink) = (ran.clone(), leads.clone());
        let mut fixture = Fixture::new(move |ui| {
            Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(SCOPE).show(ui, |ui, cx| {
                count!(cx, seen);
                let out = List::new("Items")
                    .select(SelectMode::Single)
                    .item_keys(keys)
                    .show_items(ui, &Three, &mut |key, _| ListRow::new(format!("Row {key}")));
                for event in out.events {
                    if let CollectionEvent::LeadMoved { key, .. } = event {
                        sink.borrow_mut().push(key);
                    }
                }
            });
        });
        fixture.settle();
        fixture.key(Key::ArrowDown, Modifiers::NONE);
        fixture.settle();
        let moved = leads.borrow().len();
        (ran.step.get(), moved)
    };
    assert_eq!(run(ItemKeys::Own), (0, 1), "the list takes the arrow");
    assert_eq!(run(ItemKeys::Owner), (1, 0), "the owner's command takes the arrow");
}
