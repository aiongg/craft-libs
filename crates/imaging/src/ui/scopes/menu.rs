//! [`ScopePanel::panel_menu`], [`ScopePanel::run_command`] and [`ScopePanel::scope_menu`].
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::menu::MenuEntry;
use egui::{AsIdSalt, Context};

use super::panel::{ScopePanel, ScopePanelState};

/// What a chosen menu row asks the app to do beyond the settings change it made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeMenuAction {
    /// The row changed [`ScopeSettings`](super::ScopeSettings): persist them.
    SettingsChanged,
    /// Uncached Refresh: recount at cache level 1.
    Refresh,
}

impl ScopePanel<'_> {
    /// The panel menu's rows, for the docking host's `panel_menu_entries`: Uncached Refresh; a
    /// separator; Compact View, Expanded View, All Channels View (radio marks); a separator; Show
    /// Statistics, Show Channels in Color (check marks; check rows keep the menu open). Run the
    /// chosen row with [`run_command`](Self::run_command).
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, state: &ScopePanelState) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/histogram_panel.rs:809 `panel_menu` (PhotoCraft closes on a check; the library keeps it open).
        todo!()
    }

    /// The scope menu (right-click on a plot, the Menu key, Shift+F10): Show Lab Color Readouts,
    /// Show Vectorscope; with the vectorscope on, Show Selected Region (only when
    /// `has_selection`), Show Red at 3 o'clock, Show Skin Tone Indicator. Check rows keep it open,
    /// so the rows that depend on Show Vectorscope appear in place. The panel opens it itself; a
    /// host shows the same rows elsewhere (a floating scope's header) with this call.
    pub fn scope_menu(ctx: &Context, id_salt: impl AsIdSalt, state: &ScopePanelState, has_selection: bool) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:362-378 `context_menu`, `context`.
        todo!()
    }

    /// Runs a chosen row of either menu on `state`: a view, a check or Refresh. `None` when the
    /// command is not the scope panel's.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, state: &mut ScopePanelState, command: &PanelCommand) -> Option<ScopeMenuAction> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        todo!()
    }
}
