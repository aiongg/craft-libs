//! The viewer's state: the appearance, the component tree and the cells of the shown stories.

use super::appearance::Appearance;
use super::catalog::Catalog;
use super::commands::GalleryCommands;
use super::layout::PanelShares;
use super::settings::GallerySettings;
use super::tree::{self, TreeKey};
use craft_behavior::{DockTree, Selection};
use craft_i18n::Direction;
use craft_theme::ControlSize;
use craft_ui::list::OpenSet;
use craft_ui::testing::StoryData;
use std::collections::HashMap;

/// The values each shown cell keeps between frames, by story index, direction and size.
pub type Cells = HashMap<(usize, Direction, ControlSize), StoryData>;

/// What the panels browse: the catalog, the tree's state and the shown cells.
#[non_exhaustive]
pub struct Browser {
    /// What is listed.
    pub catalog: Catalog,
    /// The text that narrows the tree.
    pub filter: String,
    /// The selected row of the tree.
    pub selection: Selection<TreeKey>,
    /// The open module groups.
    pub open: OpenSet<TreeKey>,
    /// The panel layout.
    pub dock: DockTree,
    /// Counts the resets of the stories; a cell's id scope names it, so a reset mounts the cell anew.
    pub generation: u32,
    /// The shown cells' values.
    pub cells: Cells,
}

/// The viewer: the options bar's values and the browser.
#[non_exhaustive]
pub struct Viewer {
    /// What the stories are shown under.
    pub appearance: Appearance,
    /// Whether the More popover of the options bar is open.
    pub more_open: bool,
    /// What the panels browse.
    pub browser: Browser,
}

impl Viewer {
    /// The viewer of `catalog`, in the state `settings` saved.
    pub fn new(catalog: Catalog, settings: &GallerySettings) -> Self {
        let mut open = OpenSet::new();
        for component in &catalog.components {
            let module = tree::module_of(component);
            if settings.open_modules.iter().any(|saved| saved == module) {
                open.set(&TreeKey::Module(module), true);
            }
        }
        let selection = catalog
            .component(&settings.selected)
            .map_or_else(Selection::default, |component| Selection::single(TreeKey::Component(component.type_name)));
        let browser = Browser {
            catalog,
            filter: settings.filter.clone(),
            selection,
            open,
            dock: PanelShares::load(&settings.layout).tree(),
            generation: 0,
            cells: Cells::new(),
        };
        Viewer { appearance: settings.appearance(), more_open: false, browser }
    }

    /// The registered name of the selected component.
    pub fn selected_type(&self) -> Option<&'static str> {
        self.browser.selected_type()
    }

    /// The module groups of the tree that are open.
    pub fn open_modules(&self) -> Vec<String> {
        let browser = &self.browser;
        let mut modules: Vec<_> = browser
            .catalog
            .components
            .iter()
            .map(tree::module_of)
            .filter(|module| browser.open.is_open(&TreeKey::Module(module)))
            .map(str::to_owned)
            .collect();
        modules.sort();
        modules.dedup();
        modules
    }

    /// The shares of the panel layout; the default when the tree has lost its shape.
    pub fn layout(&self) -> PanelShares {
        PanelShares::of_tree(&self.browser.dock).unwrap_or_default()
    }

    /// Runs `command`.
    pub fn run(&mut self, command: GalleryCommands) {
        match command {
            GalleryCommands::ResetLayout => self.browser.dock = PanelShares::default().tree(),
            GalleryCommands::ResetStories => self.browser.reset_stories(),
            _ => {}
        }
    }
}

impl Browser {
    /// The registered name of the selected component.
    pub fn selected_type(&self) -> Option<&'static str> {
        self.selection.items.iter().rev().find_map(|key| match key {
            TreeKey::Component(type_name) => Some(*type_name),
            _ => None,
        })
    }

    /// The indices of the stories of the selected component, in listing order.
    pub fn selected_stories(&self) -> Vec<usize> {
        let Some(type_name) = self.selected_type() else { return Vec::new() };
        let entries = &self.catalog.entries;
        (0..entries.len()).filter(|&at| entries[at].type_name == type_name).collect()
    }

    /// Gives every shown cell fresh state.
    pub fn reset_stories(&mut self) {
        self.generation += 1;
        self.cells.clear();
    }
}
