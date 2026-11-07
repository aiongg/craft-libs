//! `StatusReadout`: the document readout of a status bar: the zoom field, one chosen information item and the menu
//! that chooses it. A pattern over the children of a `Bar` with `BarKind::Status`.

use super::words::BarWords;
use crate::buttons::IconButton;
use crate::contract::{Bind, EditResponse, Entry, GlyphLabel, LabelKind, Outcome, Props as _, Show as _, ShowContainer as _, SideLabel, Sizing};
use crate::display::{Elide, Text};
use crate::form::Field;
use crate::layout_components::Row;
use crate::runtime::env::{Env, Label, UnitKind, Units};
use craft_behavior::IconId;
use egui::{Response, Ui};
use std::ops::RangeInclusive;
use std::time::Duration;

#[cfg(feature = "testing")]
mod stories;

/// The zoom range in percent when the caller gives none.
const DEFAULT_RANGE: RangeInclusive<f64> = 0.01..=100_000.0;

/// Shown for an item whose value the host does not know.
const MISSING: &str = "\u{2014}";

/// The item the readout shows, chosen in its menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatusItem {
    /// The flattened size and the size with layers.
    #[default]
    DocumentSizes,
    /// The document's colour profile name.
    DocumentProfile,
    /// The document's dimensions text.
    DocumentDimensions,
    /// The document's measurement scale text.
    MeasurementScale,
    /// Memory the app uses for open documents and what it may use.
    ScratchSizes,
    /// The share of time spent working rather than waiting on scratch.
    Efficiency,
    /// How long the last operation took.
    Timing,
    /// The name of the active tool.
    CurrentTool,
    /// The number of layers in the document.
    LayerCount,
}

/// The values the host knows this frame; an item without a value shows a dash.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct StatusValues {
    /// Flattened and layered sizes in bytes.
    pub document_sizes: Option<(u64, u64)>,
    /// The colour profile name, shown as given.
    pub document_profile: Option<String>,
    /// The dimensions text, shown as given.
    pub document_dimensions: Option<String>,
    /// The measurement scale text, shown as given.
    pub measurement_scale: Option<String>,
    /// Used and available bytes.
    pub scratch_sizes: Option<(u64, u64)>,
    /// 0 to 100.
    pub efficiency: Option<u8>,
    /// How long the last operation took.
    pub timing: Option<Duration>,
    /// The active tool's name, shown as given.
    pub current_tool: Option<String>,
    /// The number of layers.
    pub layer_count: Option<usize>,
}

impl StatusItem {
    /// Every item in the menu's order.
    pub const ALL: [StatusItem; 9] = [
        StatusItem::DocumentSizes,
        StatusItem::DocumentProfile,
        StatusItem::DocumentDimensions,
        StatusItem::MeasurementScale,
        StatusItem::ScratchSizes,
        StatusItem::Efficiency,
        StatusItem::Timing,
        StatusItem::CurrentTool,
        StatusItem::LayerCount,
    ];

    /// The item's name in the menu.
    fn name(self, words: &BarWords) -> Label {
        match self {
            StatusItem::DocumentSizes => words.readout_document_sizes.clone(),
            StatusItem::DocumentProfile => words.readout_document_profile.clone(),
            StatusItem::DocumentDimensions => words.readout_document_dimensions.clone(),
            StatusItem::MeasurementScale => words.readout_measurement_scale.clone(),
            StatusItem::ScratchSizes => words.readout_scratch_sizes.clone(),
            StatusItem::Efficiency => words.readout_efficiency.clone(),
            StatusItem::Timing => words.readout_timing.clone(),
            StatusItem::CurrentTool => words.readout_current_tool.clone(),
            StatusItem::LayerCount => words.readout_layer_count.clone(),
        }
    }

    /// The item's text for `values`, a dash when the value is missing.
    fn text(self, values: &StatusValues, words: &BarWords, env: &Env) -> String {
        let units: Units = env.units();
        let given = |value: &Option<String>| value.clone().unwrap_or_else(|| MISSING.to_owned());
        match self {
            StatusItem::DocumentSizes => values.document_sizes.map_or_else(
                || MISSING.to_owned(),
                |(flat, layered)| words.readout_doc_sizes.fill(&[("flat", &units.file_size(flat)), ("layered", &units.file_size(layered))]),
            ),
            StatusItem::DocumentProfile => given(&values.document_profile),
            StatusItem::DocumentDimensions => given(&values.document_dimensions),
            StatusItem::MeasurementScale => given(&values.measurement_scale),
            StatusItem::ScratchSizes => values.scratch_sizes.map_or_else(
                || MISSING.to_owned(),
                |(used, available)| words.readout_scratch.fill(&[("used", &units.file_size(used)), ("available", &units.file_size(available))]),
            ),
            StatusItem::Efficiency => values
                .efficiency
                .map_or_else(|| MISSING.to_owned(), |n| words.readout_efficiency_text.fill(&[("value", &units.percent(f64::from(n.min(100))))])),
            StatusItem::Timing => values
                .timing
                .map_or_else(|| MISSING.to_owned(), |time| words.readout_timing_text.fill(&[("value", &units.duration(time.as_secs_f64()))])),
            StatusItem::CurrentTool => given(&values.current_tool),
            StatusItem::LayerCount => {
                values.layer_count.map_or_else(|| MISSING.to_owned(), |n| words.readout_layers.text(n as u64, &env.format().integer(n as u64)))
            }
        }
    }
}

