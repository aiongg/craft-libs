//! [`StatusReadout`]: Photoshop's document status readout in the status bar: the zoom field, one
//! chosen information item and the ▸ menu that chooses it.

use craft_theme::ControlSize;
use egui::{AsIdSalt, Id, Ui};

use super::StatusBar;
use crate::controls::NumberField;
use crate::edit::EditEvent;
use crate::localized::{Labels as _, LocaleNumbers, Unit, Words};
use crate::menu::{MenuEntry, MenuItem, PressMenu};

/// Shown for an item whose value the host does not know.
const MISSING: &str = "—";

/// The item the readout shows, chosen in its ▸ menu (Photoshop's list).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatusItem {
    /// "Doc: 24.9M/24.9M": the flattened size and the size with layers.
    #[default]
    DocumentSizes,
    DocumentProfile,
    /// "4000 px × 3000 px (300 ppi)".
    DocumentDimensions,
    MeasurementScale,
    /// "Scratch: 1.2G/8.0G": memory the app uses for open documents and what it may use.
    ScratchSizes,
    /// The share of time spent working rather than waiting on scratch, "Efficiency: 100%".
    Efficiency,
    /// How long the last operation took, "Timing: 0.4 s".
    Timing,
    CurrentTool,
    LayerCount,
}

/// The values the host knows this frame; an item without a value shows "—".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusValues {
    /// Flattened and layered sizes in bytes.
    pub document_sizes: Option<(u64, u64)>,
    pub document_profile: Option<String>,
    pub document_dimensions: Option<String>,
    pub measurement_scale: Option<String>,
    /// Used and available bytes (craft-platform's `usage` module samples the process).
    pub scratch_sizes: Option<(u64, u64)>,
    /// 0–100.
    pub efficiency: Option<u8>,
    pub timing: Option<std::time::Duration>,
    pub current_tool: Option<String>,
    pub layer_count: Option<usize>,
}

/// What [`StatusReadout::show`] returns.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct StatusReadoutOutput {
    /// The zoom percentage typed and committed in the zoom field.
    pub zoom: Option<f32>,
    /// The item chosen in the ▸ menu; the host stores it.
    pub item: Option<StatusItem>,
}

/// The readout, drawn in a [`super::StatusBar`] slot.
///
/// ```no_run
/// use craft_ui::patterns::{StatusBar, StatusItem, StatusReadout, StatusValues};
/// # fn demo(ui: &mut egui::Ui, zoom: f32, item: &mut StatusItem, values: &StatusValues) {
/// StatusBar::new("status").show(ui, |slots| {
///     slots.leading(|ui| {
///         let out = StatusReadout::new("readout").show(ui, zoom, *item, values);
///         if let Some(i) = out.item { *item = i; }
///     });
/// });
/// # }
/// ```
///
/// - **Zoom field:** the zoom as "66.67%", a number field (Enter commits, Esc reverts; the value
///   is clamped by the host's zoom ladder). Width from the status bar's field token.
/// - **Item:** the chosen item's text; sizes in Photoshop's compact form ("10.3M", "412.5K",
///   "1.2G", steps of 1024, one decimal) written with the installed number format.
/// - **▸ menu:** a menu button listing every [`StatusItem`] as radio rows, the shown one chosen.
/// - **Keyboard:** the field and the menu button are Tab stops; the menu is a library
///   [`crate::menu::Menu`].
#[must_use = "show the readout with `show`"]
pub struct StatusReadout {
    id: Id,
    labels: Option<StatusReadoutLabels>,
}

impl StatusReadout {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id: Id::unique("craft_ui::patterns::StatusReadout").with(id_salt), labels: None }
    }

    pub fn labels(mut self, labels: StatusReadoutLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the zoom field (`zoom` in percent), the item's text and the ▸ menu.
    // pc: crates/ui-egui/src/panels/status_bar.rs:11 `status_bar`, crates/ui-egui/src/panels/chrome_ui.rs:143
    // `status_bar_pro` (zoom field, text, ▸ popup), chrome_ui.rs:45 `STATUS_INFO`. PhotoCraft's
    // Efficiency is always "100%" and it has no Timing; both are new here.
    pub fn show(self, ui: &mut Ui, zoom: f32, item: StatusItem, values: &StatusValues) -> StatusReadoutOutput {
        let labels = self.labels.unwrap_or_else(|| StatusReadoutLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let words = Words::of(ui);
        let mut out = StatusReadoutOutput::default();
        StatusBar::field(ui, |ui| {
            let edit = NumberField::new(self.id.with("zoom"), labels.zoom.as_str(), zoom, &numbers)
                .range(0.01..=100_000.0)
                .decimals(0..=2)
                .unit(Unit::new("{n}%"))
                .scrub_wrap(true)
                .size(ControlSize::Sm)
                .show(ui);
            if let Some(EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. }) = edit.event {
                out.zoom = Some(value as f32);
            }
        });
        StatusBar::info(ui, &item.text(values, &labels, &numbers, &words));
        let button = StatusBar::menu_button(ui, &labels.menu);
        let entries: Vec<MenuEntry<StatusItem>> =
            StatusItem::ALL.iter().map(|&i| MenuItem::new(i.label(&labels), i).radio(i == item).into()).collect();
        out.item = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen;
        out
    }
}

