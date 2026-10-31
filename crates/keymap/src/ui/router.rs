//! Who gets a frame's keys ([`KeyOwner`]): an open menu, a blocking workspace, a dialog, the
//! shortcut recorder, a text field, a popover, a focused widget, or the app's shortcuts.
//!
//! [`crate::egui::Dispatcher`] runs the keymap against the frame's key events; it knows whether
//! text has the keys ([`crate::Active::typing`]) and nothing else. [`KeyRouter`] decides the
//! rest before and around it, in one place for every app: what [`KeyOwner`] has the keyboard this
//! frame, which hits that owner leaves to the shortcuts, and the keys a focused control claimed
//! ([`craft_ui::behavior::KeyClaim`]). Apps call [`KeyRouter::route`] once per frame, before the UI
//! reads its keys, instead of calling the dispatcher directly.

use craft_ui::behavior::KeyClaim;
pub use craft_ui::focus::key_owner::KeyOwner;
use craft_ui::focus::key_owner::{OwnerGate, owner};
use egui::Context;

use crate::chord::{Chord, Key};
use crate::command::CommandRef;
use crate::egui::{Dispatcher, Hit, platform};
use crate::keymap::{Keymap, text_editing_chords};
use crate::scope::Active;

/// The view commands Photoshop runs while a dialog is open: Zoom In, Zoom Out, Fit on Screen and
/// 100 %, by PhotoCraft's ids. An app whose ids differ passes its own list. The menus
/// (`craft_menu::NAVIGATION_COMMANDS`) and [`KeyGate::in_dialogs`] use this one list.
pub const NAVIGATION_COMMANDS: [&str; 4] = ["view.zoomIn", "view.zoomOut", "view.fitOnScreen", "view.actualPixels"];

/// What the app knows about its own modal state this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyGate<'a> {
    /// A full-window workspace or a blocking job has the window ([`KeyOwner::Blocked`]).
    pub blocked: bool,
    /// A modal dialog is open ([`KeyOwner::Dialog`]).
    pub dialog: bool,
    /// The shortcut recorder is recording ([`KeyOwner::Recorder`];
    /// [`crate::ui::DialogState::is_recording`]).
    pub recording: bool,
    /// Commands that still run while a dialog is open, by id: [`NAVIGATION_COMMANDS`], so the
    /// menus and the keys agree.
    pub in_dialogs: &'a [&'a str],
}

impl KeyGate<'_> {
    fn owner_gate(&self) -> OwnerGate {
        OwnerGate { blocked: self.blocked, dialog: self.dialog, recording: self.recording }
    }
}

/// The app's key routing, kept between frames (it holds the dispatcher's double-press and
/// keyboard-layout state).
///
/// ```no_run
/// use craft_keymap::ui::{KeyGate, KeyRouter, NAVIGATION_COMMANDS};
/// use craft_keymap::{Active, egui::Dispatcher};
/// # fn demo(ctx: &egui::Context, router: &mut KeyRouter, keymap: &craft_keymap::Keymap, dialog_open: impl Fn() -> bool) {
/// let gate = || KeyGate { dialog: dialog_open(), in_dialogs: &NAVIGATION_COMMANDS, ..KeyGate::default() };
/// router.route(ctx, keymap, &Active::NONE, gate, |hit| {
///     // run hit.command
/// });
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct KeyRouter {
    dispatcher: Dispatcher,
}

impl KeyRouter {
    /// A router over `dispatcher` (its repeat, clipboard and layout settings stay).
    pub fn new(dispatcher: Dispatcher) -> KeyRouter {
        KeyRouter { dispatcher }
    }

    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    pub fn dispatcher_mut(&mut self) -> &mut Dispatcher {
        &mut self.dispatcher
    }

    /// Who has the keyboard: [`craft_ui::focus::key_owner::owner`] with `gate`.
    pub fn owner(ctx: &Context, gate: &KeyGate) -> KeyOwner {
        owner(ctx, &gate.owner_gate())
    }

