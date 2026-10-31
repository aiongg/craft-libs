//! Who gets a frame's keys: an open menu, a blocking workspace, a dialog, the shortcut recorder, a
//! text field, a focused widget, or the app's shortcuts.
//!
//! [`crate::egui::Dispatcher`] runs the keymap against the frame's key events; it knows whether
//! text has the keys ([`crate::Active::typing`]) and nothing else. [`KeyRouter`] decides the
//! rest before and around it, in one place for every app: what [`KeyOwner`] has the keyboard this
//! frame, which hits that owner leaves to the shortcuts, and the keys a focused control claimed
//! ([`craft_ui::behavior::KeyClaim`]). Apps call [`KeyRouter::route`] once per frame, before the UI
//! reads its keys, instead of calling the dispatcher directly.

use craft_ui::behavior::KeyClaim;
use egui::Context;

use crate::chord::{Chord, Key};
use crate::command::CommandRef;
use crate::egui::{Dispatcher, Hit, platform};
use crate::keymap::Keymap;
use crate::scope::Active;

/// What has the keyboard this frame, first match wins in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyOwner {
    /// A full-window workspace (Camera Raw, Liquify) or a blocking job (a foreground save): no app
    /// shortcut runs; the owner reads the keys itself.
    Blocked,
    /// An open menu, the menu bar holding the keyboard (after Alt or F10), or an open dropdown
    /// list. Arrows, letters, Enter and Esc are the menu's. A command chord (⌘D, F7) closes the
    /// menu and runs, as with a native menu bar.
    Menu,
    /// The Keyboard Shortcuts dialog is recording a chord: every key is the recorder's.
    Recorder,
    /// A modal dialog: only [`KeyGate::in_dialogs`] run (Zoom In, Zoom Out, Fit on Screen, 100 %);
    /// the dialog keeps every other key.
    Dialog,
    /// A text field: shortcuts with ⌘ (Ctrl) run but the field's own editing chords
    /// ([`crate::text_editing_chords`]), and so do the function keys; plain keys type.
    Text,
    /// A focused control that isn't text (a slider, a list row, a button reached with Tab): its
    /// navigation keys (arrows, Enter, Space, Esc, Tab) and the keys it claimed are its; every
    /// other shortcut runs, as in Photoshop.
    Widget,
    /// Nothing focused (the canvas): every shortcut runs.
    App,
}

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
    /// Commands that still run while a dialog is open, by id: craft-menu's
    /// `NAVIGATION_COMMANDS`, so the menus and the keys agree.
    pub in_dialogs: &'a [&'a str],
}

