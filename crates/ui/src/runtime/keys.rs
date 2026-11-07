//! The key stage (spec 9.1, 9.2, 9.3, 9.7): the input hook strips the keys the runtime owns and records
//! every key event of the pass, the begin hook turns them into the `Intents` of the focused component,
//! and claims, menu-bar keys and AccessKit action requests are routed from the same record.

use crate::runtime::commands::{self, EditCommands, FirePhase};
use crate::runtime::egui::access::node_key;
use crate::runtime::egui::input::{self as raw_input, Action, ActionKind, KeyEvent, RawInput};
use crate::runtime::focus::{self, StopRec};
use crate::runtime::frame::PerViewport;
use craft_behavior::focus::key_owner;
use crate::runtime::overlay;
use craft_behavior::focus::{EscapeFacts, EscapeStep, escape_step};
use craft_behavior::keys;
use craft_behavior::{AccessRequest, CancelRule, EditCommand, Intent, KeyOwner, MoveMods, NavMove, OwnerClass, PartRef};
use craft_host::target::Os;
use craft_keymap::{Chord, Key, KeyInput, Layout, Mods};
use egui::{Context, Id, Vec2, ViewportId};
use std::collections::HashMap;
use std::sync::Arc;

/// A menu-bar key of Windows and Linux (spec 9.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MenuKey {
    /// F10 or a lone Alt press: focus the menu bar.
    Focus,
    /// Alt and a letter: open the menu with that mnemonic.
    Letter(char),
}

/// Tab and F6 moves the input hook stripped: `true` is forward.
#[derive(Default)]
pub(crate) struct NavKeys {
    pub(crate) tabs: Vec<bool>,
    pub(crate) panes: Vec<bool>,
}

struct Press {
    key: Key,
    mods: Mods,
}

/// A key press, release or clipboard chord that the shortcut dispatcher may match, with the press that the
/// intents may have consumed.
#[derive(Clone, Copy)]
pub(crate) struct Shortcut {
    pub(crate) input: KeyInput,
    pub(crate) phase: FirePhase,
    /// The press typed text in this pass.
    typed: bool,
    /// The index of the press in `Batch::presses`.
    press: Option<usize>,
}

/// The key events the owner leaves to the app this pass, with the facts the matching needs.
#[derive(Default)]
pub(crate) struct Shortcuts {
    pub(crate) keys: Vec<Shortcut>,
    /// A text field owns the keys: plain keys wait for it.
    pub(crate) typing: bool,
    /// A dialog owns the keys: commands that do not fire in dialogs wait.
    pub(crate) dialog: bool,
    /// The layout that reads the presses.
    pub(crate) layout: Layout,
}

/// What the input hook recorded for the next pass's begin hook.
#[derive(Default)]
struct Batch {
    presses: Vec<Press>,
    shortcuts: Vec<Shortcut>,
    /// The text of the paste the focused component takes.
    paste: Option<String>,
    texts: Vec<String>,
    clips: Vec<EditCommand>,
    access: Vec<Action>,
    escapes: usize,
    claimed: Vec<Chord>,
    nav: NavKeys,
    menu: Vec<MenuKey>,
}

#[derive(Clone)]
struct Item {
    intent: Intent,
    /// A physical left or right arrow: under right-to-left text it reads as the opposite logical move.
    horizontal: bool,
}

#[derive(Clone, Default)]
struct Routed {
    items: Vec<Item>,
    nudge: Option<(Vec2, Mods)>,
    set_value: Option<f64>,
}

#[derive(Clone, Copy)]
struct Node {
    owner: Id,
    part: Option<PartRef>,
    focusable: bool,
}

#[derive(Default)]
struct AltTrack {
    down: bool,
    other_key: bool,
}

