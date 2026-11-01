//! The Navigator's panel menu: [`NavigatorPanel::panel_menu`] and
//! [`NavigatorPanel::run_command`].
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context};

use super::options::NavigatorSettings;
use crate::docking::panel_menu::PanelCommand;
use crate::menu::MenuEntry;

/// The Navigator's panel-menu hooks, for the docking host's
/// [`panel_menu_entries`](crate::docking::DockContent::panel_menu_entries): one panel-menu shape
/// for every panel.
pub struct NavigatorPanel;

/// What a chosen Navigator menu row asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavigatorMenuAction {
    /// Panel Options…: the app opens
    /// [`NavigatorOptionsDialog::new().initial(settings.options)`](super::NavigatorOptionsDialog).
    OpenOptions,
}

impl NavigatorPanel {
    /// The panel menu's rows: Panel Options… (Photoshop's only Navigator row). The dock adds its
    /// own rows after the host's. `id_salt` is the panel's, as the other panels' menus take it.
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, settings: &NavigatorSettings) -> Vec<MenuEntry<PanelCommand>> {
        // new: Photoshop's Navigator panel menu; PhotoCraft has none (facts, Navigator "Panel menu").
        todo!()
    }

    /// Runs the command of a chosen [`panel_menu`](Self::panel_menu) row: Panel Options… returns
    /// [`NavigatorMenuAction::OpenOptions`]. `None` when the command is not the Navigator's.
    pub fn run_command(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        settings: &mut NavigatorSettings,
        command: &PanelCommand,
    ) -> Option<NavigatorMenuAction> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        todo!()
    }
}