    /// Whether `owner` leaves a hit on `chord` running `command` to the shortcuts. Ctrl+Tab and
    /// Ctrl+Shift+Tab (next and previous document) run whatever has focus, as in Photoshop. A
    /// chord the focused control claimed ([`craft_ui::behavior::KeyClaim::claimed`]) is never
    /// left to the shortcuts. The command key is Cmd on macOS and Ctrl elsewhere; Control on a Mac
    /// is a text-editing key and never a command chord.
    // pc: crates/ui-egui/src/input/shortcut_dispatch.rs:158 `Focus::allows`, :212-220.
    pub fn allows(ctx: &Context, owner: KeyOwner, gate: &KeyGate, chord: &Chord, command: &CommandRef) -> bool {
        let command_chord = chord.mods.cmd;
        match owner {
            KeyOwner::Blocked | KeyOwner::Recorder | KeyOwner::Menu => false,
            KeyOwner::Dialog => !ctx.text_edit_focused() && gate.in_dialogs.contains(&command.id.as_str()),
            KeyOwner::Text | KeyOwner::Popover | KeyOwner::Widget | KeyOwner::App => {
                if KeyClaim::claimed(ctx, &chord.to_egui_shortcut()) {
                    return false;
                }
                let platform = platform(ctx);
                let ctrl_tab = chord.key == Key::Tab && (chord.mods.ctrl || (chord.mods.cmd && !platform.is_mac()));
                if ctrl_tab {
                    return true;
                }
                match owner {
                    KeyOwner::Text => {
                        chord.is_function_key() || (command_chord && !text_editing_chords(platform).contains(&chord.effective(platform)))
                    }
                    KeyOwner::Popover => command_chord,
                    KeyOwner::Widget => command_chord || !navigation(chord.key),
                    _ => true,
                }
            }
        }
    }

    /// Runs this frame's hits through `run`, in press order. First [`leave_for_shortcut`]
    /// (a command chord takes the keyboard off a menu bar with no menu open), then
    /// [`KeyRouter::owner`] and the dispatcher with `active` (typing set when the owner is
    /// [`KeyOwner::Text`]), keeping only the hits [`KeyRouter::allows`]; a declined press stays in
    /// egui's input for the focused control. Nothing runs, and no event is taken, while the owner
    /// is [`KeyOwner::Blocked`], [`KeyOwner::Menu`] or [`KeyOwner::Recorder`].
    ///
    /// `gate` is asked again after each command: when the gate or the owner changed (a command
    /// opened a dialog), the remaining presses are left in egui's input for the new owner instead
    /// of running as shortcuts. `run` executes while the frame's key events are still in egui's
    /// input, so a command reads no key events itself.
    // pc: crates/ui-egui/src/input/shortcuts.rs:469 `handle`, :662 `handle_keys`;
    // shortcut_dispatch.rs:268-272 (`key_owner` compared after each hit).
    pub fn route<'g>(&mut self, ctx: &Context, keymap: &Keymap, active: &Active, mut gate: impl FnMut() -> KeyGate<'g>, mut run: impl FnMut(Hit)) {
        craft_ui::menu::bar::leave_for_shortcut(ctx);
        let mut state = gate();
        let mut owner = KeyRouter::owner(ctx, &state);
        if matches!(owner, KeyOwner::Blocked | KeyOwner::Recorder | KeyOwner::Menu) {
            return;
        }
        let active = Active { typing: active.typing || owner == KeyOwner::Text || ctx.text_edit_focused(), ..*active };
        let mut stopped = false;
        // `allows` reads egui's data, which `input_mut` holds locked: dispatch on a copy and put
        // back the events it left.
        let mut input = ctx.input(|i| i.clone());
        self.dispatcher.dispatch_filtered(&mut input, keymap, &active, |hit| {
            if stopped || !KeyRouter::allows(ctx, owner, &state, &hit.trigger.chord, &hit.command) {
                return false;
            }
            run(hit.clone());
            let (next_state, next_owner) = {
                let next = gate();
                (next, KeyRouter::owner(ctx, &next))
            };
            stopped = next_state != state || next_owner != owner;
            (state, owner) = (next_state, next_owner);
            true
        });
        let events = input.events;
        ctx.input_mut(|i| i.events = events);
    }
}

/// The keys a focused control moves or activates with.
fn navigation(key: Key) -> bool {
    matches!(
        key,
        Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::Enter
            | Key::Space
            | Key::Tab
            | Key::Escape
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
    )
}