#[derive(Default)]
struct Stage {
    batch: Batch,
    routed: HashMap<Option<Id>, Routed>,
    claimed: Vec<(Id, Chord)>,
    claims: Vec<(Id, Chord)>,
    menu: Vec<MenuKey>,
    layout: Layout,
    nodes: HashMap<u64, Node>,
    last_nodes: HashMap<u64, Node>,
    activated: HashMap<Id, Id>,
    alt: AltTrack,
    shortcuts: Shortcuts,
    /// The paste text the focused component reads through its clipboard handle.
    pasted: Option<(Id, Arc<str>)>,
    /// Edit commands fired from the queue for the focused component.
    queued_edits: Vec<EditCommand>,
    /// The components that asked for a context menu in this pass, with the rect it opens at.
    contexts: Vec<(Id, egui::Rect)>,
}

fn with<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut Stage) -> R) -> R {
    PerViewport::<Stage>::with(ctx, viewport, f)
}

fn current<R>(ctx: &Context, f: impl FnOnce(&mut Stage) -> R) -> R {
    with(ctx, crate::runtime::egui::pass::current_viewport(ctx), f)
}

/// What the strip of one event decided.
struct Strip<'a> {
    os: Os,
    layout: &'a Layout,
    keeps_tab: bool,
    pages: bool,
    claims: &'a [Chord],
    edits: &'a dyn Fn(EditCommand) -> bool,
    /// The chords bound to the edit commands.
    edit_chords: &'a [(EditCommand, Chord)],
    menu_bar: bool,
}

impl Strip<'_> {
    /// Records `event` in `batch` and returns whether egui keeps it.
    fn event(&self, batch: &mut Batch, alt: &mut AltTrack, event: KeyEvent) -> bool {
        match event {
            KeyEvent::Press { key, input, repeat } => {
                alt.other_key = true;
                self.press(batch, key, &input, repeat)
            }
            KeyEvent::Release { input } => {
                batch.shortcuts.push(Shortcut { input, phase: FirePhase::Release, typed: false, press: None });
                true
            }
            KeyEvent::Text(text) => {
                alt.other_key = true;
                if let Some(press) = batch.shortcuts.last_mut().filter(|shortcut| shortcut.phase == FirePhase::Press) {
                    press.typed = true;
                }
                batch.texts.push(text);
                true
            }
            KeyEvent::Clip { command, text } if (self.edits)(command) => {
                batch.clips.push(command);
                if text.is_some() {
                    batch.paste = text;
                }
                false
            }
            KeyEvent::Clip { command, .. } => {
                // The platform sends the clipboard event in place of the chord: an unlisted one is the chord. The
                // web sends the chord's press as well.
                let key = match command {
                    EditCommand::Cut => Key::X,
                    EditCommand::Paste => Key::V,
                    _ => Key::C,
                };
                let input = KeyInput::new(Mods::CMD).with_typed(key);
                let sent = batch
                    .shortcuts
                    .iter()
                    .any(|shortcut| shortcut.phase == FirePhase::Press && shortcut.input.mods == Mods::CMD && shortcut.input.typed == Some(key));
                if !sent {
                    batch.shortcuts.push(Shortcut { input, phase: FirePhase::Press, typed: false, press: None });
                }
                true
            }
            KeyEvent::PointerPress => {
                alt.other_key = true;
                true
            }
            KeyEvent::Modifiers(held) => {
                self.alt_change(batch, alt, held.alt);
                true
            }
            KeyEvent::Access(request) => {
                batch.access.push(request);
                true
            }
            _ => true,
        }
    }

    /// A lone Alt press, pressed and released with no other key between, focuses the menu bar (9.7).
    fn alt_change(&self, batch: &mut Batch, alt: &mut AltTrack, held: bool) {
        if held && !alt.down {
            alt.other_key = false;
        }
        if !held && alt.down && !alt.other_key && self.menu_bar && self.os != Os::Mac {
            batch.menu.push(MenuKey::Focus);
        }
        alt.down = held;
    }

    fn press(&self, batch: &mut Batch, key: Key, input: &KeyInput, repeat: bool) -> bool {
        let mods = input.mods;
        let claimed = self.claims.iter().find(|chord| self.layout.matches(chord, input, self.os));
        let ctrl_tab = key == Key::Tab && (if self.os == Os::Mac { mods.ctrl && !mods.cmd } else { mods.cmd });
        let ctrl_tab = ctrl_tab && self.pages && !self.keeps_tab;
        let bare_tab = key == Key::Tab && !mods.cmd && !mods.ctrl && !mods.alt;
        let mnemonic = self.menu_bar && self.os != Os::Mac && mods == Mods::ALT && key.is_letter();
        let edit = self.edit_chords.iter().find(|(command, chord)| (self.edits)(*command) && self.layout.matches(chord, input, self.os));
        if let Some(chord) = claimed {
            batch.claimed.push(*chord);
            false
        } else if let Some((command, _)) = edit {
            batch.clips.push(*command);
            false
        } else if bare_tab && !self.keeps_tab {
            batch.nav.tabs.push(!mods.shift);
            false
        } else if ctrl_tab {
            batch.presses.push(Press { key, mods });
            false
        } else if key == Key::F6 && (mods.is_none() || mods == Mods::SHIFT) {
            batch.nav.panes.push(!mods.shift);
            false
        } else if key == Key::Escape && mods.is_none() {
            batch.escapes += 1;
            false
        } else if key == Key::F10 && mods.is_none() && self.menu_bar && self.os != Os::Mac {
            batch.menu.push(MenuKey::Focus);
            false
        } else if mnemonic {
            batch.menu.push(MenuKey::Letter(key.name().chars().next().unwrap_or('A')));
            false
        } else {
            batch.presses.push(Press { key, mods });
            if !repeat {
                batch.shortcuts.push(Shortcut { input: *input, phase: FirePhase::Press, typed: false, press: Some(batch.presses.len() - 1) });
            }
            true
        }
    }
}