/// The app's key routing, kept between frames (it holds the dispatcher's double-press and
/// keyboard-layout state).
///
/// ```no_run
/// use craft_keymap::ui::{KeyGate, KeyRouter};
/// use craft_keymap::{Active, egui::Dispatcher};
/// # fn demo(ctx: &egui::Context, router: &mut KeyRouter, keymap: &craft_keymap::Keymap, dialog_open: bool) {
/// let gate = KeyGate { dialog: dialog_open, in_dialogs: &["view.zoomIn", "view.zoomOut"], ..KeyGate::default() };
/// for hit in router.route(ctx, keymap, &Active::NONE, &gate) {
///     // run hit.command
/// }
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

    /// Who has the keyboard: [`KeyOwner`]'s order, reading `gate`, craft-ui's open menus and
    /// dropdowns ([`craft_ui::menu::any_open`]) and egui's focus (a focused `TextEdit` is
    /// [`KeyOwner::Text`], any other focused widget [`KeyOwner::Widget`]).
    // pc: crates/ui-egui/src/input/shortcuts.rs:482 `preflight_keys` (Camera Raw, foreground save,
    // open menu or menu bar, dropdown, stroke editor); crates/ui-egui/src/input/shortcut_dispatch.rs:142
    // `Focus::of`.
    pub fn owner(ctx: &Context, gate: &KeyGate) -> KeyOwner {
        if gate.blocked {
            KeyOwner::Blocked
        } else if craft_ui::menu::any_open(ctx) {
            KeyOwner::Menu
        } else if gate.recording {
            KeyOwner::Recorder
        } else if gate.dialog {
            KeyOwner::Dialog
        } else if ctx.text_edit_focused() {
            KeyOwner::Text
        } else if ctx.egui_wants_keyboard_input() {
            KeyOwner::Widget
        } else {
            KeyOwner::App
        }
    }

    /// Whether `owner` leaves a hit on `chord` running `command` to the shortcuts. Ctrl+Tab and
    /// Ctrl+Shift+Tab (next and previous document) run whatever has focus, as in Photoshop. A
    /// chord the focused control claimed ([`craft_ui::behavior::KeyClaim::claimed`]) is never
    /// left to the shortcuts.
    // pc: crates/ui-egui/src/input/shortcut_dispatch.rs:158 `Focus::allows`.
    pub fn allows(ctx: &Context, owner: KeyOwner, gate: &KeyGate, chord: &Chord, command: &CommandRef) -> bool {
        let command_chord = chord.mods.cmd || chord.mods.ctrl;
        match owner {
            KeyOwner::Blocked | KeyOwner::Recorder => false,
            KeyOwner::Menu => command_chord || chord.is_function_key(),
            KeyOwner::Dialog => gate.in_dialogs.contains(&command.id.as_str()),
            KeyOwner::Text | KeyOwner::Widget | KeyOwner::App => {
                if KeyClaim::claimed(ctx, &chord.to_egui_shortcut()) {
                    return false;
                }
                let ctrl_tab = chord.key == Key::Tab && (chord.mods.ctrl || (chord.mods.cmd && !platform(ctx).is_mac()));
                if ctrl_tab {
                    return true;
                }
                match owner {
                    KeyOwner::Text => chord.is_function_key() || (command_chord && !text_owned(chord)),
                    KeyOwner::Widget => command_chord || !navigation(chord.key),
                    _ => true,
                }
            }
        }
    }

    /// This frame's hits: [`KeyRouter::owner`], then the dispatcher with `active` (typing set when
    /// the owner is [`KeyOwner::Text`]), keeping only the hits [`KeyRouter::allows`]; a declined
    /// press stays in egui's input for the focused control. Nothing runs, and no event is taken,
    /// while the owner is [`KeyOwner::Blocked`], [`KeyOwner::Menu`] (but a command chord) or
    /// [`KeyOwner::Recorder`].
    // pc: crates/ui-egui/src/input/shortcuts.rs:469 `handle`, :662 `handle_keys`.
    pub fn route(&mut self, ctx: &Context, keymap: &Keymap, active: &Active, gate: &KeyGate) -> Vec<Hit> {
        let owner = KeyRouter::owner(ctx, gate);
        if matches!(owner, KeyOwner::Blocked | KeyOwner::Recorder) {
            return Vec::new();
        }
        let active = Active { typing: active.typing || owner == KeyOwner::Text || ctx.text_edit_focused(), ..*active };
        // `allows` reads egui's data, which `input_mut` holds locked: dispatch on a copy and put
        // back the events it left.
        let mut input = ctx.input(|i| i.clone());
        let hits = self
            .dispatcher
            .dispatch_filtered(&mut input, keymap, &active, |hit| KeyRouter::allows(ctx, owner, gate, &hit.trigger.chord, &hit.command));
        let events = input.events;
        ctx.input_mut(|i| i.events = events);
        hits
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

/// The chords a text field edits with: select all, the clipboard, undo and redo (without Alt),
/// its cursor keys and deletions.
fn text_owned(chord: &Chord) -> bool {
    let editing = !chord.mods.alt && matches!(chord.key, Key::A | Key::C | Key::X | Key::V | Key::Z | Key::Y);
    editing || navigation(chord.key) || matches!(chord.key, Key::Backspace | Key::Delete)
}
