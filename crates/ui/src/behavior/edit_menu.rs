//! [`EditMenu`]: the secondary-press menu of a text control (Undo, Cut, Copy, Paste, Delete and
//! Select All), as PhotoCraft's text fields have it.
//!
//! [`TextField`](crate::controls::TextField), [`SearchField`](crate::controls::SearchField) and
//! [`NumberField`](crate::controls::NumberField) open it on a secondary click, and on the Menu key
//! or Shift+F10 while they have the focus ([`ContextMenu`] does the gestures). The field keeps its
//! focus while the menu is open and while a command runs. A command is the input the keyboard
//! shortcut would send, given to the field in the next pass before its `TextEdit` runs, so the
//! field's gesture sees ordinary typing: `Cut` and `Copy` events, a Delete key press, Cmd/Ctrl+A,
//! Cmd/Ctrl+Z, and for Paste the window's clipboard request, which arrives as a `Paste` event.

use std::sync::Arc;

use egui::{Context, Event, Id, Key, KeyboardShortcut, LayerId, Modifiers, Order, Popup, Pos2, Response, Ui, ViewportCommand};

use crate::controls::Kbd;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};

/// The menu's labels, translated by the app. English by default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditMenuLabels {
    pub undo: String,
    pub cut: String,
    pub copy: String,
    pub paste: String,
    pub delete: String,
    pub select_all: String,
}

impl Default for EditMenuLabels {
    fn default() -> Self {
        Self {
            undo: "Undo".into(),
            cut: "Cut".into(),
            copy: "Copy".into(),
            paste: "Paste".into(),
            delete: "Delete".into(),
            select_all: "Select All".into(),
        }
    }
}

/// The edit menu of text controls. It has no state of its own to show: the controls open it. The
/// app installs the labels.
pub struct EditMenu;

impl EditMenu {
    /// Sets the labels the edit menus of `ctx` use. Call it with [`theme::install`](crate::theme::install)
    /// when the language changes.
    pub fn install(ctx: &Context, labels: EditMenuLabels) {
        ctx.data_mut(|d| d.insert_temp(labels_id(), Arc::new(labels)));
    }

    /// The labels installed in `ctx`, English when none are.
    pub fn labels(ctx: &Context) -> Arc<EditMenuLabels> {
        ctx.data(|d| d.get_temp::<Arc<EditMenuLabels>>(labels_id())).unwrap_or_default()
    }
}

fn labels_id() -> Id {
    Id::unique("craft_ui::behavior::edit_menu::labels")
}

/// One command of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Undo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
}

impl Action {
    /// The input the keyboard shortcut of this command sends.
    fn events(self) -> Vec<Event> {
        let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        match self {
            Action::Undo => vec![key(Key::Z, Modifiers::COMMAND)],
            Action::Cut => vec![Event::Cut],
            Action::Copy => vec![Event::Copy],
            Action::Delete => vec![key(Key::Delete, Modifiers::NONE)],
            Action::SelectAll => vec![key(Key::A, Modifiers::COMMAND)],
            // The window answers with the clipboard text as a `Paste` event.
            Action::Paste => Vec::new(),
        }
    }
}

/// The popup id of the menu of the control `field` (`ContextMenu`'s own).
fn popup_id(field: Id) -> Id {
    field.with("popup").with("context-menu")
}

fn pending_id(field: Id) -> Id {
    field.with("craft_ui::behavior::edit_menu::pending")
}

/// Whether the menu of `field` is open.
pub(crate) fn is_open(ctx: &Context, field: Id) -> bool {
    Popup::is_id_open(ctx, popup_id(field))
}

/// Whether the pointer is over the open menu of `field`: a click there is no click away from the
/// field, which keeps its focus.
pub(crate) fn pointer_on(ctx: &Context, field: Id) -> bool {
    is_open(ctx, field) && ctx.pointer_interact_pos().and_then(|pos| ctx.layer_id_at(pos)) == Some(LayerId::new(Order::Foreground, popup_id(field)))
}

/// Esc while the menu of `field` is open closes only the menu: the press is taken before the field
/// or its gesture sees it, and the field keeps the focus.
pub(crate) fn close_on_escape(ui: &Ui, field: Id) {
    if is_open(ui.ctx(), field) && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        Popup::close_id(ui.ctx(), popup_id(field));
        ui.memory_mut(|m| m.request_focus(field));
    }
}

/// Opens the menu of `field` at `pos` in the next pass: for a control whose field was covered by a
/// widget of its own when the press came.
pub(crate) fn request_open(ui: &Ui, field: Id, pos: Pos2) {
    ui.data_mut(|d| d.insert_temp(open_id(field), pos));
    ui.ctx().request_repaint();
}

fn open_id(field: Id) -> Id {
    field.with("craft_ui::behavior::edit_menu::open")
}

/// Gives `field` the input of the command chosen in its menu last pass. Call it before the
/// field's `TextEdit` runs.
pub(crate) fn run_pending(ui: &Ui, field: Id) {
    let Some(action) = ui.data_mut(|d| {
        let action = d.get_temp::<Action>(pending_id(field));
        d.remove::<Action>(pending_id(field));
        action
    }) else {
        return;
    };
    ui.memory_mut(|m| m.request_focus(field));
    if action == Action::Paste {
        ui.send_viewport_cmd(ViewportCommand::RequestPaste);
    }
    ui.input_mut(|i| i.events.extend(action.events()));
}

/// The menu of the text control whose `TextEdit` response is `field`, with `selection` whether it
/// has selected text (Cut, Copy and Delete need one). Opens on the gestures and draws while open.
pub(crate) fn show(ui: &Ui, field: &Response, selection: bool) {
    let labels = EditMenu::labels(ui.ctx());
    let shortcut = |key| Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND, key));
    let entries = vec![
        MenuItem::new(&labels.undo, Action::Undo).shortcut(shortcut(Key::Z)).into(),
        MenuEntry::Separator,
        MenuItem::new(&labels.cut, Action::Cut).shortcut(shortcut(Key::X)).enabled(selection).into(),
        MenuItem::new(&labels.copy, Action::Copy).shortcut(shortcut(Key::C)).enabled(selection).into(),
        MenuItem::new(&labels.paste, Action::Paste).shortcut(shortcut(Key::V)).into(),
        MenuItem::new(&labels.delete, Action::Delete).enabled(selection).into(),
        MenuEntry::Separator,
        MenuItem::new(&labels.select_all, Action::SelectAll).shortcut(shortcut(Key::A)).into(),
    ];
    let at = ui.data_mut(|d| {
        let pos = d.get_temp::<Pos2>(open_id(field.id));
        d.remove::<Pos2>(open_id(field.id));
        pos
    });
    if let Some(action) = ContextMenu::new(&entries).open_at(at.map(OpenAt::Pointer)).show(ui, field).chosen {
        ui.data_mut(|d| d.insert_temp(pending_id(field.id), action));
        ui.ctx().request_repaint();
    }
}
