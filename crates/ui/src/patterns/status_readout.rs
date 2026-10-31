//! [`StatusReadout`]: Photoshop's document status readout in the status bar: the zoom field, one
//! chosen information item and the ▸ menu that chooses it.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Id, Ui};

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
/// - **Item:** the chosen item's text; sizes formatted with the installed number format
///   (craft-i18n's file sizes).
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
        todo!()
    }

    pub fn labels(mut self, labels: StatusReadoutLabels) -> Self {
        todo!()
    }

    /// Draws the zoom field (`zoom` in percent), the item's text and the ▸ menu.
    // pc: crates/ui-egui/src/panels/status_bar.rs:11 `status_bar`, crates/ui-egui/src/panels/chrome_ui.rs:143
    // `status_bar_pro` (zoom field, text, ▸ popup), chrome_ui.rs:45 `STATUS_INFO`. PhotoCraft's
    // Efficiency is always "100%" and it has no Timing; both are new here.
    pub fn show(self, ui: &mut Ui, zoom: f32, item: StatusItem, values: &StatusValues) -> StatusReadoutOutput {
        todo!()
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

    /// The item's text for `values` ("Doc: 24.9M/24.9M", "—" when the value is missing).
    // pc: crates/ui-egui/src/panels/chrome_ui.rs:89 `status_info_text_with_units`;
    // crates/ui-egui/src/panels/status_bar.rs:69 `document_sizes` (flat = w × h × channels ×
    // bytes per channel; layered = raster tile bounds ∩ canvas × (channels + 1) × bytes per channel).
    pub fn text(self, values: &StatusValues, labels: &StatusReadoutLabels) -> String {
        todo!()
    }
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
    }
}
