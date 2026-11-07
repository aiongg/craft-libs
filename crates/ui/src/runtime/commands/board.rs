//! The board: state by `(command, instance)`, the instances of each scope, the queue, resolution, firing and the
//! dispatcher that turns key presses into fired commands (spec 11.3).

use super::EditCommands;
use super::{CommandChain, CommandGate, CommandInput, CommandScope, CommandState, FirePhase, FireSource, Fired, ResolvedCommand, registry};
use crate::runtime::egui::input as raw_input;
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::pass;
use crate::runtime::focus;
use craft_behavior::{EditCommand, OwnerClass, pick_instance, resolve_flags};
use craft_keymap::egui::Dispatcher;
use craft_keymap::{Active, Chord, CommandRef, Keymap, Overrides, Platform, Scopes};
use egui::{Context, Id, ViewportId};
use std::collections::HashMap;
use std::sync::Arc;

type StateKey = (String, Option<Id>);

/// A container instance that receives scoped commands.
struct Instance {
    id: Id,
    scope: CommandScope,
    viewport: ViewportId,
    /// The pass of its viewport that last registered it.
    stamp: u64,
}

struct KeymapCache {
    keymap: Arc<Keymap>,
    platform: Platform,
    overrides: Option<Arc<Overrides>>,
}

#[derive(Default)]
struct Board {
    registry: Arc<registry::Registry>,
    keymap: Option<KeymapCache>,
    gate: Option<CommandGate>,
    this: HashMap<StateKey, CommandState>,
    last: HashMap<StateKey, CommandState>,
    /// Scoped commands waiting for their instance.
    queue: Vec<Fired>,
    /// App-scope commands for the next `FrameOutput`.
    out: Vec<Fired>,
    instances: Vec<Instance>,
    /// The instances around the components that held key focus in the running pass.
    focus_notes: Vec<Id>,
    /// The same for the previous pass.
    focused: Vec<Id>,
    /// The chain of the last pass in which any component held key focus.
    last_focused: Vec<Id>,
    dispatcher: Option<Dispatcher>,
    modal_open: bool,
}

fn with<R>(ctx: &Context, f: impl FnOnce(&mut Board) -> R) -> R {
    Service::<Board>::of(ctx).with(f)
}

impl Board {
    fn live(&self, ctx: &Context, instance: &Instance) -> bool {
        instance.stamp + 1 >= pass::pass_nr(ctx, instance.viewport)
    }

    fn live_instances(&self, ctx: &Context, scope: &str) -> Vec<Id> {
        self.instances.iter().filter(|i| i.scope.name() == scope && self.live(ctx, i)).map(|i| i.id).collect()
    }

    fn is_live(&self, ctx: &Context, id: Id) -> bool {
        self.instances.iter().any(|i| i.id == id && self.live(ctx, i))
    }

    /// The scope name of `command`; commands the registry does not know belong to the application.
    fn scope_of(&self, command: &CommandRef) -> String {
        self.registry.get(&command.id).map_or_else(|| CommandScope::APP.name().to_owned(), |def| def.scope.as_str().to_owned())
    }

    /// The instance a source with no component of its own delivers `scope`'s commands to.
    fn pick(&self, ctx: &Context, scope: &str) -> Option<Id> {
        pick_instance(&self.live_instances(ctx, scope), &self.focused, &self.last_focused)
    }
}

fn is_app(scope: &str) -> bool {
    scope == CommandScope::APP.name()
}

/// Loads the linked command sets.
pub(crate) fn install(ctx: &Context) -> Result<(), registry::CommandDefError> {
    let registry = registry::Registry::load()?;
    with(ctx, |board| board.registry = Arc::new(registry));
    Ok(())
}

