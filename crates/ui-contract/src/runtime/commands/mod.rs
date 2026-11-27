//! The command board: the registry and typed sets, per-frame state, the queue, firing and the gate (spec 11).

pub(crate) mod inbox;
pub(crate) mod keymap;
pub use crate::contract::rt::commands::CommandBoard;
pub use inbox::InboxIn;
pub(crate) use inbox::{InboxCells, register as register_inboxes};
mod clipboard;
mod library;
mod registry;
pub(crate) use registry::Registry;

pub use clipboard::{ClipContent, ClipError, ClipItem, ClipboardHandle};
pub use craft_behavior::EditCommand;
pub use craft_keymap::{Binding, CommandDef, CommandKind, CommandRef, CommandScope, ModalRule, Platform, Scope, Trigger, TypingRule};
pub use library::{ChromeCommand, ChromeToggle};
pub use library::{EditCommands, NavCommand, QuitCommand, WindowCommand};
pub(crate) use library::{WindowShortcut, yields_default};
pub use registry::CommandDefError;
#[doc(hidden)]
pub use registry::{SetEntry, sets};
#[cfg(feature = "suite")]
pub(crate) use registry::{StorySelection, configure_story_sets, with_story_sets};

pub use keymap::linked_keymap;
pub(crate) use keymap::{BuiltKeymap, build_keymap, keymap_for};
#[cfg(feature = "testing")]
#[doc(hidden)]
pub use keymap::{chord_text_builds, keymap_builds};

use crate::contract::{Context, MemoKey, MemoProps, PlainData, Revision, Tracked};
use crate::primitives::Id;
use crate::runtime::env::Label;
use craft_behavior::CommandFlags;
use craft_behavior::edit_history::Step;
use craft_keymap::{Chord, Overrides};
use craft_ui_rt::WindowId;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Commands an app defines at run time (plug-ins, its tool list), all in the app scope ([`Scope::app`]). They join
/// the linked command sets: their chords fire them into `FrameOutput.fired`, and surfaces resolve their label, icon,
/// kind and chord. An id a linked set already defines stays that set's.
#[derive(Clone)]
pub struct AppCatalog {
    defs: Arc<[CommandDef]>,
    index: Arc<HashMap<String, usize>>,
    generation: u64,
    /// The default chords the app gives a linked set's command in place of the set's own (`replace_defaults`).
    replaced: Arc<[(String, Vec<Binding>)]>,
    /// Which chords fire while a text component has the keyboard (`typing_rule`); `None` keeps the keymap's default.
    typing: Option<TypingRule>,
}

impl AppCatalog {
    /// A catalog of `defs`; `generation` changes when the app's rows do. Refuses two defs with one id (`Duplicate`) and a
    /// def in another scope (`NotApp`).
    pub fn new(defs: Vec<CommandDef>, generation: u64) -> Result<Self, CommandDefError> {
        let mut index = HashMap::with_capacity(defs.len());
        for (at, def) in defs.iter().enumerate() {
            if !def.scope.is_app() {
                return Err(CommandDefError::NotApp { command: def.command.id.clone() });
            }
            registry::index_id(&mut index, &def.command.key_text(), at)?;
        }
        Ok(AppCatalog { defs: defs.into(), index: Arc::new(index), generation, replaced: Arc::from([]), typing: None })
    }

    /// Gives the linked set's command `id` the default chords `bindings` in place of the set's own; an empty list drops
    /// them. A component's chord that the app means for its own command (a canvas's `Ctrl+=` zoom where Word's
    /// `Ctrl+=` is Subscript) is dropped here, so the app's binding takes the key. They are defaults: the user's
    /// overrides still replace them, and resetting a command returns to them. A later call for the same id replaces
    /// the earlier one. An id no linked set defines is ignored; an app command's defaults are its def's.
    #[must_use]
    pub fn replace_defaults(mut self, id: impl Into<String>, bindings: Vec<Binding>) -> Self {
        let id = id.into();
        let mut replaced: Vec<(String, Vec<Binding>)> = self.replaced.iter().filter(|(at, _)| *at != id).cloned().collect();
        replaced.push((id, bindings));
        self.replaced = replaced.into();
        self
    }