/// The input hook's key steps (H3). Reads only `RawInput` and records keyed by `input.viewport_id`. Returns
/// whether it removed an event, so the hook asks for a repaint.
pub(crate) fn input_hook(ctx: &Context, input: &mut RawInput) -> bool {
    let viewport = input.viewport_id;
    // A modal layer in another viewport blocks this one (app-modal, 15.5): nothing below reaches the key stage.
    if overlay::modal_elsewhere(ctx, viewport) {
        let (removed, press) = raw_input::drop_under_modal(input);
        if press {
            overlay::note_dropped_press(ctx, viewport);
        }
        return removed;
    }
    let os = raw_input::os(ctx);
    let record = focus::record(ctx, viewport);
    let keys = record.keys;
    let menu_bar = record.menu_bar && keys.is_none_or(|keys| !matches!(keys.owner, OwnerClass::Text | OwnerClass::Recorder));
    let before = input.events.len();
    let edit_chords = if keys.is_some_and(|keys| EditCommands::ALL.iter().any(|command| keys.edits.contains(command.edit()))) {
        commands::edit_chords(ctx)
    } else {
        Arc::default()
    };
    let latin = input.latin_layout_chars.clone();
    let (batch, visible) = with(ctx, viewport, |stage| {
        let mut claims = record.claims.clone();
        claims.extend(stage.claims.iter().filter(|(owner, _)| Some(*owner) == record.focused).map(|(_, chord)| *chord));
        let edits = |command| keys.is_some_and(|keys| keys.edits.contains(command));
        stage.layout.set_latin(input.keyboard_layout_is_latin);
        let strip = Strip {
            os,
            layout: &stage.layout,
            keeps_tab: keys.is_some_and(|keys| keys.keep == focus::KeepKeys::Tab),
            pages: record.pages,
            claims: &claims,
            edits: &edits,
            edit_chords: &edit_chords,
            menu_bar,
        };
        let mut batch = Batch::default();
        let mut visible = None;
        input.events.retain(|event| {
            let read = raw_input::key_event(event, os, &latin);
            visible = match &read {
                KeyEvent::PointerPress => Some(false),
                KeyEvent::Press { .. } | KeyEvent::Text(_) => Some(true),
                _ => visible,
            };
            strip.event(&mut batch, &mut stage.alt, read)
        });
        (batch, visible)
    });
    if let Some(visible) = visible {
        focus::set_visible(ctx, viewport, visible);
    }
    with(ctx, viewport, |stage| {
        stage.batch = batch;
    });
    input.events.len() != before
}

