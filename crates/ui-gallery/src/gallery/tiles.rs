//! Tiles: several stories on their surfaces in one or two columns, as the Overview, a component's
//! tab and the Theme workspace's Preview: Components show them.
use std::collections::HashMap;

use craft_theme::SystemAppearance;
use craft_theme::{ControlSize, TypeRole};
use craft_ui::controls::IconButton;
use craft_ui::direction;
use craft_ui::docking::DockLook;
use craft_ui::icons::Icon;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Args, Story};
use craft_ui::theme::{StrokeExt as _, ToEgui, Tokens, TokensOf, scale, scope};
use egui::epaint::RectShape;
use egui::layers::ShapeIdx;
use egui::{Align, Frame, Rect, Role, Sense, Shape, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::applied::shown_mode;
use super::canvas::StoryFrame;
use super::properties::args_of;
use super::story_state::StoryStates;
use crate::appearance::Appearance;
use crate::inspector::Inspector;
use crate::persist;
use crate::usage::{Highlight, TILE_HEIGHT};

/// One tile of an Overview or a component tab.
#[derive(Clone, Copy)]
pub(crate) struct Tile {
    /// What a pick and a scroll request name: the component in the Overview, the story's id in a
    /// component tab.
    pub key: &'static str,
    pub story: Story,
    /// The header's heading: the component, or the story.
    pub title: &'static str,
    /// The story's name after the heading, when the tile shows other than the default story.
    pub note: Option<&'static str>,
    /// Whether the header has an Open button (for the component).
    pub open: bool,
}

/// How a set of tiles looks and what it reports.
pub(crate) struct Tiles<'a> {
    /// One full-width column; otherwise one or two, by the width.
    pub one_column: bool,
    /// Whether a click on a tile selects it. Without it no tile has the ring.
    pub selectable: bool,
    /// The key of the tile with the ring.
    pub selected: Option<&'static str>,
    /// Whether these are Overview tiles, in which a story with a modal dialog starts it closed
    /// ([`craft_ui::story::in_overview`]).
    pub overview: bool,
    /// Whether the headers have the Reset and Open buttons.
    pub buttons: bool,
    /// The key of the tile to scroll to; cleared once its tile has been drawn.
    pub scroll_to: &'a mut Option<&'static str>,
    /// Whether the selected tile's request for a scale ([`craft_ui::story::request_pixels_per_point`])
    /// stands; every other tile's is dropped, as a tile does not change the gallery's scale.
    pub scale_of_selected: bool,
    /// The inspector, and the key of the one tile it draws over.
    pub inspector: Option<(&'static str, &'a mut Inspector)>,
    /// The token whose shapes the tiles outline in the accent colour.
    pub highlight: Option<Highlight<'a>>,
    /// What the stories keep besides their args; a reset and the touched flag live here.
    pub states: &'a mut StoryStates,
}

/// What a set of tiles reported in a frame.
#[derive(Default)]
pub(crate) struct TileEvents {
    /// The key of the tile that was clicked.
    pub picked: Option<&'static str>,
    /// The component whose Open was pressed.
    pub opened: Option<&'static str>,
    /// The id of the story whose Reset was pressed.
    pub reset: Option<&'static str>,
    /// The keys of the tiles that outlined shapes painted with the highlighted token.
    pub highlighted: Vec<&'static str>,
}

/// The narrowest a tile may be: PhotoCraft's dock panels are 290 pt wide, and a story drawn in
/// less than a panel's width is no longer what the apps show.
pub(super) const TILE_MIN_WIDTH: f32 = 290.0;

