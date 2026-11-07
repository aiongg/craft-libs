//! The rules of the command palette: the empty query lists the commands last run, newest first; a query lists the
//! matches; a command run is remembered once, newest first.

use craft_ui::contract::Bind;
use craft_ui::menu::search::HelpEntry;
use craft_ui::patterns::panels::CommandPalette;
use craft_ui::runtime::commands::{CommandRef, EditCommands};
use craft_ui::testing::{Fixture, trial};

/// How many commands the palette remembers.
const KEPT: usize = 8;

fn entries() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new(EditCommands::Undo, "Undo").with_path(["Edit"]),
        HelpEntry::new(EditCommands::Redo, "Redo").with_path(["Edit"]),
        HelpEntry::new(EditCommands::Cut, "Cut").with_path(["Edit"]),
        HelpEntry::new(EditCommands::SelectAll, "Select All").with_path(["Select"]),
    ]
}

fn palette(recents: Vec<CommandRef>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let entries = entries();
        CommandPalette::new("Command palette", &entries, Bind::given(true)).recents(Bind::given(recents.clone())).show(ui);
    });
    fixture.settle();
    fixture
}

/// With no query the list is the commands last run, newest first, and no other command.
#[trial]
fn the_empty_query_lists_the_commands_last_run_newest_first() {
    let fixture = palette(vec![CommandRef::from(EditCommands::Redo), CommandRef::from(EditCommands::Undo)]);
    let redo = fixture.by_label("Redo").expect("the newest command is listed");
    let undo = fixture.by_label("Undo").expect("the older command is listed");
    assert!(redo.center().y < undo.center().y, "newest first");
    assert!(fixture.by_label("Cut").is_none(), "a command not run is not listed");
}

/// With no query and nothing run, the list starts with the app's entries.
#[trial]
fn the_empty_query_without_recents_lists_the_entries() {
    let fixture = palette(Vec::new());
    for label in ["Undo", "Redo", "Cut", "Select All"] {
        assert!(fixture.by_label(label).is_some(), "{label} is listed");
    }
}

/// A typed query narrows the list to the matches.
#[trial]
fn a_query_lists_the_matches_only() {
    let mut fixture = palette(Vec::new());
    fixture.type_text("sel");
    fixture.settle();
    assert!(fixture.by_label("Select All").is_some(), "the match is listed");
    assert!(fixture.by_label("Undo").is_none(), "a command that does not match is not listed");
}

/// A command run goes first, once, and the list keeps the newest few.
#[trial]
fn a_command_run_is_remembered_once_newest_first() {
    let mut recents = vec![CommandRef::from(EditCommands::Undo), CommandRef::from(EditCommands::Redo)];
    CommandPalette::remember(&mut recents, CommandRef::from(EditCommands::Redo));
    assert_eq!(recents, [CommandRef::from(EditCommands::Redo), CommandRef::from(EditCommands::Undo)]);
    for at in 0..KEPT * 2 {
        CommandPalette::remember(&mut recents, CommandRef::new(format!("test.command{at}")));
    }
    assert_eq!(recents.first(), Some(&CommandRef::new(format!("test.command{}", KEPT * 2 - 1))), "the newest is first");
    assert_eq!(recents.len(), KEPT, "only the newest are kept");
}
