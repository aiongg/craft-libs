//! Stories of the Layers panel, over [`MockLayers`] and a fake
//! thumbnail source (`FakeThumbSource` behind [`StorySource`]). The thumbnails render
//! synchronously, so no job queue and no stepping button are needed; every string the stories
//! add goes through [`LayersStoryLabels`].
//!
//! Each story keeps its own [`Demo`] (the tree, the settings, the filter) in egui memory and
//! applies the panel's events to the tree as an app would: selection, eyes, locks, open
//! groups, effects, renames, mask links, label colours and layer drops. The line under a story
//! names the last event.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/layers/panel/tree` | the 50-layer tree; args: thumbnail size, thumbnail contents, mask badges, lock column, filter bar |
//! | `composites/layers/panel/sizes` | None, Small, Medium and Large at Compact and Comfortable |
//! | `composites/layers/panel/narrow` | the least width plus a slider: blend labels drop, then the Opacity label |
//! | `composites/layers/panel/filtered` | the Kind, Name and Mode filters |
//! | `composites/layers/panel/no-active-layer` | greyed rows over a tree with no active layer, and an empty list |
//! | `composites/layers/panel/drag` | the tree to drag in: marker, ghost, outline, and the canvas preview line |
//! | `composites/layers/panel/illustrator-columns` | lock column, trailing target cell, no thumbnails, no blend rows |
//! | `composites/layers/panel/indesign-columns` | object rows under layers, no thumbnails, no filter or blend rows |
//! | `composites/layers/rows/kinds` | one row per kind |
//! | `composites/layers/rows/masks` | mask states, badges, the target bracket on each part |
//! | `composites/layers/rows/sub-rows` | Effects and Smart Filters rows, selected, hidden |
//! | `composites/layers/rows/indicators` | every indicator, then narrowing |
//! | `composites/layers/blend-rows/states` | normal, group, Background, several selected, no layer, narrow |
//! | `composites/layers/filter/types` | the three filter types |
//! | `composites/layers-options/default` | the Layers Panel Options dialog at defaults and with Large |
//!
//! The options dialog's id starts with `composites` (the skeleton's `dialogs/…` is not a story
//! level).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_theme::{LabelColor, Size};
use craft_ui::behavior::{DropPosition, SelectionPick};
use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::slider;
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::edit::EditEvent;
use craft_ui::list::ThumbSize;
use craft_ui::localized::Labels as _;
use craft_ui::menu::MenuItem;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryChoice, dialog_claim, dialog_closed, dialog_shown};
use craft_ui::theme::scope;
use craft_ui::typed_story;
use egui::{Id, Sense, Ui, vec2};

use super::filter::{FilterBy, FilterKind, LayerFilter, LayerFilterBar};
use super::mock::{MockLayers, effects, sub_row};
use super::options::{LayersOptions, LayersOptionsDialog, LayersSettings};
use super::panel::{LayerColumn, LayerDrop, LayerMove, LayersEvent, LayersPanel};
use super::rows::{BlendRows, LayerPropertyEdit};
use super::thumbs::{LayerThumb, LayerThumbKind, ThumbContents};
use super::tree::{
    LayerInfo, LayerKind, LayerLocks, LayerPart, LayerProperties, LayerRowKey, LayerSelection, LayerTree as _, LockKind, MaskInfo, SubRow, SubRows,
};

/// The components these stories cover.
pub(crate) const COMPONENTS: &[&str] = &["Layers panel", "Layer rows", "Blend rows", "Layer filter bar", "Layers options dialog"];

const PANEL: &str = "Layers panel";
const ROWS: &str = "Layer rows";
const BLEND: &str = "Blend rows";
const FILTER: &str = "Layer filter bar";
const OPTIONS: &str = "Layers options dialog";

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/layers/panel/tree", Level::Composites, PANEL, "Tree", TreeArgs::default(), show_tree),
        typed_story!("composites/layers/panel/sizes", Level::Composites, PANEL, "Sizes", NoArgs {}, show_sizes),
        typed_story!("composites/layers/panel/narrow", Level::Composites, PANEL, "Narrow", NarrowArgs { extra: 0.0 }, show_narrow),
        typed_story!("composites/layers/panel/filtered", Level::Composites, PANEL, "Filtered", NoArgs {}, show_filtered),
        typed_story!("composites/layers/panel/no-active-layer", Level::Composites, PANEL, "No active layer", NoArgs {}, show_no_active_layer),
        typed_story!("composites/layers/panel/drag", Level::Composites, PANEL, "Drag", NoArgs {}, show_drag),
        typed_story!("composites/layers/panel/illustrator-columns", Level::Composites, PANEL, "Illustrator columns", NoArgs {}, show_illustrator),
        typed_story!("composites/layers/panel/indesign-columns", Level::Composites, PANEL, "InDesign columns", NoArgs {}, show_indesign),
        typed_story!("composites/layers/rows/kinds", Level::Composites, ROWS, "Kinds", NoArgs {}, show_kinds),
        typed_story!("composites/layers/rows/masks", Level::Composites, ROWS, "Masks", MaskArgs::default(), show_masks),
        typed_story!("composites/layers/rows/sub-rows", Level::Composites, ROWS, "Sub-rows", NoArgs {}, show_sub_rows),
        typed_story!("composites/layers/rows/indicators", Level::Composites, ROWS, "Indicators", IndicatorArgs { width: 340.0 }, show_indicators),
        typed_story!("composites/layers/blend-rows/states", Level::Composites, BLEND, "States", NoArgs {}, show_blend_states),
        typed_story!("composites/layers/filter/types", Level::Composites, FILTER, "Types", NoArgs {}, show_filter_types),
        typed_story!(
            "composites/layers-options/default",
            Level::Composites,
            OPTIONS,
            "Default",
            OptionsArgs { open: true, large: false },
            show_options
        ),
    ]
}

