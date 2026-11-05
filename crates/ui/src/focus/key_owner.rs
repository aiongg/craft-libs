//! Who has the keyboard this pass: one answer for the key router, the page, digit and momentary
//! tool keys, nudging, the chrome's Tab toggle and F6.
//!
//! Every reader of single keys asks [`owner`] instead of keeping its own rule. The canvas the
//! shell registers (`AppShell::canvas`) counts as no focus at
//! all, so a focused canvas is [`KeyOwner::App`].

use egui::{Context, Id, LayerId};

/// What has the keyboard, first match wins in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyOwner {
    /// A full-window workspace (Camera Raw, Liquify) or a blocking job (a foreground save): no app
    /// shortcut runs; the owner reads the keys itself.
    Blocked,
    /// An open menu, the menu bar holding the keyboard (after Alt or F10), or an open popup list
    /// (a dropdown, a combo box, a popover). Arrows, letters, Enter and Esc are its; no shortcut
    /// runs. A command chord on a menu bar with no menu open first takes the keyboard off the bar
    /// ([`menu::bar::leave_for_shortcut`](crate::menu::bar::leave_for_shortcut)).
    Menu,
    /// The Keyboard Shortcuts dialog is recording a chord: every key is the recorder's.
    Recorder,
    /// A modal dialog (an egui modal layer, an app-modal window, or [`OwnerGate::dialog`]): only
    /// the commands the app allows in dialogs run, and none while a text field has focus.
    Dialog,
    /// A text field: shortcuts with the command key run except the field's own editing chords,
    /// and so do the function keys; plain keys type.
    Text,
    /// A non-modal popover is open (PhotoCraft's stroke editor,
    /// [`PopoverClose::outside`](crate::popover::PopoverClose::outside) off): command chords run,
    /// every other key is the popover's.
    Popover,
    /// A focused control that isn't text and isn't the canvas (a slider, a list row, a button
    /// reached with Tab): its navigation keys and the keys it claimed are its; every other
    /// shortcut runs, as in Photoshop.
    Widget,
    /// Nothing focused, or the registered canvas: every shortcut runs, and the canvas reads its
    /// own keys (page keys, digits, tool letters, nudging).
    App,
}

impl KeyOwner {
    /// Whether the owner takes every key, leaving the shortcuts and the canvas nothing:
    /// [`Blocked`](Self::Blocked), [`Menu`](Self::Menu), [`Recorder`](Self::Recorder) and
    /// [`Dialog`](Self::Dialog).
    pub fn is_exclusive(self) -> bool {
        matches!(self, KeyOwner::Blocked | KeyOwner::Menu | KeyOwner::Recorder | KeyOwner::Dialog)
    }
}

/// What the app knows about its own modal state this pass. Modal layers, menus, popups, text
/// focus and the canvas are read from `ctx`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnerGate {
    /// A full-window workspace or a blocking job has the window ([`KeyOwner::Blocked`]).
    pub blocked: bool,
    /// A modal dialog the app draws without an egui modal layer is open ([`KeyOwner::Dialog`]).
    pub dialog: bool,
    /// The shortcut recorder is recording ([`KeyOwner::Recorder`]).
    pub recording: bool,
}

/// Registers `id` as the canvas of this pass for a canvas drawn outside an
/// [`AppShell`](crate::shell::AppShell) (a story, a test); the shell does it through
/// `AppShell::canvas`. Call it on every pass the canvas is drawn.
pub fn register_canvas(ctx: &Context, id: egui::Id) {
    crate::shell::register_canvas(ctx, id);
}

/// Whether the canvas's own single keys (page keys, digits, the Space hand) run: nothing but a
/// focused widget that isn't text has the keyboard, and then the widget keeps what it claimed
/// ([`KeyClaim`](crate::behavior::KeyClaim)). Other focused widgets leave these keys to the
/// shortcuts, as in Photoshop. This is [`owner`] being [`KeyOwner::App`] or
/// [`KeyOwner::Widget`] with no modal state from the app (a default [`OwnerGate`]); an app with
/// its own gate asks [`owner`] itself.
// pc: crates/ui-egui/src/input/shortcuts.rs:824 `handle_escape_and_view_keys`, :872 page keys.
pub fn canvas_keys_free(ctx: &Context) -> bool {
    matches!(owner(ctx, &OwnerGate::default()), KeyOwner::App | KeyOwner::Widget)
}

