//! `EditMenu`: the rows of a text control's context menu.

use super::MenuItem;
use crate::contract::Show as _;
use crate::display::Divider;
use crate::runtime::commands::EditCommands;
use egui::Ui;

/// The edit rows of a text control: Undo and Redo, Cut, Copy and Paste, then Select All.
///
/// Each row is a `MenuItem::command` over an `EditCommands` command, so its label, icon, chord and enabled state
/// come from the command: the focused text part publishes which commands apply (no selection, no Cut or Copy). Show
/// the rows in the body of the `Menu::context` of the text control.
#[derive(crate::contract::Pattern)]
pub struct EditMenu;

impl EditMenu {
    /// Shows the rows in the menu body `ui`.
    pub fn show(ui: &mut Ui) {
        MenuItem::command(EditCommands::Undo).show(ui);
        MenuItem::command(EditCommands::Redo).show(ui);
        Divider::horizontal().show(ui);
        MenuItem::command(EditCommands::Cut).show(ui);
        MenuItem::command(EditCommands::Copy).show(ui);
        MenuItem::command(EditCommands::Paste).show(ui);
        Divider::horizontal().show(ui);
        MenuItem::command(EditCommands::SelectAll).show(ui);
    }
}
