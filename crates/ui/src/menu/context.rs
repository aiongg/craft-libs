//! [`ContextMenu`]: a [`Menu`] at the pointer on a secondary click, and under the focused widget on
//! the Menu key or Shift+F10.

use std::borrow::Cow;

use craft_theme::ControlSize;
use egui::{Context, Id, Key, LayerId, Modifiers, Order, Popup, PopupAnchor, Pos2, Rect, Response, SetOpenCommand, Ui};

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
/// the widget closes it, as does any click outside. A secondary click elsewhere on the widget
/// while the menu is open moves it there, starting over with nothing highlighted. Esc closes it
/// and gives the widget the focus back.
#[must_use = "You should call .show"]
pub struct ContextMenu<'a, T> {
    entries: &'a [MenuEntry<T>],
    size: ControlSize,
    request: Option<OpenAt>,
    own_gestures: bool,
    scroll_labels: Option<(Cow<'a, str>, Cow<'a, str>)>,
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
        Self { entries, size: ControlSize::Md, request: None, own_gestures: true, scroll_labels: None }
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

    /// The accessible names of a tall menu's scroll strips, translated; "Scroll menu up" and
    /// "Scroll menu down" by default.
    pub fn scroll_labels(mut self, up: impl Into<Cow<'a, str>>, down: impl Into<Cow<'a, str>>) -> Self {
        self.scroll_labels = Some((up.into(), down.into()));
        self
    }

    /// Opens the menu on `widget`'s gestures and draws it while it is open.
    pub fn show(self, ui: &Ui, widget: &Response) -> MenuResponse<T> {
        let ctx = ui.ctx();
        let popup = ContextMenu::popup_id(widget);
        let open = Popup::is_id_open(ctx, popup);
        // egui drops the focus on Escape before the menu sees the key; a widget that held it
        // while the menu was open gets it back when Escape closes the menu.
        let held = popup.with("held");
        let escaped = open && ctx.input(|i| i.key_pressed(Key::Escape));
        let mut keyboard = self.own_gestures && widget.has_focus() && !open && opens_from_keyboard(ui);
        let mut command = None;
        // Where the menu opens, or moves to while open, this pass.
        let mut place = None;
        if let Some(request) = self.request {
            place = Some(match request {
                OpenAt::Pointer(pos) => pos,
                OpenAt::Rect(rect) => below_reading_start(ui, rect),
            });
            keyboard = matches!(request, OpenAt::Rect(_));
        } else if !self.own_gestures {
            // Only the host's requests open it; a click on the widget still closes it.
            if widget.clicked_by(egui::PointerButton::Primary) {
                command = Some(SetOpenCommand::Bool(false));
            }
        } else if widget.secondary_clicked() {
            place = Some(ctx.input(|i| i.pointer.interact_pos()).unwrap_or(widget.rect.left_bottom()));
        } else if keyboard {
            place = Some(below_reading_start(ui, widget.rect));
        } else if widget.clicked_by(egui::PointerButton::Primary) {
            command = Some(SetOpenCommand::Bool(false));
        }
        let at = popup.with("at");
        if let Some(pos) = place {
            ctx.data_mut(|d| d.insert_temp(at, pos));
            command = Some(SetOpenCommand::Bool(true));
        }
        let pos = ctx.data(|d| d.get_temp::<Pos2>(at)).unwrap_or(widget.rect.left_bottom());
        // It runs from the point in the reading direction: leftwards in a right-to-left layout.
        let anchored = Popup::menu(widget).id(popup).anchor(PopupAnchor::Position(pos)).align(crate::direction::below(ui)).open_memory(command);
        let mut menu = Menu::new(self.entries).size(self.size).opened_by_keyboard(keyboard).restart(open && place.is_some());
        if let Some((up, down)) = self.scroll_labels {
            menu = menu.scroll_labels(up, down);
        }
        let shown = menu.show(ui, anchored);
        if escaped && !shown.open && ctx.data(|d| d.get_temp::<bool>(held)).unwrap_or(false) {
            widget.request_focus();
        } else if shown.open {
            let focused = widget.has_focus();
            ctx.data_mut(|d| d.insert_temp(held, focused));
        }
        shown
    }
}

/// What any context menu of a widget answers, whatever its entries.
impl ContextMenu<'_, ()> {
    /// The popup id of the context menu of `widget`.
    pub fn popup_id(widget: &Response) -> Id {
        Popup::default_response_id(widget).with("context-menu")
    }

    /// Whether the context menu of `widget` is open.
    pub fn is_open(ctx: &Context, widget: &Response) -> bool {
        Popup::is_id_open(ctx, Self::popup_id(widget))
    }

    /// Whether the pointer is over the open context menu of `widget`: a click there is no click
    /// away from the widget.
    pub fn pointer_on(ctx: &Context, widget: &Response) -> bool {
        Self::is_open(ctx, widget)
            && ctx.pointer_interact_pos().and_then(|pos| ctx.layer_id_at(pos)) == Some(LayerId::new(Order::Foreground, Self::popup_id(widget)))
    }
}

/// Where a menu opened from the keyboard for `rect` goes: below its reading start, the left
/// edge, or the right edge in a right-to-left layout.
fn below_reading_start(ui: &Ui, rect: Rect) -> Pos2 {
    if crate::direction::is_rtl(ui) { rect.right_bottom() } else { rect.left_bottom() }
}

/// Takes the Menu key or Shift+F10 if one was pressed this pass.
fn opens_from_keyboard(ui: &Ui) -> bool {
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10))
}