/// What the readout reports.
#[non_exhaustive]
pub struct StatusReadoutOutput {
    /// The readout row's response.
    pub response: Response,
    /// The zoom field's edit, in percent; `None` when the row did not run.
    pub zoom: Option<EditResponse<Option<f64>>>,
    /// The item chosen in the menu this pass; the host stores it.
    pub item: Option<StatusItem>,
}

impl Outcome for StatusReadoutOutput {
    type Value = Option<f64>;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        crate::testing::StoryOutput::of_edit(&self.response, self.zoom.as_ref().and_then(|zoom| zoom.event.as_ref()))
    }
}

/// The zoom field, the chosen item's text and the menu that chooses it, shown as children of a status `Bar`.
///
/// The field keeps no value of its own: the zoom is the caller's binding in percent. The item is the caller's value,
/// and a pick comes back in the output.
#[derive(crate::contract::Pattern)]
pub struct StatusReadout<'a> {
    zoom: Bind<'a, Option<f64>>,
    item: StatusItem,
    values: &'a StatusValues,
    range: RangeInclusive<f64>,
    tips: Vec<(StatusItem, Label)>,
}

impl<'a> StatusReadout<'a> {
    /// A readout of the zoom `zoom` in percent that shows `item` from `values`.
    pub fn new(zoom: Bind<'a, Option<f64>>, item: StatusItem, values: &'a StatusValues) -> Self {
        StatusReadout { zoom, item, values, range: DEFAULT_RANGE, tips: Vec::new() }
    }

    /// The zoom field's range in percent.
    #[must_use]
    pub fn range(mut self, percent: RangeInclusive<f64>) -> Self {
        self.range = percent;
        self
    }

    /// A tooltip for the info text while `item` is the one shown.
    #[must_use]
    pub fn tooltip(mut self, item: StatusItem, text: impl Into<Label>) -> Self {
        self.tips.retain(|(shown, _)| *shown != item);
        self.tips.push((item, text.into()));
        self
    }

    /// Shows the readout in a row: the zoom field, whose glyph label scrubs the zoom, the info text and the menu
    /// button.
    pub fn show(mut self, ui: &mut Ui) -> StatusReadoutOutput {
        let env = Env::of(ui);
        let words = env.words::<BarWords>();
        let text = self.item.text(self.values, &words, &env);
        let shown = Row::new().show(ui, |ui, _| {
            let glyph = SideLabel::new().kind(LabelKind::Glyph(GlyphLabel::icon(IconId::new("zoom_in")))).scrub(true);
            let zoom = Field::new(words.readout_zoom.clone()).side(glyph).sizing(Sizing::Content).show(ui, |ui, _| {
                crate::text::NumberField::new(words.readout_zoom.clone(), self.zoom.reborrowed())
                    .unit_kind(UnitKind::Percent)
                    .range(self.range.clone())
                    .scrub_by_label()
                    .show(ui)
            });
            let mut info = Text::new(text.clone()).elide(Elide::End);
            if let Some((_, tip)) = self.tips.iter().find(|(item, _)| *item == self.item) {
                info = info.tooltip(tip.clone());
            }
            info.show(ui);
            let more = IconButton::new(IconId::new("chevron_right")).label(words.readout_menu.clone()).show(ui);
            let entries: Vec<Entry<StatusItem>> =
                StatusItem::ALL.iter().map(|&item| Entry::new(item, item.name(&words)).checked(item == self.item)).collect();
            let menu = crate::menu::pick_values_from(ui, words.readout_menu.clone(), &more, &entries);
            (zoom.inner.map(|zoom| zoom.edit), menu.picked)
        });
        let (zoom, item) = shown.inner.unzip();
        StatusReadoutOutput { response: shown.output, zoom: zoom.flatten(), item: item.flatten() }
    }
}