/// [`canvas_keys_free`] for the chord `shortcut`: also false when the focused control claimed
/// that chord ([`KeyClaim::claimed`](crate::behavior::KeyClaim::claimed)), so a Delete shortcut
/// leaves a curve editor's Delete to the editor.
pub fn canvas_keys_free_for(ctx: &Context, shortcut: &egui::KeyboardShortcut) -> bool {
    canvas_keys_free(ctx) && !crate::behavior::KeyClaim::claimed(ctx, shortcut)
}

/// Whether single keys (tool letters, brackets, zoom chords, undo) are free for the layer
/// `layer`, the layer of a full-window workspace or of a rail inside one: `layer` is the top
/// modal layer (or no modal layer or app-modal window is open), no text field is editing, and no
/// menu, menu bar, popup list or popover has the keyboard. A focused control that is not text
/// leaves the keys free; a key it claimed is still its own ([`KeyClaim`](crate::behavior::KeyClaim)).
///
/// [`owner`] answers [`KeyOwner::Dialog`] for any modal layer, so a layer that is itself the
/// modal workspace asks this instead.
pub fn keys_free(ctx: &Context, layer: LayerId) -> bool {
    if crate::menu::any_open(ctx)
        || crate::popup_list::owns_keys(ctx)
        || crate::popup_list::passive_open(ctx)
        || crate::menu::bar::is_focused(ctx)
        || text_has_keys(ctx)
    {
        return false;
    }
    match ctx.memory(|m| m.top_modal_layer()) {
        Some(top) => top == layer,
        None => !crate::viewport::ViewportWindow::any_app_modal(ctx),
    }
}

/// For each event of this pass, whether it is a key press the focused widget claimed. Computed
/// before `input_mut`, which holds the context's lock.
pub(crate) fn claimed_presses(ctx: &Context) -> Vec<bool> {
    let events = ctx.input(|i| i.events.clone());
    events
        .iter()
        .map(|e| match e {
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                crate::behavior::KeyClaim::claimed(ctx, &egui::KeyboardShortcut::new(*modifiers, *key))
            }
            _ => false,
        })
        .collect()
}

/// Who has the keyboard this pass, in [`KeyOwner`]'s order: `gate`, then the open popups (popup
/// lists, popovers, egui popups), menus and the menu bar, modal layers, a focused text edit, a
/// non-modal popover, and last egui's focus (the registered canvas counts as none).
// pc: crates/ui-egui/src/input/shortcuts.rs:482 `preflight_keys`, :514-522 (open menu, menu bar,
// dropdown list); crates/ui-egui/src/input/shortcut_dispatch.rs:142 `Focus::of`.
pub fn owner(ctx: &Context, gate: &OwnerGate) -> KeyOwner {
    if gate.blocked {
        KeyOwner::Blocked
    } else if crate::menu::any_open(ctx) || crate::popup_list::owns_keys(ctx) || crate::menu::bar::is_focused(ctx) {
        KeyOwner::Menu
    } else if gate.recording {
        KeyOwner::Recorder
    } else if gate.dialog || ctx.memory(|m| m.top_modal_layer().is_some()) || crate::viewport::ViewportWindow::any_app_modal(ctx) {
        KeyOwner::Dialog
    } else if text_has_keys(ctx) {
        KeyOwner::Text
    } else if crate::popup_list::passive_open(ctx) {
        KeyOwner::Popover
    } else {
        let focused = ctx.memory(|m| m.focused());
        if focused.is_none() || focused == crate::shell::registered_canvas(ctx) { KeyOwner::App } else { KeyOwner::Widget }
    }
}

/// Whether a text field has the keys this pass: one has the focus, or egui took the focus off it
/// at the start of this pass because Esc or Tab was pressed in it. The field has read that key
/// (Esc cancels its edit, Tab leaves it), so for the rest of the pass the key is not the app's.
/// The next pass the field is no longer the owner.
fn text_has_keys(ctx: &Context) -> bool {
    let last = Id::unique("craft_ui::focus::key_owner::last_text");
    if ctx.text_edit_focused() {
        let id = ctx.memory(|m| m.focused());
        ctx.data_mut(|d| d.insert_temp(last, id));
        return true;
    }
    let Some(Some(id)) = ctx.data(|d| d.get_temp::<Option<Id>>(last)) else {
        return false;
    };
    ctx.memory(|m| m.had_focus_last_frame(id)) && ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab))
}