    /// Sets which chords fire while a text component has the keyboard. The keymap's default,
    /// [`TypingRule::CommandKeysOnly`], leaves Alt chords to the text; an app whose document editor fires Alt+letter
    /// commands while the person types (a word processor's Alt+Q) gives [`TypingRule::PlainKeysWait`]. It holds for
    /// every text component of the app's windows; a binding in an active context still always works.
    #[must_use]
    pub fn typing_rule(mut self, rule: TypingRule) -> Self {
        self.typing = Some(rule);
        self
    }

    /// The default chords the app gives the linked set's command `id`, when it replaced them.
    pub(crate) fn replaced_defaults(&self, id: &str) -> Option<&[Binding]> {
        self.replaced.iter().find(|(at, _)| at == id).map(|(_, bindings)| bindings.as_slice())
    }

    /// The typing rule the app set.
    pub(crate) fn typing(&self) -> Option<TypingRule> {
        self.typing
    }

    /// The app's version of the catalog. The board rebuilds the keymap when it differs from the last frame's, and never
    /// compares the definitions, so an app gives a new generation exactly when its rows change.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The catalog's rows in the order the app gave them, each with its scope (always [`Scope::app`]), kind and
    /// default bindings, as a shortcuts list reads them.
    pub fn defs(&self) -> &[CommandDef] {
        &self.defs
    }

    /// The def with the command's key, else the def with the bare id.
    pub(crate) fn get(&self, command: &CommandRef) -> Option<&CommandDef> {
        let at = self.index.get(command.key_text().as_ref()).or_else(|| self.index.get(command.id.as_str()))?;
        self.defs.get(*at)
    }
}

