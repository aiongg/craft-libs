//! `LayerFilter`, `FilterBy`, `FilterKind` and `LayerFilterBar`: the filter row and its matching
//! rule (F11.05).
//!
//! The matching rule ([`LayerFilter::shows`]) is a plain function, so a host's control channel can
//! read and set the criteria (PhotoCraft's `ui.set layerFilter {by, kinds, name, mode}`) without
//! the panel.

#[cfg(test)]
mod tests;

use craft_theme::ControlSize;
use craft_ui::controls::{IconButton, SearchField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::panel::LayersStyle;
use super::tree::{BlendChoice, LayerInfo, LayerKind};

/// What the Layers panel filters by: the filter bar's type select (PhotoCraft's three types;
/// Photoshop's Effect, Attribute, Color, Selected and Artboard wait for the features that need
/// them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterBy {
    /// Layer kinds, toggled with five buttons.
    #[default]
    Kind,
    /// Part of the layer name, typed in a search field.
    Name,
    /// One blend mode, picked in a select.
    Mode,
}

/// A layer kind the Kind filter toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterKind {
    /// Pixel layers.
    Pixel,
    /// Adjustment and fill layers.
    Adjustment,
    /// Type layers.
    Type,
    /// Shape layers.
    Shape,
    /// Smart objects.
    SmartObject,
}

impl FilterKind {
    /// Every kind, in the filter bar's order.
    pub const ALL: [FilterKind; 5] = [FilterKind::Pixel, FilterKind::Adjustment, FilterKind::Type, FilterKind::Shape, FilterKind::SmartObject];

    /// PhotoCraft's control-channel name: `"pixel"`, `"adjustment"`, `"type"`, `"shape"`,
    /// `"smart"`.
    pub const fn name(self) -> &'static str {
        match self {
            FilterKind::Pixel => "pixel",
            FilterKind::Adjustment => "adjustment",
            FilterKind::Type => "type",
            FilterKind::Shape => "shape",
            FilterKind::SmartObject => "smart",
        }
    }
}

impl LayerKind {
    /// The kind the Kind filter matches this layer by: adjustment and fill layers are
    /// [`FilterKind::Adjustment`], text is [`FilterKind::Type`]; groups, artboards, paths, images
    /// and objects match no kind (`None`), as PhotoCraft's groups.
    pub const fn filter_kind(self) -> Option<FilterKind> {
        // pc: crates/ui-egui/src/panels/layer_filter_ui.rs:110-119 `kind` ("" for groups).
        match self {
            LayerKind::Pixel => Some(FilterKind::Pixel),
            LayerKind::Adjustment | LayerKind::Fill => Some(FilterKind::Adjustment),
            LayerKind::Text => Some(FilterKind::Type),
            LayerKind::Shape => Some(FilterKind::Shape),
            LayerKind::SmartObject => Some(FilterKind::SmartObject),
            LayerKind::Group | LayerKind::Artboard | LayerKind::Path | LayerKind::Image | LayerKind::Object => None,
        }
    }
}

/// The filter's criteria. The app stores it (PhotoCraft: in `UiState`, not saved with the
/// document) and passes it to [`LayersPanel::filter`](super::panel::LayersPanel::filter); every
/// field has a serde default.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LayerFilter {
    /// The filter type.
    pub by: FilterBy,
    /// The kinds toggled on, in the order they were toggled.
    pub kinds: Vec<FilterKind>,
    /// The name text as typed; matched trimmed and ignoring case.
    pub name: String,
    /// The blend mode's app key the Mode filter matches; `"normal"` at first.
    pub mode: String,
}

impl Default for LayerFilter {
    /// Kind, no kinds toggled, no name, mode `"normal"`: shows every layer.
    fn default() -> Self {
        Self { by: FilterBy::Kind, kinds: Vec::new(), name: String::new(), mode: String::from("normal") }
    }
}