/// Takes the Tab and F6 moves the input hook recorded, for the focus service's begin hook.
pub(crate) fn take_nav(ctx: &Context) -> NavKeys {
    current(ctx, |stage| std::mem::take(&mut stage.batch.nav))
}

/// The begin hook's intents: the pass's key events, AccessKit requests and claimed chords, routed to the
/// focused component (or to the component that owns a requested node).
pub(crate) fn begin(ctx: &Context, press_captured: bool) -> bool {
    let os = raw_input::os(ctx);
    let target = focus::focused_rec(ctx);
    let (batch, nodes) = current(ctx, |stage| {
        stage.routed.clear();
        stage.contexts.clear();
        stage.claimed.clear();
        stage.activated.clear();
        stage.claims.clear();
        stage.last_nodes = std::mem::take(&mut stage.nodes);
        (std::mem::take(&mut stage.batch), stage.last_nodes.clone())
    });
    let mut routed: HashMap<Option<Id>, Routed> = HashMap::new();
    let mut activated = HashMap::new();
    let mut consumed = vec![false; batch.presses.len()];
    if let Some(rec) = &target {
        for (press, taken) in batch.presses.iter().zip(&mut consumed) {
            *taken = route_press(ctx, os, rec, press, &mut routed, &mut activated);
        }
        for text in &batch.texts {
            let typed = rec.stop.role.type_ahead && !text.trim().is_empty();
            if matches!(rec.stop.keys.owner, OwnerClass::Text) || typed {
                routed.entry(Some(rec.stop.id)).or_default().items.push(Item { intent: Intent::Text(Arc::from(text.as_str())), horizontal: false });
            }
        }
    }
    let focused = target.as_ref().map(|rec| rec.stop.id);
    // The web sends a chord's key press and its clipboard event: the command reaches the component once.
    let mut clips = batch.clips.clone();
    clips.dedup();
    for command in &clips {
        push(&mut routed, focused, Intent::Edit(*command), false);
    }
    let queued = current(ctx, |stage| std::mem::take(&mut stage.queued_edits));
    for command in queued {
        push(&mut routed, focused, Intent::Edit(command), false);
    }
    let shortcuts = owned_shortcuts(ctx, target.as_ref(), &batch, &consumed);
    let pasted = batch.paste.as_deref().zip(focused).map(|(text, id)| (id, Arc::<str>::from(text)));
    let escapes = craft_behavior::focus::route_escapes(batch.escapes, press_captured);
    for _ in 0..escapes.to_components {
        // A row's child that claims Escape keeps it; any other returns focus to its row.
        let claims_escape = target.as_ref().is_some_and(|rec| rec.stop.keys.claims.iter().any(|chord| chord.key == Key::Escape));
        if !claims_escape && target.as_ref().is_some_and(|rec| focus::leave_row(ctx, rec.stop.key()).is_some()) {
            continue;
        }
        push(&mut routed, focused, Intent::Cancel, false);
    }
    let claimed = batch.claimed.iter().filter_map(|chord| focused.map(|id| (id, *chord))).collect::<Vec<_>>();
    for request in &batch.access {
        route_access(ctx, request, &nodes, &mut routed, &mut activated);
    }
    current(ctx, |stage| {
        stage.routed = routed;
        stage.claimed = claimed;
        stage.activated = activated;
        stage.menu = batch.menu;
        stage.shortcuts = shortcuts;
        if !stage.shortcuts.keys.is_empty() {
            stage.shortcuts.layout = stage.layout.clone();
        }
        stage.pasted = pasted;
    });
    escapes.cancels_press
}

