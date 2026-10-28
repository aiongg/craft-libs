//! [`ContextMenu`]: a [`Menu`] at the pointer on a secondary click, and under the focused widget on
//! the Menu key or Shift+F10.

use craft_theme::ControlSize;
use egui::{Key, Modifiers, Popup, PopupAnchor, Pos2, Rect, Response, SetOpenCommand, Ui};

use super::item::MenuEntry;
use super::nav::Nav;
use super::popup::{Menu, MenuResponse};
use crate::popup_list;

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
    request: Option<OpenAt>,
    own_gestures: bool,
}

/// Where a host asks a [`ContextMenu`] to open this pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpenAt {
    /// At a point, as for a secondary click.
    Pointer(Pos2),
    /// With its top left at the rect's bottom left, as for the Menu key: the first enabled row is
    /// highlighted and the mnemonics are underlined.
    Rect(Rect),
}

impl<'a, T: Clone> ContextMenu<'a, T> {
    pub fn new(entries: &'a [MenuEntry<T>]) -> Self {
        Self { entries, size: ControlSize::Md, request: None, own_gestures: true }
    }

    /// The rows' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Opens the menu this pass where a widget that reports its own context-menu gestures (a
    /// [`List`](crate::list::List), a ruler) says. The menu belongs to the widget passed to
    /// [`show`](Self::show), which keeps the keyboard focus and gets it back on Escape.
    pub fn open_at(mut self, at: Option<OpenAt>) -> Self {
        self.request = at;
        self
    }

    /// Whether the secondary click and the Menu key on the widget open the menu by themselves;
    /// `true` by default. A widget that reports the gestures through [`open_at`](Self::open_at)
    /// turns them off.
    pub fn own_gestures(mut self, own: bool) -> Self {
        self.own_gestures = own;
        self
    }

    /// Opens the menu on `widget`'s gestures and draws it while it is open.
    pub fn show(self, ui: &Ui, widget: &Response) -> MenuResponse<T> {
        let ctx = ui.ctx();
        let base = Popup::default_response_id(widget).with("context-menu");
        let mut generation = ctx.data(|d| d.get_temp::<u32>(base)).unwrap_or(0);
        let mut popup = base.with(generation);
        let mut open = Popup::is_id_open(ctx, popup);
        // egui drops the focus on Escape before the menu sees the key; a widget that held it
        // while the menu was open gets it back when Escape closes the menu.
        let held = base.with("held");
        let escaped = open && ctx.input(|i| i.key_pressed(Key::Escape));
        // A secondary press on the widget while the menu is open moves it to the pointer. egui
        // closes a popup on a click whose press began while it was open, so the menu reopens
        // under a new id, which no press began in.
        let secondary_press =
            self.own_gestures && widget.contains_pointer() && ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));
        if open && (secondary_press || self.request.is_some()) {
            Popup::close_id(ctx, popup);
            popup_list::record_open(ctx, popup, false);
            Nav::forget(ctx, popup);
            generation = generation.wrapping_add(1);
            ctx.data_mut(|d| d.insert_temp(base, generation));
            popup = base.with(generation);
            open = false;
        }
        let at = popup.with("at");
        let mut keyboard = self.own_gestures && widget.has_focus() && !open && opens_from_keyboard(ui);
        let mut command = None;
        if let Some(request) = self.request {
            let pos = match request {
                OpenAt::Pointer(pos) => pos,
                OpenAt::Rect(rect) => rect.left_bottom(),
            };
            keyboard = matches!(request, OpenAt::Rect(_));
            ctx.data_mut(|d| d.insert_temp(at, pos));
            command = Some(SetOpenCommand::Bool(true));
        } else if !self.own_gestures {
            // Only the host's requests open it; a click on the widget still closes it.
            if widget.clicked_by(egui::PointerButton::Primary) {
                command = Some(SetOpenCommand::Bool(false));
            }
        } else if widget.secondary_clicked() {
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
        let shown = Menu::new(self.entries).size(self.size).opened_by_keyboard(keyboard).show(ui, anchored);
        if escaped && !shown.open && ctx.data(|d| d.get_temp::<bool>(held)).unwrap_or(false) {
            widget.request_focus();
        } else if shown.open {
            let focused = widget.has_focus();
            ctx.data_mut(|d| d.insert_temp(held, focused));
        }
        shown
    }
}

/// Takes the Menu key or Shift+F10 if one was pressed this pass.
fn opens_from_keyboard(ui: &Ui) -> bool {
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10))
}