/// The tiles in one or two columns: two once each column keeps [`TILE_MIN_WIDTH`]. A row's tiles
/// share its height. A tile has a header (the heading, Reset while the story's args differ from
/// its defaults or its preview was used, Open) above its story; a click on the tile that none of its controls takes picks
/// it.
pub(crate) fn tiles_view(
    ui: &mut Ui,
    tiles: &[Tile],
    mut config: Tiles<'_>,
    args: &mut HashMap<String, Args>,
    saved: &HashMap<String, Vec<persist::SavedArg>>,
    appearance: &Appearance,
    system: SystemAppearance,
) -> TileEvents {
    let shown = shown_mode(ui.ctx());
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        appearance.overrides(o);
        o.mode = Some(appearance.mode(shown));
        o.contrast = Some(appearance.contrast(system));
    };
    // The ring and the Used by outline belong to the gallery, not to the appearance the stories are shown in.
    let ring = Tokens::of(ui).color.accent.solid.to_egui();
    let mut events = TileEvents::default();
    scope(ui, overrides, |ui| {
        let tokens = Tokens::of(ui);
        let gap = tokens.space.gap.section;
        let rtl = direction::is_rtl(ui);
        let width = ui.available_width();
        let columns = if config.one_column || width < 2.0 * TILE_MIN_WIDTH + gap { 1 } else { 2 };
        let tile_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let area = ui.available_rect_before_wrap();
        // A tile is as tall as its story; a story that fills the height it is given gets a viewport's.
        let reach = area.height();
        for row in tiles.chunks(columns) {
            let top = ui.cursor().min.y;
            let mut bottom = top;
            let mut placed = Vec::new();
            for (i, tile) in row.iter().enumerate() {
                let story = &tile.story;
                let x = if rtl { area.max.x - (i + 1) as f32 * tile_width - i as f32 * gap } else { area.min.x + i as f32 * (tile_width + gap) };
                let slot = ui.painter().add(Shape::Noop);
                let sense = if config.selectable { Sense::CLICK } else { Sense::hover() };
                let mut cell = ui.new_child(
                    UiBuilder::new()
                        .id_salt((tile.key, config.states.generation(story.id)))
                        .sense(sense)
                        .max_rect(Rect::from_min_max(pos2(x, top), pos2(x + tile_width, top + reach))),
                );
                let cell_id = cell.response().id;
                // Text in a tile is not selectable: a click on it selects the tile, as in the apps.
                cell.style_mut().interaction.selectable_labels = false;
                let surface = StoryFrame::of(&cell, story.surface);
                let mut header_rect = Rect::NOTHING;
                let used = Frame::new().inner_margin(surface.margin).show(&mut cell, |ui| {
                    surface.space(ui);
                    let tile_columns = Rect::from_x_y_ranges(x..=x + tile_width, f32::MIN..=f32::MAX);
                    ui.set_clip_rect(ui.clip_rect().intersect(tile_columns));
                    ui.set_min_width(ui.available_width());
                    let args = args_of(args, saved, story);
                    let changed = *args != story.defaults() || config.states.touched(story.id);
                    let strip = ui.painter().add(Shape::Noop);
                    let header = direction::horizontal(ui, |ui| {
                        ui.add(Text::new(tile.title).role(TypeRole::Heading));
                        if let Some(note) = tile.note {
                            ui.add(Text::new(format!("· {note}")).content(Content::Muted));
                        }
                        if config.buttons {
                            ui.with_layout(direction::trailing(ui), |ui| {
                                if tile.open && ui.add(IconButton::new(Icon::Open, format!("Open {}", tile.title)).size(ControlSize::Sm)).clicked() {
                                    events.opened = Some(story.component);
                                }
                                if changed && ui.add(IconButton::new(Icon::Undo, format!("Reset {}", tile.title)).size(ControlSize::Sm)).clicked() {
                                    events.reset = Some(story.id);
                                }
                            });
                        }
                    });
                    header_rect = header.response.rect;
                    let fill = DockLook::of(ui).tab_strip;
                    let strip_rect = Rect::from_min_max(pos2(x + 1.0, top + 1.0), pos2(x + tile_width - 1.0, header_rect.bottom() + 2.0));
                    ui.painter().set(strip, Shape::rect_filled(strip_rect, 0.0, fill));
                    // Only the selected tile of a component tab sets the gallery's scale; the request
                    // pending before this tile drew stays.
                    let sets_scale = config.scale_of_selected && config.selected == Some(tile.key);
                    let pending = if sets_scale { None } else { craft_ui::story::take_pixels_per_point_request(ui.ctx()) };
                    let layer = ui.layer_id();
                    let first = ui.ctx().graphics(|g| g.get(layer).map_or(0, |list| list.next_idx().0));
                    let body = ui.push_id(story.id, |ui| {
                        let ctx = ui.ctx().clone();
                        craft_ui::story::with_overview(&ctx, config.overview, || (story.show)(ui, args));
                        ui.unique_id()
                    });
                    if config.inspector.as_ref().is_none_or(|(key, _)| *key != tile.key) {
                        config.states.observe(ui.ctx(), story.id, body.response.rect, layer, cell_id);
                    }
                    // A story wider than the tile is cut at the tile's edges, never painted over a
                    // neighbour. A story that sets a clip rect of its own is cut here as well.
                    ui.ctx().graphics_mut(|g| {
                        if let Some(list) = g.get_mut(layer) {
                            for i in first..list.next_idx().0 {
                                list.mutate_shape(ShapeIdx(i), |shape| shape.clip_rect = shape.clip_rect.intersect(tile_columns));
                            }
                        }
                    });
                    if !sets_scale {
                        let _ = craft_ui::story::take_pixels_per_point_request(ui.ctx());
                        if let Some(ppp) = pending {
                            craft_ui::story::request_pixels_per_point(ui.ctx(), ppp);
                        }
                    }
                    if let Some(highlight) = config.highlight.as_mut() {
                        let size = vec2(tile_width.floor(), TILE_HEIGHT);
                        // The probe drew the story at the origin; here the header sits above it.
                        let origin = vec2(x, body.response.rect.min.y - surface.margin.top as f32);
                        let rects = highlight.tile(ui.ctx(), appearance, story, args, size, origin);
                        if !rects.is_empty() {
                            let stroke = scale::Stroke::W1.egui(ui.ctx(), ring);
                            for rect in rects {
                                ui.painter().rect_stroke(rect, 0.0, stroke, StrokeKind::Outside);
                            }
                            events.highlighted.push(tile.key);
                        }
                    }
                    if let Some((key, inspector)) = config.inspector.as_mut()
                        && *key == tile.key
                    {
                        inspector.overlay(ui, "Canvas".to_owned(), body.response.rect, body.inner.accesskit_id(), tokens.size.control.sm, rtl);
                    }
                });
                if config.scroll_to.is_some_and(|key| key == tile.key) {
                    cell.scroll_to_rect(used.response.rect, Some(Align::TOP));
                    *config.scroll_to = None;
                }
                let selected = config.selected == Some(tile.key);
                let highlighted = events.highlighted.last() == Some(&tile.key);
                if config.selectable {
                    let response = cell.response();
                    response.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, format!("Select {}", tile.title)));
                    if response.clicked() {
                        events.picked = Some(tile.key);
                    }
                    let on_header = response.ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| header_rect.contains(p));
                    if tile.open && response.double_clicked() && on_header {
                        events.opened = Some(story.component);
                    }
                }
                if let Some(highlight) = config.highlight.as_mut() {
                    highlight.clip_last(used.response.rect);
                }
                bottom = bottom.max(used.response.rect.bottom());
                placed.push((slot, x, surface, selected, highlighted));
            }
            for (slot, x, surface, selected, highlighted) in placed {
                let rect = Rect::from_min_max(pos2(x, top), pos2(x + tile_width, bottom));
                let mut shapes = Vec::new();
                if let Some(fill) = surface.fill {
                    shapes.push(Shape::Rect(RectShape::new(rect, 0.0, fill, surface.edge, StrokeKind::Inside)));
                }
                if highlighted {
                    // Inside the selection ring when the tile has both.
                    let inset = if selected { scale::Space::S2.px() } else { 0.0 };
                    shapes.push(Shape::Rect(RectShape::stroke(rect.shrink(inset), 0.0, scale::Stroke::W2.egui(ui.ctx(), ring), StrokeKind::Inside)));
                }
                if selected {
                    shapes.push(Shape::Rect(RectShape::stroke(rect, 0.0, scale::Stroke::W2.egui(ui.ctx(), ring), StrokeKind::Inside)));
                }
                ui.painter().set(slot, Shape::Vec(shapes));
            }
            ui.allocate_rect(Rect::from_min_max(pos2(area.min.x, top), pos2(area.max.x, bottom)), Sense::hover());
            ui.add_space(gap);
        }
    });
    events
}