fn push(routed: &mut HashMap<Option<Id>, Routed>, target: Option<Id>, intent: Intent, horizontal: bool) {
    routed.entry(target).or_default().items.push(Item { intent, horizontal });
}

/// The shortcut events the key owner leaves to the app (9.3). The owner is the focused component's class, or
/// `Blocked` while a modal layer is open. `Menu` and `Recorder` keep every key; a `Widget` or popup keeps the keys
/// it activates with, the keys of its intents and the keys its filter keeps from the key stage; a text field
/// keeps the text its presses typed.
fn owned_shortcuts(ctx: &Context, target: Option<&StopRec>, batch: &Batch, consumed: &[bool]) -> Shortcuts {
    let present: Vec<OwnerClass> = target.iter().map(|rec| rec.stop.keys.owner).collect();
    let owner = key_owner(commands::modal_open(ctx), &present);
    let held_by = |keys: Option<&focus::KeyUse>, at: usize| {
        let Some(keys) = keys else { return false };
        let key = batch.presses[at].key;
        let kept = match keys.keep {
            focus::KeepKeys::Tab => key == Key::Tab,
            focus::KeepKeys::InlineArrows => matches!(key, Key::Left | Key::Right),
            focus::KeepKeys::BlockArrows => matches!(key, Key::Left | Key::Right | Key::Up | Key::Down),
            _ => false,
        };
        consumed[at] || kept
    };
    let (typing, dialog) = (owner == KeyOwner::Class(OwnerClass::Text), owner == KeyOwner::Class(OwnerClass::Dialog));
    let widget = matches!(owner, KeyOwner::Class(OwnerClass::Widget | OwnerClass::Popup | OwnerClass::PassivePopup));
    let silent = matches!(owner, KeyOwner::Class(OwnerClass::Menu | OwnerClass::Recorder));
    let keys = if silent {
        Vec::new()
    } else {
        let owner_keys = target.map(|rec| &rec.stop.keys);
        batch
            .shortcuts
            .iter()
            .filter(|shortcut| !(typing && shortcut.typed))
            .filter(|shortcut| !(widget && shortcut.press.is_some_and(|at| held_by(owner_keys, at))))
            .copied()
            .collect()
    };
    Shortcuts { keys, typing, dialog, layout: Layout::default() }
}

/// Takes the shortcut events the key stage left to the app this pass, for the dispatcher at H7.
pub(crate) fn take_shortcuts(ctx: &Context) -> Shortcuts {
    current(ctx, |stage| std::mem::take(&mut stage.shortcuts))
}

/// Asks the focused component of the running viewport to take `command` as `Intent::Edit` in the next pass.
/// `false` when it does not list the command. A paste asks the platform for the clipboard text, which arrives as
/// the paste event that becomes the intent.
pub(crate) fn queue_edit(ctx: &Context, command: EditCommand) -> bool {
    let viewport = crate::runtime::egui::pass::current_viewport(ctx);
    let record = focus::record(ctx, viewport);
    if record.focused.is_none() || !record.keys.is_some_and(|keys| keys.edits.contains(command)) {
        return false;
    }
    if command == EditCommand::Paste {
        crate::runtime::egui::clipboard::request_paste(ctx);
    } else {
        with(ctx, viewport, |stage| stage.queued_edits.push(command));
        crate::runtime::egui::repaint::request_of(ctx, viewport);
    }
    true
}

/// The text of the paste the focused component `id` takes this pass.
pub(crate) fn pasted_text(ctx: &Context, id: Id) -> Option<Arc<str>> {
    current(ctx, |stage| stage.pasted.as_ref().filter(|(owner, _)| *owner == id).map(|(_, text)| Arc::clone(text)))
}

