//! The format options asked after the file name is chosen (File › Save, Save As, Save a Copy):
//! JPEG, layered TIFF, Targa and DDS. Each has OK and Cancel; Enter is OK unless Cancel has
//! focus; Cancel abandons the save (PhotoCraft's status reads "Save cancelled").

use egui::Context;
use serde::{Deserialize, Serialize};

use super::super::dialog_host::{DialogResult, StackDialog};
use super::super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The words of the format option dialogs, English by default.
    #[non_exhaustive]
    pub struct FormatOptionsLabels {
        jpeg_title: "formatOptions.jpegTitle" = "JPEG Options";
        tiff_title: "formatOptions.tiffTitle" = "TIFF Options";
        targa_title: "formatOptions.targaTitle" = "Targa Options";
        dds_title: "formatOptions.ddsTitle" = "DDS Options";
        image_options: "formatOptions.imageOptions" = "Image Options";
        quality: "formatOptions.quality" = "Quality";
        low: "formatOptions.low" = "Low";
        medium: "formatOptions.medium" = "Medium";
        high: "formatOptions.high" = "High";
        maximum: "formatOptions.maximum" = "Maximum";
        format_options: "formatOptions.formatOptions" = "Format Options";
        baseline: "formatOptions.baseline" = "Baseline (\"Standard\")";
        baseline_optimized: "formatOptions.baselineOptimized" = "Baseline Optimized";
        progressive: "formatOptions.progressive" = "Progressive";
        discard_layers: "formatOptions.discardLayers" = "Discard Layers and Save a Copy";
        layers_note: "formatOptions.layersNote" = "Including layers will increase file size.";
        dont_show: "formatOptions.dontShow" = "Don't show again";
        resolution: "formatOptions.resolution" = "Resolution";
        bits_16: "formatOptions.bits16" = "16 bits/pixel";
        bits_24: "formatOptions.bits24" = "24 bits/pixel";
        bits_32: "formatOptions.bits32" = "32 bits/pixel";
        compression: "formatOptions.compression" = "Compression";
        none: "formatOptions.none" = "None";
        mipmaps: "formatOptions.mipmaps" = "Generate Mipmaps";
    }
}

/// JPEG's encoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JpegFormat {
    #[default]
    Baseline,
    BaselineOptimized,
    Progressive,
}

/// What JPEG Options returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JpegOptions {
    /// Photoshop's scale, 0–12; [`jpeg_quality_percent`] gives the encoder's 1–100.
    pub quality: u8,
    pub format: JpegFormat,
}

/// JPEG Options: the Quality field 0–12 with its slider and the preset list (Low 3, Medium 5,
/// High 8, Maximum 10; the list shows the label of any value, [`jpeg_quality_label`]), then
/// Format Options (Baseline ("Standard"), Baseline Optimized, Progressive). Scans and the size
/// readout of Photoshop are not offered. Opens at the file's own quality when the document came
/// from a JPEG ([`own_quality`](Self::own_quality)), else at `quality`. Result: [`JpegOptions`].
pub struct JpegOptionsDialog {
    id: DialogId,
    options: JpegOptions,
}

impl JpegOptionsDialog {
    /// The dialog at `quality` (0–12), PhotoCraft's Export preference by default.
    pub fn new(quality: u8) -> Self {
        Self { id: dialog_id("jpeg-options"), options: JpegOptions { quality, format: JpegFormat::Baseline } }
    }

    /// The quality the document's JPEG was saved with, when known.
    pub fn own_quality(mut self, quality: Option<u8>) -> Self {
        if let Some(quality) = quality {
            self.options.quality = quality;
        }
        self
    }
}

impl StackDialog for JpegOptionsDialog {
    type Result = JpegOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/jpeg_options_ui.rs:195 show, :246 confirm
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<JpegOptions>> {
        todo!()
    }
}

/// The label of a 0–12 quality: Low 0–4, Medium 5–7, High 8–9, Maximum 10–12.
// pc: crates/ui-egui/src/dialogs/jpeg_options_ui.rs:100 quality_label
pub fn jpeg_quality_label(quality: u8, labels: &FormatOptionsLabels) -> &str {
    todo!()
}

/// The encoder's 1–100 quality for Photoshop's 0–12: `round(clamp(q, 0, 12) / 12 × 99 + 1)`.
// pc: crates/engine/src/cmd/file_cmds/save.rs:131 jpeg_quality
pub fn jpeg_quality_percent(quality: u8) -> u8 {
    todo!()
}

/// What layered-TIFF Options returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TiffAnswer {
    /// Save a flattened copy instead of the layers.
    pub discard_layers: bool,
    /// Don't ask again; the app clears its "ask before saving layered TIFF" preference.
    pub dont_show_again: bool,
}

/// TIFF Options, asked only for a layered TIFF while the app's "ask before saving layered TIFF"
/// preference is on: the note "Including layers will increase file size.", the "Discard Layers
/// and Save a Copy" box and "Don't show again". Result: [`TiffAnswer`].
pub struct TiffOptionsDialog {
    id: DialogId,
    answer: TiffAnswer,
}

impl Default for TiffOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl TiffOptionsDialog {
    pub fn new() -> Self {
        Self { id: dialog_id("tiff-options"), answer: TiffAnswer { discard_layers: false, dont_show_again: false } }
    }
}

impl StackDialog for TiffOptionsDialog {
    type Result = TiffAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/tiff_options_ui.rs:44
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<TiffAnswer>> {
        todo!()
    }
}

/// Targa Options: Resolution as 24 or 32 bits/pixel radio buttons; not asked for grayscale.
/// Opens at `bits` (PhotoCraft: 32 when the image has alpha, else 24). Result: the bits, 24 or 32.
pub struct TargaOptionsDialog {
    id: DialogId,
    bits: u8,
}

impl TargaOptionsDialog {
    pub fn new(bits: u8) -> Self {
        Self { id: dialog_id("targa-options"), bits }
    }
}

impl StackDialog for TargaOptionsDialog {
    type Result = u8;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/targa_options_ui.rs:58
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<u8>> {
        todo!()
    }
}

/// DDS block compression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DdsCompression {
    #[default]
    None,
    /// BC1 / DXT1.
    Bc1,
    /// BC2 / DXT3.
    Bc2,
    /// BC3 / DXT5.
    Bc3,
}

/// What DDS Options returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DdsOptions {
    pub compression: DdsCompression,
    pub mipmaps: bool,
}

/// DDS Options: Compression (None, BC1 / DXT1, BC2 / DXT3, BC3 / DXT5) and Generate Mipmaps;
/// opens at None and off. Result: [`DdsOptions`].
pub struct DdsOptionsDialog {
    id: DialogId,
    options: DdsOptions,
}

impl Default for DdsOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl DdsOptionsDialog {
    pub fn new() -> Self {
        Self { id: dialog_id("dds-options"), options: DdsOptions::default() }
    }
}

impl StackDialog for DdsOptionsDialog {
    type Result = DdsOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/dds_options_ui.rs:68
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DdsOptions>> {
        todo!()
    }
}
