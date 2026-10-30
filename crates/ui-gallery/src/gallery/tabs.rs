//! The centre tabs that draw tiles: the Overview, a component's tab, and the Theme workspace's
//! Preview: Components.
use craft_theme::{ControlSize, SystemAppearance};
use craft_ui::controls::Select;
use craft_ui::story::{Level, Story};
use craft_ui::theme::{Tokens, TokensOf};
use egui::ScrollArea;
use egui::Ui;

use super::canvas::canvas_frame;
use super::catalog::{default_story_of, nav};
use super::picks::Picks;
use super::properties::ArgStore;
use super::tiles::{Tile, TileEvents, Tiles, tiles_view};
use crate::appearance::Appearance;
use crate::fields;
use crate::inspector::Inspector;

/// What a tab draws from and reports its picks to.
pub(super) struct TabEnv<'a> {
    pub(super) stories: &'a [Story],
    /// The Components panel's filter text.
    pub(super) filter: &'a str,
    pub(super) appearance: &'a Appearance,
    pub(super) system: SystemAppearance,
    pub(super) args: &'a mut ArgStore,
    pub(super) inspector: &'a mut Inspector,
    pub(super) picks: &'a mut Picks,
    /// Inspect mode is on.
    pub(super) inspecting: bool,
}

impl TabEnv<'_> {
    /// One tile per component: the story the Overview shows for it.
    fn component_tiles(&self, components: &[&'static str]) -> Vec<Tile> {
        components
            .iter()
            .filter_map(|c| {
                let story = Picks::story_in(self.stories, &self.picks.overview_story, c)?;
                let default = default_story_of(self.stories, c)?;
                Some(Tile { key: c, story, title: c, note: (story.id != default.id).then_some(story.name), open: true })
            })
            .collect()
    }
}

/// The Overview tab: every component of one level, each tile on its story's surface. The level
/// follows the selected component; each tile draws the story it was given.
pub(super) fn overview(ui: &mut Ui, env: &mut TabEnv<'_>) -> TileEvents {
    let nav = nav(env.stories, &env.filter.to_lowercase());
    let selected = env.picks.overview_selected;
    let shown = env.picks.overview_level.and_then(|l| nav.iter().position(|(level, _)| *level == l)).unwrap_or(0);
    let mut level = shown;
    let mut events = TileEvents::default();
    let tiles = nav.get(shown).map(|(_, components)| env.component_tiles(components)).unwrap_or_default();
    let (inspecting, appearance, system) = (env.inspecting, env.appearance, env.system);
    canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        let Some((shown_level, _)) = nav.get(shown) else {
            ui.label("No components match the filter.");
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Level:");
            let mut select = Select::new("gallery.overview.level", "Level", &mut level).fit(true).size(ControlSize::Sm);
            for (i, (level, _)) in nav.iter().enumerate() {
                select = select.option(i, level.title());
            }
            let _ = select.policy(fields::policy()).show(ui);
        });
        let inspector = selected.filter(|_| inspecting).map(|c| (c, &mut *env.inspector));
        let config = Tiles {
            one_column: false,
            selectable: true,
            selected,
            buttons: true,
            scale_of_selected: false,
            scroll_to: &mut env.picks.scroll_to,
            inspector,
        };
        ScrollArea::vertical().id_salt(shown_level.slug()).auto_shrink([false, false]).show(ui, |ui| {
            let (args, saved) = env.args.parts();
            events = tiles_view(ui, &tiles, config, args, saved, appearance, system);
        });
    });
    if level != shown {
        env.picks.overview_level = nav.get(level).map(|(l, _)| *l);
    }
    if let Some(component) = events.picked {
        env.picks.pick_overview(component);
    }
    events
}

/// A component's tab: every story of the component as a tile in one column.
pub(super) fn component_tab(ui: &mut Ui, env: &mut TabEnv<'_>, component: &'static str) -> TileEvents {
    let tiles: Vec<Tile> = env
        .stories
        .iter()
        .filter(|s| s.component == component)
        .map(|s| Tile { key: s.id, story: *s, title: s.name, note: None, open: false })
        .collect();
    let selected = Picks::story_in(env.stories, &env.picks.tab_story, component).map(|s| s.id);
    let mut events = TileEvents::default();
    let (inspecting, appearance, system) = (env.inspecting, env.appearance, env.system);
    canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        let inspector = selected.filter(|_| inspecting).map(|id| (id, &mut *env.inspector));
        let config = Tiles {
            one_column: true,
            selectable: true,
            selected,
            buttons: true,
            scale_of_selected: true,
            scroll_to: &mut env.picks.scroll_to,
            inspector,
        };
        ScrollArea::vertical().id_salt(("gallery.tab", component)).auto_shrink([false, false]).show(ui, |ui| {
            let (args, saved) = env.args.parts();
            events = tiles_view(ui, &tiles, config, args, saved, appearance, system);
        });
    });
    if let Some(id) = events.picked {
        env.picks.pick_tab(component, id);
    }
    events
}

/// The Theme workspace's Preview: Components: the Controls level's tiles.
pub(super) fn preview_components(ui: &mut Ui, env: &mut TabEnv<'_>) -> TileEvents {
    let components: Vec<&'static str> =
        nav(env.stories, "").into_iter().find(|(level, _)| *level == Level::Controls).map_or_else(Vec::new, |(_, c)| c);
    let tiles: Vec<Tile> = components
        .iter()
        .filter_map(|c| default_story_of(env.stories, c).map(|story| Tile { key: c, story: *story, title: c, note: None, open: true }))
        .collect();
    let mut events = TileEvents::default();
    let (appearance, system) = (env.appearance, env.system);
    canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        let mut scroll_to = None;
        let config = Tiles {
            one_column: false,
            selectable: false,
            selected: None,
            buttons: true,
            scale_of_selected: false,
            scroll_to: &mut scroll_to,
            inspector: None,
        };
        ScrollArea::vertical().id_salt("gallery.preview").auto_shrink([false, false]).show(ui, |ui| {
            let (args, saved) = env.args.parts();
            events = tiles_view(ui, &tiles, config, args, saved, appearance, system);
        });
    });
    events
}