impl LayerFilter {
    /// Whether the panel is filtered. As Photoshop, Mode always filters (by Normal at first);
    /// Kind only once a kind is toggled, Name only once the trimmed text is not empty.
    pub fn active(&self) -> bool {
        // pc: crates/ui-egui/src/panels/layer_filter_ui.rs:40-46 `Filter::active`.
        match self.by {
            FilterBy::Kind => !self.kinds.is_empty(),
            FilterBy::Name => !self.name.trim().is_empty(),
            FilterBy::Mode => true,
        }
    }

    /// Whether a layer is listed under this filter: always when the filter isn't
    /// [`active`](Self::active); Kind: its [`LayerKind::filter_kind`] is one of `kinds` (a group
    /// matches none); Name: `info.name` lowercased contains the trimmed `name` lowercased; Mode:
    /// `info.blend_key == mode`.
    pub fn shows(&self, info: &LayerInfo) -> bool {
        // pc: crates/ui-egui/src/panels/layer_filter_ui.rs:49-59 `Filter::shows`.
        if !self.active() {
            return true;
        }
        match self.by {
            FilterBy::Kind => info.kind.filter_kind().is_some_and(|kind| self.kinds.contains(&kind)),
            FilterBy::Name => info.name.to_lowercase().contains(&self.name.trim().to_lowercase()),
            FilterBy::Mode => info.blend_key == self.mode,
        }
    }
}

craft_ui::words! {
    /// The words of the filter bar.
    pub struct LayerFilterLabels {
        /// The type select's name for a screen reader.
        filter_type: "layers.filter.type" = "Filter type";
        /// The type select's entries.
        kind: "layers.filter.kind" = "Kind";
        name: "layers.filter.name" = "Name";
        mode: "layers.filter.mode" = "Mode";
        /// The kind toggles' tooltips and names.
        pixel: "layers.filter.pixel" = "Filter for pixel layers";
        adjustment: "layers.filter.adjustment" = "Filter for adjustment layers";
        type_layers: "layers.filter.typeLayers" = "Filter for type layers";
        shape: "layers.filter.shape" = "Filter for shape layers";
        smart_object: "layers.filter.smartObject" = "Filter for smart objects";
        /// The name field's placeholder.
        search: "layers.filter.search" = "Search";
        /// The name field's name for a screen reader.
        search_layers: "layers.filter.searchLayers" = "Search layers";
        /// The name field's clear button.
        clear_search: "layers.filter.clearSearch" = "Clear search";
        /// The mode select's name for a screen reader.
        blend_mode: "layers.filter.blendMode" = "Filter blend mode";
    }
}

/// PhotoCraft's filter row: the type select (Kind, Name, Mode; `layers.filter.type_width` wide),
/// then that type's controls, `layers.filter.gap` apart (`layers.filter.type_gap` more after the select).
///
/// ```ignore
/// let out = LayerFilterBar::new("layer-filter", &mut ui_state.layer_filter).blend_modes(&modes).show(ui);
/// if out.changed { /* the app stores the filter; the panel reads it next time it's drawn */ }
/// ```
///
/// - **Kind**: five Small [`IconButton`] toggles in
///   [`FilterKind::ALL`] order (icons image, adjustment, type, square, app window), pressed while
///   on; a click adds or removes the kind.
/// - **Name**: a [`SearchField`] over the rest of the row,
///   named "Search layers".
/// - **Mode**: a [`Select`] over the rest of the row with
///   `blend_modes` (the app gives Pass Through first, as PhotoCraft).
/// - Keyboard: each control is its own Tab stop; the select opens with Space or Alt+Down.
/// - AccessKit: the type select is a `ComboBox` named "Filter type"; the toggles are toggle
///   buttons named by their tooltip with their pressed state.
/// - Right to left, the controls run from the right edge.
#[must_use = "show the filter bar with `show`"]
pub struct LayerFilterBar<'a> {
    id_salt: IdSalt,
    filter: &'a mut LayerFilter,
    blend_modes: &'a [BlendChoice],
    labels: Option<LayerFilterLabels>,
}

/// What [`LayerFilterBar::show`] returns.
#[derive(Debug)]
#[must_use = "the host stores a changed filter"]
pub struct LayerFilterOutput {
    /// The criteria changed this pass (a toggle, a typed letter, a mode pick, the type select).
    pub changed: bool,
    /// The bar's row.
    pub response: Response,
}

