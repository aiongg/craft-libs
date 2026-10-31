//! Who gets a frame's keys: an open menu, a blocking workspace, a dialog, the shortcut recorder, a
//! text field, a focused widget, or the app's shortcuts.
//!
//! [`crate::egui::Dispatcher`] runs the keymap against the frame's key events; it knows whether
//! text has the keys ([`crate::Active::typing`]) and nothing else. [`KeyRouter`] decides the
//! rest before and around it, in one place for every app: what [`KeyOwner`] has the keyboard this
//! frame, which hits that owner leaves to the shortcuts, and the keys a focused control claimed
//! ([`craft_ui::behavior::KeyClaim`]). Apps call [`KeyRouter::route`] once per frame, before the UI
//! reads its keys, instead of calling the dispatcher directly.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use crate::chord::Chord;
use crate::command::CommandRef;
use crate::egui::{Dispatcher, Hit};
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
        todo!()
    }

    pub fn dispatcher(&self) -> &Dispatcher {
        todo!()
    }

    pub fn dispatcher_mut(&mut self) -> &mut Dispatcher {
        todo!()
    }

    /// Who has the keyboard: [`KeyOwner`]'s order, reading `gate`, craft-ui's open menus and
    /// dropdowns ([`craft_ui::menu::any_open`]) and egui's focus (a focused `TextEdit` is
    /// [`KeyOwner::Text`], any other focused widget [`KeyOwner::Widget`]).
    // pc: crates/ui-egui/src/input/shortcuts.rs:482 `preflight_keys` (Camera Raw, foreground save,
    // open menu or menu bar, dropdown, stroke editor); crates/ui-egui/src/input/shortcut_dispatch.rs:142
    // `Focus::of`.
    pub fn owner(ctx: &Context, gate: &KeyGate) -> KeyOwner {
        todo!()
    }

    /// Whether `owner` leaves a hit on `chord` running `command` to the shortcuts. Ctrl+Tab and
    /// Ctrl+Shift+Tab (next and previous document) run whatever has focus, as in Photoshop. A
    /// chord the focused control claimed ([`craft_ui::behavior::KeyClaim::claimed`]) is never
    /// left to the shortcuts.
    // pc: crates/ui-egui/src/input/shortcut_dispatch.rs:163 `Focus::allows`.
    pub fn allows(ctx: &Context, owner: KeyOwner, gate: &KeyGate, chord: &Chord, command: &CommandRef) -> bool {
        todo!()
    }

    /// This frame's hits: [`KeyRouter::owner`], then the dispatcher with `active` (typing set when
    /// the owner is [`KeyOwner::Text`]), keeping only the hits [`KeyRouter::allows`]; a declined
    /// press stays in egui's input for the focused control. Nothing runs, and no event is taken,
    /// while the owner is [`KeyOwner::Blocked`], [`KeyOwner::Menu`] (but a command chord) or
    /// [`KeyOwner::Recorder`].
    // pc: crates/ui-egui/src/input/shortcuts.rs:469 `handle`, :662 `handle_keys`.
    pub fn route(&mut self, ctx: &Context, keymap: &Keymap, active: &Active, gate: &KeyGate) -> Vec<Hit> {
        todo!()
    }
}
