//! The key stage (spec 9.1, 9.2, 9.3, 9.7): the input hook strips the keys the runtime owns and records
//! every key event of the pass, the begin hook turns them into the `Intents` of the focused component,
//! and claims, menu-bar keys and AccessKit action requests are routed from the same record.

use crate::runtime::commands::{self, EditCommands, FirePhase};
use crate::runtime::egui::access::node_key;
use crate::runtime::egui::input::{self as raw_input, Action, ActionKind, KeyEvent, RawInput};
use crate::runtime::focus::{self, StopRec};
use crate::runtime::frame::PerViewport;
use crate::runtime::overlay;
use craft_behavior::focus::key_owner;
use craft_behavior::focus::{EscapeFacts, EscapeStep, escape_step};
use craft_behavior::keys;
use craft_behavior::{AccessRequest, CancelRule, EditCommand, EditorInput, Intent, KeyOwner, MoveMods, NavMove, OwnerClass, PartRef};
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
    /// The latest key press of the pass, which a following text event was typed by.
    last_press: Option<KeyInput>,
    /// What this pass's typed text taught: the physical key, the modifiers and the text.
    learned: Vec<(Key, Mods, String)>,
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
    set_value: Option<(Option<PartRef>, f64)>,
    /// A `Reset` the label step routed: it resets the whole binding, whatever part holds focus.
    whole_reset: bool,
}

#[derive(Clone, Copy)]
struct Node {
    owner: Id,
    part: Option<PartRef>,
    focusable: bool,
    /// The node's expanded state this pass, `None` when it has none.
    expanded: Option<bool>,
}

#[derive(Default)]
struct AltTrack {
    down: bool,
    other_key: bool,
    /// The right Alt key is down: text typed with it (AltGr) teaches the layout nothing.
    right: bool,
}

