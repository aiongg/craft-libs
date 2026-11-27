//! `take_command` and `take_step` on the command board (docs/contract-rt.md 15.8 and the lead's
//! decision on 15.13 item 1).
//!
//! A node that takes the commands of a `CommandSet` owns one cell per set: the last command of the set
//! fired at it, with a sequence number. The board handler its first take registers writes that cell
//! at event time, so a fire reruns only the node that reads it. A take returns the command when its
//! sequence is newer than the last one the node consumed, and records the consumed sequence in the
//! node's memory. It never writes the cell it read, so it causes no rerun in the same frame.
//!
//! A run cannot reach the store, so the driver does the cell work around the hooks: before `update`
//! it reads the cells the node registered (tracked reads) into [`InboxIn`]; a take of a set with no
//! cell yet asks for one ([`InboxIn::want`]), and after the run the driver creates it and registers
//! the board handlers ([`register`]).

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use craft_keymap::CommandRef;
use craft_ui_rt::{State, Store};

use super::{CommandSet, Fired};
use crate::contract::Id;
use crate::runtime::memory::MemoryStage;

/// The last command of one set fired at one node, and its sequence number (0: none yet).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Inbox {
    pub(crate) seq: u64,
    pub(crate) fired: Option<Fired>,
}

/// One set's cell of a node, as the node's memory keeps it. `alive` is dropped with the node's
/// memory, which ends its board routes.
#[derive(Clone)]
pub(crate) struct InboxCell {
    pub(crate) set: TypeId,
    pub(crate) cell: State<Inbox>,
    alive: Rc<()>,
}

/// The cells a node registered, in its memory.
#[derive(Clone, Default)]
pub(crate) struct InboxCells(pub(crate) Vec<InboxCell>);

/// The sequence each set's last take consumed, in the node's memory.
#[derive(Clone, Default)]
struct Consumed(Vec<(TypeId, u64)>);

/// A set a take asked a cell for, with the commands to route to it.
pub(crate) struct Want {
    set: TypeId,
    commands: Vec<CommandRef>,
}

/// What a run's takes read: each registered set's inbox as of the run, and the sets that asked for a
/// cell.
#[derive(Default)]
pub struct InboxIn {
    pub(crate) read: Vec<(TypeId, Inbox)>,
    pub(crate) wants: Vec<Want>,
}

impl InboxIn {
    /// Reads the node's cells (a tracked read each: a fire at the node reruns it).
    pub(crate) fn read(cells: &InboxCells, cx: &mut impl craft_ui_rt::Reads) -> InboxIn {
        InboxIn { read: cells.0.iter().map(|c| (c.set, c.cell.get(cx))).collect(), wants: Vec::new() }
    }

    /// The next command of `S` fired at the node that it has not taken, which `filter` accepts. `None`
    /// when there is none; a set with no cell yet asks for one.
    pub(crate) fn take<S: CommandSet, T>(
        &mut self,
        memory: &mut MemoryStage,
        id: Id,
        filter: impl Fn(&CommandRef) -> Option<T>,
    ) -> Option<(T, Fired)> {
        let set = TypeId::of::<S>();
        let Some((_, inbox)) = self.read.iter().find(|(s, _)| *s == set) else {
            if !self.wants.iter().any(|w| w.set == set) {
                let commands = S::defs().map(|defs| defs.into_iter().map(|def| def.command).collect()).unwrap_or_default();
                self.wants.push(Want { set, commands });
            }
            return None;
        };
        let fired = inbox.fired.as_ref()?;
        let consumed = memory.shown::<Consumed>(crate::runtime::memory::sub(id, "commands"));
        let at = match consumed.0.iter().position(|(s, _)| *s == set) {
            Some(at) => at,
            None => {
                consumed.0.push((set, 0));
                consumed.0.len() - 1
            }
        };
        if consumed.0[at].1 >= inbox.seq {
            return None;
        }
        consumed.0[at].1 = inbox.seq;
        filter(&fired.command).map(|taken| (taken, fired.clone()))
    }
}

/// The nodes that take one command, newest registration last.
#[derive(Default)]
pub(crate) struct Routes(Vec<(CommandRef, Id, State<Inbox>, Weak<()>)>);

impl Routes {
    /// Writes `fired` into the inbox of the node it is routed to: the named target, else the newest
    /// live registration (the innermost instance shown last).
    fn deliver(&mut self, fired: &Fired, store: &mut Store) {
        self.0.retain(|(_, _, _, alive)| alive.strong_count() > 0);
        let route = self
            .0
            .iter()
            .rev()
            .find(|(c, id, _, _)| *c == fired.command && fired.target.is_some_and(|t| t == *id))
            .or_else(|| self.0.iter().rev().find(|(c, _, _, _)| *c == fired.command));
        if let Some((_, _, cell, _)) = route {
            let seq = store.peek(*cell).seq + 1;
            cell.set(store, Inbox { seq, fired: Some(fired.clone()) });
        }
    }
}

/// Creates the cells the run's takes asked for, routes their commands to them on `board` and keeps
/// them in the node's memory. One registration per node and set: a later run finds the cell.
pub(crate) fn register(wants: Vec<Want>, id: Id, memory: &mut MemoryStage, board: &crate::runtime::commands::CommandBoard, store: &mut Store) {
    if wants.is_empty() {
        return;
    }
    let routes = board.routes();
    for want in wants {
        let cells = memory.shown::<InboxCells>(crate::runtime::memory::sub(id, "inbox"));
        if cells.0.iter().any(|c| c.set == want.set) {
            continue;
        }
        let cell = store.create(Inbox::default());
        let alive = Rc::new(());
        for command in want.commands {
            let fresh = !routes.borrow().0.iter().any(|(c, _, _, _)| *c == command);
            routes.borrow_mut().0.push((command.clone(), id, cell, Rc::downgrade(&alive)));
            if fresh {
                let routes: Rc<RefCell<Routes>> = routes.clone();
                board.on(command, move |fired: &Fired, store: &mut Store| routes.borrow_mut().deliver(fired, store));
            }
        }
        cells.0.push(InboxCell { set: want.set, cell, alive });
    }
}
