//! [`ContextMenu`]: a [`Menu`] at the pointer on a secondary click, and under the focused widget on
//! the Menu key or Shift+F10.

use craft_theme::ControlSize;
use egui::{Key, Modifiers, Popup, PopupAnchor, Pos2, Response, SetOpenCommand, Ui};

use super::item::MenuEntry;
use super::popup::{Menu, MenuResponse};

/// A context menu of a widget.
///
/// ```
/// use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
/// # egui::__run_test_ui(|ui| {
/// let entries: Vec<MenuEntry<&str>> = vec![MenuItem::new("Copy", "copy").into(), MenuItem::new("Paste", "paste").into()];
/// let canvas = ui.button("Canvas");
/// if let Some(command) = ContextMenu::new(&entries).show(ui, &canvas).chosen {
///     // Run it.
/// }
/// # });
/// ```
///
/// A secondary click on the widget opens the menu at the pointer. While the widget has the keyboard
/// focus (so it must be focusable), the Menu key and Shift+F10 open it with its top left at the
/// widget's bottom left; the first enabled row is highlighted and the mnemonics are underlined.
/// The widget keeps the focus, and the menu takes its keys as [`Menu`] does. A primary click on
/// the widget closes it, as does any click outside.
#[must_use = "You should call .show"]
pub struct ContextMenu<'a, T> {
    entries: &'a [MenuEntry<T>],
    size: ControlSize,
}

impl<'a, T: Clone> ContextMenu<'a, T> {
    pub fn new(entries: &'a [MenuEntry<T>]) -> Self {
        Self { entries, size: ControlSize::Md }
    }

    /// The rows' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Opens the menu on `widget`'s gestures and draws it while it is open.
    pub fn show(self, ui: &Ui, widget: &Response) -> MenuResponse<T> {
        let ctx = ui.ctx();
        let popup = Popup::default_response_id(widget).with("context-menu");
        let at = popup.with("at");
        let open = Popup::is_id_open(ctx, popup);
        let keyboard = widget.has_focus() && !open && opens_from_keyboard(ui);
        let mut command = None;
        if widget.secondary_clicked() {
            let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(widget.rect.left_bottom());
            ctx.data_mut(|d| d.insert_temp(at, pos));
            command = Some(SetOpenCommand::Bool(true));
        } else if keyboard {
            ctx.data_mut(|d| d.insert_temp(at, widget.rect.left_bottom()));
            command = Some(SetOpenCommand::Bool(true));
        } else if widget.clicked_by(egui::PointerButton::Primary) {
            command = Some(SetOpenCommand::Bool(false));
        }
        let pos = ctx.data(|d| d.get_temp::<Pos2>(at)).unwrap_or(widget.rect.left_bottom());
        let anchored = Popup::menu(widget).id(popup).anchor(PopupAnchor::Position(pos)).open_memory(command);
        Menu::new(self.entries).size(self.size).opened_by_keyboard(keyboard).show(ui, anchored)
    }
}

/// Takes the Menu key or Shift+F10 if one was pressed this pass.
fn opens_from_keyboard(ui: &Ui) -> bool {
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10))
}
