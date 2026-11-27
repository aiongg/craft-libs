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
use std::sync::Arc;

use craft_behavior::{EditCommand, OwnerClass};
use craft_keymap::{Active, Chord, CommandRef, Keymap, Lookup, ModalRule};
use craft_ui_rt::{BuildCx, EventCx, Handled, Key, KeyInput, NodeId, Reads, Runtime, State, Store, WindowId, Writes};

use super::node::{ComponentNode, Kept, ScopeNode};
use super::overlay::LayerScope;
use super::ui::Ui;
use crate::contract::context::Context;
use crate::primitives::Id;
use crate::runtime::commands::{
    CommandChain, CommandGate, CommandKind, CommandScope, CommandSet, CommandState, FirePhase, FireSource, Fired, QuitCommand, Registry,
    ResolvedCommand, Shown,
};
use crate::runtime::env::Label;
use crate::runtime::focus::{EditSet, KeyUse};

/// What a command does when it runs: called at event time with the store, where it writes the app
/// state the command changes.
type Handler = Box<dyn FnMut(&Fired, &mut Store)>;

struct Board {
    keymap: Keymap,
    /// The linked command sets' definitions: labels, icons, kinds and home scopes for `resolve`.
    registry: Option<Arc<Registry>>,
    /// Bumps whenever the keymap is replaced, so a node that shows chords reruns.
    keymap_cell: Option<State<u64>>,
    /// The commands a focused component takes as `Intent::Edit` (`CommandSet::edits`).
    edits: Vec<(CommandRef, EditCommand)>,
    /// Each command's state cell, per instance of its scope (`None`: the app's, or a command no instance publishes),
    /// with the value last written to it. Every write goes through the board, so the value is the cell's: code with
    /// no store at hand (a hook's `Commands::resolve_command`) reads it and logs the cell read for its node.
    states: Vec<StateEntry>,
    /// Each handler with the node it was registered for (`Ui::on_command`; `None`: the board's own,
    /// `CommandBoard::on`). One handler per owner and command.
    handlers: Vec<(Option<NodeId>, CommandRef, Handler)>,
    /// The app scope's handler: every fired command with no handler on its firing path goes to it.
    app: Option<Handler>,
    /// The app's one gate over commands (`CommandGate`), and the cell its key is written to, so a node that resolves
    /// a command runs again when the gate's props change.
    gate: Option<CommandGate>,
    gate_cell: Option<State<u128>>,
    /// Each scope declared a child of another (`Ui::declare_command_scope`), with its parent: an instance of the child
    /// also receives the parent's commands. The workspace scope of a full-window dialog is a child of the view scope.
    parents: Vec<(&'static str, &'static str)>,
    /// Moves when a declaration changes what a scope receives, so the nodes that resolve commands run again. Read
    /// with the gate's cell.
    parents_cell: Option<State<u64>>,
}

/// Where a resolve or a fire stands against the window's modal layers (`ModalRule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum ModalAt {
    /// No modal layer is open in the window.
    Free,
    /// A modal layer is open and the resolving or firing node is inside the top one.
    Inside,
    /// A modal layer is open and the node is outside the top one.
    Outside,
}

impl ModalAt {
    /// Where `node` stands in its window; `Free` for no node.
    pub(crate) fn of(rt: &Runtime, node: Option<NodeId>) -> ModalAt {
        let Some(node) = node else { return ModalAt::Free };
        let Some(window) = rt.window_of(node) else { return ModalAt::Free };
        match super::layers::top_modal(rt, window) {
            None => ModalAt::Free,
            Some(top) if super::layers::layer_child_of(rt, node) == Some(top.node) => ModalAt::Inside,
            Some(_) => ModalAt::Outside,
        }
    }

    /// Whether a command with `rule` is refused here; `declared`: its instance publishes the command's state.
    fn refuses(self, rule: ModalRule, declared: bool) -> bool {
        match (rule, self) {
            (_, ModalAt::Free) | (ModalRule::Always, _) => false,
            (ModalRule::Blocked, _) => true,
            (ModalRule::InDialogs, at) => at == ModalAt::Outside,
            (ModalRule::Declared, at) => !(at == ModalAt::Inside && declared),
            // A rule this board does not know refuses as the default rule does.
            (_, _) => true,
        }
    }
}

/// The edit commands the focused stop of a window declares (#3569): the window's cell, written when focus moves
/// (from the newly focused stop's `KeyUse::edits` and `inactive`), read tracked where an edit command resolves, so a
/// surface that shows Cut runs again only when what the focused stop declares changes. A declared edit resolves
/// enabled unless the stop marks it inactive, with the app's title and without the app's hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FocusedEdits {
    pub(crate) edits: EditSet,
    pub(crate) inactive: EditSet,
}

impl FocusedEdits {
    /// Nothing focused, or a stop that declares no edit.
    pub(crate) const NONE: FocusedEdits = FocusedEdits { edits: EditSet::NONE, inactive: EditSet::NONE };

