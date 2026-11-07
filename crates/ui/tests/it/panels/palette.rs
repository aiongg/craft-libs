//! The rules of the command palette: the empty query lists the commands last run, newest first; a query lists the
//! matches; a command run is remembered once, newest first; Escape clears the query before it closes the box; the
//! pointer moves the active row only when it moves.

use craft_ui::contract::Bind;
use craft_ui::menu::search::HelpEntry;
use craft_ui::patterns::panels::CommandPalette;
use craft_ui::runtime::commands::{CommandRef, CommandState, EditCommands};
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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

/// The node of the row of the command `label`: a row is named by its label, then its menu path.
fn row(fixture: &Fixture, label: &str) -> Option<Target> {
    let tree = fixture.tree();
    let key = tree.keys().into_iter().find(|key| tree.name_of(*key).is_some_and(|name| name == label || name.starts_with(&format!("{label}, "))))?;
    fixture.target(key)
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
    let redo = row(&fixture, "Redo").expect("the newest command is listed");
    let undo = row(&fixture, "Undo").expect("the older command is listed");
    assert!(redo.center().y < undo.center().y, "newest first");
    assert!(row(&fixture, "Cut").is_none(), "a command not run is not listed");
}

/// With no query and nothing run, the list starts with the app's entries.
#[trial]
fn the_empty_query_without_recents_lists_the_entries() {
    let fixture = palette(Vec::new());
    for label in ["Undo", "Redo", "Cut", "Select All"] {
        assert!(row(&fixture, label).is_some(), "{label} is listed");
    }
}

/// A typed query narrows the list to the matches.
#[trial]
fn a_query_lists_the_matches_only() {
    let mut fixture = palette(Vec::new());
    fixture.type_text("sel");
    fixture.settle();
    assert!(row(&fixture, "Select All").is_some(), "the match is listed");
    assert!(row(&fixture, "Undo").is_none(), "a command that does not match is not listed");
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

/// A palette that stays open until it closes itself, and the command it reports as picked.
struct Rig {
    fixture: Fixture,
    picked: Rc<RefCell<Option<CommandRef>>>,
}

fn rig() -> Rig {
    let (open, picked) = (Rc::new(Cell::new(true)), Rc::new(RefCell::new(None)));
    let (shared_open, shared_picked) = (Rc::clone(&open), Rc::clone(&picked));
    let mut fixture = Fixture::new(move |ui| {
        let entries = entries();
        let mut now = shared_open.get();
        let out = CommandPalette::new("Command palette", &entries, Bind::bound(&mut now)).show(ui);
        shared_open.set(now);
        if out.picked.is_some() {
            *shared_picked.borrow_mut() = out.picked;
        }
    });
    fixture.settle();
    Rig { fixture, picked }
}

/// Escape clears a typed query and leaves the palette open; Escape on the empty query closes it.
#[trial]
fn escape_clears_the_query_before_it_closes_the_palette() {
    let mut rig = rig();
    rig.fixture.type_text("sel");
    rig.fixture.settle();
    assert!(row(&rig.fixture, "Undo").is_none(), "the query narrows the list");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    rig.fixture.settle();
    assert!(row(&rig.fixture, "Undo").is_some(), "the query is empty again and the palette is still open");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    rig.fixture.settle();
    assert!(row(&rig.fixture, "Undo").is_none(), "Escape on the empty query closes the palette");
}

/// A pointer resting over a row does not take the active row back from the arrow keys; it moves the row only when it moves.
#[trial]
fn a_resting_pointer_leaves_the_active_row_to_the_arrow_keys() {
    let mut rig = rig();
    // A command the app has not enabled is listed but skipped by the active row.
    for command in [EditCommands::Undo, EditCommands::Redo, EditCommands::Cut, EditCommands::SelectAll] {
        rig.fixture.command_state(command, CommandState::new().enabled(true));
    }
    rig.fixture.settle();
    let first = row(&rig.fixture, "Undo").expect("the row is listed").center();
    let resting = row(&rig.fixture, "Select All").expect("the row is listed").center();
    // The pointer moves onto the last row, which makes it the active one; then it rests there.
    rig.fixture.frame(vec![pointer::moved(first)]);
    rig.fixture.frame(vec![pointer::moved(resting)]);
    rig.fixture.idle(2);
    rig.fixture.key(Key::ArrowUp, Modifiers::NONE);
    rig.fixture.idle(2);
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(rig.picked.borrow().clone(), Some(CommandRef::from(EditCommands::Cut)), "the row above the one under the pointer is active");
}
