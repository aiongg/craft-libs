//! Commands as the contexts of a container body and the fixture's command input give them.

use super::kit::{Dock, Pane, PaneCommand};
use craft_behavior::Mixed;
use craft_ui::buttons::IconButton;
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::commands::{CommandState, Commands as _};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

/// A body context resolves a scoped command inside an instance of its scope, enabled and with the scope's chord, and
/// resolves nothing outside any instance.
#[trial]
fn a_body_resolves_a_scoped_command_only_inside_its_instance() {
    let seen = Rc::new(RefCell::new((None, None)));
    let out = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let inside = Pane::new().show(ui, |_, cx| cx.resolve_command(PaneCommand::ZoomIn)).inner.flatten();
        let outside = Dock::new().show(ui, |_, cx| cx.resolve_command(PaneCommand::ZoomIn)).inner.flatten();
        *out.borrow_mut() = (inside, outside);
    });
    fixture.settle();
    let (inside, outside) = seen.take();
    let inside = inside.expect("a pane is above the body");
    assert!(inside.enabled && inside.chord.is_some(), "enabled, with the scoped chord: {inside:?}");
    assert_eq!(outside, None, "no pane above the dock's body");
}

/// A command state set on the fixture reaches the components bound to the command until it is set again.
#[trial]
fn fixture_command_state_checks_a_bound_icon_button() {
    use craft_ui::runtime::commands::EditCommands;
    let mut fixture = Fixture::new(|ui| {
        IconButton::command(craft_behavior::IconId::new("check"), EditCommands::Undo).show(ui);
    });
    fixture.settle();
    let button = fixture.by_role(craft_ui::contract::Role::BUTTON)[0];
    fixture.command_state(EditCommands::Undo, CommandState::new().checked(true)).idle(3);
    assert_eq!(fixture.tree().toggled_of(button.node), Some(Mixed::Same(true)));
    fixture.command_state(EditCommands::Undo, CommandState::new().checked(false)).idle(3);
    assert_eq!(fixture.tree().toggled_of(button.node), Some(Mixed::Same(false)));
}