    /// What the stop with `keys` declares; nothing for no stop.
    pub(crate) fn of(keys: Option<KeyUse>) -> FocusedEdits {
        keys.map_or(FocusedEdits::NONE, |keys| FocusedEdits { edits: keys.edits, inactive: keys.inactive })
    }

    /// The cell of `ctx`'s window, made on first use.
    pub(crate) fn cell(ctx: &Context, store: &mut Store) -> State<FocusedEdits> {
        ctx.focused_edits_cell(store)
    }

    /// Writes `edits` as what the focused stop of `ctx`'s window declares, when it differs: the focus change that
    /// moved to the stop calls it (node.rs).
    pub(crate) fn set(ctx: &Context, cx: &mut impl Writes, edits: FocusedEdits) {
        ctx.set_focused_edits(cx, edits);
    }

    /// What the focused stop of `ctx`'s window declares, read tracked by the node `cx` builds.
    pub(crate) fn get(ctx: &Context, cx: &mut BuildCx<'_>) -> FocusedEdits {
        let cell = FocusedEdits::cell(ctx, cx.store_mut());
        cell.get(cx)
    }

    /// Whether the stop declares `edit`, and then whether it is active now.
    fn declares(self, edit: EditCommand) -> Option<bool> {
        self.edits.contains(edit).then(|| !self.inactive.contains(edit))
    }
}

/// What a fire reaches besides the store: the runtime actions the board runs itself once the handlers ran. A command
/// fired by a key, a menu pick or the palette dismisses the layer children that close on a command; Quit then asks
/// every window to close through its close path, so the app's close guard asks about unsaved work as for a window's
/// close button.
pub(crate) trait FireCx {
    fn store(&mut self) -> &mut Store;
    /// Asks every window to close ([`Runtime::quit`]).
    fn quit(&mut self);
    /// Dismisses the layer children of the firing window that close on a command and do not own `from`.
    fn dismiss_on_command(&mut self, from: Option<NodeId>);
}

impl FireCx for EventCx<'_> {
    fn store(&mut self) -> &mut Store {
        self.store_mut()
    }

    fn quit(&mut self) {
        EventCx::quit(self);
    }

    fn dismiss_on_command(&mut self, from: Option<NodeId>) {
        EventCx::dismiss_on_command(self, from);
    }
}

impl FireCx for BuildCx<'_> {
    fn store(&mut self) -> &mut Store {
        self.store_mut()
    }

    fn quit(&mut self) {
        BuildCx::quit(self);
    }

    fn dismiss_on_command(&mut self, from: Option<NodeId>) {
        BuildCx::dismiss_on_command(self, from);
    }
}

/// A fire from outside the tree (the harness, automation) into `window` of the runtime.
pub(crate) struct OutsideFire<'a> {
    pub(crate) rt: &'a mut Runtime,
    pub(crate) window: WindowId,
}

impl FireCx for OutsideFire<'_> {
    fn store(&mut self) -> &mut Store {
        self.rt.store_mut()
    }

    fn quit(&mut self) {
        self.rt.quit();
    }

    fn dismiss_on_command(&mut self, from: Option<NodeId>) {
        self.rt.dismiss_on_command(self.window, from);
    }
}

struct StateEntry {
    command: CommandRef,
    instance: Option<Id>,
    cell: State<CommandState>,
    value: CommandState,
}

/// The app's commands: the keymap, each command's state as a cell and what each command does.
#[derive(Clone)]
pub struct CommandBoard(Rc<RefCell<Board>>);

impl CommandBoard {
    /// A board over `keymap` and the linked command sets, with no handlers and every command enabled.
    pub fn new(keymap: Keymap) -> CommandBoard {
        CommandBoard(Rc::new(RefCell::new(Board {
            keymap,
            registry: Registry::shared(false).ok(),
            keymap_cell: None,
            edits: Vec::new(),
            states: Vec::new(),
            handlers: Vec::new(),
            app: None,
            gate: None,
            gate_cell: None,
            parents: Vec::new(),
            parents_cell: None,
        })))
    }

    /// The board with the edit commands of `S` (`#[command(edit = ...)]`): a focused component that
    /// lists an edit takes its chords as `Intent::Edit`.
    #[must_use]
    pub fn with_edits<S: CommandSet>(self) -> CommandBoard {
        self.0.borrow_mut().edits.extend(S::edits().into_iter().map(|(id, edit)| (CommandRef::from(id), edit)));
        self
    }

    /// The board with the story command sets' definitions (`AppSpec::stories`).
    #[must_use]
    pub fn with_stories(self) -> CommandBoard {
        self.0.borrow_mut().registry = Registry::shared(true).ok();
        self
    }

    /// Replaces the keymap (the user changed a binding); the nodes that show chords run again.
    pub fn set_keymap(&self, cx: &mut impl Writes, keymap: Keymap) {
        let cell = self.keymap_cell(cx.store_mut());
        self.0.borrow_mut().keymap = keymap;
        let generation = *cx.store_mut().peek(cell) + 1;
        cell.set(cx, generation);
    }