/// One key press of the focused component: the table's row, else the activation keys, else a nudge. Returns
/// whether the component's intents took the press.
fn route_press(
    ctx: &Context,
    os: Os,
    rec: &StopRec,
    press: &Press,
    routed: &mut HashMap<Option<Id>, Routed>,
    activated: &mut HashMap<Id, Id>,
) -> bool {
    let stop = &rec.stop;
    let id = Some(stop.id);
    let horizontal = matches!(press.key, Key::Left | Key::Right);
    let plain = press.mods.is_none();
    match keys::intent(os, press.key, press.mods, stop.role) {
        Some(Intent::Move(mv, mods)) => {
            // Up, Down, Page, Home and End leave a row's children: the collection takes the move.
            let vertical = matches!(mv, NavMove::Up | NavMove::Down | NavMove::PageUp | NavMove::PageDown | NavMove::Start | NavMove::End);
            if let Some(owner) = vertical.then(|| focus::leave_row(ctx, rec.stop.key())).flatten() {
                push(routed, Some(owner), Intent::Move(mv, mods), horizontal);
            } else if focus::roving_move(ctx, rec.stop.key(), mv, mods, horizontal).is_none() {
                push(routed, id, Intent::Move(mv, mods), horizontal);
            }
        }
        Some(Intent::Activate) => route_activate(rec, routed, activated),
        Some(intent) => push(routed, id, intent, false),
        None if press.key == Key::Enter && plain => {
            if stop.keys.activate.enter() {
                route_activate(rec, routed, activated);
            } else {
                push(routed, id, Intent::Submit, false);
            }
        }
        None if press.key == Key::Space && plain && stop.keys.activate.space() => route_activate(rec, routed, activated),
        None if stop.role.nudge => {
            let Some(nudge) = nudge_of(press.key, press.mods) else { return false };
            routed.entry(id).or_default().nudge = Some(nudge);
        }
        None => return false,
    }
    true
}

/// `Activate` of a group member goes to the group's root, which reads `activated_member`.
fn route_activate(rec: &StopRec, routed: &mut HashMap<Option<Id>, Routed>, activated: &mut HashMap<Id, Id>) {
    let target = if rec.stop.group.is_some() {
        activated.insert(rec.stop.root, rec.stop.id);
        rec.stop.root
    } else {
        rec.stop.id
    };
    push(routed, Some(target), Intent::Activate, false);
}

/// An arrow key as a unit direction with the modifiers held.
fn nudge_of(key: Key, mods: Mods) -> Option<(Vec2, Mods)> {
    let direction = match key {
        Key::Left => -Vec2::X,
        Key::Right => Vec2::X,
        Key::Up => -Vec2::Y,
        Key::Down => Vec2::Y,
        _ => return None,
    };
    Some((direction, mods))
}

/// Routes an AccessKit action request to the owner of its target node, whatever the focus (9.2 rule 2).
fn route_access(
    ctx: &Context,
    request: &Action,
    nodes: &HashMap<u64, Node>,
    routed: &mut HashMap<Option<Id>, Routed>,
    activated: &mut HashMap<Id, Id>,
) {
    let Some(node) = nodes.get(&request.node()).copied() else { return };
    let owner = Some(node.owner);
    if node.focusable {
        let target = node.part.map_or(focus::FocusTarget::Id(node.owner), |part| focus::FocusTarget::Part(node.owner, part));
        focus::request_now(ctx, target);
    }
    match request.kind() {
        ActionKind::Focus if node.focusable => {}
        ActionKind::Click => match focus::stop_rec(ctx, (node.owner, node.part)).filter(|rec| rec.stop.group.is_some()) {
            Some(rec) => route_activate(&rec, routed, activated),
            None => push(routed, owner, Intent::Activate, false),
        },
        ActionKind::Increment => push(routed, owner, Intent::Move(NavMove::Forward, MoveMods::PLAIN), false),
        ActionKind::Decrement => push(routed, owner, Intent::Move(NavMove::Back, MoveMods::PLAIN), false),
        ActionKind::ShowContextMenu => push(routed, owner, Intent::ContextMenu, false),
        ActionKind::SetValue(value) => routed.entry(owner).or_default().set_value = Some(value),
        ActionKind::Focus | ActionKind::Other => push(routed, owner, Intent::Access(request.request(node.part)), false),
    }
}