impl<'a> LayerFilterBar<'a> {
    /// The bar editing `filter`; `id_salt` keeps the search field's focus.
    pub fn new(id_salt: impl AsIdSalt, filter: &'a mut LayerFilter) -> Self {
        Self { id_salt: IdSalt::new(id_salt), filter, blend_modes: &[], labels: None }
    }

    /// The modes the Mode select offers, in order; usually [`LayerTree::blend_modes`](super::tree::LayerTree::blend_modes)
    /// with groups, so Pass Through is in it.
    pub fn blend_modes(mut self, modes: &'a [BlendChoice]) -> Self {
        self.blend_modes = modes;
        self
    }

    /// The bar's words; the installed words by default.
    pub fn labels(mut self, labels: LayerFilterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the bar on one row of the available width.
    pub fn show(self, ui: &mut Ui) -> LayerFilterOutput {
        // pc: crates/ui-egui/src/panels/layer_filter_ui.rs:122-158 `bar` (gap 2, type select 64 and
        // 4 more after it, kind buttons 22, name field, mode dropdown).
        let Self { id_salt, filter, blend_modes, labels } = self;
        let labels = labels.unwrap_or_else(|| LayerFilterLabels::of(ui));
        let style = LayersStyle::of(&Tokens::of(ui));
        let size = ControlSize::Sm;
        let mut changed = false;
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = style.filter_gap;
            let mut by = filter.by;
            let picked = Select::new((id_salt, "by"), labels.filter_type.as_str(), &mut by)
                .option(FilterBy::Kind, labels.kind.as_str())
                .option(FilterBy::Name, labels.name.as_str())
                .option(FilterBy::Mode, labels.mode.as_str())
                .width(style.filter_type_width)
                .size(size)
                .show(ui);
            ui.add_space(style.filter_type_gap);
            if picked.event.as_ref().and_then(EditEvent::value).is_some() && by != filter.by {
                filter.by = by;
                changed = true;
            }
            match filter.by {
                FilterBy::Kind => {
                    let kinds = [
                        (FilterKind::Pixel, Icon::Image, &labels.pixel),
                        (FilterKind::Adjustment, Icon::AdjustmentLayer, &labels.adjustment),
                        (FilterKind::Type, Icon::Text, &labels.type_layers),
                        (FilterKind::Shape, Icon::Square, &labels.shape),
                        (FilterKind::SmartObject, Icon::AppWindow, &labels.smart_object),
                    ];
                    for (kind, icon, tip) in kinds {
                        let was = filter.kinds.contains(&kind);
                        let mut on = was;
                        ui.add(IconButton::new(icon, tip.as_str()).size(size).toggle(&mut on).id_salt((id_salt, kind.name())));
                        if on != was {
                            if on {
                                filter.kinds.push(kind);
                            } else {
                                filter.kinds.retain(|k| *k != kind);
                            }
                            changed = true;
                        }
                    }
                }
                FilterBy::Name => {
                    let edit = SearchField::new((id_salt, "name"), labels.search_layers.as_str(), &filter.name, labels.clear_search.as_str())
                        .placeholder(labels.search.as_str())
                        .size(size)
                        .show(ui);
                    if let Some(text) = edit.event.as_ref().and_then(EditEvent::value)
                        && *text != filter.name
                    {
                        filter.name.clone_from(text);
                        changed = true;
                    }
                }
                FilterBy::Mode => {
                    let mut mode = filter.mode.clone();
                    let mut select = Select::new((id_salt, "mode"), labels.blend_mode.as_str(), &mut mode).width(ui.available_width()).size(size);
                    for choice in blend_modes {
                        if choice.separator_before {
                            select = select.separator();
                        }
                        select = select.option(choice.key.clone(), choice.label.as_str());
                    }
                    let picked = select.show(ui);
                    if picked.event.as_ref().and_then(EditEvent::value).is_some() && mode != filter.mode {
                        filter.mode = mode;
                        changed = true;
                    }
                }
            }
        });
        LayerFilterOutput { changed, response: row.response }
    }
}