#[derive(Default)]
struct Stage {
    batch: Batch,
    routed: HashMap<Option<Id>, Routed>,
    claimed: Vec<(Id, Chord)>,
    claims: Vec<(Id, Chord)>,
    /// Chords a container holds for itself while focus is anywhere in the viewport (a window mode), from the
    /// pass that registered them.
    captures: Vec<(Id, Chord)>,
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
    /// Components with an open edit run this pass: Escape cancels the run before it leaves a row's child.
    open_edits: Vec<Id>,
    last_open_edits: Vec<Id>,
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
                batch.last_press = Some(input);
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
                if let Some(input) = batch.last_press.take().filter(|_| !alt.right)
                    && let Some(physical) = input.physical
                {
                    batch.learned.push((physical, input.mods, text.clone()));
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
                // The event stands for the chord, so the shortcut path is its only reader: egui does not copy or paste
                // on top of the command.
                false
            }
            KeyEvent::PointerPress => {
                alt.other_key = true;
                true
            }
            KeyEvent::RightAlt(down) => {
                alt.right = down;
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
    let os = crate::runtime::windows::os_of(ctx);
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
        claims.extend(stage.captures.iter().map(|(_, chord)| *chord));
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
        for (physical, mods, text) in std::mem::take(&mut batch.learned) {
            stage.layout.observe(physical, mods, &text);
        }
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
    let os = crate::runtime::windows::os_of(ctx);
    let target = focus::focused_rec(ctx);
    let (batch, nodes, captures) = current(ctx, |stage| {
        stage.routed.clear();
        stage.contexts.clear();
        stage.claimed.clear();
        stage.activated.clear();
        stage.claims.clear();
        stage.last_open_edits = std::mem::take(&mut stage.open_edits);
        stage.last_nodes = std::mem::take(&mut stage.nodes);
        (std::mem::take(&mut stage.batch), stage.last_nodes.clone(), std::mem::take(&mut stage.captures))
    });
    let mut routed: HashMap<Option<Id>, Routed> = HashMap::new();
    let mut activated = HashMap::new();
    let mut consumed = vec![false; batch.presses.len()];
    let dropdown = target.as_ref().and_then(|rec| Dropdown::of(ctx, rec));
    if let Some(rec) = &target {
        for (press, taken) in batch.presses.iter().zip(&mut consumed) {
            *taken = route_press(ctx, os, rec, press, dropdown, &mut routed, &mut activated);
        }
        for text in &batch.texts {
            let typed = rec.stop.role.type_ahead && !text.trim().is_empty();
            let to_list = dropdown.filter(|dropdown| !dropdown.text && !text.trim().is_empty());
            if let Some(dropdown) = to_list {
                push(&mut routed, Some(dropdown.list), Intent::Text(Arc::from(text.as_str())), false);
            } else if matches!(rec.stop.keys.owner, OwnerClass::Text) || typed {
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
        let claims_escape = target.as_ref().is_some_and(|rec| {
            rec.stop.keys.claims.iter().any(|chord| chord.key == Key::Escape) || current(ctx, |stage| stage.last_open_edits.contains(&rec.stop.id))
        });
        if !claims_escape && target.as_ref().is_some_and(|rec| focus::leave_row(ctx, rec.stop.key()).is_some()) {
            continue;
        }
        push(&mut routed, focused, Intent::Cancel, false);
    }
    // A text opener's list keys arrive as claimed chords, withheld from its text part: they go to the list.
    let mut claimed = Vec::new();
    for chord in &batch.claimed {
        match dropdown.and_then(|dropdown| dropdown.intent_of(chord)) {
            Some(intent) => push(&mut routed, dropdown.map(|dropdown| dropdown.list), intent, false),
            None => claimed.extend(captures.iter().find(|(_, held)| held == chord).map(|(owner, _)| *owner).or(focused).map(|id| (id, *chord))),
        }
    }
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
        if let (Some(_), Some(rec)) = (dropdown.filter(|dropdown| dropdown.text), &target) {
            stage.claims.extend(LIST_KEYS.iter().filter(|key| dropdown_intent(**key, true).is_some()).map(|key| (rec.stop.id, Chord::plain(*key))));
        }
    });
    escapes.cancels_press
}

fn push(routed: &mut HashMap<Option<Id>, Routed>, target: Option<Id>, intent: Intent, horizontal: bool) {
    routed.entry(target).or_default().items.push(Item { intent, horizontal });
}

/// The shortcut events the key owner leaves to the app (9.3). The owner is the focused component's class, or
/// `Blocked` while a modal layer is stacked above the focus; inside a layer its `Overlay.keys` class ranks with
/// the component's. `Menu` and `Recorder` keep every key; a `Widget` or popup keeps the keys
/// it activates with, the keys of its intents and the keys its filter keeps from the key stage; a text field
/// keeps the text its presses typed.
fn owned_shortcuts(ctx: &Context, target: Option<&StopRec>, batch: &Batch, consumed: &[bool]) -> Shortcuts {
    let layer = overlay::key_layer(ctx, &target.map(|rec| focus::scope_chain(ctx, rec)).unwrap_or_default());
    let own = target.map(|rec| rec.stop.keys.owner);
    let mut present: Vec<OwnerClass> = own.into_iter().collect();
    if let overlay::LayerKeys::Class(class) = layer {
        present.push(class);
    }
    let owner = key_owner(layer == overlay::LayerKeys::Blocked, &present);
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
    let silent = matches!(owner, KeyOwner::Class(OwnerClass::Menu | OwnerClass::Recorder));
    let dialog = owner == KeyOwner::Class(OwnerClass::Dialog);
    // The focused component keeps its own keys inside a dialog: a text field its typing, a widget its activation.
    let blocked = owner == KeyOwner::Blocked;
    let typing = !blocked && own == Some(OwnerClass::Text);
    let widget = !blocked && matches!(own, Some(OwnerClass::Widget | OwnerClass::Popup | OwnerClass::PassivePopup));
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

/// The collection of an open `KeepAnchor` dropdown whose opener has the key focus.
#[derive(Clone, Copy)]
struct Dropdown {
    /// The dropdown's layer.
    layer: Id,
    /// The collection inside the dropdown's layer.
    list: Id,
    /// The opener is a text owner: its typing and caret keys stay with it, and Tab goes to the list.
    text: bool,
}

/// The keys an opener hands to its dropdown's list. Left, Right and Space stay with the opener.
const LIST_KEYS: [Key; 8] = [Key::Up, Key::Down, Key::PageUp, Key::PageDown, Key::Home, Key::End, Key::Enter, Key::Tab];

impl Dropdown {
    fn of(ctx: &Context, opener: &StopRec) -> Option<Dropdown> {
        let (layer, list) = focus::registration::dropdown_target(ctx, opener.stop.key())?;
        Some(Dropdown { layer, list, text: matches!(opener.stop.keys.owner, OwnerClass::Text) })
    }

    /// The intent a claimed chord of the opener becomes for the list.
    fn intent_of(self, chord: &Chord) -> Option<Intent> {
        chord.mods.is_none().then(|| dropdown_intent(chord.key, self.text)).flatten()
    }
}

/// The list intent of a plain key the opener hands on: vertical moves and Page keys, Home and End (a text opener
/// keeps them for its caret), and Enter as the activation. A text opener's Tab accepts the lead too: there Tab is
/// not the way to the next stop while the list is open.
fn dropdown_intent(key: Key, text: bool) -> Option<Intent> {
    let moved = |mv| Some(Intent::Move(mv, MoveMods::PLAIN));
    match key {
        Key::Up => moved(NavMove::Up),
        Key::Down => moved(NavMove::Down),
        Key::PageUp => moved(NavMove::PageUp),
        Key::PageDown => moved(NavMove::PageDown),
        Key::Home if !text => moved(NavMove::Start),
        Key::End if !text => moved(NavMove::End),
        Key::Enter => Some(Intent::Activate),
        Key::Tab if text => Some(Intent::Activate),
        _ => None,
    }
}

/// One key press of the focused component: the table's row, else the activation keys, else a nudge. The keys of
/// an open dropdown's list (`LIST_KEYS`) go to the dropdown's collection first. Returns whether the component's
/// intents took the press.
fn route_press(
    ctx: &Context,
    os: Os,
    rec: &StopRec,
    press: &Press,
    dropdown: Option<Dropdown>,
    routed: &mut HashMap<Option<Id>, Routed>,
    activated: &mut HashMap<Id, Id>,
) -> bool {
    let stop = &rec.stop;
    let id = Some(stop.id);
    let horizontal = matches!(press.key, Key::Left | Key::Right);
    let plain = press.mods.is_none();
    if let Some(dropdown) = dropdown
        && plain
        && let Some(intent) = dropdown_intent(press.key, dropdown.text)
    {
        push(routed, Some(dropdown.list), intent, false);
        return true;
    }
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
        Some(Intent::Activate) => route_activate(ctx, rec, routed, activated),
        Some(Intent::Close) => match dropdown {
            Some(dropdown) => overlay::request_close(ctx, dropdown.layer, overlay::CloseReason::Requested),
            None => push(routed, id, Intent::Close, false),
        },
        Some(intent) => push(routed, id, intent, false),
        None if press.key == Key::Enter && plain => {
            if stop.keys.activate.enter() {
                route_activate(ctx, rec, routed, activated);
            } else {
                push(routed, id, Intent::Submit, false);
            }
        }
        None if press.key == Key::Space && plain && stop.keys.activate.space() => route_activate(ctx, rec, routed, activated),
        None if press.key == Key::Tab && plain && stop.keys.keep == focus::KeepKeys::Tab => push(routed, id, Intent::Tab, false),
        None if stop.role.nudge => {
            let Some(nudge) = nudge_of(press.key, press.mods) else { return false };
            routed.entry(id).or_default().nudge = Some(nudge);
        }
        None => return false,
    }
    true
}

/// `Activate` of a group member goes to the group's root, which reads `activated_member`. A child or part of a
/// collection row activates itself.
fn route_activate(ctx: &Context, rec: &StopRec, routed: &mut HashMap<Option<Id>, Routed>, activated: &mut HashMap<Id, Id>) {
    let target = if rec.stop.group.is_some() && !focus::is_row_child(ctx, &rec.stop) {
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
        ActionKind::Expand | ActionKind::Collapse if node.expanded.is_some() => {
            if node.expanded == Some(matches!(request.kind(), ActionKind::Collapse)) {
                // A sub-node with no stop of its own (a chevron) names its part: the component decides what it opens.
                if node.part.is_some() && !node.focusable {
                    push(routed, owner, Intent::Access(request.request(node.part)), false);
                    return;
                }
                match focus::stop_rec(ctx, (node.owner, node.part)).filter(|rec| rec.stop.group.is_some()) {
                    Some(rec) => route_activate(ctx, &rec, routed, activated),
                    None => push(routed, owner, Intent::Activate, false),
                }
            }
        }
        ActionKind::Click => match focus::stop_rec(ctx, (node.owner, node.part)).filter(|rec| rec.stop.group.is_some()) {
            Some(rec) => route_activate(ctx, &rec, routed, activated),
            None => push(routed, owner, Intent::Activate, false),
        },
        ActionKind::Increment | ActionKind::Decrement if node.part.is_some() && !node.focusable => {
            push(routed, owner, Intent::Access(request.request(node.part)), false)
        }
        ActionKind::Increment => push(routed, owner, Intent::Move(NavMove::Forward, MoveMods::PLAIN), false),
        ActionKind::Decrement => push(routed, owner, Intent::Move(NavMove::Back, MoveMods::PLAIN), false),
        ActionKind::ShowContextMenu => push(routed, owner, Intent::ContextMenu, false),
        ActionKind::SetValue(value) => routed.entry(owner).or_default().set_value = Some((node.part, value)),
        ActionKind::Focus | ActionKind::Expand | ActionKind::Collapse | ActionKind::Other => {
            push(routed, owner, Intent::Access(request.request(node.part)), false)
        }
    }
}

/// An owned copy of the intents routed to one id this pass; read through `cx.intents()`.
#[derive(Clone, Debug, Default)]
pub struct Intents {
    items: Vec<(Intent, bool)>,
    nudge: Option<(Vec2, Mods)>,
    set_value: Option<(Option<PartRef>, f64)>,
    whole_reset: bool,
}

impl Intents {
    /// Takes a `Reset`, once per pass: `Some(true)` when the label step sent it (the whole binding resets),
    /// `Some(false)` when a key sent it (it resets the focused part when that part has its own reset).
    pub(crate) fn take_reset(&mut self) -> Option<bool> {
        self.take(Intent::Reset).then_some(self.whole_reset)
    }

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