/// Before each frame: last frame's state is the fallback, the app's states and gate are this frame's, and the
/// keymap follows the user's overrides.
pub(crate) fn set_input(ctx: &Context, input: &CommandInput) {
    let platform = craft_keymap::egui::platform(ctx);
    with(ctx, |board| {
        board.last = std::mem::take(&mut board.this);
        for (command, state) in &input.states {
            board.this.insert((command.id.clone(), None), state.clone());
        }
        board.gate = input.gate.clone();
        let stale = board.keymap.as_ref().is_none_or(|cache| {
            cache.platform != platform
                || match (&cache.overrides, &input.overrides) {
                    (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                    (None, None) => false,
                    _ => true,
                }
        });
        if stale {
            let mut scopes = Scopes::new();
            for def in board.registry.defs() {
                if !def.scope.is_app() {
                    scopes = scopes.with_panel(def.scope.as_str());
                }
            }
            let mut keymap = Keymap::new(board.registry.defs().to_vec(), scopes, platform);
            if let Some(overrides) = &input.overrides {
                keymap.set_overrides((**overrides).clone());
            }
            board.keymap = Some(KeymapCache { keymap: Arc::new(keymap), platform, overrides: input.overrides.clone() });
        }
    });
}

/// The chords the keymap binds to the edit commands, which the input hook removes from the raw input when the
/// focused component lists the command.
pub(crate) fn edit_chords(ctx: &Context) -> Vec<(EditCommand, Chord)> {
    with(ctx, |board| {
        let Some(cache) = &board.keymap else { return Vec::new() };
        EditCommands::ALL
            .iter()
            .flat_map(|command| {
                let bindings = cache.keymap.bindings(&CommandRef::from(*command));
                bindings.into_iter().filter(|binding| !binding.trigger.double).map(move |binding| (command.edit(), binding.trigger.chord))
            })
            .collect()
    })
}

/// The app-scope commands fired since the last call.
pub(crate) fn take_fired(ctx: &Context) -> Vec<Fired> {
    with(ctx, |board| std::mem::take(&mut board.out))
}

/// Records that the container `id` is an instance of `scope` in the running pass of its viewport.
pub(crate) fn register_instance(ctx: &Context, scope: CommandScope, id: Id) {
    let viewport = pass::current_viewport(ctx);
    let stamp = pass::pass_nr(ctx, viewport);
    with(ctx, |board| match board.instances.iter_mut().find(|i| i.id == id) {
        Some(instance) => {
            instance.scope = scope;
            instance.viewport = viewport;
            instance.stamp = stamp;
        }
        None => board.instances.push(Instance { id, scope, viewport, stamp }),
    });
}

/// Records that a component inside `chain` holds key focus in the running pass.
pub(crate) fn note_focus(ctx: &Context, chain: &CommandChain) {
    if !chain.is_empty() {
        with(ctx, |board| board.focus_notes = chain.ids().collect());
    }
}

pub(crate) fn set_modal_open(ctx: &Context, open: bool) {
    with(ctx, |board| board.modal_open = open);
}

/// `command` as seen by `instance`, or by the app for an app-scope command.
fn resolve(ctx: &Context, command: &CommandRef, instance: Option<Id>) -> ResolvedCommand {
    with(ctx, |board| {
        let def = board.registry.get(&command.id);
        let scope = board.scope_of(command);
        let instance = if is_app(&scope) { None } else { instance };
        let key = (command.id.clone(), instance);
        let state = board.this.get(&key).or_else(|| board.last.get(&key));
        let flags = resolve_flags(board.this.get(&key).map(CommandState::flags), board.last.get(&key).map(CommandState::flags));
        let gate = board.gate.as_ref().is_none_or(|gate| gate.allows(command));
        let modal = !(board.modal_open && def.is_some_and(|def| def.modal == craft_keymap::ModalRule::Blocked));
        let live = is_app(&scope) || instance.is_some_and(|id| board.is_live(ctx, id));
        let chord =
            board.keymap.as_ref().and_then(|cache| cache.keymap.bindings(command).into_iter().find(|b| !b.trigger.double).map(|b| b.trigger.chord));
        ResolvedCommand {
            label: state.and_then(|s| s.title_ref().cloned()).unwrap_or_else(|| Arc::from(def.map_or(command.id.as_str(), |def| def.label.as_str()))),
            icon: def.and_then(|def| def.icon.clone()),
            chord,
            enabled: flags.enabled && gate && modal && live,
            checked: flags.checked,
            hint: state.and_then(|s| s.hint_ref().cloned()),
            kind: def.map_or(craft_keymap::CommandKind::Command, |def| def.kind.clone()),
            command: command.clone(),
            instance,
        }
    })
}

/// Step 1: the bound command of a component, resolved against its own instance.
pub(crate) fn resolve_bound(ctx: &Context, chain: &CommandChain, command: &CommandRef) -> ResolvedCommand {
    let scope = with(ctx, |board| board.scope_of(command));
    resolve(ctx, command, chain.innermost(&scope))
}

fn queue(ctx: &Context, fired: Fired) {
    with(ctx, |board| if fired.target.is_some() { board.queue.push(fired) } else { board.out.push(fired) });
    // A scoped entry is taken next pass, so the root viewport runs one.
    raw_input::request_repaint_of(ctx, ViewportId::ROOT);
}

/// Step 17: the command of a component that was activated and enabled.
pub(crate) fn fire_bound(ctx: &Context, command: &ResolvedCommand, by_key: bool) {
    if !command.enabled {
        return;
    }
    let source = if by_key { FireSource::Key } else { FireSource::Pointer };
    queue(
        ctx,
        Fired { command: command.command.clone(), target: command.instance, source, phase: FirePhase::Press, cycle: false, alt_variant: false },
    );
}

pub(crate) fn fire_from(ctx: &Context, command: &CommandRef, source: FireSource) -> bool {
    let target = with(ctx, |board| {
        let scope = board.scope_of(command);
        (is_app(&scope), board.pick(ctx, &scope))
    });
    let (app, instance) = target;
    if !app && instance.is_none() {
        return false;
    }
    let resolved = resolve(ctx, command, instance);
    if !resolved.enabled {
        return false;
    }
    queue(ctx, Fired { command: command.clone(), target: instance, source, phase: FirePhase::Press, cycle: false, alt_variant: false });
    true
}

pub(crate) fn publish(ctx: &Context, chain: &CommandChain, command: &CommandRef, state: CommandState) {
    let scope = with(ctx, |board| board.scope_of(command));
    debug_assert!(!is_app(&scope), "{command} is an app-scope command: its state comes from `FrameInput.commands`");
    if let Some(instance) = chain.innermost(&scope) {
        with(ctx, |board| board.this.insert((command.id.clone(), Some(instance)), state));
    }
}

pub(crate) fn take<S>(ctx: &Context, instance: Option<Id>, from_ref: fn(&CommandRef) -> Option<S>) -> Option<(S, Fired)> {
    let instance = instance?;
    with(ctx, |board| {
        let at = board.queue.iter().position(|fired| fired.target == Some(instance) && from_ref(&fired.command).is_some())?;
        let fired = board.queue.remove(at);
        Some((from_ref(&fired.command)?, fired))
    })
}

/// H7: the key presses this pass, through the dispatcher and the gate, to their commands.
pub(crate) fn begin(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let (keymap, dispatcher, panel) = with(ctx, |board| {
        board.focused = std::mem::take(&mut board.focus_notes);
        if !board.focused.is_empty() {
            board.last_focused = board.focused.clone();
        }
        let panel = board.focused.last().and_then(|id| board.instances.iter().find(|i| i.id == *id)).map(|i| i.scope.name());
        (board.keymap.as_ref().map(|cache| cache.keymap.clone()), board.dispatcher.take(), panel)
    });
    let owner = focus::record(ctx, viewport).keys.map(|keys| keys.owner);
    let presses = raw_input::has_shortcut_events(ctx);
    let dispatcher = dispatcher.unwrap_or_else(|| Dispatcher::new().with_clipboard_events(true));
    let (Some(keymap), true) = (keymap, presses && owner != Some(OwnerClass::Recorder)) else {
        with(ctx, |board| board.dispatcher = Some(dispatcher));
        return;
    };
    let active = Active { panel, contexts: &[], typing: owner == Some(OwnerClass::Text) };
    // The dispatcher edits a copy: the gate reads the board, which must not run under the context's input lock.
    let mut snapshot = raw_input::snapshot(ctx);
    let mut dispatcher = dispatcher;
    let hits = dispatcher.dispatch_filtered(&mut snapshot, &keymap, &active, |hit| {
        let instance = with(ctx, |board| {
            let scope = board.scope_of(&hit.command);
            board.pick(ctx, &scope)
        });
        resolve(ctx, &hit.command, instance).enabled
    });
    raw_input::replace_events(ctx, snapshot.events);
    with(ctx, |board| board.dispatcher = Some(dispatcher));
    for hit in hits {
        let (scope, instance) = with(ctx, |board| {
            let scope = board.scope_of(&hit.command);
            let instance = board.pick(ctx, &scope);
            (scope, instance)
        });
        let target = if is_app(&scope) { None } else { instance };
        queue(
            ctx,
            Fired {
                command: hit.command,
                target,
                source: FireSource::Key,
                phase: FirePhase::Press,
                cycle: hit.cycle.len() > 1,
                alt_variant: hit.alt_variant,
            },
        );
    }
}

/// The end hook: instances that stopped registering are gone, and so are the commands queued for them.
pub(crate) fn end(ctx: &Context) {
    with(ctx, |board| {
        let live = pass::live_viewports(ctx);
        let alive: Vec<bool> = board.instances.iter().map(|i| board.live(ctx, i) && live.contains(&i.viewport)).collect();
        let mut keep = alive.iter();
        board.instances.retain(|_| keep.next().copied().unwrap_or(false));
        let ids: Vec<Id> = board.instances.iter().map(|i| i.id).collect();
        board.queue.retain(|fired| fired.target.is_some_and(|id| ids.contains(&id)));
    });
}
