//! `ModifierKeysPanel`: on-screen toggles for the modifier keys, for a touch screen without a keyboard.

use crate::contract::{Bind, Pattern};
use craft_keymap::Mods;
use egui::{Response, Ui};

/// A panel of latching modifier-key toggles: a press on one holds that modifier for the next click or key.
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
pub struct ModifierKeysPanel<'a> {
    latched: Bind<'a, Mods>,
}

impl<'a> ModifierKeysPanel<'a> {
    /// A panel over the latched modifiers in `latched`.
    pub fn new(latched: Bind<'a, Mods>) -> Self {
        let _ = latched;
        todo!("tp.T16")
    }

    /// Shows the panel.
    pub fn show(self, ui: &mut Ui) -> Response {
        let _ = (ui, &self.latched);
        todo!("tp.T16")
    }
}
