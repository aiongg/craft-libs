//! A command scoped to a popover runs on its chord while the focus is on a control inside the popover.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, CommandSet, Show as _, ShowContainer as _};
use craft_ui::popover::Popover;
use craft_ui::runtime::commands::{CommandScope, Commands as _};
use craft_ui::runtime::focus::Initial;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

/// The commands of the card scope: a bare arrow, which a focused button routes to itself as a move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "card", scope = "trial.card")]
#[non_exhaustive]
pub enum CardCommand {
    #[command(label = "Forward", default = "Right", modal = "always")]
    Forward,
}

const CARD: CommandScope = <CardCommand as CommandSet>::SCOPE;

#[trial]
fn a_popover_scope_command_runs_with_the_focus_on_a_button_inside() {
    let taken = Rc::new(Cell::new(0));
    let seen = taken.clone();
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(CARD).show(ui, |ui, cx| {
            while cx.take_command::<CardCommand>().is_some() {
                seen.set(seen.get() + 1);
            }
            Button::new("Next").show(ui);
        });
    });
    fixture.settle();
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.settle();
    assert_eq!(taken.get(), 1, "the chord reaches the popover's scope from a focused button");
}