    fn keymap_cell(&self, store: &mut Store) -> State<u64> {
        let mut board = self.0.borrow_mut();
        *board.keymap_cell.get_or_insert_with(|| store.create(0))
    }

    /// The keymap's generation, read through its cell: a node that shows chords runs when the keymap changes.
    pub(crate) fn keymap_generation<C: Reads + Writes>(&self, cx: &mut C) -> u64 {
        let cell = self.keymap_cell(cx.store_mut());
        cell.get(cx)
    }

    /// Runs `handler` when `command` fires and no node on the firing path registered one of its own.
    pub fn on(&self, command: impl Into<CommandRef>, handler: impl FnMut(&Fired, &mut Store) + 'static) {
        self.register(None, command.into(), Box::new(handler));
    }

    /// Registers `handler` for `command` under `owner`, replacing that owner's earlier handler for it.
    pub(crate) fn register(&self, owner: Option<NodeId>, command: CommandRef, handler: Handler) {
        let mut board = self.0.borrow_mut();
        board.handlers.retain(|(o, c, _)| !(*o == owner && *c == command));
        board.handlers.push((owner, command, handler));
    }

    /// The firing path of a command fired from `from`: `from` and its ancestors, nearest first. Handlers of nodes that
    /// left the tree go here.
    pub(crate) fn path(&self, rt: &Runtime, from: Option<NodeId>) -> Vec<NodeId> {
        self.0.borrow_mut().handlers.retain(|(owner, _, _)| owner.is_none_or(|node| rt.window_of(node).is_some()));
        let mut path = Vec::new();
        let mut at = from;
        while let Some(node) = at {
            path.push(node);
            at = rt.parent(node);
        }
        path
    }

    /// Runs `fired`'s command along `path` (nearest first) when it is enabled: the handler of the nearest node on the
    /// path that registered one, else the board's own, else the app scope's. Returns whether a handler ran. Every fire
    /// goes through here: a key, `Ui::fire`, `Cx::fire`, a component's activation, the harness. The handler runs
    /// outside the board's borrow, so it may set command states or register handlers.
    /// `modal` and `focused` are where the firing node stands, worked out once per fire by the caller. After the
    /// handler, the board's runtime actions run (`FireCx`).
    pub(crate) fn run(&self, fire: &mut dyn FireCx, path: &[NodeId], fired: &Fired, modal: ModalAt, focused: FocusedEdits) -> bool {
        if !self.enabled(&fired.command) || self.refused(&fired.command, self.declared(&fired.command, None, focused), modal) {
            return false;
        }
        let ran = self.run_handler(fire.store(), path, fired);
        if matches!(fired.source, FireSource::Key | FireSource::Menu | FireSource::Palette) {
            fire.dismiss_on_command(path.first().copied());
        }
        if fired.phase != FirePhase::Repeat && QuitCommand::from_ref(&fired.command).is_some() {
            fire.quit();
        }
        ran
    }

    /// The handler part of [`CommandBoard::run`]: whether a handler ran.
    fn run_handler(&self, store: &mut Store, path: &[NodeId], fired: &Fired) -> bool {
        let taken = {
            let mut board = self.0.borrow_mut();
            let rank = |owner: &Option<NodeId>| match owner {
                Some(node) => path.iter().position(|n| n == node),
                None => Some(path.len()),
            };
            let at = board
                .handlers
                .iter()
                .enumerate()
                .filter(|(_, (_, c, _))| *c == fired.command)
                .filter_map(|(at, (owner, _, _))| rank(owner).map(|rank| (rank, at)))
                .min()
                .map(|(_, at)| at);
            at.map(|at| board.handlers.remove(at))
        };
        if let Some((owner, command, mut handler)) = taken {
            handler(fired, store);
            let mut board = self.0.borrow_mut();
            // One registered for the same owner and command while it ran wins.
            if !board.handlers.iter().any(|(o, c, _)| *o == owner && *c == command) {
                board.handlers.push((owner, command, handler));
            }
            return true;
        }
        let app = self.0.borrow_mut().app.take();
        let Some(mut handler) = app else { return false };
        handler(fired, store);
        let mut board = self.0.borrow_mut();
        // An app handler set while this one ran replaces it.
        if board.app.is_none() {
            board.app = Some(handler);
        }
        true
    }

    /// [`CommandBoard::run`] from the node `cx` builds.
    pub(crate) fn run_from(&self, cx: &mut BuildCx<'_>, fired: &Fired) -> bool {
        let path = self.path(cx.runtime(), Some(cx.node()));
        let modal = ModalAt::of(cx.runtime(), Some(cx.node()));
        let focused = cx.provided::<Context>().map_or(FocusedEdits::NONE, |ctx| ctx.focused_edits());
        self.run(cx, &path, fired, modal, focused)
    }

    /// The gate's key cell, made on first use.
    fn gate_cell(&self, store: &mut Store) -> State<u128> {
        let mut board = self.0.borrow_mut();
        *board.gate_cell.get_or_insert_with(|| store.create(0))
    }

