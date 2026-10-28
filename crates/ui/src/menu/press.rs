//! [`PressMenu`]: a [`Menu`] from a button, opened on the press so one gesture can press the
//! button, drag onto a row and release to run it (PhotoCraft's `press_menu`).

use craft_theme::ControlSize;
use egui::{Id, Key, Modifiers, Popup, Response, SetOpenCommand, Ui};

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
/// first enabled row highlighted); with the menu open those keys are the menu's.
#[must_use = "You should call .show"]
pub struct PressMenu<'a, T> {
    entries: &'a [MenuEntry<T>],
    size: ControlSize,
}

impl<'a, T: Clone> PressMenu<'a, T> {
    pub fn new(entries: &'a [MenuEntry<T>]) -> Self {
        Self { entries, size: ControlSize::Md }
    }

    /// The rows' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Opens the menu on `button`'s gestures and draws it while it is open.
    pub fn show(self, ui: &Ui, button: &Response) -> MenuResponse<T> {
        let ctx = ui.ctx();
        let id = Popup::default_response_id(button);
        let open = Popup::is_id_open(ctx, id);
        let mut command = None;
        let mut keyboard = false;
        if button.secondary_clicked() {
            command = Some(SetOpenCommand::Bool(true));
        } else if button.enabled() && pressed_on(ui, button) {
            if !open {
                ctx.data_mut(|d| d.insert_temp(gesture(), id));
            }
            command = Some(SetOpenCommand::Bool(!open));
        } else if !open && button.enabled() && button.has_focus() && button.clicked() {
            // The key that clicked the button must not also run the menu's first row.
            keyboard = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter) | i.consume_key(Modifiers::NONE, Key::Space));
            if keyboard {
                command = Some(SetOpenCommand::Bool(true));
            }
        }
        let owner = ctx.data(|d| d.get_temp::<Id>(gesture())) == Some(id);
        let popup = Popup::menu(button).open_memory(command);
        let shown = Menu::new(self.entries).size(self.size).pressing(owner).opened_by_keyboard(keyboard).show(ui, popup);
        // The gesture ends with the press; its release was handled in the pass above.
        if owner && (released(ui) || !shown.open) {
            ctx.data_mut(|d| d.remove::<Id>(gesture()));
        }
        shown
    }
}