/// A typed set of commands, written by `#[derive(CommandSet)]`.
pub trait CommandSet: Copy + Send + Sync + 'static {
    /// The scope the set's commands belong to.
    const SCOPE: CommandScope;

    /// The set's keys are those of a text component, meant for typing (`#[command(typing)]`): its scope is a keymap
    /// context, active while an instance of it is around the focus, so its keys come before every other scope's and
    /// work while the component types, as a type tool's formatting keys do.
    const TYPING: bool = false;

    /// The command of this set that `command` names.
    fn from_ref(command: &CommandRef) -> Option<Self>;

    #[doc(hidden)]
    fn id(self) -> &'static str;

    #[doc(hidden)]
    fn defs() -> Result<Vec<CommandDef>, CommandDefError>;

    /// The commands that have no default binding on a platform, as `(id, platform)`.
    #[doc(hidden)]
    fn no_default_on() -> Vec<(&'static str, Platform)>;

    /// The commands a focused component takes as an `Intent::Edit`, as `(id, edit)`: the `edit` option of the
    /// command's attribute.
    #[doc(hidden)]
    fn edits() -> Vec<(&'static str, EditCommand)>;

    /// The momentary commands whose held key makes a scope's keys work, as `(id, scope)`: the `holds` option of the
    /// command's attribute.
    #[doc(hidden)]
    fn holds() -> Vec<(&'static str, CommandScope)>;

    /// The defaults of one platform, as `(id, platform, binding)`: on that platform they replace the command's other
    /// defaults.
    #[doc(hidden)]
    fn platform_defaults() -> Result<Vec<(&'static str, Platform, Binding)>, CommandDefError>;
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
    /// The input time, in seconds of egui's input clock, of the event that fired it: for a key, when its press or
    /// release arrived, also when the runtime held the event back to a later frame; the pass time for the other
    /// sources. A key's hold is the time between its press and its release, however late a frame runs them.
    pub time: f64,
    /// The modifiers the firing input carried: a key's chord, a component's activation, the frame's held modifiers for
    /// an injected activation. Automation and the palette carry none.
    pub mods: craft_keymap::Mods,
    /// For a key, the place of its event among the pass's events, which the editor inputs of the pass share, so an
    /// editor runs the command between the inputs typed before and after it (`Cx::editor_steps`). `None` for the other
    /// sources and for a key whose hold settles by its deadline or a modifier change, or whose window lost focus.
    pub place: Option<usize>,
    /// The clipboard's content, for a paste the platform delivered with its event (the clipboard event that stands for
    /// the paste chord) when no focused component lists Paste: the command that event fires carries it, so the app
    /// pastes what the event held in the pass it runs the command. `None` for every other command, and for a paste
    /// whose content the app reads from the clipboard itself.
    pub pasted: Option<ClipContent>,
    /// The screen point supplied when requesting this paste, if any.
    pub paste_at: Option<craft_ui_rt::Pos2>,
}

impl Fired {
    /// `command` fired from `source` in `phase` at the input time `time`, for the app scope (no target), without a group
    /// cycle, the Alt variant or paste content.
    pub fn new(command: CommandRef, source: FireSource, phase: FirePhase, time: f64) -> Fired {
        Fired {
            command,
            target: None,
            source,
            phase,
            cycle: false,
            alt_variant: false,
            time,
            mods: craft_keymap::Mods::NONE,
            place: None,
            pasted: None,
            paste_at: None,
        }
    }

    /// The command with `mods` as the modifiers its firing input carried.
    pub fn mods(self, mods: craft_keymap::Mods) -> Fired {
        Fired { mods, ..self }
    }
}

/// Where a fired command came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
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
    /// An AccessKit action on a part of a component.
    Access,
    /// A keytip badge, sequence or digit.
    Keytip,
}

/// The phase of the key press that fired a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirePhase {
    /// The press.
    Press,
    /// A repeat of a held key, with `RootEnv::shortcuts_repeat` on.
    Repeat,
    /// The release of a momentary command's or a tool's key: a momentary command ends here, and a spring-loaded tool
    /// gives back the previous one.
    Release,
}

/// What the app or an instance says about a command this frame.
#[derive(Clone, Debug, Default, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct CommandState {
    flags: CommandFlags,
    title: Option<Arc<str>>,
    icon: Option<Cow<'static, str>>,
    hint: Option<Arc<str>>,
    hint_alone: bool,
    marked: bool,
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

    /// Whether the command's content has something to show, such as comments on a panel. A rail marks the command's
    /// button while the command is not checked (the panel is closed); the marker's description is the command's `hint`.
    #[must_use]
    pub fn marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// A label for this frame in place of the command's own, such as "Undo Move".
    #[must_use]
    pub fn title(mut self, title: impl Into<Arc<str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// An icon for this frame in place of the command's own, by catalogue name or registered app set name.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Cow<'static, str>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// The accessible description and second tooltip line.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<Arc<str>>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Opt-in: on a control whose label shows whole and that has no tooltip of its own, the hint is the whole tooltip,
    /// with the chord after it, in place of the label and chord over the hint (the default).
    #[must_use]
    pub fn hint_alone(mut self, alone: bool) -> Self {
        self.hint_alone = alone;
        self
    }

    /// Whether the command can fire, as [`enabled`](Self::enabled) set it.
    pub fn is_enabled(&self) -> bool {
        self.flags.enabled
    }

    /// The checked state [`checked`](Self::checked) set; `None` for a command that is not a toggle or radio.
    pub fn is_checked(&self) -> Option<bool> {
        self.flags.checked
    }

    pub(crate) fn is_marked(&self) -> bool {
        self.marked
    }

    pub(crate) fn flags(&self) -> CommandFlags {
        self.flags
    }

    pub(crate) fn title_ref(&self) -> Option<&Arc<str>> {
        self.title.as_ref()
    }

    pub(crate) fn icon_ref(&self) -> Option<&Cow<'static, str>> {
        self.icon.as_ref()
    }

    pub(crate) fn hint_ref(&self) -> Option<&Arc<str>> {
        self.hint.as_ref()
    }

    pub(crate) fn is_hint_alone(&self) -> bool {
        self.hint_alone
    }
}

/// A command as a surface shows and fires it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedCommand {
    /// The state's title, else the command's label, else its id.
    pub label: Arc<str>,
    /// The command's catalogue label (the registry's), whatever title the state sets; `None` for a command the
    /// registry does not define.
    pub catalog_label: Option<Arc<str>>,
    /// The icon name: the state's icon, else the command's own.
    pub icon: Option<Cow<'static, str>>,
    /// The chord surfaces show: the first single-press binding that runs the command.
    pub chord: Option<Chord>,
    /// `chord` as the platform writes it, from the table the keymap builds with its bindings.
    pub chord_text: Option<Arc<str>>,
    /// The state ANDed with the gate, the modal rule and the instance's liveness.
    pub enabled: bool,
    /// The checked state of a toggle or radio command.
    pub checked: Option<bool>,
    /// The command's content has something to show; the marker's description is `hint`.
    pub marked: bool,
    /// The accessible description and second tooltip line.
    pub hint: Option<Arc<str>>,
    /// The hint is the whole tooltip of a control whose label shows whole ([`CommandState::hint_alone`]).
    pub hint_alone: bool,
    /// What the command is.
    pub kind: CommandKind,
    /// The command is an edit a focused component takes as its own (it has an `EditCommand` link): a control bound to
    /// it keeps the focus where it is on a press, so the edit reaches the focused component.
    pub edit: bool,
    /// Shared by every fact that shows this resolution, so a settled view's facts copy no command.
    pub(crate) command: Arc<CommandRef>,
    /// Something besides the command's own state refuses it: the gate, the modal rule, the instance's liveness or a
    /// focused component whose edit does nothing now. A key whose app-scope command its state alone disables is
    /// declined (`FrameOutput::declined`); a refused one is not.
    pub(crate) refused: bool,
    #[doc(hidden)]
    pub instance: Option<Id>,
}