    /// Sets the app's gate over commands (`None`: every command passes): a command it refuses resolves disabled and
    /// does not fire. The nodes that resolve commands run again only when the gate's key changes.
    pub fn set_gate(&self, cx: &mut impl Writes, gate: Option<CommandGate>) {
        let cell = self.gate_cell(cx.store_mut());
        let key = gate.as_ref().map_or(0, CommandGate::key);
        self.0.borrow_mut().gate = gate;
        cell.set_if_changed(cx, key);
    }

    /// Whether something besides the command's own state refuses it: the gate, or the modal rule where `modal` says
    /// the resolving or firing node stands. `declared`: [`CommandBoard::declared`] there.
    fn refused(&self, command: &CommandRef, declared: bool, modal: ModalAt) -> bool {
        let board = self.0.borrow();
        if board.gate.as_ref().is_some_and(|gate| !gate.allows(command)) {
            return true;
        }
        if modal == ModalAt::Free {
            return false;
        }
        modal.refuses(board.modal_rule(command), declared)
    }

    /// Whether `command` is declared where it resolves or fires (`ModalRule::Declared`): `instance`, the instance of
    /// its scope it resolves for, publishes its state, or it is an edit the focused stop takes (`focused`).
    fn declared(&self, command: &CommandRef, instance: Option<Id>, focused: FocusedEdits) -> bool {
        let board = self.0.borrow();
        (instance.is_some() && board.states.iter().any(|e| e.command == *command && e.instance == instance))
            || board.edits.iter().any(|(c, edit)| c == command && focused.edits.contains(*edit))
    }

    /// The edit command `command` is, when the board's edit sets list it.
    pub(crate) fn edit_command(&self, command: &CommandRef) -> Option<EditCommand> {
        self.0.borrow().edits.iter().find(|(c, _)| c == command).map(|(_, edit)| *edit)
    }

    /// Runs `handler` for every fired command that has no handler of its own: the app scope's
    /// commands. The app host passes `App::logic` here; a later call replaces the handler.
    pub fn on_app(&self, handler: impl FnMut(&Fired, &mut Store) + 'static) {
        self.0.borrow_mut().app = Some(Box::new(handler));
    }

    /// The cell of `command`'s state for `instance`, created enabled on first use.
    pub(crate) fn cell(&self, store: &mut Store, command: &CommandRef, instance: Option<Id>) -> State<CommandState> {
        let mut board = self.0.borrow_mut();
        if let Some(entry) = board.states.iter().find(|e| e.command == *command && e.instance == instance) {
            return entry.cell;
        }
        let cell = store.create(CommandState::new());
        board.states.push(StateEntry { command: command.clone(), instance, cell, value: CommandState::new() });
        cell
    }

    /// The value of `command`'s state for `instance`, read without recording (the caller logs the read).
    fn value(&self, command: &CommandRef, instance: Option<Id>) -> CommandState {
        let board = self.0.borrow();
        board.states.iter().find(|e| e.command == *command && e.instance == instance).map_or_else(CommandState::new, |e| e.value.clone())
    }

    /// The instance of `command`'s scope its state is kept for, seen from `chain`: `Some(None)` for a command that
    /// resolves at the app, `None` for a scoped command with no instance in `chain`.
    fn instance_in(&self, chain: &CommandChain, command: &CommandRef) -> Option<Option<Id>> {
        let (scope, app) = self.scope_of(command);
        if app {
            return Some(None);
        }
        let board = self.0.borrow();
        chain.innermost_where(|of| board.within(of, &scope)).map(Some)
    }

    /// Declares that an instance of `child` also receives the commands of `parent`, and that its shortcuts rank inside
    /// the parent's; the nodes that resolve commands run again when this changes what a scope receives.
    pub(crate) fn declare_child(&self, cx: &mut impl Writes, child: CommandScope, parent: CommandScope) {
        let cell = self.parents_cell(cx.store_mut());
        {
            let mut board = self.0.borrow_mut();
            let entry = (child.name(), parent.name());
            if board.parents.contains(&entry) {
                return;
            }
            board.parents.retain(|(c, _)| *c != entry.0);
            board.parents.push(entry);
        }
        let generation = *cx.store_mut().peek(cell) + 1;
        cell.set(cx, generation);
    }

    fn parents_cell(&self, store: &mut Store) -> State<u64> {
        let mut board = self.0.borrow_mut();
        *board.parents_cell.get_or_insert_with(|| store.create(0))
    }

    /// Sets `command`'s state; the nodes that read it run in the next frame, and only they.
    pub fn set_state(&self, cx: &mut impl Writes, command: impl Into<CommandRef>, state: CommandState) {
        self.set_state_of(cx, &command.into(), None, state);
    }