craft_ui::words! {
    /// The words the Layers stories add around the panel.
    pub(crate) struct LayersStoryLabels {
        /// The line under a story; `{event}` is the last event the panel reported.
        last_event: "layersStory.lastEvent" = "Last event: {event}";
        no_event: "layersStory.noEvent" = "No event yet.";
        /// The canvas preview of a drag in progress; `{move}` describes the move.
        preview: "layersStory.preview" = "Canvas preview: {move}";
        no_preview: "layersStory.noPreview" = "No canvas preview.";
        /// The narrow story's readout.
        least_width: "layersStory.leastWidth" = "Least width: {n} points";
        drag_hint: "layersStory.dragHint" = "Drag a row over the others to see the drop marker, the ghost and the outline.";
        /// A tile caption of the sizes story: `{preset}` is the Size preset, `{thumbnails}` the thumbnail size.
        size_caption: "layersStory.sizeCaption" = "{preset}, {thumbnails}";
        compact: "layersStory.compact" = "Compact";
        comfortable: "layersStory.comfortable" = "Comfortable";
        thumbnails_none: "layersStory.thumbnailsNone" = "None";
        thumbnails_small: "layersStory.thumbnailsSmall" = "Small";
        thumbnails_medium: "layersStory.thumbnailsMedium" = "Medium";
        thumbnails_large: "layersStory.thumbnailsLarge" = "Large";
        /// Tile captions.
        filter_kind: "layersStory.filterKind" = "Filter by Kind";
        filter_name: "layersStory.filterName" = "Filter by Name";
        filter_mode: "layersStory.filterMode" = "Filter by Mode";
        no_active_layer: "layersStory.noActiveLayer" = "No active layer";
        empty_list: "layersStory.emptyList" = "No layers";
        blend_normal: "layersStory.blendNormal" = "Normal layer";
        blend_group: "layersStory.blendGroup" = "Group";
        blend_background: "layersStory.blendBackground" = "Background";
        blend_several: "layersStory.blendSeveral" = "Several layers selected";
        blend_none: "layersStory.blendNone" = "No active layer";
        blend_narrow: "layersStory.blendNarrow" = "Narrow";
        /// The filter story's count.
        matches: "layersStory.matches" = "{shown} of {total} layers match";
        /// The row menu's entries in the stories.
        duplicate: "layersStory.duplicate" = "Duplicate Layer";
        delete: "layersStory.delete" = "Delete Layer";
        /// The options dialog story's result lines.
        result: "layersStory.result" = "Result: {value}";
        cancelled: "layersStory.cancelled" = "Cancelled";
    }
}

/// The panel's tree, settings and filter, and the last event, kept in egui memory.
struct Demo {
    tree: MockLayers,
    settings: LayersSettings,
    filter: LayerFilter,
    last: Option<String>,
    preview: Option<LayerMove<u32>>,
}

impl Demo {
    fn new(tree: MockLayers) -> Self {
        Self { tree, settings: LayersSettings::default(), filter: LayerFilter::default(), last: None, preview: None }
    }

    fn with_filter(mut self, filter: LayerFilter) -> Self {
        self.filter = filter;
        self
    }

    fn apply(&mut self, events: Vec<LayersEvent<u32>>) {
        for event in events {
            if !matches!(event, LayersEvent::Properties(LayerPropertyEdit::BlendHover(_))) {
                self.last = Some(format!("{event:?}"));
            }
            match event {
                LayersEvent::Select(select) => self.tree.select(select.layer, select.pick, select.part),
                LayersEvent::ThumbClick { layer, part, .. } => self.tree.select(Some(layer), SelectionPick::Replace, Some(part)),
                LayersEvent::Toggle(toggle) => {
                    if let Some(edit) = toggle.edit.value() {
                        self.tree.set_toggle(&edit.row, toggle.column, edit.on);
                    }
                }
                LayersEvent::ShowOnly(LayerRowKey::Layer(id)) => self.tree.show_only(id),
                LayersEvent::LabelColor { rows, color } => {
                    for id in rows {
                        self.tree.layer_mut(id).info.label = color;
                    }
                }
                LayersEvent::Open(change) => {
                    for layer in &mut self.tree.layers {
                        if change.all || layer.id == change.row {
                            layer.open = change.open;
                        }
                    }
                }
                LayersEvent::FxOpen { layer, open, all } => {
                    for item in &mut self.tree.layers {
                        if all || item.id == layer {
                            item.effects_open = open;
                        }
                    }
                }
                LayersEvent::Rename { layer, text } => self.tree.layer_mut(layer).info.name = text,
                LayersEvent::MaskLink { layer, part, linked } => {
                    if let Some(mask) = self.tree.layer_mut(layer).info.masks.iter_mut().find(|mask| mask.part == part) {
                        mask.linked = linked;
                    }
                }
                LayersEvent::Drop(edit) => {
                    if matches!(edit, EditEvent::Commit { .. } | EditEvent::Set { .. })
                        && let Some(LayerDrop::Layers(mv)) = edit.value()
                    {
                        self.tree.move_layers(mv);
                    }
                }
                _ => {}
            }
        }
    }
}

