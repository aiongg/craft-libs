//! Context menus from menu-model rows.

use craft_menu::{Item, Node, context};
use craft_ui::buttons::Button;
use craft_ui::contract::{Show as _, ShowContainer as _};
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