/// An owned copy of the intents routed to one id this pass; read through `cx.intents()`.
#[derive(Clone, Debug, Default)]
pub struct Intents {
    items: Vec<(Intent, bool)>,
    nudge: Option<(Vec2, Mods)>,
    set_value: Option<f64>,
}

impl Intents {
    /// Takes an intent of the kind of `intent`, once per pass: the first taker gets it.
    pub fn take(&mut self, intent: Intent) -> bool {
        match self.items.iter().position(|(item, taken)| !taken && item.same_kind(&intent)) {
            Some(at) => {
                self.items[at].1 = true;
                true
            }
            None => false,
        }
    }

    /// Takes the next move.
    pub fn take_move(&mut self) -> Option<(NavMove, MoveMods)> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Move(..)))?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Move(mv, mods) => Some((mv, mods)),
            _ => None,
        }
    }

    /// Takes the next edit command.
    pub fn take_edit(&mut self) -> Option<EditCommand> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Edit(_)))?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Edit(command) => Some(command),
            _ => None,
        }
    }

    /// Takes the next typed text.
    pub fn take_text(&mut self) -> Option<Arc<str>> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Text(_)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Text(text) => Some(Arc::clone(text)),
            _ => None,
        }
    }

    /// Takes the physical arrow nudge for canvas content: a unit direction and the modifiers.
    pub fn take_nudge(&mut self) -> Option<(Vec2, Mods)> {
        self.nudge.take()
    }

    /// Takes the AccessKit `SetValue` request.
    pub fn take_set_value(&mut self) -> Option<f64> {
        self.set_value.take()
    }

    /// Whether an intent of the kind of `intent` is routed and not taken.
    pub fn untaken(&self, intent: &Intent) -> bool {
        self.items.iter().any(|(item, taken)| !taken && item.same_kind(intent))
    }

    /// Takes the next `Intent::Access` request.
    pub fn take_access(&mut self) -> Option<AccessRequest> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Access(_)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
            _ => None,
        }
    }
}

/// The intents routed to `id` this pass, with physical horizontal moves read in the component's direction.
pub(crate) fn intents_of(ctx: &Context, id: Option<Id>, rtl: bool) -> Intents {
    let routed = current(ctx, |stage| stage.routed.get(&id).cloned()).unwrap_or_default();
    let flip = |mv| match (rtl, mv) {
        (true, NavMove::Back) => NavMove::Forward,
        (true, NavMove::Forward) => NavMove::Back,
        (_, mv) => mv,
    };
    let items = routed
        .items
        .into_iter()
        .map(|item| match item.intent {
            Intent::Move(mv, mods) if item.horizontal => (Intent::Move(flip(mv), mods), false),
            intent => (intent, false),
        })
        .collect();
    Intents { items, nudge: routed.nudge, set_value: routed.set_value }
}

/// Writes back what a component left untaken, so a scope's `Submit` and `Cancel` rules see it.
pub(crate) fn settle(ctx: &Context, id: Option<Id>, intents: &Intents) {
    current(ctx, |stage| {
        let Some(routed) = stage.routed.get_mut(&id) else { return };
        let mut taken = intents.items.iter().map(|(_, taken)| *taken);
        routed.items.retain(|_| !taken.next().unwrap_or(false));
        if intents.nudge.is_none() {
            routed.nudge = None;
        }
        if intents.set_value.is_none() {
            routed.set_value = None;
        }
    });
}