/// The edits the stories apply to a [`MockLayers`], as an app's commands would.
impl MockLayers {
    fn select(&mut self, layer: Option<u32>, pick: SelectionPick, part: Option<LayerPart>) {
        let Some(layer) = layer.filter(|_| pick != SelectionPick::Clear) else {
            self.selection = LayerSelection::default();
            return;
        };
        let target = part.unwrap_or(self.selection.target);
        match pick {
            SelectionPick::Toggle => {
                if let Some(at) = self.selection.layers.iter().position(|selected| *selected == layer) {
                    self.selection.layers.remove(at);
                } else {
                    self.selection.layers.push(layer);
                }
                self.selection.active = Some(layer);
            }
            SelectionPick::Range => {
                let at = |id: u32| self.position(id).unwrap_or(0);
                let from = at(self.selection.active.unwrap_or(layer));
                let (low, high) = (from.min(at(layer)), from.max(at(layer)));
                self.selection.layers = self.layers[low..=high].iter().map(|l| l.id).collect();
                self.selection.active = Some(layer);
            }
            _ => self.selection = LayerSelection { layers: vec![layer], active: Some(layer), target },
        }
        self.selection.target = target;
    }

    /// An eye or lock cell, or an effects or Smart Filter row's eye.
    fn set_toggle(&mut self, row: &LayerRowKey<u32>, column: LayerColumn, on: bool) {
        match row {
            LayerRowKey::Layer(id) => {
                let info = &mut self.layer_mut(*id).info;
                match column {
                    LayerColumn::Visibility => info.visible = on,
                    _ => info.locks = LayerLocks { all: on, ..LayerLocks::default() },
                }
            }
            LayerRowKey::Effects(id, item) => {
                if let Some(rows) = &mut self.layer_mut(*id).effects {
                    set_sub_row(rows, *item, on);
                }
            }
            LayerRowKey::Filters(id, item) => {
                if let Some(rows) = &mut self.layer_mut(*id).smart_filters {
                    set_sub_row(rows, *item, on);
                }
            }
            LayerRowKey::Mask(..) => {}
        }
    }

    /// Alt-click on an eye: only `id` shows, or every layer again when it was the only one.
    fn show_only(&mut self, id: u32) {
        let alone = self.layers.iter().all(|layer| layer.id == id || !layer.info.visible);
        for layer in &mut self.layers {
            layer.info.visible = alone || layer.id == id;
        }
    }

    fn has_ancestor(&self, id: u32, ancestor: u32) -> bool {
        let mut at = self.parent(&id);
        while let Some(parent) = at {
            if parent == ancestor {
                return true;
            }
            at = self.parent(&parent);
        }
        false
    }

    /// The layer at `at` and its descendants, which follow it.
    fn block_len(&self, at: usize) -> usize {
        let id = self.layers[at].id;
        1 + self.layers[at + 1..].iter().take_while(|layer| self.has_ancestor(layer.id, id)).count()
    }

    fn position(&self, id: u32) -> Option<usize> {
        self.layers.iter().position(|layer| layer.id == id)
    }

    /// Moves the layers (a copy moves too: the story has no new ids to give).
    fn move_layers(&mut self, mv: &LayerMove<u32>) {
        if mv.layers.iter().any(|id| *id == mv.target || self.has_ancestor(mv.target, *id)) {
            return;
        }
        let mut moved = Vec::new();
        for id in &mv.layers {
            let Some(at) = self.position(*id) else { continue };
            let len = self.block_len(at);
            moved.push(self.layers.drain(at..at + len).collect::<Vec<_>>());
        }
        let Some(target) = self.position(mv.target) else { return };
        let (index, parent) = match mv.position {
            DropPosition::Before => (target, self.layers[target].parent),
            DropPosition::After => (target + self.block_len(target), self.layers[target].parent),
            DropPosition::Into => {
                self.layers[target].open = true;
                (target + 1, Some(mv.target))
            }
        };
        let mut block = Vec::new();
        for mut layers in moved {
            layers[0].parent = parent;
            block.extend(layers);
        }
        self.layers.splice(index..index, block);
    }
}

fn set_sub_row(rows: &mut SubRows, item: Option<usize>, on: bool) {
    match item {
        None => rows.header.on = on,
        Some(at) => {
            if let Some(row) = rows.items.get_mut(at) {
                row.on = on;
            }
        }
    }
}

/// The fake pictures, with a blank for the layers whose row draws a kind tile.
struct StorySource {
    pictures: FakeThumbSource<LayerThumb<u32>>,
    tiles: Vec<u32>,
}