    /// Sets `command`'s state for `instance` of its scope (`Commands::publish`).
    pub(crate) fn set_state_of(&self, cx: &mut impl Writes, command: &CommandRef, instance: Option<Id>, state: CommandState) {
        let cell = self.cell(cx.store_mut(), command, instance);
        {
            let mut board = self.0.borrow_mut();
            let Some(entry) = board.states.iter_mut().find(|e| e.command == *command && e.instance == instance) else { return };
            if entry.value == state {
                return;
            }
            entry.value = state.clone();
        }
        cell.set(cx, state);
    }

    /// Publishes `command`'s state for the innermost instance of its scope in `chain` (`Commands::publish`): the write
    /// reaches the cell when the running node's build ends. A scoped command with no instance in `chain` publishes
    /// nothing.
    pub(crate) fn publish(&self, chain: &CommandChain, command: &CommandRef, state: CommandState) {
        if let Some(instance) = self.instance_in(chain, command) {
            crate::contract::context::log_publish(self.clone(), command.clone(), instance, state);
        }
    }

    /// `command`'s state, read through its cell: a node that reads it runs when it changes.
    pub fn state<C: Reads + Writes>(&self, cx: &mut C, command: impl Into<CommandRef>) -> CommandState {
        let command = command.into();
        let cell = self.cell(cx.store_mut(), &command, None);
        cell.get(cx)
    }

    /// Whether `command` may fire, read without recording: no state of it is disabled.
    pub(crate) fn enabled(&self, command: &CommandRef) -> bool {
        self.0.borrow().states.iter().filter(|e| e.command == *command).all(|e| e.value.is_enabled())
    }

    /// The home scope of `command` (the app scope for a command no linked set defines), and whether it resolves at
    /// the app: the app scope and the held scopes.
    fn scope_of(&self, command: &CommandRef) -> (String, bool) {
        let board = self.0.borrow();
        let scope = board.registry.as_ref().and_then(|registry| registry.get(command.id.as_str())).map(|def| def.scope.as_str().to_owned());
        let scope = scope.unwrap_or_else(|| CommandScope::APP.name().to_owned());
        let app = scope == CommandScope::APP.name() || board.registry.as_ref().is_some_and(|registry| registry.is_held_scope(&scope));
        (scope, app)
    }

    /// `command` as a surface in `chain` shows it: its label, icon and chord, and its state for the innermost instance
    /// of its scope in `chain`, read through the state's cell (a node that shows it runs when it changes). `None` for a
    /// scoped command with no instance in `chain`.
    /// `focused`: what the window's focused stop declares, which the caller read tracked for an edit command
    /// ([`FocusedEdits::get`]).
    pub(crate) fn resolve<C: Reads + Writes>(
        &self,
        cx: &mut C,
        chain: &CommandChain,
        command: &CommandRef,
        modal: ModalAt,
        focused: FocusedEdits,
    ) -> Option<Arc<ResolvedCommand>> {
        let instance = self.instance_in(chain, command)?;
        let cell = self.cell(cx.store_mut(), command, instance);
        let state = cell.get(cx);
        self.gate_cell(cx.store_mut()).get(cx);
        self.parents_cell(cx.store_mut()).get(cx);
        Some(self.resolved(command, instance, &state, modal, focused))
    }

    /// [`CommandBoard::resolve`] for code with no store at hand (a hook's `Commands::resolve_command`): the reads of
    /// the state's cell, the gate's and, for an edit command, the focused stop's declaration are logged for the node
    /// whose build runs. `modal` is where that node stands.
    pub(crate) fn resolve_logged(&self, ctx: &Context, chain: &CommandChain, command: &CommandRef, modal: ModalAt) -> Option<Arc<ResolvedCommand>> {
        let instance = self.instance_in(chain, command)?;
        crate::contract::context::log_command_read(self.clone(), command.clone(), instance);
        let board = self.clone();
        let edit = self.edit_command(command).is_some();
        // The modal rule's answer moves with the window's layers: a node that stands under one reads the open-layer
        // cell and runs again when the stack changes.
        let layered = modal != ModalAt::Free || self.0.borrow().modal_rule(command) != ModalRule::Always;
        crate::contract::context::log_build_read(move |cx| {
            board.gate_cell(cx.store_mut()).get(cx);
            board.parents_cell(cx.store_mut()).get(cx);
            if layered {
                super::layers::track(cx);
            }
            if edit && let Some(ctx) = cx.provided::<Context>() {
                FocusedEdits::get(&ctx, cx);
            }
        });
        let focused = if edit { ctx.focused_edits() } else { FocusedEdits::NONE };
        Some(self.resolved(command, instance, &self.value(command, instance), modal, focused))
    }

