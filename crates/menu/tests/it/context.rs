//! Context menus from menu-model rows.

use craft_menu::{Item, Node, context};
use craft_ui::buttons::Button;
use craft_ui::contract::ShowContainer as _;
use craft_ui::menu::Menu;
use craft_ui::runtime::commands::{CommandState, FireSource};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2};

fn rows() -> Vec<Node> {
    vec![
        Node::from(Item::new("edit.copy", "Copy")),
        Node::from(Item::new("edit.paste", "Paste")),
        Node::Separator,
        Node::submenu("Layer", vec![Node::from(Item::new("layer.duplicate", "Duplicate Layer"))]),
    ]
}

/// A context menu of the rows over a button, opened by a secondary click on it.
fn rig() -> Fixture {
    let nodes = rows();
    let mut fixture = Fixture::new(move |ui| {
        let target = Button::new(TARGET).show(ui);
        Menu::context("Canvas menu", &target).show(ui, |ui, _| context::rows(ui, &nodes));
    });
    fixture.settle();
    fixture
}

const TARGET: &str = "Canvas";

fn open(fixture: &mut Fixture) {
    let at: Pos2 = fixture.by_label(TARGET).expect("the target has a node").center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
}

#[trial]
fn a_secondary_click_opens_the_rows_and_a_click_on_one_fires_its_command() {
    let mut fixture = rig();
    open(&mut fixture);
    assert!(fixture.by_label("Copy").is_some() && fixture.by_label("Layer").is_some());
    let copy = fixture.by_label("Copy").expect("the Copy row");
    fixture.click(&copy);
    fixture.settle();
    let fired = take_output(fixture.ctx()).fired;
    assert_eq!(fired.iter().map(|fired| (fired.command.id.as_str(), fired.source)).collect::<Vec<_>>(), [("edit.copy", FireSource::Menu)]);
}

#[trial]
fn a_row_follows_the_commands_state_and_a_disabled_one_cannot_be_chosen() {
    let mut fixture = rig();
    fixture.command_state("edit.copy", CommandState::new().enabled(false));
    open(&mut fixture);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
    assert!(take_output(fixture.ctx()).fired.iter().all(|fired| fired.command.id != "edit.copy"), "a disabled command does not fire");
}