/// The app's one gate over commands: a dialog is open, a workspace is not ready, a recorder has the keys or an
/// operation is pending. A command the gate refuses resolves disabled and does not fire.
///
/// The gate is a `fn` of its props, as a memoized view's `show` is: the props hold everything the gate reads, so the
/// runtime asks it once per command while the props' key stays the same, and again only when it changes.
#[derive(Clone)]
pub struct CommandGate {
    key: u128,
    allows: Arc<dyn Fn(&CommandRef) -> bool + Send + Sync>,
}

impl CommandGate {
    /// A gate that lets through the commands `allows` accepts under `props`.
    pub fn new<P: MemoProps + Send + Sync + 'static>(props: P, allows: fn(&P, &CommandRef) -> bool) -> Self {
        let mut key = MemoKey::new();
        key.feed_hash(&std::any::TypeId::of::<P>());
        key.write(&(allows as usize).to_le_bytes());
        props.key(&mut key);
        CommandGate { key: key.finish(), allows: Arc::new(move |command| allows(&props, command)) }
    }

    /// The key of the gate's props and `fn`: equal keys give equal answers.
    pub(crate) fn key(&self) -> u128 {
        self.key
    }

    pub(crate) fn allows(&self, command: &CommandRef) -> bool {
        (self.allows)(command)
    }
}

/// The app-scope command states an app holds, in a [`Tracked`] it hands to [`CommandInput::states`] every frame. The
/// runtime reads them again only when the revision moves.
#[derive(Clone, Default, PlainData)]
pub struct CommandStates {
    #[plain_data(assume = "command refs and states are plain values; the list is copied on write")]
    states: Arc<Vec<(CommandRef, CommandState)>>,
    /// The positions in `states` of each command id's entries, so a lookup compares only the entries with its id.
    #[plain_data(assume = "ids and positions are plain values; the map is copied on write")]
    by_id: Arc<HashMap<String, Vec<usize>>>,
}

impl CommandStates {
    /// No states.
    pub fn new() -> Self {
        CommandStates::default()
    }

    /// Gives `command` `state`, in place of the state it had.
    pub fn set(&mut self, command: impl Into<CommandRef>, state: CommandState) {
        let command = command.into();
        match self.position(&command) {
            Some(at) => {
                if let Some((_, held)) = Arc::make_mut(&mut self.states).get_mut(at) {
                    *held = state;
                }
            }
            None => {
                let states = Arc::make_mut(&mut self.states);
                Arc::make_mut(&mut self.by_id).entry(command.id.clone()).or_default().push(states.len());
                states.push((command, state));
            }
        }
    }

    /// Removes the state of `command`, which then resolves with its own label, enabled.
    pub fn remove(&mut self, command: &CommandRef) -> Option<CommandState> {
        let at = self.position(command)?;
        let by_id = Arc::make_mut(&mut self.by_id);
        by_id.retain(|_, positions| {
            positions.retain(|&p| p != at);
            for p in positions.iter_mut().filter(|p| **p > at) {
                *p -= 1;
            }
            !positions.is_empty()
        });
        Some(Arc::make_mut(&mut self.states).remove(at).1)
    }

    /// The state of `command`.
    pub fn get(&self, command: &CommandRef) -> Option<&CommandState> {
        self.states.get(self.position(command)?).map(|(_, state)| state)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(CommandRef, CommandState)> {
        self.states.iter()
    }

    /// Where `command`'s entry is in `states`: among the entries with its id, the one equal to it.
    fn position(&self, command: &CommandRef) -> Option<usize> {
        self.by_id.get(command.id.as_str())?.iter().copied().find(|&at| {
            #[cfg(feature = "testing")]
            STATE_COMPARISONS.with(|n| n.set(n.get() + 1));
            self.states.get(at).is_some_and(|(held, _)| held == command)
        })
    }
}

impl PartialEq for CommandStates {
    fn eq(&self, other: &CommandStates) -> bool {
        self.states == other.states
    }
}

impl fmt::Debug for CommandStates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandStates").field("states", &self.states).finish()
    }
}

