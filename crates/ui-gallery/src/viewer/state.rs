//! The viewer's state: the root values the stories are shown under, the component tree, the story each component
//! shows, the dock layout of each workspace and the theme workspace.

use super::appearance::VariantAxis;
use super::catalog::Catalog;
use super::commands::GalleryCommands;
use super::layout::{self, PanelId};
use super::settings::GallerySettings;
use super::tree::{self, TreeKey};
use crate::mock::MockState;
use crate::theme::ThemeWorkspace;
use craft_behavior::Selection;
use craft_behavior::dock::Workspace as Dock;
use craft_ui::contract::Level;
use craft_ui::list::OpenSet;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::{StoryData, StorySnapshot};
use std::collections::HashMap;
use std::sync::Arc;

/// The values each shown story keeps between frames, by story index.
pub type Cells = HashMap<usize, StoryData>;

/// What the panels browse: the catalog, the tree's state and the shown cells.
#[non_exhaustive]
pub struct Browser {
    /// What is listed.
    pub catalog: Catalog,
    /// The text that narrows the tree.
    pub filter: String,
    /// The selected row of the tree.
    pub selection: Selection<TreeKey>,
    /// The open level groups.
    pub open: OpenSet<TreeKey>,
    /// The components workspace's dock layout.
    pub dock: Dock<PanelId>,
    /// The story each component's card shows, by the component's registered name and the story's name. A component
    /// without an entry shows its first story.
    pub story_of: HashMap<&'static str, &'static str>,
    /// The level the Overview shows, as its slug; empty until one is chosen.
    pub level: String,
    /// Counts the resets of the stories; a card's id scope names it, so a reset mounts the card anew.
    pub generation: u32,
    /// The shown stories' values.
    pub cells: Cells,
    /// The values each shown story started with, by story index, which Reset in the Properties panel goes back to.
    pub starts: HashMap<usize, StorySnapshot>,
    /// The component whose tab a panel asked to open in this pass.
    pub open_request: Option<&'static str>,
}

/// What the dock shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Workspace {
    /// The component tree, the overview and component tabs, the stories and the target story's panels.
    #[default]
    Components,
    /// The theme workspace: the preview, and the tokens, contrast problems and app sizes of the chosen theme.
    Theme,
}

/// What the theme workspace's preview shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Preview {
    /// A mock application.
    #[default]
    App,
    /// The Controls level's overview.
    Components,
}

/// The viewer: the options bar's values and the browser.
#[non_exhaustive]
pub struct Viewer {
    /// The root values the window and every story run under, with the chosen theme.
    pub env: RootEnv,
    /// What the dock shows.
    pub workspace: Workspace,
    /// What the theme workspace's preview shows.
    pub preview: Preview,
    /// The Variants view of the target story, over the front centre tab, while one is open.
    pub variants: Option<VariantAxis>,
    /// Whether the More popover of the options bar is open.
    pub more_open: bool,
    /// What the panels browse.
    pub browser: Browser,
    /// What the theme panels share.
    pub themes: ThemeWorkspace,
    /// The theme workspace's dock layout.
    pub theme_dock: Dock<PanelId>,
    /// The mock application the theme workspace's App preview shows.
    pub mock: MockState,
    /// The revision of the theme workspace's app layer that `env.sizes` holds; none before the first sync.
    sizes_synced: Option<u64>,
}

impl Viewer {
    /// The viewer of `catalog`, in the state `settings` saved.
    pub fn new(catalog: Catalog, settings: &GallerySettings) -> Self {
        let browser = Browser {
            catalog,
            filter: String::new(),
            selection: Selection::default(),
            open: OpenSet::new(),
            dock: layout::default_components(),
            story_of: HashMap::new(),
            level: String::new(),
            generation: 0,
            cells: Cells::new(),
            starts: HashMap::new(),
            open_request: None,
        };
        let mut viewer = Viewer {
            env: RootEnv::default(),
            workspace: Workspace::default(),
            preview: Preview::default(),
            variants: None,
            more_open: false,
            browser,
            themes: ThemeWorkspace::new(),
            theme_dock: layout::default_theme(),
            mock: MockState::default(),
            sizes_synced: None,
        };
        viewer.restore(settings);
        viewer
    }

