//! The panel-menu pair of every preset panel: Grid View, List View, New Group… (`PresetPanelMenu::{panel_menu, run_command}`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context};

use super::files::PresetFiles;
use crate::docking::panel_menu::PanelCommand;
use crate::menu::MenuEntry;
use crate::presets::{PresetLabels, PresetState, PresetView};

/// The menu command of New Group….
pub(crate) const CMD_NEW_GROUP: &str = "presets.newGroup";

/// What a preset panel's menu has beyond the two views.
pub struct PresetMenuOptions<'a, K> {
    pub(super) new_group: Option<bool>,
    pub(super) files: Option<&'a PresetFiles<'a, K>>,
    pub(super) selected: Option<&'a K>,
}

impl<K> Default for PresetMenuOptions<'_, K> {
    fn default() -> Self {
        Self { new_group: None, files: None, selected: None }
    }
}

impl<'a, K> PresetMenuOptions<'a, K> {
    /// Grid View and List View only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lists New Group… (for a panel with groups), disabled when `enabled` is false (the
    /// library's nesting is reached).
    pub fn new_group(mut self, enabled: bool) -> Self {
        self.new_group = Some(enabled);
        self
    }

    /// Lists the file entries of `files` (Load…, Replace…, Save…; Styles' Import and Export).
    pub fn files(mut self, files: &'a PresetFiles<'a, K>) -> Self {
        self.files = Some(files);
        self
    }

    /// The selected preset, for Save…'s export.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }
}

/// What [`PresetPanelMenu::run_command`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetMenuAction {
    /// Grid View or List View set `state.view`.
    View(PresetView),
    /// New Group… was chosen: the app makes the group and opens its rename
    /// (`PresetBrowser::start_rename`).
    NewGroup,
    /// A file entry started a pick; its answer comes from `PresetFiles::take_events`.
    PickStarted,
}

/// The panel-menu pair of a preset panel, in the shared panel-menu shape: Grid View and List
/// View as radio rows ([`PresetState::view_menu_items`]), a separator, New Group…, the file
/// entries, and then the app adds its own rows (Swatches' Reset, Styles' Import and Export when
/// they are not `PresetFiles`). The commands come back as `DockEvent::PanelCommand`.
///
/// ```ignore
/// let files = PresetFiles::new("gradients", "Gradients", &["grd"]);
/// let options = PresetMenuOptions::new().new_group(true).files(&files);
/// let entries = PresetPanelMenu::panel_menu(ctx, "gradients", &state, &labels, &options);
/// if let Some(action) = PresetPanelMenu::run_command(ctx, "gradients", &mut state, &command, &options) { .. }
/// for event in files.take_events(ctx, "gradients").events { /* Loaded { bytes, .. } */ }
/// ```
pub struct PresetPanelMenu;

impl PresetPanelMenu {
    /// The rows above, Grid View or List View checked by `state.view`.
    pub fn panel_menu<K>(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &PresetState<K>,
        labels: &PresetLabels,
        options: &PresetMenuOptions<'_, K>,
    ) -> Vec<MenuEntry<PanelCommand>> {
        // new: E11 (Grid View, List View in every preset panel); color: crates/color/src/ui/swatches.rs:525 (`panel_menu`)
        todo!()
    }

    /// Carries out a row of [`panel_menu`](Self::panel_menu): a view row sets `state.view`, New
    /// Group… reports [`PresetMenuAction::NewGroup`] (nothing when it is disabled), a file row
    /// starts its pick for the component `id_salt`. `None` for a command that is not the menu's.
    pub fn run_command<K>(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &mut PresetState<K>,
        command: &PanelCommand,
        options: &PresetMenuOptions<'_, K>,
    ) -> Option<PresetMenuAction> {
        // color: crates/color/src/ui/swatches.rs:549-640 (`run_command`)
        todo!()
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the panel-menu pair (task PR3).
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // The rows are Grid View and List View with the current view checked, then New Group… when asked.
    #[test]
    #[ignore = "skeleton"]
    fn panel_menu_lists_grid_and_list_view_and_marks_the_current_one() {
        todo!()
    }

    // `run_command` of List View sets `state.view`; New Group… reports `NewGroup`, nothing when disabled.
    #[test]
    #[ignore = "skeleton"]
    fn panel_menu_commands_set_the_view_and_start_a_group() {
        todo!()
    }
}