    fn resolved(
        &self,
        command: &CommandRef,
        instance: Option<Id>,
        state: &CommandState,
        modal: ModalAt,
        focused: FocusedEdits,
    ) -> Arc<ResolvedCommand> {
        let refused = self.refused(command, self.declared(command, instance, focused), modal);
        // An edit the focused stop declares is the stop's to do: enabled unless the stop marks it inactive, with the
        // app's title and without the app's hint, which describes the app's own action.
        let declared = self.edit_command(command).and_then(|edit| focused.declares(edit));
        let board = self.0.borrow();
        let def = board.registry.as_ref().and_then(|registry| registry.get(command.id.as_str()));
        let chord = board.keymap.shown_chord(command);
        let chord_text = chord.map(|chord| Arc::from(chord.display(board.keymap.platform())));
        let flags = state.flags();
        Arc::new(ResolvedCommand {
            label: state.title_ref().cloned().unwrap_or_else(|| Arc::from(def.map_or(command.id.as_str(), |def| def.label.as_str()))),
            catalog_label: def.map(|def| Arc::from(def.label.as_str())),
            icon: state.icon_ref().cloned().or_else(|| def.and_then(|def| def.icon.clone())),
            chord,
            chord_text,
            enabled: declared.unwrap_or(flags.enabled) && !refused,
            refused,
            checked: flags.checked,
            marked: state.is_marked(),
            hint: if declared.is_some() { None } else { state.hint_ref().cloned() },
            hint_alone: state.is_hint_alone(),
            kind: def.map_or(CommandKind::Command, |def| def.kind.clone()),
            edit: board.edits.iter().any(|(c, _)| c == command),
            command: Arc::new(command.clone()),
            instance,
        })
    }

    /// The command `chord` runs at the app scope, when it is bound to one.
    pub(crate) fn holder(&self, chord: &Chord) -> Option<CommandRef> {
        match self.0.borrow().keymap.lookup(chord, false, &Active::NONE) {
            Lookup::Run(command) => Some(command),
            _ => None,
        }
    }

    /// The edit command `chord` is bound to, when it is one.
    pub(crate) fn edit_of(&self, chord: &Chord) -> Option<EditCommand> {
        let board = self.0.borrow();
        let Lookup::Run(command) = board.keymap.lookup(chord, false, &Active::NONE) else { return None };
        board.edits.iter().find(|(c, _)| *c == command).map(|(_, edit)| *edit)
    }

    /// Runs `command`'s handler as fired by `source` at `time` from outside the tree into `fire`'s window (the
    /// harness), when the command is enabled; whether a handler ran.
    pub(crate) fn fire(&self, fire: &mut OutsideFire<'_>, command: CommandRef, source: FireSource, time: f64) -> bool {
        let fired = Fired::new(command, source, FirePhase::Press, time);
        self.run(fire, &[], &fired, ModalAt::Free, FocusedEdits::NONE)
    }

    /// The command a press of `chord` runs: the enabled command the keymap binds it to that the modal rule lets
    /// through where the focused stop stands (`modal`, `focused`; `ModalAt::Free` asks only whether the chord is
    /// bound), the first of a tool group that shares the chord (the app's handler picks the next). `typing`: the
    /// focused stop types, so the keymap's typing rule holds back the chords that type.
    pub(crate) fn command_for(&self, chord: &Chord, typing: bool, modal: ModalAt, focused: FocusedEdits) -> Option<CommandRef> {
        let lookup = {
            let board = self.0.borrow();
            let active = Active { panel: None, contexts: &[], typing };
            let accept = |command: &CommandRef| self.enabled(command) && !self.refused(command, self.declared(command, None, focused), modal);
            board.keymap.lookup_where(chord, false, &active, &[], &accept)
        };
        match lookup {
            Lookup::Run(command) => Some(command),
            Lookup::Cycle(group) => group.into_iter().next(),
            _ => None,
        }
    }

    /// Runs `command` as fired by the key press `k`, along the focused node's path; whether a handler ran. `modal`
    /// and `focused` are where the focused stop stands, as the key's command lookup found them.
    pub(crate) fn run_key(&self, cx: &mut EventCx<'_>, command: CommandRef, k: &KeyInput, modal: ModalAt, focused: FocusedEdits) -> bool {
        let phase = if k.repeat { FirePhase::Repeat } else { FirePhase::Press };
        let fired = Fired::new(command, FireSource::Key, phase, cx.time()).mods(k.mods);
        let path = self.path(cx.runtime(), cx.focused());
        self.run(cx, &path, &fired, modal, focused)
    }
}

impl Board {
    /// Whether an instance of `scope` is also an instance of `ancestor`: they are one scope, or `ancestor` is among the
    /// scopes `scope` was declared a child of.
    fn within(&self, scope: &str, ancestor: &str) -> bool {
        let mut at = scope;
        for _ in 0..=self.parents.len() {
            if at == ancestor {
                return true;
            }
            match self.parents.iter().find(|(child, _)| *child == at) {
                Some((_, parent)) => at = parent,
                None => return false,
            }
        }
        false
    }

    /// `command`'s modal rule: its definition in the linked command sets, else in the keymap's catalog, else the
    /// default rule.
    fn modal_rule(&self, command: &CommandRef) -> ModalRule {
        let linked = self.registry.as_ref().and_then(|registry| registry.get(command.id.as_str())).map(|def| def.modal);
        linked.or_else(|| self.keymap.catalog().iter().find(|def| def.command == *command).map(|def| def.modal)).unwrap_or_default()
    }
}

