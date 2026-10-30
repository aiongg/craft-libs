use std::collections::HashMap;

use craft_ui::docking::{Action, Workspace};
use craft_ui::story::{Level, Story};
use egui::Rect;

use super::Gallery;
use super::catalog::default_story_of;
use crate::layout::{GalleryWorkspace, PanelId};
use crate::persist::{self, KEY, Saved};

/// An in-memory eframe storage.
#[derive(Default)]
struct Memory(HashMap<String, String>);

impl eframe::Storage for Memory {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }

    fn flush(&mut self) {}
}

/// Frames of the gallery in a 1280 x 800 window.
fn run_frames(ctx: &egui::Context, gallery: &mut Gallery, frames: usize) {
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
    for _ in 0..frames {
        ctx.run_ui(input.clone(), |ui| gallery.show(ui)).drop_without_applying_deltas();
    }
}

/// A component with several stories, and its second story.
fn component_with_stories(gallery: &Gallery) -> (&'static str, Story) {
    let multi = gallery.stories.iter().find(|s| gallery.stories.iter().filter(|o| o.component == s.component).count() > 1).unwrap().component;
    (multi, *gallery.stories.iter().filter(|s| s.component == multi).nth(1).unwrap())
}

#[test]
fn a_fresh_gallery_shows_the_overview_with_nothing_selected() {
    let gallery = Gallery::new(None);
    assert_eq!(gallery.front_tab(), PanelId::View);
    assert_eq!(gallery.overview_selected(), None);
    assert_eq!(gallery.selected(), None);
    assert!(gallery.open_tabs().is_empty());
}

#[test]
fn picking_a_component_scrolls_the_overview_to_it_and_opens_no_tab() {
    let ctx = egui::Context::default();
    let mut gallery = Gallery::new(None);
    // The last component of another level, so the pick changes the level shown.
    let story = gallery.stories.iter().rev().find(|s| s.level != Level::Specimens).copied().unwrap();
    gallery.select_component(story.component);
    assert_eq!(gallery.picks.scroll_to, Some(story.component));
    run_frames(&ctx, &mut gallery, 2);
    assert_eq!(gallery.picks.scroll_to, None, "the tile was drawn and scrolled to");
    assert_eq!(gallery.front_tab(), PanelId::View);
    assert!(gallery.open_tabs().is_empty());
    assert_eq!(gallery.selected(), default_story_of(&gallery.stories, story.component).map(|s| s.id));
}

#[test]
fn opening_a_component_twice_makes_one_tab() {
    let ctx = egui::Context::default();
    let mut gallery = Gallery::new(None);
    let (component, second) = component_with_stories(&gallery);
    gallery.open_component(component);
    gallery.select(second.id);
    gallery.open_component(component);
    run_frames(&ctx, &mut gallery, 2);
    assert_eq!(gallery.open_tabs(), [component]);
    assert_eq!(gallery.front_tab(), PanelId::Component(component));
    assert_eq!(gallery.selected(), Some(second.id));
}

#[test]
fn the_side_panels_follow_the_centre_tab_last_brought_to_the_front_even_when_one_is_floated() {
    let mut gallery = Gallery::new(None);
    let (component, _) = component_with_stories(&gallery);
    gallery.open_component(component);
    let tab = PanelId::Component(component);
    gallery.components_dock.apply(Action::Float { panel: tab, rect: [40.0, 40.0, 400.0, 300.0] }, PanelId::permissions).unwrap();
    gallery.activate(PanelId::View);
    assert_eq!(gallery.front_tab(), PanelId::View);
    gallery.activate(tab);
    assert_eq!(gallery.front_tab(), tab);
}

#[test]
fn selecting_a_component_sets_the_overviews_level_and_the_level_survives_a_save() {
    let mut gallery = Gallery::new(None);
    let story = gallery.stories.iter().rev().find(|s| s.level != Level::Foundations).copied().unwrap();
    gallery.select_component(story.component);
    assert_eq!(gallery.picks.overview_level, Some(story.level));
    let mut storage = Memory::default();
    eframe::App::save(&mut gallery, &mut storage);
    assert_eq!(Gallery::new(Some(&storage)).picks.overview_level, Some(story.level));
}

