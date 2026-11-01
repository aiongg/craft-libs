//! The panel-menu pair of every preset panel: Grid View, List View, New Group… (`PresetPanelMenu::{panel_menu, run_command}`).

use egui::{AsIdSalt, Context};

use super::files::PresetFiles;
use crate::docking::panel_menu::PanelCommand;
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem};
use crate::presets::{PresetLabels, PresetState, PresetView};

/// The menu command of New Group….
pub(crate) const CMD_NEW_GROUP: &str = "presets.newGroup";

/// The menu commands of the two views.
pub(crate) const CMD_GRID_VIEW: &str = "presets.gridView";
pub(crate) const CMD_LIST_VIEW: &str = "presets.listView";

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
/// let entries = PresetPanelMenu::panel_menu(ctx, "gradients", &state, &options);
/// if let Some(action) = PresetPanelMenu::run_command(ctx, "gradients", &mut state, &command, &options) { .. }
/// for event in files.take_events(ctx, "gradients").events { /* Loaded { bytes, .. } */ }
/// ```
pub struct PresetPanelMenu;

impl PresetPanelMenu {
    /// The rows above, Grid View or List View checked by `state.view`, worded by the context's
    /// [`PresetLabels`].
    pub fn panel_menu<K>(
        ctx: &Context,
        _id_salt: impl AsIdSalt,
        state: &PresetState<K>,
        options: &PresetMenuOptions<'_, K>,
    ) -> Vec<MenuEntry<PanelCommand>> {
        let labels = PresetLabels::of_ctx(ctx);
        let mut out: Vec<MenuEntry<PanelCommand>> = state
            .view_menu_items(&labels)
            .into_iter()
            .map(|item| {
                let command = match item.command() {
                    Some(PresetView::List) => CMD_LIST_VIEW,
                    _ => CMD_GRID_VIEW,
                };
                MenuItem::new(item.label(), command.to_owned()).kind(item.item_kind()).into()
            })
            .collect();
        let files = options.files.map(|files| files.entries(ctx, options.selected)).unwrap_or_default();
        if options.new_group.is_some() || !files.is_empty() {
            out.push(MenuEntry::Separator);
        }
        if let Some(enabled) = options.new_group {
            out.push(MenuItem::new(labels.new_group.as_str(), CMD_NEW_GROUP.to_owned()).enabled(enabled).into());
        }
        out.extend(files);
        out
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
        match command.as_str() {
            CMD_GRID_VIEW => {
                state.view = PresetView::Grid;
                Some(PresetMenuAction::View(PresetView::Grid))
            }
            CMD_LIST_VIEW => {
                state.view = PresetView::List;
                Some(PresetMenuAction::View(PresetView::List))
            }
            CMD_NEW_GROUP => options.new_group.filter(|enabled| *enabled).map(|_| PresetMenuAction::NewGroup),
            other => options.files.is_some_and(|files| files.start(ctx, id_salt, other, options.selected)).then_some(PresetMenuAction::PickStarted),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the panel-menu pair (task PR3).
    use super::*;

    fn labels() -> PresetLabels {
        <PresetLabels as crate::localized::Labels>::english()
    }

    fn rows(entries: &[MenuEntry<PanelCommand>]) -> Vec<(String, Option<bool>)> {
        entries.iter().filter_map(MenuEntry::item).map(|i| (i.label().to_owned(), i.item_kind().checked())).collect()
    }

    // The rows are Grid View and List View with the current view checked, then New Group… when asked.
    #[test]
    fn panel_menu_lists_grid_and_list_view_and_marks_the_current_one() {
        let ctx = Context::default();
        let mut state = PresetState::<u32>::new();
        state.view = PresetView::List;
        let plain = PresetPanelMenu::panel_menu(&ctx, "p", &state, &PresetMenuOptions::new());
        assert_eq!(rows(&plain), [("Grid View".to_owned(), Some(false)), ("List View".to_owned(), Some(true))]);
        let grouped = PresetPanelMenu::panel_menu(&ctx, "p", &state, &PresetMenuOptions::new().new_group(true));
        assert_eq!(grouped.len(), 4);
        assert_eq!(grouped[2], MenuEntry::Separator);
        assert_eq!(grouped[3].item().map(MenuItem::label), Some(labels().new_group.as_str()));
    }

    // `run_command` of List View sets `state.view`; New Group… reports `NewGroup`, nothing when disabled.
    #[test]
    fn panel_menu_commands_set_the_view_and_start_a_group() {
        let ctx = Context::default();
        let mut state = PresetState::<u32>::new();
        let on = PresetMenuOptions::new().new_group(true);
        let off = PresetMenuOptions::new().new_group(false);
        let run = |state: &mut PresetState<u32>, command: &str, options: &PresetMenuOptions<'_, u32>| {
            PresetPanelMenu::run_command(&ctx, "p", state, &command.to_owned(), options)
        };
        assert_eq!(run(&mut state, CMD_LIST_VIEW, &on), Some(PresetMenuAction::View(PresetView::List)));
        assert_eq!(state.view, PresetView::List);
        assert_eq!(run(&mut state, CMD_NEW_GROUP, &on), Some(PresetMenuAction::NewGroup));
        assert_eq!(run(&mut state, CMD_NEW_GROUP, &off), None);
        assert_eq!(run(&mut state, "other.command", &on), None);
    }
}
