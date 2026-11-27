//! The command board on craft-ui-rt (docs/contract-rt.md section 10, "Commands"). The host makes
//! one board per runtime from its keymap and gives it to each window root (`WindowEnv::commands`),
//! which provides it to everything under it.
//!
//! - Dispatch: the window root is the last claimant on every key's path. A chord no node took is
//!   looked up in the keymap, with typing when the focused stop types (the keymap's typing rule
//!   then decides which chords still fire, and a stop with `KeyUse::command_keys` lets command and
//!   function keys through), and the bound command runs: its handler is called at event time with
//!   the store, so it changes app state by writing cells and only the nodes that read those cells
//!   run. A focused stop's claims (`KeyUse::claims`) and its edit chords never get here: the stop
//!   takes them first ([`super::keys`]).
//! - State: each command's `CommandState` is a cell. A disabled command's bindings are passed
//!   over (the chord's next binding in an outer scope runs, else nothing), read without
//!   recording; a node that shows a command's state reads its cell (`Ui::command_state`), so a
//!   state change runs exactly those nodes.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::{EditCommand, OwnerClass};
use craft_keymap::{Active, Chord, CommandRef, Keymap, Lookup};
use craft_ui_rt::{EventCx, Handled, Key, KeyInput, Reads, State, Store, Writes};

use super::node::{ComponentNode, Kept, ScopeNode};
use super::overlay::LayerScope;
use super::ui::Ui;
use crate::runtime::commands::{CommandSet, CommandState, FirePhase, FireSource, Fired};

/// What a command does when it runs: called at event time with the store, where it writes the app
/// state the command changes.
type Handler = Box<dyn FnMut(&Fired, &mut Store)>;

struct Board {
    keymap: Keymap,
    /// The commands a focused component takes as `Intent::Edit` (`CommandSet::edits`).
    edits: Vec<(CommandRef, EditCommand)>,
    states: Vec<(CommandRef, State<CommandState>)>,
    handlers: Vec<(CommandRef, Handler)>,
}

/// The app's commands: the keymap, each command's state as a cell and what each command does.
#[derive(Clone)]
pub struct CommandBoard(Rc<RefCell<Board>>);

impl CommandBoard {
    /// A board over `keymap`, with no handlers and every command enabled.
    pub fn new(keymap: Keymap) -> CommandBoard {
        CommandBoard(Rc::new(RefCell::new(Board { keymap, edits: Vec::new(), states: Vec::new(), handlers: Vec::new() })))
    }

    /// The board with the edit commands of `S` (`#[command(edit = ...)]`): a focused component that
    /// lists an edit takes its chords as `Intent::Edit`.
    #[must_use]
    pub fn with_edits<S: CommandSet>(self) -> CommandBoard {
        self.0.borrow_mut().edits.extend(S::edits().into_iter().map(|(id, edit)| (CommandRef::from(id), edit)));
        self
    }

    /// Runs `handler` when `command` fires.
    pub fn on(&self, command: impl Into<CommandRef>, handler: impl FnMut(&Fired, &mut Store) + 'static) {
        let command = command.into();
        let mut board = self.0.borrow_mut();
        board.handlers.retain(|(c, _)| *c != command);
        board.handlers.push((command, Box::new(handler)));
    }

    /// The cell of `command`'s state, created enabled on first use.
    fn cell(&self, store: &mut Store, command: &CommandRef) -> State<CommandState> {
        let mut board = self.0.borrow_mut();
        if let Some((_, cell)) = board.states.iter().find(|(c, _)| c == command) {
            return *cell;
        }
        let cell = store.create(CommandState::new());
        board.states.push((command.clone(), cell));
        cell
    }

    /// Sets `command`'s state; the nodes that read it run in the next frame, and only they.
    pub fn set_state(&self, cx: &mut impl Writes, command: impl Into<CommandRef>, state: CommandState) {
        let command = command.into();
        let cell = self.cell(cx.store_mut(), &command);
        cell.set_if_changed(cx, state);
    }

    /// `command`'s state, read through its cell: a node that reads it runs when it changes.
    pub fn state<C: Reads + Writes>(&self, cx: &mut C, command: impl Into<CommandRef>) -> CommandState {
        let command = command.into();
        let cell = self.cell(cx.store_mut(), &command);
        cell.get(cx)
    }