impl StatusItem {
    /// Every item in the ▸ menu's order.
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

    /// The item's text for `values` ("Doc: 24.9M/24.9M", "—" when the value is missing), numbers
    /// in `numbers`' digits and the layer count in `words`' plural forms.
    // pc: crates/ui-egui/src/panels/chrome_ui.rs:89 `status_info_text_with_units`;
    // crates/ui-egui/src/panels/status_bar.rs:69 `document_sizes` (flat = w × h × channels ×
    // bytes per channel; layered = raster tile bounds ∩ canvas × (channels + 1) × bytes per channel).
    pub fn text(self, values: &StatusValues, labels: &StatusReadoutLabels, numbers: &LocaleNumbers, words: &Words) -> String {
        let text = |value: &Option<String>| value.clone().unwrap_or_else(|| MISSING.to_owned());
        let size = |bytes: u64| compact_size(bytes, labels, numbers);
        match self {
            StatusItem::DocumentSizes => values.document_sizes.map_or_else(
                || MISSING.to_owned(),
                |(flat, layered)| craft_i18n::fmt(&labels.doc_prefix, &[("flat", &size(flat)), ("layered", &size(layered))]),
            ),
            StatusItem::DocumentProfile => text(&values.document_profile),
            StatusItem::DocumentDimensions => text(&values.document_dimensions),
            StatusItem::MeasurementScale => text(&values.measurement_scale),
            StatusItem::ScratchSizes => values.scratch_sizes.map_or_else(
                || MISSING.to_owned(),
                |(used, available)| craft_i18n::fmt(&labels.scratch_prefix, &[("used", &size(used)), ("available", &size(available))]),
            ),
            StatusItem::Efficiency => values.efficiency.map_or_else(
                || MISSING.to_owned(),
                |n| craft_i18n::fmt(&labels.efficiency_text, &[("n", &numbers.format(f64::from(n.min(100)), 0..=0))]),
            ),
            StatusItem::Timing => values
                .timing
                .map_or_else(|| MISSING.to_owned(), |t| craft_i18n::fmt(&labels.timing_text, &[("n", &numbers.format(t.as_secs_f64(), 1..=1))])),
            StatusItem::CurrentTool => text(&values.current_tool),
            StatusItem::LayerCount => {
                values.layer_count.map_or_else(|| MISSING.to_owned(), |n| words.count("statusReadout.layers", n as u64, "{n} Layer", "{n} Layers"))
            }
        }
    }

    /// The item's name in the ▸ menu.
    fn label(self, labels: &StatusReadoutLabels) -> &str {
        match self {
            StatusItem::DocumentSizes => &labels.document_sizes,
            StatusItem::DocumentProfile => &labels.document_profile,
            StatusItem::DocumentDimensions => &labels.document_dimensions,
            StatusItem::MeasurementScale => &labels.measurement_scale,
            StatusItem::ScratchSizes => &labels.scratch_sizes,
            StatusItem::Efficiency => &labels.efficiency,
            StatusItem::Timing => &labels.timing,
            StatusItem::CurrentTool => &labels.current_tool,
            StatusItem::LayerCount => &labels.layer_count,
        }
    }
}

/// Photoshop's compact byte format with one decimal: "10.3M", "412.5K", "1.2G".
fn compact_size(bytes: u64, labels: &StatusReadoutLabels, numbers: &LocaleNumbers) -> String {
    const KIB: f64 = 1024.0;
    let bytes = bytes as f64;
    let (value, template) = if bytes >= KIB * KIB * KIB {
        (bytes / (KIB * KIB * KIB), &labels.size_giga)
    } else if bytes >= KIB * KIB {
        (bytes / (KIB * KIB), &labels.size_mega)
    } else {
        (bytes / KIB, &labels.size_kilo)
    };
    craft_i18n::fmt(template, &[("n", &numbers.format(value, 1..=1))])
}

crate::words! {
    /// The readout's labels, English by default.
    #[non_exhaustive]
    pub struct StatusReadoutLabels {
        zoom: "statusReadout.zoom" = "Zoom";
        menu: "statusReadout.menu" = "Show";
        document_sizes: "statusReadout.documentSizes" = "Document Sizes";
        document_profile: "statusReadout.documentProfile" = "Document Profile";
        document_dimensions: "statusReadout.documentDimensions" = "Document Dimensions";
        measurement_scale: "statusReadout.measurementScale" = "Measurement Scale";
        scratch_sizes: "statusReadout.scratchSizes" = "Scratch Sizes";
        efficiency: "statusReadout.efficiency" = "Efficiency";
        timing: "statusReadout.timing" = "Timing";
        current_tool: "statusReadout.currentTool" = "Current Tool";
        layer_count: "statusReadout.layerCount" = "Layer Count";
        doc_prefix: "statusReadout.docPrefix" = "Doc: {flat}/{layered}";
        scratch_prefix: "statusReadout.scratchPrefix" = "Scratch: {used}/{available}";
        efficiency_text: "statusReadout.efficiencyText" = "Efficiency: {n}%";
        timing_text: "statusReadout.timingText" = "Timing: {n} s";
        size_kilo: "statusReadout.sizeKilo" = "{n}K";
        size_mega: "statusReadout.sizeMega" = "{n}M";
        size_giga: "statusReadout.sizeGiga" = "{n}G";
    }
}