#[cfg(feature = "testing")]
thread_local! {
    static STATE_COMPARISONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many held commands this thread's [`CommandStates`] lookups have compared with the command looked up.
#[cfg(feature = "testing")]
#[doc(hidden)]
pub fn command_state_comparisons() -> u64 {
    STATE_COMPARISONS.with(std::cell::Cell::get)
}

/// App-scope command state for this frame, set from app state before the pass.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct CommandInput {
    pub(crate) states: Option<(Revision, CommandStates)>,
    pub(crate) overrides: Option<Arc<Overrides>>,
    pub(crate) gate: Option<CommandGate>,
    pub(crate) catalog: Option<AppCatalog>,
    pub(crate) holds: Option<f64>,
    pub(crate) paste_reads_clipboard: bool,
}

impl CommandInput {
    /// No state, no overrides and no gate.
    pub fn new() -> Self {
        CommandInput::default()
    }

    /// The app-scope command states the app holds. The runtime keeps the revision and reads the states again only
    /// when it moves.
    #[must_use]
    pub fn states(mut self, states: &Tracked<CommandStates>) -> Self {
        self.states = Some((states.revision(), (**states).clone()));
        self
    }

    /// The user's binding overrides; the keymap is rebuilt when the `Arc` changes.
    #[must_use]
    pub fn overrides(mut self, overrides: Arc<Overrides>) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Turns hold bindings on with the app's hold delay in seconds, `Some(delay)`; `None`, the default, turns them off.
    /// The keymap is rebuilt when this changes.
    #[must_use]
    pub fn holds(mut self, delay: Option<f64>) -> Self {
        self.holds = delay;
        self
    }

    /// The frame's app catalog; the keymap is rebuilt when the platform, the overrides or the catalog's generation change.
    #[must_use]
    pub fn catalog(mut self, catalog: AppCatalog) -> Self {
        self.catalog = Some(catalog);
        self
    }

    /// The one gate over commands. The runtime asks it once per command while the gate's props keep their key, whichever
    /// components, passes and frames resolve the command.
    #[must_use]
    pub fn gate(mut self, gate: CommandGate) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Whether the library's Paste ([`EditCommands::Paste`]), fired at the app by a menu row, a toolbar button or the
    /// palette, first asks the platform for the clipboard in the pass that fires it and reaches the app with the
    /// clipboard's content, as ⌘V does. Off (the default), it reaches the app at once with no content and the app
    /// reads its clipboard itself.
    #[must_use]
    pub fn paste_reads_clipboard(mut self, on: bool) -> Self {
        self.paste_reads_clipboard = on;
        self
    }
}

/// The chain of container instances that receive scoped commands, innermost last. A private inherited field of
/// the env.
#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandChain(Arc<[(CommandScope, Id)]>);

crate::stays!(CommandChain, CommandState, CommandRef, CommandScope);

impl CommandChain {
    pub(crate) fn pushed(&self, scope: CommandScope, id: Id) -> Self {
        CommandChain(self.0.iter().copied().chain([(scope, id)]).collect())
    }

    /// The innermost instance of `scope`.
    #[doc(hidden)]
    pub fn innermost(&self, scope: &str) -> Option<Id> {
        self.0.iter().rev().find(|(s, _)| s.name() == scope).map(|(_, id)| *id)
    }

    /// The innermost instance of any scope.
    pub(crate) fn innermost_any(&self) -> Option<Id> {
        self.0.last().map(|(_, id)| *id)
    }

    /// The innermost instance whose scope name `is`.
    pub(crate) fn innermost_where(&self, mut is: impl FnMut(&str) -> bool) -> Option<Id> {
        self.0.iter().rev().find(|(s, _)| is(s.name())).map(|(_, id)| *id)
    }

    pub(crate) fn ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.0.iter().map(|(_, id)| *id)
    }
}