    /// Whether `command` may fire, read without recording.
    fn enabled(&self, store: &Store, command: &CommandRef) -> bool {
        self.0.borrow().states.iter().find(|(c, _)| c == command).is_none_or(|(_, cell)| store.peek(*cell).is_enabled())
    }

    /// The edit command `chord` is bound to, when it is one.
    pub(crate) fn edit_of(&self, chord: &Chord) -> Option<EditCommand> {
        let board = self.0.borrow();
        let Lookup::Run(command) = board.keymap.lookup(chord, false, &Active::NONE) else { return None };
        board.edits.iter().find(|(c, _)| *c == command).map(|(_, edit)| *edit)
    }

    /// Runs the command a press of `k` is bound to, when it is enabled. `typing`: the focused stop
    /// types, so the keymap's typing rule holds back the chords that type.
    fn dispatch(&self, cx: &mut EventCx<'_>, k: &KeyInput, typing: bool) -> Handled {
        let chord = Chord::new(k.mods, k.key);
        let lookup = {
            let board = self.0.borrow();
            let store = cx.runtime().store();
            let active = Active { panel: None, contexts: &[], typing };
            board.keymap.lookup_where(&chord, false, &active, &[], &|command| self.enabled(store, command))
        };
        let command = match lookup {
            Lookup::Run(command) => command,
            // Tools of one group share the chord: the app's handler picks the next; the first stands in.
            Lookup::Cycle(group) => match group.into_iter().next() {
                Some(command) => command,
                None => return Handled::No,
            },
            _ => return Handled::No,
        };
        let fired = Fired::new(command.clone(), FireSource::Key, if k.repeat { FirePhase::Repeat } else { FirePhase::Press }, cx.time()).mods(k.mods);
        // The handler runs outside the board's borrow: it may set command states.
        let taken = {
            let mut board = self.0.borrow_mut();
            board.handlers.iter().position(|(c, _)| *c == command).map(|at| board.handlers.remove(at))
        };
        if let Some((command, mut handler)) = taken {
            handler(&fired, cx.store_mut());
            self.0.borrow_mut().handlers.push((command, handler));
        }
        Handled::Yes
    }
}

/// The window root's command stage, after Tab and F6: the command a key press no node took is bound
/// to runs.
pub(crate) fn window_command(root: &Kept, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
    if !k.down || matches!(k.key, Key::Tab | Key::Escape) {
        return Handled::No;
    }
    let Some(board) = root.commands.clone() else { return Handled::No };
    let typing = cx.focused().and_then(|f| focused_keys(cx, f)).is_some_and(|keys| {
        let typing = matches!(keys.owner, OwnerClass::Text | OwnerClass::Recorder);
        // A stop with `command_keys` lets command chords and function keys run while it types.
        let command_key = Chord::new(k.mods, k.key).has_command() || is_function(k.key);
        typing && !(keys.command_keys && command_key)
    });
    board.dispatch(cx, k, typing)
}

fn is_function(key: Key) -> bool {
    matches!(key, Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::F5 | Key::F6 | Key::F7 | Key::F8 | Key::F9 | Key::F10 | Key::F11 | Key::F12)
}

/// The kept key use of the focused node `node`.
fn focused_keys(cx: &EventCx<'_>, node: craft_ui_rt::NodeId) -> Option<crate::runtime::focus::KeyUse> {
    let rt = cx.runtime();
    let kept = rt.local::<ComponentNode>(node).or_else(|| rt.local::<ScopeNode>(node)).or_else(|| rt.local::<LayerScope>(node).map(|lk| &lk.kept))?;
    kept.focus.stop
}

impl Ui<'_, '_> {
    /// The state of `command` from the window's command board, read through its cell: this scope
    /// runs again when it changes. Enabled with no board.
    pub fn command_state(&mut self, command: impl Into<CommandRef>) -> CommandState {
        match self.cx.provided::<CommandBoard>() {
            Some(board) => board.state(self.cx, command),
            None => CommandState::new(),
        }
    }
}