    /// Takes the state `settings` saved, keeping the catalog, the cells and the theme workspace.
    pub fn restore(&mut self, settings: &GallerySettings) {
        settings.restore_env(&mut self.env);
        self.workspace = settings.workspace();
        self.preview = settings.preview();
        let browser = &mut self.browser;
        browser.filter.clone_from(&settings.filter);
        browser.open = OpenSet::new();
        for level in settings.open_levels.iter().filter_map(|saved| tree::level_named(saved)) {
            browser.open.set(&TreeKey::Level(level), true);
        }
        browser.selection = browser
            .catalog
            .component(&settings.selected)
            .map_or_else(Selection::default, |component| Selection::single(TreeKey::Component(component.type_name)));
        browser.level.clone_from(&settings.level);
        browser.story_of = settings.story_of(&browser.catalog);
        let known = |name: &str| browser.catalog.component(name).is_some();
        browser.dock = layout::load(&settings.dock, Workspace::Components, &known);
        self.theme_dock = layout::load(&settings.theme_dock, Workspace::Theme, &known);
    }

    /// Makes the root's app sizes the theme workspace's app layer, when the layer was edited since the last sync.
    pub fn sync_sizes(&mut self) {
        let revision = self.themes.sizes_revision();
        if self.sizes_synced != Some(revision) {
            self.env.sizes = Arc::new(self.themes.sizes().clone());
            self.sizes_synced = Some(revision);
        }
    }

    /// The registered name of the selected component.
    pub fn selected_type(&self) -> Option<&'static str> {
        self.browser.selected_type()
    }

    /// The slugs of the levels of the tree that are open, in level order.
    pub fn open_levels(&self) -> Vec<String> {
        let browser = &self.browser;
        Level::ALL.into_iter().filter(|level| browser.open.is_open(&TreeKey::Level(*level))).map(tree::slug).collect()
    }

    /// The dock layout of the shown workspace.
    pub fn dock_mut(&mut self) -> &mut Dock<PanelId> {
        match self.workspace {
            Workspace::Components => &mut self.browser.dock,
            Workspace::Theme => &mut self.theme_dock,
        }
    }

    /// Opens the tab of the component registered as `type_name` beside the Overview and shows it.
    pub fn open_component(&mut self, type_name: &'static str) {
        let _ = self.browser.dock.set_visible(PanelId::Component(type_name), true, Some(PanelId::Overview), false, |panel| panel.permissions());
    }

    /// Puts the shown workspace's panels back at their default places. The open component tabs are documents and stay.
    pub fn reset_layout(&mut self) {
        let workspace = self.workspace;
        let tabs = layout::open_tabs(&self.browser.dock);
        *self.dock_mut() = layout::default_for(workspace);
        if workspace == Workspace::Components {
            for tab in tabs {
                self.open_component(tab);
            }
            let _ = self.browser.dock.set_visible(PanelId::Overview, true, None, false, |panel| panel.permissions());
        }
    }

    /// Runs `command`.
    pub fn run(&mut self, command: GalleryCommands) {
        match command {
            GalleryCommands::ResetLayout => self.reset_layout(),
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

    /// The index in the catalog of the story the selected component shows: the one picked for it, else its first.
    pub fn target_story(&self) -> Option<usize> {
        let stories = self.selected_stories();
        let picked = self
            .story_of
            .get(self.selected_type()?)
            .and_then(|name| stories.iter().copied().find(|&at| self.catalog.entries[at].registration.story.name == *name));
        picked.or_else(|| stories.first().copied())
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
        self.starts.clear();
    }
}