/// The commands a node holds (`Ui::hold_commands`), in the node's coordinates.
#[derive(Clone)]
struct ShownFact {
    node: NodeId,
    commands: Arc<[CommandRef]>,
    local: craft_ui_rt::Rect,
    label: Label,
}

/// Every node's held commands in one window, kept in the window's `Context`.
#[derive(Default)]
struct ShownFacts(Vec<ShownFact>);

/// The window root's command stage, after Tab and the pane keys: the command a key press no node took is bound to
/// runs, and the text the platform sends with the press does not type. A plain Escape is the scopes' (ui-rt dismisses
/// the topmost layer child that asks); an Escape with Shift or Alt that the keymap binds runs its command. Quit, run
/// from its key, then asks the windows to close through their close path, so the app's close guard asks about unsaved
/// work as for a window's close button.
pub(crate) fn window_command(root: &Kept, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
    if !k.down || k.key == Key::Tab || (k.key == Key::Escape && k.mods.is_none()) {
        return Handled::No;
    }
    let Some(board) = root.commands.clone() else { return Handled::No };
    let keys = cx.focused().and_then(|f| focused_keys(cx, f));
    let typing = keys.is_some_and(|keys| {
        let typing = matches!(keys.owner, OwnerClass::Text | OwnerClass::Recorder);
        // A stop with `command_keys` lets command chords and function keys run while it types.
        let command_key = Chord::new(k.mods, k.key).has_command() || is_function(k.key);
        typing && !(keys.command_keys && command_key)
    });
    // Where the focused stop stands against the modal layers: worked out once for the key, for both the lookup and
    // the fire.
    let modal = ModalAt::of(cx.runtime(), cx.focused());
    note_modal_check();
    let focused = FocusedEdits::of(keys);
    let Some(command) = board.command_for(&Chord::new(k.mods, k.key), typing, modal, focused) else { return Handled::No };
    board.run_key(cx, command, k, modal, focused);
    cx.withhold_text();
    Handled::Yes
}