#[test]
fn open_tabs_and_the_selections_survive_a_save_and_load() {
    let ctx = egui::Context::default();
    let mut gallery = Gallery::new(None);
    let (component, second) = component_with_stories(&gallery);
    let other = gallery.stories.iter().find(|s| s.component != component).unwrap().component;
    gallery.open_component(component);
    run_frames(&ctx, &mut gallery, 2);
    gallery.select(second.id);
    gallery.open_component(other);
    run_frames(&ctx, &mut gallery, 2);
    let mut storage = Memory::default();
    eframe::App::save(&mut gallery, &mut storage);
    let restored = Gallery::new(Some(&storage));
    assert_eq!(restored.open_tabs(), gallery.open_tabs());
    assert_eq!(restored.front_tab(), PanelId::Component(other));
    assert_eq!(restored.overview_selected(), Some(component));
    assert_eq!(restored.picks.overview_story, gallery.picks.overview_story);
    assert_eq!(restored.picks.tab_story, gallery.picks.tab_story);
    assert_eq!(restored.picks.tab_story.get(component), Some(&second.id));
}

#[test]
fn a_component_the_registry_lacks_is_dropped_from_the_saved_layout_and_selections() {
    use craft_ui::docking::{Layout, Node};
    let centre = Node::Tabs { panels: vec![PanelId::View, PanelId::Component("No such component")], active: 1 };
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, persist::LAYOUT_KEY, &Workspace::new(Layout { root: Some(centre), floating: Vec::new() }));
    let saved = Saved {
        overview_selected: Some("No such component".to_owned()),
        tab_story: HashMap::from([("No such component".to_owned(), "nope/nope/nope".to_owned())]),
        ..Saved::default()
    };
    eframe::set_value(&mut storage, KEY, &saved);
    let gallery = Gallery::new(Some(&storage));
    assert!(gallery.open_tabs().is_empty());
    assert!(gallery.components_dock.hidden.iter().all(|(panel, _)| panel.is_registered()));
    assert_eq!(gallery.components_dock.validate(), Ok(()));
    assert_eq!(gallery.overview_selected(), None);
    assert!(gallery.picks.tab_story.is_empty());
}

#[test]
fn state_saved_with_a_view_and_a_selected_story_loads() {
    let ctx = egui::Context::default();
    let gallery = Gallery::new(None);
    let (component, second) = component_with_stories(&gallery);
    let mut storage = Memory::default();
    eframe::Storage::set_string(&mut storage, KEY, format!("(selected:Some(\"{}\"),view:Component,filter:\"x\")", second.id));
    let mut gallery = Gallery::new(Some(&storage));
    run_frames(&ctx, &mut gallery, 2);
    assert_eq!(gallery.components.filter, "x");
    assert_eq!(gallery.overview_selected(), Some(component));
    assert_eq!(gallery.selected(), Some(second.id));
}

#[test]
fn the_dock_layout_round_trips_through_storage() {
    let mut gallery = Gallery::new(None);
    gallery.activate(PanelId::Tokens);
    gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
    let mut storage = Memory::default();
    eframe::App::save(&mut gallery, &mut storage);
    let restored = Gallery::new(Some(&storage));
    assert_eq!(restored.components_dock, gallery.components_dock);
    assert_ne!(restored.components_dock, crate::layout::default_workspace());
}

#[test]
fn an_invalid_saved_layout_falls_back_to_the_default() {
    use craft_ui::docking::{Layout, Node};
    let broken = Workspace::new(Layout { root: Some(Node::Tabs { panels: vec![PanelId::View, PanelId::View], active: 0 }), floating: Vec::new() });
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, persist::LAYOUT_KEY, &broken);
    assert_eq!(Gallery::new(Some(&storage)).components_dock, crate::layout::default_workspace());
}

#[test]
fn a_layout_saved_before_the_views_frame_loads_as_the_default_and_keeps_the_rest() {
    use craft_ui::docking::{Layout, Node};
    // The earlier layout held the token browser as a tab beside the canvas in the centre.
    let centre = Node::Tabs { panels: vec![PanelId::View, PanelId::Tokens], active: 1 };
    let old = Workspace::new(Layout { root: Some(centre), floating: Vec::new() });
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, KEY, &Saved { filter: "button".to_owned(), ..Saved::default() });
    eframe::set_value(&mut storage, "craft-ui-gallery.layout", &old);
    let gallery = Gallery::new(Some(&storage));
    assert_eq!(gallery.components_dock, crate::layout::default_workspace());
    assert_eq!(gallery.components.filter, "button");
}