impl StorySource {
    fn new(tree: &MockLayers) -> Self {
        let tiles = tree
            .layers
            .iter()
            .filter(|layer| matches!(layer.info.kind, LayerKind::Group | LayerKind::Adjustment | LayerKind::Text | LayerKind::Object))
            .map(|layer| layer.id)
            .collect();
        Self { pictures: FakeThumbSource::new().aspect(4, 3), tiles }
    }
}

impl ThumbSource<LayerThumb<u32>> for StorySource {
    fn revision(&self, key: &LayerThumb<u32>, request: &ThumbRequest) -> Revision {
        self.pictures.revision(key, request)
    }

    fn render(&self, key: &LayerThumb<u32>, request: &ThumbRequest) -> ThumbRender {
        if key.kind == LayerThumbKind::Pixels && self.tiles.contains(&key.layer) {
            return ThumbRender::Blank;
        }
        self.pictures.render(key, request)
    }
}

/// Per-instance state kept in egui memory under `name`.
fn shared<T: Send + 'static>(ui: &Ui, name: &str, init: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let key = ui.make_persistent_id(("craft_layers::stories", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(init()))).clone())
}

fn lock<T>(state: &Mutex<T>) -> MutexGuard<'_, T> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Which parts of the panel a story shows.
#[derive(Clone, Copy)]
struct Look {
    lock_column: bool,
    blend_rows: bool,
    filter_bar: bool,
    target_cell: bool,
}

impl Look {
    const FULL: Look = Look { lock_column: false, blend_rows: true, filter_bar: true, target_cell: false };
    const ROWS_ONLY: Look = Look { lock_column: false, blend_rows: false, filter_bar: false, target_cell: false };
}

/// How wide a panel is drawn.
#[derive(Clone, Copy)]
enum Width {
    Points(f32),
    /// The panel's least width plus this many points.
    LeastPlus(f32),
}

/// The panel over `demo` in a box `height` high; applies its events and returns its least width.
fn show_panel(ui: &mut Ui, key: &str, demo: &mut Demo, look: Look, width: Width, height: f32) -> f32 {
    let labels = LayersStoryLabels::of(ui);
    let thumbs = Thumbnails::<LayerThumb<u32>>::in_context(ui.ctx(), "craft-layers-stories", ThumbnailOptions::default());
    let source = StorySource::new(&demo.tree);
    let Demo { tree, settings, filter, .. } = demo;
    let mut panel = LayersPanel::new(key, &*tree)
        .settings(settings)
        .filter(filter)
        .thumbnails(&thumbs, &source)
        .lock_column(look.lock_column)
        .blend_rows(look.blend_rows)
        .filter_bar(look.filter_bar)
        .row_menu(|_| {
            vec![
                MenuItem::new(labels.duplicate.as_str(), String::from("duplicate")).into(),
                MenuItem::new(labels.delete.as_str(), String::from("delete")).into(),
            ]
        });
    if look.target_cell {
        panel = panel.trailing(18.0, |ui, _, info| {
            let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
            ui.painter().circle_stroke(rect.center(), 5.0, stroke);
            if info.visible {
                ui.painter().circle_filled(rect.center(), 2.0, stroke.color);
            }
        });
    }
    let wide = match width {
        Width::Points(points) => points,
        Width::LeastPlus(extra) => panel.min_width(ui) + extra,
    };
    let size = vec2(wide, height);
    let out = ui
        .allocate_ui(size, |ui| {
            ui.set_min_size(size);
            panel.show(ui)
        })
        .inner;
    demo.preview = out.drop_preview;
    demo.apply(out.events);
    out.min_width
}

