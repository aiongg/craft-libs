//! The command board: the registry and typed sets, per-frame state, the queue, firing and the gate (spec 11).

mod board;
mod clipboard;
mod library;
mod registry;

pub use clipboard::{ClipContent, ClipError, ClipItem, ClipboardHandle};
pub use craft_keymap::{Binding, CommandDef, CommandKind, CommandRef, CommandScope, ModalRule};
pub use library::{EditCommands, WindowCommand};
pub use registry::CommandDefError;
#[doc(hidden)]
pub use registry::{COMMAND_SETS, SetEntry};

pub(crate) use board::{begin, edit_chords, end, install, modal_open, note_focus, platform, register_instance, set_input, take_fired};
pub(crate) use board::{fire_bound, resolve_bound};

use craft_behavior::CommandFlags;
use craft_keymap::{Chord, Overrides};
use egui::{Context, Id};
use std::borrow::Cow;
use std::sync::Arc;

/// A typed set of commands, written by `#[derive(CommandSet)]`.
pub trait CommandSet: Copy + Send + Sync + 'static {
    /// The scope the set's commands belong to.
    const SCOPE: CommandScope;

    /// The command of this set that `command` names.
    fn from_ref(command: &CommandRef) -> Option<Self>;

    #[doc(hidden)]
    fn id(self) -> &'static str;

    #[doc(hidden)]
    fn defs() -> Result<Vec<CommandDef>, CommandDefError>;
}

/// A command a surface or a key fired, as the instance or the app takes it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Fired {
    /// The command.
    pub command: CommandRef,
    /// The scope instance the command was routed to; `None` for app-scope commands.
    pub target: Option<Id>,
    /// Where it came from.
    pub source: FireSource,
    /// The key phase that fired it.
    pub phase: FirePhase,
    /// Tools of one group share the chord: the app picks the next one.
    pub cycle: bool,
    /// The chord pressed was a bound chord of the command with Alt added, and the command opted in.
    pub alt_variant: bool,
}

/// Where a fired command came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FireSource {
    /// A shortcut.
    Key,
    /// A menu row.
    Menu,
    /// The command palette.
    Palette,
    /// A component the pointer or the keyboard activated.
    Pointer,
    /// An automation script.
    Automation,
}

/// The phase of the key press that fired a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FirePhase {
    /// The press.
    Press,
    /// A repeat of a held key.
    Repeat,
    /// The release; momentary tools act here.
    Release,
}

/// What the app or an instance says about a command this frame.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CommandState {
    flags: CommandFlags,
    title: Option<Arc<str>>,
    hint: Option<Arc<str>>,
}

impl CommandState {
    /// An enabled, unchecked-less state with the command's own label.
    pub fn new() -> Self {
        CommandState::default()
    }

    /// Whether the command can fire.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.flags.enabled = enabled;
        self
    }

    /// The checked state of a toggle or radio command.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.flags.checked = Some(checked);
        self
    }

    /// A label for this frame in place of the command's own, such as "Undo Move".
    #[must_use]
    pub fn title(mut self, title: impl Into<Arc<str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The accessible description and second tooltip line.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<Arc<str>>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub(crate) fn flags(&self) -> CommandFlags {
        self.flags
    }

    pub(crate) fn title_ref(&self) -> Option<&Arc<str>> {
        self.title.as_ref()
    }

    pub(crate) fn hint_ref(&self) -> Option<&Arc<str>> {
        self.hint.as_ref()
    }
}

/// A command as a surface shows and fires it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedCommand {
    /// The state's title, else the command's label, else its id.
    pub label: Arc<str>,
    /// The icon name.
    pub icon: Option<Cow<'static, str>>,
    /// The first single-press binding.
    pub chord: Option<Chord>,
    /// The state ANDed with the gate, the modal rule and the instance's liveness.
    pub enabled: bool,
    /// The checked state of a toggle or radio command.
    pub checked: Option<bool>,
    /// The accessible description and second tooltip line.
    pub hint: Option<Arc<str>>,
    /// What the command is.
    pub kind: CommandKind,
    pub(crate) command: CommandRef,
    pub(crate) instance: Option<Id>,
}

/// The app's one gate over commands: a dialog is open, a workspace is not ready, a recorder has the keys or an
/// operation is pending. A command the gate refuses resolves disabled and does not fire.
#[derive(Clone)]
pub struct CommandGate(Arc<dyn Fn(&CommandRef) -> bool + Send + Sync>);

