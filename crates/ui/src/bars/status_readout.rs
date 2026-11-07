//! `StatusReadout`: the document readout of a status bar: the zoom field, one chosen information item and the menu
//! that chooses it. A pattern over the children of a `Bar` with `BarKind::Status`.

use super::words::StatusReadoutWords;
use crate::buttons::IconButton;
use crate::contract::{Bind, BodyCx, EditResponse, Entry, Outcome, Props as _, Show as _, ShowContainer as _, WordSetCx as _};
use crate::display::{Elide, Text};
use crate::runtime::env::{Label, UnitKind};
use craft_behavior::IconId;
use craft_i18n::Format;
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

/// The unit of a compact byte size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SizeUnit {
    /// Thousands of bytes (steps of 1024).
    Kilo,
    /// Millions of bytes.
    Mega,
    /// Billions of bytes.
    Giga,
}

/// A byte count in the largest of kilo, mega and giga that keeps the value at 1 or more (kilo below a mega).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompactSize {
    /// The count in `unit`.
    pub value: f64,
    /// The unit.
    pub unit: SizeUnit,
}

/// `bytes` in steps of 1024.
pub fn compact_size(bytes: u64) -> CompactSize {
    const STEP: f64 = 1024.0;
    let bytes = bytes as f64;
    let (divisor, unit) = if bytes >= STEP * STEP * STEP {
        (STEP * STEP * STEP, SizeUnit::Giga)
    } else if bytes >= STEP * STEP {
        (STEP * STEP, SizeUnit::Mega)
    } else {
        (STEP, SizeUnit::Kilo)
    };
    CompactSize { value: bytes / divisor, unit }
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
    fn name(self, words: &StatusReadoutWords) -> Label {
        match self {
            StatusItem::DocumentSizes => words.document_sizes.clone(),
            StatusItem::DocumentProfile => words.document_profile.clone(),
            StatusItem::DocumentDimensions => words.document_dimensions.clone(),
            StatusItem::MeasurementScale => words.measurement_scale.clone(),
            StatusItem::ScratchSizes => words.scratch_sizes.clone(),
            StatusItem::Efficiency => words.efficiency.clone(),
            StatusItem::Timing => words.timing.clone(),
            StatusItem::CurrentTool => words.current_tool.clone(),
            StatusItem::LayerCount => words.layer_count.clone(),
        }
    }

    /// The item's text for `values`, a dash when the value is missing.
    fn text(self, values: &StatusValues, words: &StatusReadoutWords, format: &Format) -> String {
        let given = |value: &Option<String>| value.clone().unwrap_or_else(|| MISSING.to_owned());
        let size = |bytes: u64| {
            let compact = compact_size(bytes);
            let template = match compact.unit {
                SizeUnit::Kilo => &words.size_kilo,
                SizeUnit::Mega => &words.size_mega,
                SizeUnit::Giga => &words.size_giga,
            };
            template.fill(&[("n", &format.number(compact.value, 1))])
        };
        match self {
            StatusItem::DocumentSizes => values.document_sizes.map_or_else(
                || MISSING.to_owned(),
                |(flat, layered)| words.doc_prefix.fill(&[("flat", &size(flat)), ("layered", &size(layered))]),
            ),
            StatusItem::DocumentProfile => given(&values.document_profile),
            StatusItem::DocumentDimensions => given(&values.document_dimensions),
            StatusItem::MeasurementScale => given(&values.measurement_scale),
            StatusItem::ScratchSizes => values.scratch_sizes.map_or_else(
                || MISSING.to_owned(),
                |(used, available)| words.scratch_prefix.fill(&[("used", &size(used)), ("available", &size(available))]),
            ),
            StatusItem::Efficiency => values.efficiency.map_or_else(
                || MISSING.to_owned(),
                |n| words.efficiency_text.fill(&[("n", &format.integer(u64::from(n.min(100))))]),
            ),
            StatusItem::Timing => values
                .timing
                .map_or_else(|| MISSING.to_owned(), |time| words.timing_text.fill(&[("n", &format.number(time.as_secs_f64(), 1))])),
            StatusItem::CurrentTool => given(&values.current_tool),
            StatusItem::LayerCount => {
                values.layer_count.map_or_else(|| MISSING.to_owned(), |n| words.layers.text(n as u64, &format.integer(n as u64)))
            }
        }
    }
}

/// What the readout reports.
#[non_exhaustive]
pub struct StatusReadoutOutput {
    /// The zoom field's edit, in percent.
    pub zoom: EditResponse<Option<f64>>,
    /// The item chosen in the menu this pass; the host stores it.
    pub item: Option<StatusItem>,
}

impl Outcome for StatusReadoutOutput {
    type Value = Option<f64>;

    fn response(&self) -> &Response {
        &self.zoom.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.zoom.story_facts()
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

    /// Shows the readout as children of the bar body `cx` belongs to.
    pub fn show_in(self, ui: &mut Ui, cx: &mut BodyCx<'_>) -> StatusReadoutOutput {
        let words = cx.words::<StatusReadoutWords>();
        let text = self.item.text(self.values, &words, cx.format());
        let zoom = crate::text::NumberField::new(words.zoom.clone(), self.zoom).unit_kind(UnitKind::Percent).range(self.range).show(ui);
        let mut info = Text::new(text).elide(Elide::End);
        if let Some((_, tip)) = self.tips.iter().find(|(item, _)| *item == self.item) {
            info = info.tooltip(tip.clone());
        }
        info.show(ui);
        let more = IconButton::new(IconId::new("chevron_right")).label(words.menu.clone()).show(ui);
        let entries: Vec<Entry<StatusItem>> =
            StatusItem::ALL.iter().map(|&item| Entry::new(item, item.name(&words)).checked(item == self.item)).collect();
        let menu = crate::menu::pick_from(ui, words.menu.clone(), &more, &entries);
        StatusReadoutOutput { zoom: zoom.edit, item: menu.picked }
    }
}
