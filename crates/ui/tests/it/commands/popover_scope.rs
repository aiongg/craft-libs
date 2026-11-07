//! A command scoped to a popover runs on its chord while the focus is on a control inside the popover.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, CommandSet, Entry, Show as _, ShowContainer as _};
use craft_ui::pickers::Select;
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
    #[command(label = "Lower", default = "Down", modal = "always")]
    Lower,
}

const CARD: CommandScope = <CardCommand as CommandSet>::SCOPE;

#[trial]
fn a_popover_scope_command_runs_with_the_focus_on_a_button_inside() {
    let taken = Rc::new(Cell::new(0));
    let seen = taken.clone();
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(CARD).show(ui, |ui, cx| {
            while let Some((command, _)) = cx.take_command::<CardCommand>() {
                if command == CardCommand::Forward {
                    seen.set(seen.get() + 1);
                }
            }
            Button::new("Next").show(ui);
        });
    });
    fixture.settle();
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.settle();
    assert_eq!(taken.get(), 1, "the chord reaches the popover's scope from a focused button");
}

/// A select takes the arrows it steps with, so a command of the popover scope on the same key does not run.
#[trial]
fn a_select_inside_a_popover_keeps_its_arrow_over_a_scope_command() {
    let (lowered, picked) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0u8)));
    let (seen, shown) = (lowered.clone(), picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        Popover::new("Card", Bind::given(true)).initial_focus(Initial::First).command_scope(CARD).show(ui, |ui, cx| {
            while let Some((command, _)) = cx.take_command::<CardCommand>() {
                if command == CardCommand::Lower {
                    seen.set(seen.get() + 1);
                }
            }
            let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply")];
            let mut mode = shown.get();
            Select::new("Blend mode", Bind::bound(&mut mode), &entries).id_salt("mode").show(ui);
            shown.set(mode);
        });
    });
    fixture.settle();
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.settle();
    assert_eq!(picked.get(), 1, "Down steps the select");
    assert_eq!(lowered.get(), 0, "the scope command does not run on the same press");
}
