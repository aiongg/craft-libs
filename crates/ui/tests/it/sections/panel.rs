//! `Panel`: the header's menu opens from the menu key as well as from a click on the menu button.

use craft_behavior::IconId;
use craft_ui::contract::{CommandSet, Commands as _, Show as _, ShowContainer as _};
use craft_ui::menu::MenuItem;
use craft_ui::runtime::commands::CommandScope;
use craft_ui::sections::{HeaderAction, HeaderCommand, Panel};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const SCOPE: CommandScope = <HeaderCommands as CommandSet>::SCOPE;

#[trial]
fn a_click_on_a_panels_menu_button_opens_the_panel_menu() {
    let mut fixture = Fixture::new(|ui| {
        Panel::new("Layers")
            .actions([HeaderAction::Menu])
            .menu(|ui, _| {
                MenuItem::new("Rename").show(ui);
            })
            .show(ui, |_, _| {});
    });
    fixture.settle();
    let button = fixture.by_label("More actions").expect("the header shows its menu button");
    fixture.click(&button);
    assert!(fixture.by_label("Rename").is_some(), "a click on the menu button opens the panel menu");
}

/// The commands of the panel scope the header icons below run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "panelTrial", scope = "trial.panel")]
#[non_exhaustive]
enum HeaderCommands {
    #[command(label = "Open card")]
    Open,
}

/// Runs a panel with an Open header command and counts how often the command reached the panel's scope.
fn open_count(act: impl FnOnce(&mut Fixture)) -> usize {
    let ran = Rc::new(Cell::new(0));
    let seen = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        Panel::new("Card").commands([HeaderCommand::new(IconId::new("open"), HeaderCommands::Open)]).command_scope(SCOPE).show(ui, |_, cx| {
            while cx.take_command::<HeaderCommands>().is_some() {
                seen.set(seen.get() + 1);
            }
        });
    });
    fixture.settle();
    act(&mut fixture);
    fixture.settle();
    ran.get()
}

/// A header command is a button named by its command that a click and the keyboard both run, as an action part does.
#[trial]
fn a_panel_header_command_runs_on_a_click_and_on_enter() {
    let clicked = open_count(|fixture| {
        let button = fixture.by_label("Open card").expect("the header shows the command's icon, named by the command");
        fixture.click(&button);
    });
    assert_eq!(clicked, 1, "a click runs the command");
    let keyed = open_count(|fixture| {
        fixture.key(Key::Tab, Modifiers::NONE).settle();
        fixture.key(Key::Enter, Modifiers::NONE);
    });
    assert_eq!(keyed, 1, "Tab reaches the icon and Enter runs the command");
}
