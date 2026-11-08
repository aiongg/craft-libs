//! Context menus from menu-model rows.

use craft_menu::{Item, Node, context};
use craft_ui::contract::ShowContainer as _;
use craft_ui::menu::Menu;
use craft_ui::runtime::commands::{CommandState, FireSource};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Sense, Vec2};

fn rows() -> Vec<Node> {
    vec![
        Node::from(Item::new("edit.copy", "Copy")),
        Node::from(Item::new("edit.paste", "Paste")),
        Node::Separator,
        Node::submenu("Layer", vec![Node::from(Item::new("layer.duplicate", "Duplicate Layer"))]),
    ]
}

/// A context menu of the rows over an area at the room's corner, opened by a secondary click at `AT`.
fn rig() -> Fixture {
    let nodes = rows();
    let mut fixture = Fixture::new(move |ui| {
        let target = ui.allocate_response(Vec2::new(100.0, 40.0), Sense::click());
        Menu::context("Canvas menu", &target).show(ui, |ui, _| context::rows(ui, &nodes));
    });
    fixture.settle();
    fixture
}

const AT: Pos2 = Pos2::new(20.0, 10.0);

fn open(fixture: &mut Fixture) {
    fixture.frame(vec![pointer::moved(AT)]);
    fixture.frame(vec![pointer::secondary(AT, true)]);
    fixture.frame(vec![pointer::secondary(AT, false)]);
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
    assert_eq!(fired.iter().map(|fired| (fired.command.id.as_str(), fired.source)).collect::<Vec<_>>(), [("edit.copy", FireSource::Pointer)]);
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