#[cfg(feature = "testing")]
thread_local! {
    /// How many times the window's command stage worked out where a key stands against the modal layers.
    static MODAL_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn note_modal_check() {
    #[cfg(feature = "testing")]
    MODAL_CHECKS.with(|checks| checks.set(checks.get() + 1));
}

/// How many times the window's command stage worked out where a key stands against the modal layers on this thread:
/// one per key it handles (count trials).
#[cfg(feature = "testing")]
#[doc(hidden)]
pub fn modal_checks() -> usize {
    MODAL_CHECKS.with(std::cell::Cell::get)
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

    /// Runs `handler` when `command` fires from this node or a node under it, for as long as this
    /// node stays. Calling it again from a later run of the node replaces the handler; it never adds
    /// a second one. Nothing happens in a measuring run or with no board.
    pub fn on_command(&mut self, command: impl Into<CommandRef>, handler: impl FnMut(&Fired, &mut Store) + 'static) {
        if self.is_measuring() {
            return;
        }
        if let Some(board) = self.cx.provided::<CommandBoard>() {
            let node = self.cx.node();
            board.register(Some(node), command.into(), Box::new(handler));
        }
    }

    /// Fires `command` from this node now: the handler of the nearest node on its path that registered
    /// one, else the board's, runs at once when the command is enabled. True when a handler ran.
    pub fn fire(&mut self, command: impl Into<CommandRef>, source: FireSource) -> bool {
        if self.is_measuring() {
            return false;
        }
        let Some(board) = self.cx.provided::<CommandBoard>() else { return false };
        let fired = Fired::new(command.into(), source, FirePhase::Press, self.cx.now());
        board.run_from(self.cx, &fired)
    }

    /// `command` as this scope shows it: its label, icon, chord and state for the innermost instance of its scope
    /// around this cursor, read through the state's cell (this scope runs again when it changes). `None` for a scoped
    /// command with no instance around, or with no board.
    pub fn resolve_command(&mut self, command: impl Into<CommandRef>) -> Option<Arc<ResolvedCommand>> {
        let board = self.cx.provided::<CommandBoard>()?;
        let chain = self.env.view().command_chain().clone();
        super::layers::track(self.cx);
        let modal = ModalAt::of(self.cx.runtime(), Some(self.cx.node()));
        let command = command.into();
        let focused = self.focused_edits_for(&board, &command);
        board.resolve(self.cx, &chain, &command, modal, focused)
    }

    /// What the window's focused stop declares, read tracked only for an edit command: a surface that shows other
    /// commands does not run when focus moves.
    fn focused_edits_for(&mut self, board: &CommandBoard, command: &CommandRef) -> FocusedEdits {
        if board.edit_command(command).is_some() { FocusedEdits::get(&self.ctx, self.cx) } else { FocusedEdits::NONE }
    }

    /// `command` as [`Ui::resolve_command`] shows it from the window under the modal layer `layer` (this cursor's own
    /// modal layer, which closes before the command runs: the command palette): that layer does not block it.
    pub fn resolve_beneath(&mut self, layer: Id, command: impl Into<CommandRef>) -> Option<Arc<ResolvedCommand>> {
        let board = self.cx.provided::<CommandBoard>()?;
        let chain = self.env.view().command_chain().clone();
        let modal = self.modal_beneath(layer);
        let command = command.into();
        let focused = self.focused_edits_for(&board, &command);
        board.resolve(self.cx, &chain, &command, modal, focused)
    }

    /// Fires `command` as [`Ui::fire`] does, judged as [`Ui::resolve_beneath`] judges it, for a surface that closes the
    /// modal layer `layer` with the pick. Focus returns from the layer first, so the command reaches the stop focus
    /// returns to, as a menu pick's does. True when a handler ran.
    pub fn fire_beneath(&mut self, layer: Id, command: impl Into<CommandRef>, source: FireSource) -> bool {
        if self.is_measuring() {
            return false;
        }
        let Some(board) = self.cx.provided::<CommandBoard>() else { return false };
        let modal = self.modal_beneath(layer);
        self.return_from_pick(crate::runtime::focus::PickedIn::Layer(layer));
        let fired = Fired::new(command.into(), source, FirePhase::Press, self.cx.now());
        let from = self.cx.runtime().focused(self.cx.window()).unwrap_or_else(|| self.cx.node());
        let path = board.path(self.cx.runtime(), Some(from));
        let focused = self.ctx.focused_edits();
        board.run(self.cx, &path, &fired, modal, focused)
    }

    /// Where this cursor stands against the modal layers below the modal layer `layer`, as the window is once it
    /// closes.
    fn modal_beneath(&mut self, layer: Id) -> ModalAt {
        super::layers::track(self.cx);
        let rt = self.cx.runtime();
        let open = super::layers::open(rt, self.cx.window());
        let below: Vec<_> = open
            .iter()
            .take_while(|o| o.entry.id != layer)
            .filter(|o| super::layers::counts(&o.entry, crate::runtime::overlay::OpenFilter::Modal))
            .collect();
        match below.last() {
            None => ModalAt::Free,
            Some(top) if super::layers::layer_child_of(rt, self.cx.node()) == Some(top.node) => ModalAt::Inside,
            Some(_) => ModalAt::Outside,
        }
    }

    /// Records that this node shows `commands` (a menu title holds its menu's commands) at `rect`, with `label`, for
    /// `Ui::command_shown`: one record per node, replaced by its next run and gone with the node. Nothing in a
    /// measuring run.
    pub fn hold_commands(&mut self, rect: craft_ui_rt::Rect, commands: &Arc<[CommandRef]>, label: Label) {
        if self.is_measuring() || commands.is_empty() {
            return;
        }
        let node = self.cx.node();
        let local = rect.translate(-self.flow.origin.to_vec2());
        let fact = ShownFact { node, commands: Arc::clone(commands), local, label };
        self.ctx.service::<ShownFacts, _>(|facts| {
            facts.0.retain(|f| f.node != node);
            facts.0.push(fact);
        });
    }

    /// Where `command` shows in this window: the rect of a node that holds it, now, unless the node lies under the
    /// window's top modal layer. Untracked: a surface asks it when it acts (automation, a key tip).
    pub fn command_shown(&mut self, command: &CommandRef) -> Option<Shown> {
        let rt = self.cx.runtime();
        let window = self.cx.window();
        let top = super::layers::top_modal(rt, window).map(|top| top.node);
        let facts: Vec<ShownFact> = self.ctx.service::<ShownFacts, _>(|facts| {
            facts.0.retain(|f| rt.window_of(f.node).is_some());
            facts.0.iter().filter(|f| f.commands.contains(command)).cloned().collect()
        });
        facts.into_iter().find_map(|fact| {
            if top.is_some() && super::layers::layer_child_of(rt, fact.node) != top {
                return None;
            }
            let origin = rt.window_rect(fact.node)?.min.to_vec2();
            Some(Shown { rect: fact.local.translate(origin), window, label: fact.label, holder: true })
        })
    }

    /// The command `chord` runs at the app scope, when it is bound to one.
    pub fn command_holder(&self, chord: &Chord) -> Option<CommandRef> {
        self.cx.provided::<CommandBoard>()?.holder(chord)
    }

    /// Declares that an instance of `child` also receives the commands of `parent`, and that its shortcuts rank inside
    /// the parent's (the workspace scope of a full-window dialog is a child of the view scope). Nothing in a measuring
    /// run or with no board.
    pub fn declare_command_scope(&mut self, child: CommandScope, parent: CommandScope) {
        if self.is_measuring() {
            return;
        }
        if let Some(board) = self.cx.provided::<CommandBoard>() {
            board.declare_child(self.cx, child, parent);
        }
    }

    /// The keymap's generation, read through its cell: this scope runs again when the keymap changes. 0 with no board.
    pub fn keymap_generation(&mut self) -> u64 {
        match self.cx.provided::<CommandBoard>() {
            Some(board) => board.keymap_generation(self.cx),
            None => 0,
        }
    }
}