/// How the contexts reach the board. Sealed: only the pipeline's contexts implement it.
#[doc(hidden)]
pub trait CommandAccess: crate::runtime::memory::sealed::Sealed {
    /// The window's context.
    fn command_ctx(&self) -> &crate::contract::Context;
    /// The instances above this context.
    fn command_chain(&self) -> &CommandChain;
    /// The component this context belongs to, when it has an id of its own.
    fn command_owner(&self) -> Option<crate::primitives::Id>;
    /// The run's command inboxes and the node memory the takes record in; `None` for a context
    /// without a node of its own (a settled or measuring `Cx`), which takes nothing.
    fn command_inbox(&mut self) -> Option<(&mut InboxIn, &mut crate::runtime::memory::MemoryStage, crate::primitives::Id)> {
        None
    }
    /// The paste this context's run takes: the focused component that lists Paste gets the clipboard content the
    /// platform delivered to it. `None` for a context without a node of its own.
    fn command_pasted(&self) -> Option<ClipContent> {
        None
    }
    /// Whether this context belongs to a measuring run, whose effects are fenced.
    fn command_fenced(&self) -> bool;
}

/// The chapter-11 context trait that `Cx`, `BodyCx` and `ItemCx` implement. "This instance" is the innermost
/// container at or above the context whose `ContainerProps.command_scope` is the command's scope.
pub trait Commands: CommandAccess {
    /// The one clipboard path.
    fn clipboard(&self) -> ClipboardHandle {
        ClipboardHandle::new(self.command_ctx(), self.command_owner(), self.command_pasted(), self.command_fenced())
    }

    /// This frame's state of a scoped command, for this instance. Publishing an app-scope command here is a
    /// debug assertion.
    fn publish(&mut self, command: impl Into<CommandRef>, state: CommandState) {
        if let Some(board) = self.command_ctx().board() {
            board.publish(self.command_chain(), &command.into(), state);
        }
    }

    /// The next command of set `S` fired at this instance that it has not taken. A fire writes the
    /// instance's inbox cell at event time, so only this node reruns; the take records what it
    /// consumed in the node's memory and writes no cell.
    fn take_command<S: CommandSet>(&mut self) -> Option<(S, Fired)> {
        let (inbox, memory, id) = self.command_inbox()?;
        inbox.take::<S, S>(memory, id, S::from_ref)
    }

    /// Declares Undo and Redo on this instance, enabled as `can_undo` and `can_redo` say, and takes the
    /// next of them fired at it. A body with its own history calls it in a `while let` loop and steps
    /// that history.
    fn take_step(&mut self, can_undo: bool, can_redo: bool) -> Option<Step> {
        fn step(command: &CommandRef) -> Option<Step> {
            match EditCommands::from_ref(command)? {
                EditCommands::Undo => Some(Step::Undo),
                EditCommands::Redo => Some(Step::Redo),
                _ => None,
            }
        }
        self.publish(EditCommands::Undo, CommandState::new().enabled(can_undo));
        self.publish(EditCommands::Redo, CommandState::new().enabled(can_redo));
        let (inbox, memory, id) = self.command_inbox()?;
        inbox.take::<EditCommands, Step>(memory, id, step).map(|(step, _)| step)
    }

    /// `command` as this context shows it: the label, icon and chord with the scoped bindings of the scopes above
    /// the context, and the enabled state of its instance. `None` when the command is scoped and no instance of its
    /// scope is above the context.
    fn resolve_command(&self, command: impl Into<CommandRef>) -> Option<Arc<ResolvedCommand>> {
        self.command_ctx().board()?.resolve_logged(self.command_chain(), &command.into())
    }
}

/// Where a command was shown, from `Ui::command_shown`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Shown {
    /// The rect it was shown in, in window coordinates.
    pub rect: craft_ui_rt::Rect,
    /// The window it was shown in.
    pub window: WindowId,
    /// The label it was shown with.
    pub label: Label,
    /// Whether the rect is a holder's rather than a bound component's.
    pub holder: bool,
}

/// An undo or redo a text part or editor had nothing to do for goes to the app command of the same id (11.3): it runs
/// on the board from the node whose build is running, when that build ends. Outside a build nothing runs.
pub(crate) fn fall_through(ctx: &Context, command: craft_behavior::EditCommand) {
    let Some(command) = EditCommands::of(command) else { return };
    let Some(board) = ctx.board() else { return };
    let fired = Fired::new(CommandRef::from(command), FireSource::Key, FirePhase::Press, ctx.time());
    crate::contract::context::log_build_read(move |cx| {
        board.run_from(cx, &fired);
    });
}
