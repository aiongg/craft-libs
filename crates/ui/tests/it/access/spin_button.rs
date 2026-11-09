//! A number field is one spin button node: the numeric value and the editable text on the node that takes focus.

use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::testing::access::{NodeKey, focused_key, is_interactive, name};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::text::set_value_request;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::NumberField;
use egui::{Id, Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const NAME: &str = "Spacing";
const START: f64 = 8.0;
/// The text an AccessKit SetValue carries, and the number it reads as.
const TYPED: &str = "12";
const TYPED_NUMBER: f64 = 12.0;

struct Spin {
    fixture: Fixture,
    value: Rc<RefCell<Option<f64>>>,
    id: Rc<Cell<Id>>,
}

fn spin() -> Spin {
    let (value, id) = (Rc::new(RefCell::new(Some(START))), Rc::new(Cell::new(Id::NULL)));
    let (bound, seen) = (value.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = NumberField::new(NAME, Bind::bound(&mut *bound.borrow_mut())).show(ui);
        seen.set(out.response.id);
    });
    fixture.settle();
    Spin { fixture, value, id }
}

/// The node of the field: the one interactive node its name names.
fn node(spin: &Spin) -> NodeKey {
    let tree = spin.fixture.tree();
    let named: Vec<NodeKey> = tree.keys().into_iter().filter(|key| name(tree, *key) == Some(NAME) && is_interactive(tree, *key)).collect();
    assert_eq!(named.len(), 1, "one node carries the field's name: {:?}", named.iter().map(|key| tree.role_of(*key)).collect::<Vec<_>>());
    named[0]
}

#[trial]
fn a_number_field_is_one_spin_button_node_that_tab_focuses() {
    let mut spin = spin();
    let key = node(&spin);
    {
        let tree = spin.fixture.tree();
        assert_eq!(key, NodeKey::of(spin.id.get()), "the field's own node is the one");
        assert_eq!(tree.role_of(key), Some(Role::SPIN_BUTTON));
        assert_eq!(tree.numeric(spin.id.get()).and_then(|numeric| numeric.value), Some(START), "the node carries the number");
        assert!(tree.value_of(key).is_some_and(|text| text.starts_with('8')), "and the text: {:?}", tree.value_of(key));
        assert!(tree.with_role(Role::TEXT_INPUT).is_empty(), "the text widget has no node of its own");
    }
    assert_ne!(focused_key(spin.fixture.tree()), key, "the field starts unfocused");
    spin.fixture.key(Key::Tab, Modifiers::NONE);
    spin.fixture.idle(2);
    assert_eq!(focused_key(spin.fixture.tree()), key, "Tab lands on the spin button node");
}

#[trial]
fn access_increment_steps_and_set_value_sets_the_number() {
    let mut spin = spin();
    let target = spin.fixture.by_label(NAME).expect("the field has a node");
    spin.fixture.access(AccessActionKind::Increment, &target);
    spin.fixture.idle(2);
    let stepped = spin.value.borrow().expect("the field holds a number");
    assert!(stepped > START, "Increment steps the value up: {stepped}");
    spin.fixture.frame(vec![set_value_request(spin.id.get(), TYPED)]);
    spin.fixture.idle(2);
    assert_eq!(*spin.value.borrow(), Some(TYPED_NUMBER), "SetValue with text sets the number it reads");
}