impl CommandGate {
    /// A gate that lets `allows` commands through.
    pub fn new(allows: impl Fn(&CommandRef) -> bool + Send + Sync + 'static) -> Self {
        CommandGate(Arc::new(allows))
    }

    pub(crate) fn allows(&self, command: &CommandRef) -> bool {
        (self.0)(command)
    }
}

/// App-scope command state for this frame, set from app state before the pass.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct CommandInput {
    pub(crate) states: Vec<(CommandRef, CommandState)>,
    pub(crate) overrides: Option<Arc<Overrides>>,
    pub(crate) gate: Option<CommandGate>,
}

impl CommandInput {
    /// No state, no overrides and no gate.
    pub fn new() -> Self {
        CommandInput::default()
    }

    /// The state of an app-scope command.
    #[must_use]
    pub fn state(mut self, command: impl Into<CommandRef>, state: CommandState) -> Self {
        self.states.push((command.into(), state));
        self
    }

    /// The user's binding overrides; the keymap is rebuilt when the `Arc` changes.
    #[must_use]
    pub fn overrides(mut self, overrides: Arc<Overrides>) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// The one gate over commands.
    #[must_use]
    pub fn gate(mut self, gate: CommandGate) -> Self {
        self.gate = Some(gate);
        self
    }
}

/// The chain of container instances that receive scoped commands, innermost last. A private inherited field of
/// the env.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct CommandChain(Arc<[(CommandScope, Id)]>);

impl CommandChain {
    pub(crate) fn pushed(&self, scope: CommandScope, id: Id) -> Self {
        CommandChain(self.0.iter().copied().chain([(scope, id)]).collect())
    }

    /// The innermost instance of `scope`.
    pub(crate) fn innermost(&self, scope: &str) -> Option<Id> {
        self.0.iter().rev().find(|(s, _)| s.name() == scope).map(|(_, id)| *id)
    }

    pub(crate) fn ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.0.iter().map(|(_, id)| *id)
    }
}

/// How the contexts reach the board. Sealed: only the pipeline's contexts implement it.
#[doc(hidden)]
pub trait CommandAccess: crate::runtime::memory::sealed::Sealed {
    /// The egui context.
    fn command_ctx(&self) -> &Context;
    /// The instances above this context.
    fn command_chain(&self) -> &CommandChain;
    /// The component this context belongs to, when it has an id of its own.
    fn command_owner(&self) -> Option<Id>;
}

/// The chapter-11 context trait that `Cx`, `BodyCx` and `ItemCx` implement. "This instance" is the innermost
/// container at or above the context whose `ContainerProps.command_scope` is the command's scope.
pub trait Commands: CommandAccess {
    /// The one clipboard path.
    fn clipboard(&self) -> ClipboardHandle {
        ClipboardHandle::new(self.command_ctx(), self.command_owner())
    }

    /// This frame's state of a scoped command, for this instance. Publishing an app-scope command here is a
    /// debug assertion.
    fn publish(&mut self, command: impl Into<CommandRef>, state: CommandState) {
        board::publish(self.command_ctx(), self.command_chain(), &command.into(), state);
    }

    /// The next command of set `S` routed to this instance this pass; each is taken once, so a discard pass does
    /// not repeat it.
    fn take_command<S: CommandSet>(&mut self) -> Option<(S, Fired)> {
        board::take(self.command_ctx(), self.command_chain().innermost(S::SCOPE.name()), S::from_ref)
    }
}

/// Fires `command` from a surface that has no component of its own: a menu, the palette or automation. Scoped
/// commands go to the live instance that contains key focus, else the one that contained it most recently, else
/// the first live one. Returns whether the command was queued: a disabled command, or a scoped command with no
/// live instance, is not.
pub fn fire(ctx: &Context, command: impl Into<CommandRef>, source: FireSource) -> bool {
    board::fire_from(ctx, &command.into(), source)
}

/// `command` as a surface with no component of its own shows it: the label, icon, chord, enabled and checked state
/// and hint, against the instance that [`fire`] would deliver it to. A scoped command with no live instance
/// resolves disabled.
pub fn resolve(ctx: &Context, command: impl Into<CommandRef>) -> ResolvedCommand {
    board::resolve_global(ctx, &command.into())
}

/// Declares that a modal layer is open in some viewport, so `ModalRule::Blocked` commands do not fire.
// F5-LATER: the overlay host calls this when a `Rank::Modal` layer opens or closes
#[expect(dead_code, reason = "called by the overlay host (F5)")]
pub(crate) fn set_modal_open(ctx: &Context, open: bool) {
    board::set_modal_open(ctx, open);
}