/// Routes `intent` to the component `target` for the rest of this pass, as if the key stage had: the label step
/// sends `Reset` to the control behind a label.
pub(crate) fn route(ctx: &Context, target: Id, intent: Intent) {
    current(ctx, |stage| push(&mut stage.routed, Some(target), intent, false));
}

/// Takes an untaken `Submit` for the innermost scope with a default action.
pub(crate) fn take_submit(ctx: &Context) -> bool {
    take_untaken(ctx, &Intent::Submit)
}

fn take_untaken(ctx: &Context, intent: &Intent) -> bool {
    let focused = focus::focused_rec(ctx).map(|rec| rec.stop.id);
    current(ctx, |stage| {
        let Some(routed) = stage.routed.get_mut(&focused) else { return false };
        match routed.items.iter().position(|item| item.intent.same_kind(intent)) {
            Some(at) => {
                routed.items.remove(at);
                true
            }
            None => false,
        }
    })
}

/// What an untaken Escape does in a scope: `None` when no Escape arrived or a component took it. One scope
/// takes each Escape.
pub(crate) fn scope_escape(ctx: &Context, rule: CancelRule, press_captured: bool, scope_clearable: bool) -> Option<EscapeStep> {
    if !take_untaken(ctx, &Intent::Cancel) {
        return None;
    }
    let facts = EscapeFacts::new().press_captured(press_captured).in_row_child(focus::in_row_child(ctx)).scope_clearable(scope_clearable);
    Some(escape_step(facts, rule))
}

/// Reads the claimed chord: whether it arrived this pass for `id`, consuming it. Also true for a chord
/// withheld from the component's text or editor part.
pub(crate) fn take_claimed(ctx: &Context, id: Id, chord: Chord) -> bool {
    current(ctx, |stage| match stage.claimed.iter().position(|(owner, claimed)| *owner == id && *claimed == chord) {
        Some(at) => {
            stage.claimed.remove(at);
            true
        }
        None => false,
    })
}

/// A state-dependent claim of `owner`, held from the next frame's input hook on while `owner` is the focused
/// component.
pub(crate) fn claim(ctx: &Context, owner: Id, chord: Chord) {
    current(ctx, |stage| stage.claims.push((owner, chord)));
}

/// Records that `id` asked for a context menu in this pass, to open at `rect`: a secondary-button release on it, or
/// a `ContextMenu` intent it did not take (9.2 rule 3).
pub(crate) fn request_context(ctx: &Context, id: Id, rect: egui::Rect) {
    current(ctx, |stage| {
        if !stage.contexts.iter().any(|(owner, _)| *owner == id) {
            stage.contexts.push((id, rect));
        }
    });
}

/// The rect `id`'s context request of this pass opens at, when it made one. A `Trigger::Context` opener reads it.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the overlay triggers (F5) read it"))]
pub(crate) fn context_request(ctx: &Context, id: Id) -> Option<egui::Rect> {
    current(ctx, |stage| stage.contexts.iter().find(|(owner, _)| *owner == id).map(|(_, rect)| *rect))
}

/// The member whose `Activate` the group `root` reads this pass.
pub(crate) fn activated_member(ctx: &Context, root: Id) -> Option<Id> {
    current(ctx, |stage| stage.activated.get(&root).copied())
}

/// The menu-bar keys of the pass (spec 9.7).
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn take_menu_keys(ctx: &Context) -> Vec<MenuKey> {
    current(ctx, |stage| std::mem::take(&mut stage.menu))
}

/// Registers an AccessKit node so a request that targets it reaches `owner`.
pub(crate) fn register_node(ctx: &Context, node: Id, owner: Id, part: Option<PartRef>, focusable: bool) {
    current(ctx, |stage| {
        stage.nodes.insert(node_key(node), Node { owner, part, focusable });
    });
}