    /// Takes an intent equal to `intent` in every field, once per pass: `Intent::delete(false, Granularity::Grapheme)`
    /// takes Backspace and leaves Delete, which `take` would also take.
    pub fn take_exact(&mut self, intent: &Intent) -> bool {
        match self.items.iter().position(|(item, taken)| !taken && item == intent) {
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

    /// Takes the next move that scrolls (page up or down, start, end): what a focused component that does not
    /// take it leaves to the scroll owner around it (9.2 rule 5).
    pub(crate) fn take_scroll_move(&mut self) -> Option<NavMove> {
        let at = self.items.iter().position(|(item, taken)| {
            !taken && matches!(item, Intent::Move(NavMove::PageUp | NavMove::PageDown | NavMove::Start | NavMove::End, _))
        })?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Move(mv, _) => Some(mv),
            _ => None,
        }
    }

    /// Takes the next AccessKit request that asks a scroll region to scroll, or with `into_view` to show its target.
    pub(crate) fn take_scroll_access(&mut self, into_view: bool) -> Option<AccessRequest> {
        use craft_behavior::AccessAction as Action;
        let at = self.items.iter().position(|(item, taken)| {
            !taken
                && matches!(
                    item,
                    Intent::Access(request)
                        if request.target.is_none()
                            && (request.action == Action::ScrollIntoView) == into_view
                            && matches!(
                                request.action,
                                Action::ScrollUp | Action::ScrollDown | Action::ScrollLeft | Action::ScrollRight | Action::ScrollIntoView | Action::SetScrollOffset
                            )
                )
        })?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
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

    /// Takes the AccessKit `SetValue` request aimed at the component's own node.
    pub fn take_set_value(&mut self) -> Option<f64> {
        self.take_set_value_of(None)
    }

    /// Takes the AccessKit `SetValue` request aimed at the sub-node of `part`.
    pub fn take_set_value_at(&mut self, part: impl Into<PartRef>) -> Option<f64> {
        self.take_set_value_of(Some(part.into()))
    }

    fn take_set_value_of(&mut self, part: Option<PartRef>) -> Option<f64> {
        match self.set_value {
            Some((target, value)) if target == part => {
                self.set_value = None;
                Some(value)
            }
            _ => None,
        }
    }

    /// Whether an intent of the kind of `intent` is routed and not taken.
    pub fn untaken(&self, intent: &Intent) -> bool {
        self.items.iter().any(|(item, taken)| !taken && item.same_kind(intent))
    }

    /// Takes the next intent an editor reads as input: typed text, a move, a delete, an edit command, Enter as a
    /// newline, Tab, and Escape as a cancel. An arrow move the editor's `keep` does not keep stays for the
    /// component's navigation (18.4), and with `KeepKeys::None` so do Home, End and the page keys: the editor is
    /// a part of a grid that navigates until a cell opens.
    pub(crate) fn take_editor_input(&mut self, keep: focus::KeepKeys) -> Option<EditorInput> {
        let kept = |item: &Intent| match item {
            Intent::Move(NavMove::Up | NavMove::Down, _) => matches!(keep, focus::KeepKeys::BlockArrows | focus::KeepKeys::Tab),
            Intent::Move(NavMove::Back | NavMove::Forward, _) => keep != focus::KeepKeys::None,
            Intent::Move(NavMove::Start | NavMove::End | NavMove::PageUp | NavMove::PageDown, _) => keep != focus::KeepKeys::None,
            _ => true,
        };
        let at = self.items.iter().position(|(item, taken)| !taken && kept(item) && editor_input_of(item).is_some())?;
        self.items[at].1 = true;
        editor_input_of(&self.items[at].0)
    }

    /// Takes the next `Intent::Access` request that `wanted` accepts.
    pub(crate) fn take_access_where(&mut self, wanted: impl Fn(&AccessRequest) -> bool) -> Option<AccessRequest> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Access(request) if wanted(request)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
            _ => None,
        }
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

fn editor_input_of(intent: &Intent) -> Option<EditorInput> {
    match intent {
        Intent::Text(text) => Some(EditorInput::Text(Arc::clone(text))),
        Intent::Move(mv, mods) => Some(EditorInput::Move(*mv, *mods)),
        Intent::Delete { forward, by, .. } => Some(EditorInput::delete(*forward, *by)),
        Intent::Edit(command) => Some(EditorInput::Edit(*command)),
        Intent::Submit => Some(EditorInput::Newline),
        Intent::Tab => Some(EditorInput::Tab),
        Intent::Cancel => Some(EditorInput::Cancel),
        _ => None,
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
    Intents { items, nudge: routed.nudge, set_value: routed.set_value, whole_reset: routed.whole_reset }
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

/// Routes a `Reset` of the whole binding to the component `target` for the rest of this pass: the label step sends
/// it for a double click on the label and for the reset dot.
pub(crate) fn route_whole_reset(ctx: &Context, target: Id) {
    current(ctx, |stage| {
        push(&mut stage.routed, Some(target), Intent::Reset, false);
        stage.routed.entry(Some(target)).or_default().whole_reset = true;
    });
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

/// Records that `owner` has an open edit run: the next Escape cancels the run and stays with the component.
pub(crate) fn hold_escape(ctx: &Context, owner: Id) {
    current(ctx, |stage| stage.open_edits.push(owner));
}

/// A state-dependent claim of `owner`, held from the next frame's input hook on while `owner` is the focused
/// component.
pub(crate) fn claim(ctx: &Context, owner: Id, chord: Chord) {
    current(ctx, |stage| stage.claims.push((owner, chord)));
}

/// Holds `chord` for `owner` from the next input hook on, whichever component has focus: the key stage withholds
/// it from every component and `take_claimed` reads it. Used while a container is in a mode that owns the arrow
/// keys, Enter and Escape (the window move and size modes, 16.3); the claim lapses when it is not renewed.
pub(crate) fn capture(ctx: &Context, owner: Id, chord: Chord) {
    current(ctx, |stage| {
        if !stage.captures.contains(&(owner, chord)) {
            stage.captures.push((owner, chord));
        }
    });
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
pub(crate) fn take_menu_keys(ctx: &Context) -> Vec<MenuKey> {
    current(ctx, |stage| std::mem::take(&mut stage.menu))
}

/// Records the expanded state of the node registered by `register_node`: an `Expand` or `Collapse` request on a node
/// with the state asks to change it, and acts as the node's activation.
pub(crate) fn register_expanded(ctx: &Context, node: Id, expanded: Option<bool>) {
    current(ctx, |stage| {
        if let Some(entry) = stage.nodes.get_mut(&node_key(node)) {
            entry.expanded = expanded;
        }
    });
}

/// Registers an AccessKit node so a request that targets it reaches `owner`.
pub(crate) fn register_node(ctx: &Context, node: Id, owner: Id, part: Option<PartRef>, focusable: bool) {
    current(ctx, |stage| {
        stage.nodes.insert(node_key(node), Node { owner, part, focusable, expanded: None });
    });
}
