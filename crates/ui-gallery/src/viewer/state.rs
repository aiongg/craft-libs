//! The viewer's state: the root values the stories are shown under, the component tree and the cells of the shown
//! stories, and the theme workspace.

use super::appearance::CellFilter;
use super::catalog::Catalog;
use super::commands::GalleryCommands;
use super::layout::{PanelShares, theme_layout};
use super::settings::GallerySettings;
use super::tree::{self, TreeKey};
use crate::theme::ThemeWorkspace;
use craft_behavior::dock::Layout;
use craft_behavior::{GroupId, Selection};
use craft_i18n::Direction;
use craft_theme::ControlSize;
use craft_ui::list::OpenSet;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::StoryData;
use std::collections::HashMap;
use std::sync::Arc;

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
    pub dock: Layout<GroupId>,
    /// Counts the resets of the stories; a cell's id scope names it, so a reset mounts the cell anew.
    pub generation: u32,
    /// The shown cells' values.
    pub cells: Cells,
}

/// What the panels area shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Workspace {
    /// The component tree, the shown stories and the component's facts.
    #[default]
    Stories,
    /// The theme workspace: the tokens, contrast problems and app sizes of the chosen theme.
    Theme,
}

/// The viewer: the options bar's values and the browser.
#[non_exhaustive]
pub struct Viewer {
    /// The root values the window and every story run under, with the chosen theme.
    pub env: RootEnv,
    /// Which cells of a story are shown.
    pub cell_filter: CellFilter,
    /// What the panels area shows.
    pub workspace: Workspace,
    /// Whether the More popover of the options bar is open.
    pub more_open: bool,
    /// What the panels browse.
    pub browser: Browser,
    /// What the theme panels share.
    pub themes: ThemeWorkspace,
    /// The layout of the theme panels.
    pub theme_dock: Layout<GroupId>,
}

impl Viewer {
    /// The viewer of `catalog`, in the state `settings` saved.
    pub fn new(catalog: Catalog, settings: &GallerySettings) -> Self {
        let browser = Browser {
            catalog,
            filter: String::new(),
            selection: Selection::default(),
            open: OpenSet::new(),
            dock: PanelShares::default().tree(),
            generation: 0,
            cells: Cells::new(),
        };
        let mut viewer = Viewer {
            env: RootEnv::default(),
            cell_filter: CellFilter::default(),
            workspace: Workspace::default(),
            more_open: false,
            browser,
            themes: ThemeWorkspace::new(),
            theme_dock: theme_layout(),
        };
        viewer.restore(settings);
        viewer
    }

    /// Takes the state `settings` saved, keeping the catalog, the cells and the theme workspace.
    pub fn restore(&mut self, settings: &GallerySettings) {
        settings.restore_env(&mut self.env);
        self.cell_filter = settings.cell_filter();
        let browser = &mut self.browser;
        browser.filter.clone_from(&settings.filter);
        browser.open = OpenSet::new();
        for component in &browser.catalog.components {
            let module = tree::module_of(component);
            if settings.open_modules.iter().any(|saved| saved == module) {
                browser.open.set(&TreeKey::Module(module), true);
            }
        }
        browser.selection = browser
            .catalog
            .component(&settings.selected)
            .map_or_else(Selection::default, |component| Selection::single(TreeKey::Component(component.type_name)));
        browser.dock = PanelShares::load(&settings.layout).tree();
    }

    /// Makes the root's app sizes the theme workspace's app layer, when they differ.
    pub fn sync_sizes(&mut self) {
        if *self.env.sizes != self.themes.sizes {
            self.env.sizes = Arc::new(self.themes.sizes.clone());
        }
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
            GalleryCommands::ResetLayout => {
                self.browser.dock = PanelShares::default().tree();
                self.theme_dock = theme_layout();
            }
            GalleryCommands::ResetStories => self.browser.reset_stories(),
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
