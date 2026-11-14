//! `BrushesPanel`: the Brushes panel, a Size field over a preset browser.

use std::hash::Hash;

use craft_behavior::EditEvents;
use craft_ui::contract::{Bind, EditResponse, Outcome};
use craft_ui::patterns::presets::{PresetEvent, PresetFooterEvent, PresetRow, PresetState};
use craft_ui::runtime::drag::DropZone;
use craft_ui::runtime::env::Label;
use craft_ui::tile_grid::TileZoom;
use egui::{Response, Ui};

use super::picker::BrushPick;
use super::settings::BrushPickerSettings;

/// The thumbnail zoom range of the brush lists.
#[allow(dead_code)] // P3(panels-w12): pn.C49 the picker takes its range from here.
pub(super) const BRUSH_THUMBS: TileZoom = TileZoom::bounded(
    *BrushPickerSettings::SCALE_RANGE.start() as f64,
    *BrushPickerSettings::SCALE_RANGE.end() as f64,
    0.05,
    1.0,
);

/// The Brushes panel: a Size field over a [`PresetBrowser`](craft_ui::patterns::presets::PresetBrowser)
/// of the brush presets, with Import as the footer's one action.
#[must_use = "show the panel with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
#[pattern(probe(BrushesPanel<'static, u32>))]
pub struct BrushesPanel<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    label: Label,
    rows: &'a [PresetRow<K>],
    state: Bind<'a, PresetState<K>>,
    diameter: Bind<'a, f64>,
    max_size: f64,
    /// The selected preset, which the host keeps.
    #[prop]
    selected: Option<K>,
    /// Where files dropped on the panel go.
    #[prop]
    drop_zone: Option<DropZone>,
    /// Shows the footer's Import action.
    #[prop]
    import: bool,
}

/// What [`BrushesPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the panel's edits and picks"]
pub struct BrushesPanelOutput<K> {
    /// The panel.
    pub response: Response,
    /// The Size field's edit.
    pub diameter: EditResponse<f64>,
    /// The picks, in the order they happened.
    pub picks: Vec<BrushPick<K>>,
}

impl<K> Outcome for BrushesPanelOutput<K> {
    type Value = craft_ui::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> BrushesPanel<'a, K> {
    /// A panel named `label` over the presets in `rows`, with its view in `state` and the brush
    /// size in `diameter` up to `max_size`.
    pub fn new(label: impl Into<Label>, rows: &'a [PresetRow<K>], state: Bind<'a, PresetState<K>>, diameter: Bind<'a, f64>, max_size: f64) -> Self {
        BrushesPanel { label: label.into(), rows, state, diameter, max_size, selected: None, drop_zone: None, import: false }
    }

    /// Shows the panel.
    pub fn show(self, ui: &mut Ui) -> BrushesPanelOutput<K> {
        let BrushesPanel { label, rows, state, diameter, max_size, selected, drop_zone, import } = self;
        let _ = (ui, label, rows, state, diameter, max_size, selected, drop_zone, import, EditEvents::<f64>::new());
        todo!("pn.C49")
    }
}

/// Maps the browser's events to picks: `Select` and `Activate` to their picks, the footer's New to
/// `NewPreset`, the footer's one custom action to `Import`, every other event to `List`.
#[allow(dead_code)] // P3(panels-w12): pn.C49 the picker maps its browser events through this.
pub(super) fn picks_of<K>(events: Vec<PresetEvent<K>>) -> Vec<BrushPick<K>> {
    events
        .into_iter()
        .map(|event| match event {
            PresetEvent::Select { key, .. } => BrushPick::Select(key),
            PresetEvent::Activate { key, .. } => BrushPick::Activate(key),
            PresetEvent::Footer(PresetFooterEvent::New) => BrushPick::NewPreset,
            PresetEvent::Footer(PresetFooterEvent::Custom { index: 0, .. }) => BrushPick::Import,
            other => BrushPick::List(other),
        })
        .collect()
}