fn status(ui: &mut Ui, demo: &Demo) {
    let labels = LayersStoryLabels::of(ui);
    let text = match &demo.last {
        Some(event) => craft_i18n::fmt(&labels.last_event, &[("event", event.as_str())]),
        None => labels.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

fn panel_width(ui: &Ui) -> f32 {
    ui.available_width().min(380.0)
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ThumbArg {
    None,
    Small,
    Medium,
    Large,
}

impl ThumbArg {
    fn size(self) -> ThumbSize {
        match self {
            ThumbArg::None => ThumbSize::None,
            ThumbArg::Small => ThumbSize::Small,
            ThumbArg::Medium => ThumbSize::Medium,
            ThumbArg::Large => ThumbSize::Large,
        }
    }
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct TreeArgs {
    /// Panel Options: Thumbnail Size.
    thumbnails: ThumbArg,
    /// Panel Options: Thumbnail Contents is Layer Bounds.
    #[arg(name = "clip to layer bounds")]
    layer_bounds: bool,
    #[arg(name = "mask badges")]
    mask_badges: bool,
    #[arg(name = "lock column")]
    lock_column: bool,
    #[arg(name = "show filters")]
    filters: bool,
}

impl Default for TreeArgs {
    fn default() -> Self {
        Self { thumbnails: ThumbArg::Medium, layer_bounds: false, mask_badges: false, lock_column: false, filters: true }
    }
}

fn contents(layer_bounds: bool) -> ThumbContents {
    if layer_bounds { ThumbContents::LayerBounds } else { ThumbContents::EntireDocument }
}

fn show_tree(ui: &mut Ui, args: &mut TreeArgs) {
    let demo = shared(ui, "tree", || Demo::new(MockLayers::fifty()));
    let mut demo = lock(&demo);
    demo.settings.options.thumbnail_size = args.thumbnails.size();
    demo.settings.options.thumbnail_contents = contents(args.layer_bounds);
    demo.settings.options.show_layer_mask_badges = args.mask_badges;
    demo.settings.show_filters = args.filters;
    let look = Look { lock_column: args.lock_column, ..Look::FULL };
    let width = panel_width(ui);
    show_panel(ui, "tree", &mut demo, look, Width::Points(width), 560.0);
    status(ui, &demo);
}

fn show_sizes(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    let presets = [(Size::Compact, labels.compact.as_str()), (Size::Comfortable, labels.comfortable.as_str())];
    let sizes = [
        (ThumbSize::None, labels.thumbnails_none.as_str()),
        (ThumbSize::Small, labels.thumbnails_small.as_str()),
        (ThumbSize::Medium, labels.thumbnails_medium.as_str()),
        (ThumbSize::Large, labels.thumbnails_large.as_str()),
    ];
    ui.horizontal_wrapped(|ui| {
        for (preset, preset_name) in presets {
            for (size, size_name) in sizes {
                ui.vertical(|ui| {
                    let caption = craft_i18n::fmt(&labels.size_caption, &[("preset", preset_name), ("thumbnails", size_name)]);
                    ui.add(Text::new(caption).content(Content::Muted));
                    scope(
                        ui,
                        |o| o.size = Some(preset),
                        |ui| {
                            let key = format!("sizes-{preset:?}-{size:?}");
                            let demo = shared(ui, &key, || Demo::new(MockLayers::sample()));
                            let mut demo = lock(&demo);
                            demo.settings.options.thumbnail_size = size;
                            show_panel(ui, &key, &mut demo, Look::ROWS_ONLY, Width::Points(230.0), 260.0);
                        },
                    );
                });
            }
        }
    });
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct NarrowArgs {
    /// Points added to the panel's least width.
    #[arg(range = 0.0..=240.0)]
    extra: f32,
}

fn show_narrow(ui: &mut Ui, args: &mut NarrowArgs) {
    let demo = shared(ui, "narrow", || Demo::new(MockLayers::fifty()));
    let mut demo = lock(&demo);
    let least = show_panel(ui, "narrow", &mut demo, Look::FULL, Width::LeastPlus(args.extra), 420.0);
    let labels = LayersStoryLabels::of(ui);
    let line = craft_i18n::fmt(&labels.least_width, &[("n", format!("{least:.0}").as_str())]);
    ui.add(Text::new(line).content(Content::Muted));
    status(ui, &demo);
}

fn show_filtered(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    let filters = [
        (labels.filter_kind.as_str(), LayerFilter { by: FilterBy::Kind, kinds: vec![FilterKind::Pixel, FilterKind::Type], ..LayerFilter::default() }),
        (labels.filter_name.as_str(), LayerFilter { by: FilterBy::Name, name: String::from("o"), ..LayerFilter::default() }),
        (labels.filter_mode.as_str(), LayerFilter { by: FilterBy::Mode, mode: String::from("multiply"), ..LayerFilter::default() }),
    ];
    ui.horizontal_wrapped(|ui| {
        for (n, (caption, filter)) in filters.into_iter().enumerate() {
            ui.vertical(|ui| {
                ui.add(Text::new(caption).content(Content::Muted));
                let key = format!("filtered-{n}");
                let demo = shared(ui, &key, || Demo::new(MockLayers::fifty()).with_filter(filter));
                let mut demo = lock(&demo);
                show_panel(ui, &key, &mut demo, Look::FULL, Width::Points(300.0), 380.0);
            });
        }
    });
}

fn show_no_active_layer(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    let mut inactive = MockLayers::sample();
    inactive.selection = LayerSelection::default();
    let tiles = [("no-active", labels.no_active_layer.as_str(), inactive), ("empty", labels.empty_list.as_str(), MockLayers::default())];
    ui.horizontal_wrapped(|ui| {
        for (key, caption, tree) in tiles {
            ui.vertical(|ui| {
                ui.add(Text::new(caption).content(Content::Muted));
                let demo = shared(ui, key, || Demo::new(tree));
                let mut demo = lock(&demo);
                show_panel(ui, key, &mut demo, Look::FULL, Width::Points(300.0), 360.0);
            });
        }
    });
}

fn show_drag(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    ui.add(Text::new(labels.drag_hint.as_str()).content(Content::Muted));
    let demo = shared(ui, "drag", || Demo::new(MockLayers::fifty()));
    let mut demo = lock(&demo);
    demo.settings.options.preview_on_canvas_when_reordering = true;
    let width = panel_width(ui);
    show_panel(ui, "drag", &mut demo, Look::FULL, Width::Points(width), 520.0);
    let line = match &demo.preview {
        Some(preview) => craft_i18n::fmt(&labels.preview, &[("move", format!("{preview:?}").as_str())]),
        None => labels.no_preview.clone(),
    };
    ui.add(Text::new(line).content(Content::Muted));
    status(ui, &demo);
}

fn named(name: &str, kind: LayerKind) -> LayerInfo {
    LayerInfo::new(name, kind)
}

fn show_illustrator(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "illustrator", || {
        let mut tree = MockLayers::default();
        let layer = tree.add(None, named("Layer 1", LayerKind::Group), true);
        for name in ["Path", "Compound Path", "Group"] {
            tree.add(Some(layer), named(name, LayerKind::Path), false);
        }
        let art = tree.add(None, named("Layer 2", LayerKind::Group), true);
        let logo = tree.add(Some(art), named("Logo", LayerKind::Path), false);
        tree.layer_mut(logo).info.locks.all = true;
        tree.selection = LayerSelection { layers: vec![2], active: Some(2), target: LayerPart::Pixels };
        let mut demo = Demo::new(tree);
        demo.settings.options.thumbnail_size = ThumbSize::None;
        demo
    });
    let mut demo = lock(&demo);
    let look = Look { lock_column: true, target_cell: true, ..Look::ROWS_ONLY };
    show_panel(ui, "illustrator", &mut demo, look, Width::Points(300.0), 320.0);
    status(ui, &demo);
}

fn show_indesign(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "indesign", || {
        let mut tree = MockLayers::default();
        let layer = tree.add(None, named("Layer 1", LayerKind::Group), true);
        tree.add(Some(layer), named("Headline", LayerKind::Object), false);
        tree.add(Some(layer), named("Body frame", LayerKind::Object), false);
        tree.add(Some(layer), named("Photo frame", LayerKind::Object), false);
        let grid = tree.add(None, named("Grid", LayerKind::Group), true);
        tree.add(Some(grid), named("Guides", LayerKind::Object), false);
        tree.selection = LayerSelection { layers: vec![3], active: Some(3), target: LayerPart::Pixels };
        let mut demo = Demo::new(tree);
        demo.settings.options.thumbnail_size = ThumbSize::None;
        demo
    });
    let mut demo = lock(&demo);
    let look = Look { lock_column: true, ..Look::ROWS_ONLY };
    show_panel(ui, "indesign", &mut demo, look, Width::Points(300.0), 320.0);
    status(ui, &demo);
}

fn show_kinds(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "kinds", || {
        let mut tree = MockLayers::default();
        let kinds = [
            ("Pixel", LayerKind::Pixel),
            ("Group", LayerKind::Group),
            ("Adjustment", LayerKind::Adjustment),
            ("Fill", LayerKind::Fill),
            ("Type", LayerKind::Text),
            ("Shape", LayerKind::Shape),
            ("Smart Object", LayerKind::SmartObject),
            ("Artboard", LayerKind::Artboard),
            ("Path", LayerKind::Path),
            ("Image", LayerKind::Image),
            ("Object", LayerKind::Object),
        ];
        for (name, kind) in kinds {
            tree.add(None, named(name, kind), false);
        }
        Demo::new(tree)
    });
    let mut demo = lock(&demo);
    let width = panel_width(ui);
    show_panel(ui, "kinds", &mut demo, Look::ROWS_ONLY, Width::Points(width), 520.0);
    status(ui, &demo);
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PartArg {
    Pixels,
    #[choice(name = "layer mask")]
    LayerMask,
    #[choice(name = "vector mask")]
    VectorMask,
}

impl PartArg {
    fn part(self) -> LayerPart {
        match self {
            PartArg::Pixels => LayerPart::Pixels,
            PartArg::LayerMask => LayerPart::LayerMask,
            PartArg::VectorMask => LayerPart::VectorMask,
        }
    }
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct MaskArgs {
    /// The target part of the active layer ("Both masks").
    target: PartArg,
    #[arg(name = "mask badges")]
    badges: bool,
}

impl Default for MaskArgs {
    fn default() -> Self {
        Self { target: PartArg::LayerMask, badges: true }
    }
}

fn show_masks(ui: &mut Ui, args: &mut MaskArgs) {
    let demo = shared(ui, "masks", || {
        let mut tree = MockLayers::default();
        let mask = |part, enabled, linked| MaskInfo { part, enabled, linked };
        let rows = [
            ("Pixel mask", vec![mask(LayerPart::LayerMask, true, true)]),
            ("Unlinked pixel mask", vec![mask(LayerPart::LayerMask, true, false)]),
            ("Disabled pixel mask", vec![mask(LayerPart::LayerMask, false, true)]),
            ("Vector mask", vec![mask(LayerPart::VectorMask, true, true)]),
            ("Disabled vector mask", vec![mask(LayerPart::VectorMask, false, false)]),
            ("Both masks", vec![mask(LayerPart::LayerMask, true, true), mask(LayerPart::VectorMask, true, false)]),
        ];
        for (name, masks) in rows {
            let mut info = named(name, LayerKind::Pixel);
            info.masks = masks;
            tree.add(None, info, false);
        }
        tree.selection = LayerSelection { layers: vec![6], active: Some(6), target: LayerPart::LayerMask };
        Demo::new(tree)
    });
    let mut demo = lock(&demo);
    demo.settings.options.show_layer_mask_badges = args.badges;
    demo.settings.options.thumbnail_size = ThumbSize::Large;
    demo.tree.selection.target = args.target.part();
    let width = panel_width(ui);
    show_panel(ui, "masks", &mut demo, Look::ROWS_ONLY, Width::Points(width), 520.0);
    status(ui, &demo);
}

fn show_sub_rows(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "sub-rows", || {
        let mut tree = MockLayers::default();
        let mut smart = named("Smart Object", LayerKind::SmartObject);
        smart.has_smart_filters = true;
        let smart = tree.add(None, smart, false);
        tree.layer_mut(smart).effects_open = true;
        tree.layer_mut(smart).smart_filters = Some(SubRows {
            header: SubRow { label: String::from("Smart Filters"), on: true, eye_shown: true, icon: None, mask: true },
            items: vec![sub_row("Gaussian Blur"), SubRow { on: false, ..sub_row("Sharpen") }],
        });
        let mut styled = named("Styled", LayerKind::Pixel);
        styled.has_effects = true;
        let styled = tree.add(None, styled, false);
        tree.layer_mut(styled).effects_open = true;
        let mut fx = effects(&["Drop Shadow", "Stroke"]);
        fx.items[1].on = false;
        tree.layer_mut(styled).effects = Some(fx);
        let mut hidden = named("Hidden effects", LayerKind::Pixel);
        hidden.has_effects = true;
        let hidden = tree.add(None, hidden, false);
        tree.layer_mut(hidden).effects_open = true;
        let mut fx = effects(&["Inner Glow"]);
        fx.header.on = false;
        tree.layer_mut(hidden).effects = Some(fx);
        let mut closed = named("Closed effects", LayerKind::Pixel);
        closed.has_effects = true;
        let closed = tree.add(None, closed, false);
        tree.layer_mut(closed).effects = Some(effects(&["Bevel"]));
        tree.selection = LayerSelection { layers: vec![smart], active: Some(smart), target: LayerPart::Pixels };
        Demo::new(tree)
    });
    let mut demo = lock(&demo);
    let width = panel_width(ui);
    show_panel(ui, "sub-rows", &mut demo, Look::ROWS_ONLY, Width::Points(width), 420.0);
    status(ui, &demo);
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct IndicatorArgs {
    /// The panel's width, down to its least.
    #[arg(range = 160.0..=380.0)]
    width: f32,
}

fn show_indicators(ui: &mut Ui, args: &mut IndicatorArgs) {
    let demo = shared(ui, "indicators", || {
        let mut tree = MockLayers::default();
        let mut all = named("Every indicator", LayerKind::SmartObject);
        all.has_effects = true;
        all.has_smart_filters = true;
        all.linked = true;
        all.clipped = true;
        all.label = Some(LabelColor::Red);
        all.blend_key = String::from("multiply");
        all.blend_label = Some(String::from("Multiply"));
        all.locks = LayerLocks { position: true, ..LayerLocks::default() };
        all.warning = Some(String::from("The linked file is missing."));
        tree.add(None, all, false);
        let mut off = named("Smart Filters off", LayerKind::SmartObject);
        off.has_smart_filters = true;
        off.smart_filters_enabled = false;
        tree.add(None, off, false);
        let mut locked = named("Locked", LayerKind::Pixel);
        locked.locks = LayerLocks { all: true, ..LayerLocks::default() };
        tree.add(None, locked, false);
        let mut fx = named("Effects", LayerKind::Pixel);
        fx.has_effects = true;
        tree.add(None, fx, false);
        let mut linked = named("Linked", LayerKind::Pixel);
        linked.linked = true;
        tree.add(None, linked, false);
        Demo::new(tree)
    });
    let mut demo = lock(&demo);
    show_panel(ui, "indicators", &mut demo, Look::ROWS_ONLY, Width::Points(args.width), 320.0);
    status(ui, &demo);
}

/// The properties of one blend-rows tile, edited by its own rows.
#[derive(Clone)]
struct BlendDemo {
    properties: Option<LayerProperties>,
    last: Option<String>,
}

fn properties(group: bool, editable: bool, locks: LayerLocks) -> LayerProperties {
    LayerProperties { blend_key: String::from(if group { "passThrough" } else { "normal" }), group, opacity: 100.0, fill: 100.0, locks, editable }
}

fn set_lock(locks: &mut LayerLocks, kind: LockKind, on: bool) {
    match kind {
        LockKind::Transparency => locks.transparency = on,
        LockKind::Pixels => locks.pixels = on,
        LockKind::Position => locks.position = on,
        LockKind::Artboard => locks.artboard = on,
        LockKind::All => locks.all = on,
    }
}

fn show_blend_states(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    let background = LayerLocks { all: true, ..LayerLocks::default() };
    let several = LayerLocks { pixels: true, ..LayerLocks::default() };
    let tiles = [
        ("normal", labels.blend_normal.as_str(), Some(properties(false, true, LayerLocks::default())), 300.0),
        ("group", labels.blend_group.as_str(), Some(properties(true, true, LayerLocks::default())), 300.0),
        ("background", labels.blend_background.as_str(), Some(properties(false, false, background)), 300.0),
        ("several", labels.blend_several.as_str(), Some(properties(false, true, several)), 300.0),
        ("none", labels.blend_none.as_str(), None, 300.0),
        ("narrow", labels.blend_narrow.as_str(), Some(properties(false, true, LayerLocks::default())), 200.0),
    ];
    for (key, caption, start, width) in tiles {
        ui.add(Text::new(caption).content(Content::Muted));
        let group = start.as_ref().is_some_and(|p| p.group);
        let modes = MockLayers::default().blend_modes(group, "");
        slider::with_values(ui, key, [BlendDemo { properties: start, last: None }], |ui, state| {
            let tile = &mut state[0];
            let out = ui.allocate_ui(vec2(width, 0.0), |ui| BlendRows::new(key, tile.properties.as_ref(), &modes).show(ui)).inner;
            for edit in out.events {
                tile.last = Some(format!("{edit:?}"));
                let Some(properties) = tile.properties.as_mut() else { continue };
                match &edit {
                    LayerPropertyEdit::Blend(event) => {
                        if let Some(blend) = event.value() {
                            properties.blend_key.clone_from(blend);
                        }
                    }
                    LayerPropertyEdit::Opacity(event) => properties.opacity = event.value().copied().unwrap_or(properties.opacity),
                    LayerPropertyEdit::Fill(event) => properties.fill = event.value().copied().unwrap_or(properties.fill),
                    LayerPropertyEdit::Lock { kind, on } => set_lock(&mut properties.locks, *kind, *on),
                    _ => {}
                }
            }
            if let Some(event) = &tile.last {
                let line = craft_i18n::fmt(&labels.last_event, &[("event", event.as_str())]);
                ui.add(Text::new(line).content(Content::Muted));
            }
        });
        ui.add_space(ui.spacing().item_spacing.y);
    }
}

fn show_filter_types(ui: &mut Ui, _: &mut NoArgs) {
    let labels = LayersStoryLabels::of(ui);
    let modes = MockLayers::default().blend_modes(false, "");
    let total = MockLayers::fifty().layers;
    let tiles = [
        ("kind", labels.filter_kind.as_str(), LayerFilter { kinds: vec![FilterKind::Pixel], ..LayerFilter::default() }),
        ("name", labels.filter_name.as_str(), LayerFilter { by: FilterBy::Name, name: String::from("mask"), ..LayerFilter::default() }),
        ("mode", labels.filter_mode.as_str(), LayerFilter { by: FilterBy::Mode, mode: String::from("multiply"), ..LayerFilter::default() }),
    ];
    for (key, caption, start) in tiles {
        ui.add(Text::new(caption).content(Content::Muted));
        slider::with_values(ui, key, [start], |ui, state| {
            let filter = &mut state[0];
            let width = ui.available_width().min(380.0);
            ui.allocate_ui(vec2(width, 0.0), |ui| {
                let _ = LayerFilterBar::new(key, filter).blend_modes(&modes).show(ui);
            });
            let shown = total.iter().filter(|layer| filter.shows(&layer.info)).count();
            let line = craft_i18n::fmt(&labels.matches, &[("shown", shown.to_string().as_str()), ("total", total.len().to_string().as_str())]);
            ui.add(Text::new(line).content(Content::Muted));
        });
        ui.add_space(ui.spacing().item_spacing.y);
    }
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct OptionsArgs {
    open: bool,
    /// Opens with Thumbnail Size Large instead of the defaults.
    large: bool,
}

fn show_options(ui: &mut Ui, args: &mut OptionsArgs) {
    let labels = LayersStoryLabels::of(ui);
    let host = shared(ui, "options-dialog", DialogHost::new);
    let ctx = ui.ctx().clone();
    let instance: Id = ui.make_persistent_id("craft_layers::stories::options");
    let (result_key, signature_key) = (instance.with("result"), instance.with("signature"));
    let mut host = lock(&host);
    if dialog_shown(ui, args.open) && dialog_claim(&ctx, "layers-options", instance) {
        let previous: Option<bool> = ui.data(|d| d.get_temp::<Option<bool>>(signature_key)).flatten();
        if previous.is_some_and(|previous| previous != args.large) {
            let _ = host.cancel::<LayersOptionsDialog>(&ctx);
            let _ = host.take::<LayersOptionsDialog>();
        }
        ui.data_mut(|d| d.insert_temp(signature_key, Some(args.large)));
        if !host.is_open::<LayersOptionsDialog>() {
            let thumbnail_size = if args.large { ThumbSize::Large } else { ThumbSize::Medium };
            host.open(LayersOptionsDialog::new().initial(LayersOptions { thumbnail_size, ..LayersOptions::default() }));
            ui.data_mut(|d| d.remove_temp::<String>(result_key));
        }
        host.show(&ctx);
        if let Some(result) = host.take::<LayersOptionsDialog>() {
            let text = match result {
                DialogResult::Ok(options) => craft_i18n::fmt(&labels.result, &[("value", format!("{options:?}").as_str())]),
                DialogResult::Cancel => labels.cancelled.clone(),
            };
            ui.data_mut(|d| d.insert_temp(result_key, text));
            dialog_closed(ui, &mut args.open);
        }
    } else {
        let _ = host.cancel::<LayersOptionsDialog>(&ctx);
        let _ = host.take::<LayersOptionsDialog>();
        ui.data_mut(|d| d.insert_temp(signature_key, None::<bool>));
    }
    if let Some(text) = ui.data(|d| d.get_temp::<String>(result_key)) {
        ui.add(Text::new(text).content(Content::Muted));
    }
}