#[test]
fn a_fresh_gallery_opens_in_the_components_workspace() {
    assert_eq!(Gallery::new(None).workspace(), GalleryWorkspace::Components);
}

#[test]
fn switching_workspaces_keeps_each_layout_as_it_was() {
    let ctx = egui::Context::default();
    let mut gallery = Gallery::new(None);
    gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
    let moved = gallery.components_dock.clone();
    gallery.set_workspace(GalleryWorkspace::Theme);
    run_frames(&ctx, &mut gallery, 2);
    assert_eq!(gallery.theme_dock, crate::layout::default_theme_workspace());
    gallery.set_workspace(GalleryWorkspace::Components);
    run_frames(&ctx, &mut gallery, 2);
    assert_eq!(gallery.components_dock, moved);
}

#[test]
fn each_layout_saves_under_its_own_key_and_a_broken_one_costs_only_itself() {
    let mut gallery = Gallery::new(None);
    gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
    gallery.theme_dock.apply(Action::Float { panel: PanelId::Changes, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
    let mut storage = Memory::default();
    eframe::App::save(&mut gallery, &mut storage);
    let restored = Gallery::new(Some(&storage));
    assert_eq!(restored.components_dock, gallery.components_dock);
    assert_eq!(restored.theme_dock, gallery.theme_dock);
    assert_ne!(restored.theme_dock, crate::layout::default_theme_workspace());

    eframe::Storage::set_string(&mut storage, persist::THEME_LAYOUT_KEY, "(version:9,layout:(root:Some(Frobnicate(".to_owned());
    let restored = Gallery::new(Some(&storage));
    assert_eq!(restored.theme_dock, crate::layout::default_theme_workspace());
    assert_eq!(restored.components_dock, gallery.components_dock);
}

#[test]
fn an_unreadable_saved_layout_costs_only_the_layout() {
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, KEY, &Saved { filter: "button".to_owned(), ..Saved::default() });
    eframe::Storage::set_string(&mut storage, persist::LAYOUT_KEY, "(version:9,layout:(root:Some(Frobnicate(".to_owned());
    let gallery = Gallery::new(Some(&storage));
    assert_eq!(gallery.components_dock, crate::layout::default_workspace());
    assert_eq!(gallery.components.filter, "button", "the rest of the saved state is kept");
}

#[test]
fn state_saved_with_the_removed_compare_fields_loads() {
    let mut storage = Memory::default();
    eframe::Storage::set_string(
        &mut storage,
        KEY,
        "(filter:\"button\",compare:true,compare_themes:[\"craft.pro-light-gray\"],compare_high:true)".to_owned(),
    );
    let gallery = Gallery::new(Some(&storage));
    assert_eq!(gallery.components.filter, "button");
    assert_eq!(gallery.variants.axis(), None);
}

/// Stories whose content has a width of its own and so cannot shrink to a narrow tile: pictures
/// of a document or a window, rows of controls that do not wrap, event logs and floating
/// layers. The tile's clip cuts them at the tile's edges.
const FIXED_WIDTH: &[&str] = &[
    "specimens/icons/compare",
    "specimens/type/faces",
    "primitives/state-layer/states",
    "primitives/canvas/document",
    "primitives/canvas/scrollbars",
    "primitives/guides/guides",
    "primitives/smart-guides/move",
    "primitives/marquee/drag",
    "primitives/rulers/canvas",
    "primitives/transform-box/free-transform",
    "primitives/bezier-paths/edit",
    "primitives/warp-grid/warp",
    "primitives/pen-draft/draw",
    "primitives/gradient-line/edit",
    "primitives/gradient-line/with-editor",
    "foundations/edit/log",
    "controls/segmented/text",
    "controls/select/wheel-steps",
    "controls/combo-box/font-size",
    "controls/slider/event-log",
    "controls/swatch/palette",
    "controls/badge/tones",
    "controls/kbd/plain",
    "controls/kbd/keycaps",
    "controls/text-field/states",
    "controls/text-field/event-log",
    "controls/number-field/event-log",
    "composites/form-grid/image-size",
    "composites/gradient/dialog",
    "composites/transform-fields/box",
    "composites/levels-editor/event-log",
    "composites/list/ten-thousand-rows",
    "composites/list/reorder",
    "composites/list/reorder-tree",
    "composites/list/reorder-refused",
    "composites/list/reorder-copy",
    "composites/menu/context-menu",
    "composites/color/full",
    "patterns/options-bar/brush-and-transform",
    "patterns/app-preview/photo-editor",
    "patterns/dock-area/floating",
    "patterns/dialog/frame",
    "patterns/dialog/modeless",
    "patterns/dialog/footer",
    "patterns/dialog/stack",
    "patterns/dialog/prompt",
    "patterns/dialog/preview",
    "patterns/full-window/workspace",
    "patterns/viewport/embedded",
];

/// Stories that draw in layers of their own (dialogs, windows) and are placed on the screen, not
/// in the tile; a tile cannot clip another layer.
const FLOATING: &[&str] = &[
    "patterns/dock-area/floating",
    "patterns/dialog/modeless",
    "patterns/dialog/footer",
    "patterns/dialog/stack",
    "patterns/dialog/prompt",
    "patterns/dialog/preview",
    "patterns/full-window/workspace",
    "patterns/viewport/embedded",
];

/// How far a focus ring, edge or shadow may reach past its widget; the tile's margin takes it.
const DECORATION_SLACK: f32 = 2.5;

/// Draws every story in an Overview tile at the narrowest tile width.
/// Returns the ids of the stories with a shape beyond the tile's left or right edge before any
/// clip, and the ids with a shape whose clip rect reaches past the tile.
fn drawn_past_their_tile(width: f32) -> (Vec<&'static str>, Vec<&'static str>) {
    use super::tiles::{Tile, Tiles, tiles_view};
    use craft_theme::SystemAppearance;
    use craft_ui::theme::install;

    let gallery = Gallery::new(None);
    let appearance = gallery.appearance.clone();
    let system = SystemAppearance::default();
    let (mut painted, mut clipped) = (Vec::new(), Vec::new());
    for story in gallery.stories().to_vec() {
        let ctx = egui::Context::default();
        let base = appearance.configure(&ctx, None, false, system);
        let tiles = [Tile { key: story.component, story, title: story.component, note: None, open: false }];
        let mut args = HashMap::new();
        // The screen is wider than the tile, so only the tile's own clip keeps a shape inside it.
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width + 400.0, 800.0))),
            ..egui::RawInput::default()
        };
        let mut shapes = Vec::new();
        for _ in 0..3 {
            let mut out = ctx.run_ui(input.clone(), |ui| {
                install(ui.ctx(), &base);
                let mut scroll_to = None;
                let config = Tiles {
                    one_column: false,
                    selectable: false,
                    selected: None,
                    buttons: false,
                    scale_of_selected: false,
                    scroll_to: &mut scroll_to,
                    inspector: None,
                    highlight: None,
                };
                ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 800.0))), |ui| {
                    tiles_view(ui, &tiles, config, &mut args, &HashMap::new(), &appearance, system);
                });
            });
            shapes = std::mem::take(&mut out.shapes);
            out.drop_without_applying_deltas();
        }
        let outside = |r: Rect| r.min.x < -DECORATION_SLACK || r.max.x > width + DECORATION_SLACK;
        let bounds = |c: &egui::epaint::ClippedShape| {
            let r = c.shape.visual_bounding_rect();
            (r.is_finite() && r.is_positive()).then_some(r)
        };
        if shapes.iter().filter_map(bounds).any(outside) {
            painted.push(story.id);
        }
        if shapes.iter().filter_map(|c| bounds(c).filter(|r| r.intersects(c.clip_rect)).map(|r| r.intersect(c.clip_rect))).any(outside) {
            clipped.push(story.id);
        }
    }
    (painted, clipped)
}

#[test]
fn no_story_paints_past_its_overview_tile() {
    let (painted, clipped) = drawn_past_their_tile(super::tiles::TILE_MIN_WIDTH);
    let clipped: Vec<_> = clipped.into_iter().filter(|id| !FLOATING.contains(id)).collect();
    assert!(clipped.is_empty(), "stories visible outside a {} pt tile: {clipped:?}", super::tiles::TILE_MIN_WIDTH);
    let painted: Vec<_> = painted.into_iter().filter(|id| !FIXED_WIDTH.contains(id)).collect();
    assert!(painted.is_empty(), "stories that paint outside a {} pt tile: {painted:?}", super::tiles::TILE_MIN_WIDTH);
}
