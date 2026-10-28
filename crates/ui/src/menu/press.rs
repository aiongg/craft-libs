//! [`PressMenu`]: a [`Menu`] from a button, opened on the press so one gesture can press the
//! button, drag onto a row and release to run it (PhotoCraft's `press_menu`).

use std::borrow::Cow;

use craft_theme::ControlSize;
use egui::accesskit::HasPopup;
use egui::{Id, Key, Modifiers, PointerButton, Popup, Response, SetOpenCommand, Ui};

use super::item::MenuEntry;
use super::popup::{Menu, MenuResponse};
use crate::controls::{pressed_on, released};

/// Where the press-drag gesture in progress is kept: the popup whose button's press started it.
/// Several press menus can show in one pass; each reads only its own gesture.
fn gesture() -> Id {
    Id::unique("craft-ui-press-menu-gesture")
}

/// A menu that opens from a button.
///
/// ```
/// use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
/// # egui::__run_test_ui(|ui| {
/// let entries: Vec<MenuEntry<&str>> = vec![MenuItem::new("New layer", "new").into(), MenuItem::new("Delete", "delete").into()];
/// let button = ui.button("Layers");
/// if let Some(command) = PressMenu::new(&entries).show(ui, &button).chosen {
///     // Run it.
/// }
/// # });
/// ```
///
/// A primary press on the button opens the menu (a press on the open button closes it), and the
/// menu follows the pointer while the press is held: releasing on an enabled row runs it,
/// releasing on the button or on nothing leaves the menu open, as does a plain click. A secondary
/// click opens it too, and Enter or Space on the focused button opens it from the keyboard (the
/// first enabled row highlighted); with the menu open those keys are the menu's. Esc closes it and
/// gives the button the focus back. The button is a menu button for AccessKit (`has_popup` menu,
/// `expanded` while the menu is open); an AccessKit click opens the menu as from the keyboard, or
/// closes it when open.
#[must_use = "You should call .show"]
pub struct PressMenu<'a, T> {
    entries: &'a [MenuEntry<T>],
    size: ControlSize,
    scroll_labels: Option<(Cow<'a, str>, Cow<'a, str>)>,
    focus_owner: Option<(Id, bool)>,
}

impl<'a, T: Clone> PressMenu<'a, T> {
    pub fn new(entries: &'a [MenuEntry<T>]) -> Self {
        Self { entries, size: ControlSize::Md, scroll_labels: None, focus_owner: None }
    }

    /// The rows' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The accessible names of a tall menu's scroll strips, translated; "Scroll menu up" and
    /// "Scroll menu down" by default.
    pub fn scroll_labels(mut self, up: impl Into<Cow<'a, str>>, down: impl Into<Cow<'a, str>>) -> Self {
        self.scroll_labels = Some((up.into(), down.into()));
        self
    }

    /// For a button that is a focus position of a widget holding egui's keyboard focus itself (a
    /// tab strip's overflow button): `open` opens the menu this pass as from the keyboard, and
    /// Escape gives the focus back to `owner`, the widget's id.
    pub fn keyboard_owner(mut self, owner: Id, open: bool) -> Self {
        self.focus_owner = Some((owner, open));
        self
    }

    /// Opens the menu on `button`'s gestures and draws it while it is open.
    pub fn show(self, ui: &Ui, button: &Response) -> MenuResponse<T> {
        let ctx = ui.ctx();
        let id = Popup::default_response_id(button);
        let open = Popup::is_id_open(ctx, id);
        let holder = self.focus_owner.map_or(button.id, |(owner, _)| owner);
        // egui drops the focus on Escape before the menu sees the key; a button that held it while
        // the menu was open gets it back when Escape closes the menu.
        let held = id.with("held");
        let escaped = open && ctx.input(|i| i.key_pressed(Key::Escape));
        let mut command = None;
        let mut keyboard = false;
        if button.secondary_clicked() {
            command = Some(SetOpenCommand::Bool(true));
        } else if button.enabled() && !open && self.focus_owner.is_some_and(|(_, open)| open) {
            keyboard = true;
            command = Some(SetOpenCommand::Bool(true));
        } else if button.enabled() && pressed_on(ui, button) {
            if !open {
                ctx.data_mut(|d| d.insert_temp(gesture(), id));
            }
            command = Some(SetOpenCommand::Bool(!open));
        } else if button.enabled() && button.clicked() && !button.clicked_by(PointerButton::Primary) {
            // Enter or Space on the focused button, or an AccessKit click. With the menu open the
            // keys are the menu's.
            let key = ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
            if !open {
                // The key that clicked the button must not also run the menu's first row.
                ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter) | i.consume_key(Modifiers::NONE, Key::Space));
                keyboard = true;
                command = Some(SetOpenCommand::Bool(true));
            } else if !key {
                command = Some(SetOpenCommand::Bool(false));
            }
        }
        let owner = ctx.data(|d| d.get_temp::<Id>(gesture())) == Some(id);
        let popup = Popup::menu(button).open_memory(command);
        let mut menu = Menu::new(self.entries).size(self.size).pressing(owner).opened_by_keyboard(keyboard);
        if let Some((up, down)) = self.scroll_labels {
            menu = menu.scroll_labels(up, down);
        }
        let shown = menu.show(ui, popup);
        if escaped && !shown.open && ctx.data(|d| d.get_temp::<bool>(held)).unwrap_or(false) {
            ctx.memory_mut(|m| m.request_focus(holder));
        } else if shown.open {
            let focused = ctx.memory(|m| m.has_focus(holder));
            ctx.data_mut(|d| d.insert_temp(held, focused));
        }
        ctx.accesskit_node_builder(button.id, |n| {
            n.set_has_popup(HasPopup::Menu);
            n.set_expanded(shown.open);
        });
        // The gesture ends with the press; its release was handled in the pass above.
        if owner && (released(ui) || !shown.open) {
            ctx.data_mut(|d| d.remove::<Id>(gesture()));
        }
        shown
    }
}
