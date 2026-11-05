//! The format options asked after the file name is chosen (File › Save, Save As, Save a Copy):
//! JPEG, layered TIFF, Targa and DDS. Each has OK and Cancel; Enter is OK unless Cancel has
//! focus; Cancel abandons the save (PhotoCraft's status reads "Save cancelled").

use egui::Context;
use serde::{Deserialize, Serialize};

use super::super::dialog::{Dialog, Outcome};
use super::super::dialog_host::{DialogResult, StackDialog};
use super::super::footer::Footer;
use super::super::size::Width;
use super::super::stack::{DialogId, DialogStack, dialog_id};
use super::super::state::DialogState;
use crate::composites::{FormGrid, SectionLabel};
use crate::controls::{Checkbox, NumberField, RadioGroup, Select, Slider};
use crate::edit::EditEvent;
use crate::localized::{LocaleNumbers, SharedLabels as _};

crate::words! {
    /// The words of the format option dialogs, English by default.
    #[non_exhaustive]
    pub struct FormatOptionsLabels {
        jpeg_title: "formatOptions.jpegTitle" = "JPEG Options";
        tiff_title: "formatOptions.tiffTitle" = "TIFF Options";
        targa_title: "formatOptions.targaTitle" = "Targa Options";
        dds_title: "formatOptions.ddsTitle" = "DDS Options";
        bc1: "formatOptions.bc1" = "BC1 / DXT1";
        bc2: "formatOptions.bc2" = "BC2 / DXT3";
        bc3: "formatOptions.bc3" = "BC3 / DXT5";
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
        preset: "formatOptions.preset" = "Preset";
    }
}

/// JPEG's encoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JpegFormat {
    #[default]
    /// Standard sequential JPEG.
    Baseline,
    /// Sequential JPEG with optimized Huffman tables (smaller file).
    BaselineOptimized,
    /// Progressive JPEG that fills in over several scans.
    Progressive,
}

/// What JPEG Options returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JpegOptions {
    /// Photoshop's scale, 0–12; [`jpeg_quality_percent`] gives the encoder's 1–100.
    pub quality: u8,
    /// The JPEG encoding.
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
    state: DialogState<JpegOptions>,
}

/// The highest value of Photoshop's JPEG quality scale.
const MAX_QUALITY: u8 = 12;

/// Photoshop's quality presets and the value each one sets.
const QUALITY_PRESETS: [u8; 4] = [3, 5, 8, 10];

/// The dialog's answer, for the pass an outcome closes it.
fn finished<T>(outcome: Option<Outcome>, value: T) -> Option<DialogResult<T>> {
    match outcome? {
        Outcome::Ok => Some(DialogResult::Ok(value)),
        Outcome::Cancel | Outcome::Close => Some(DialogResult::Cancel),
        _ => None,
    }
}

impl JpegOptionsDialog {
    /// The dialog at `quality` (0–12), PhotoCraft's Export preference by default.
    pub fn new(quality: u8) -> Self {
        Self {
            id: dialog_id("jpeg-options"),
            options: JpegOptions { quality: quality.min(MAX_QUALITY), format: JpegFormat::Baseline },
            state: DialogState::new(),
        }
    }

    /// The quality the document's JPEG was saved with, when known.
    pub fn own_quality(mut self, quality: Option<u8>) -> Self {
        if let Some(quality) = quality {
            self.options.quality = quality.min(MAX_QUALITY);
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
        let labels = FormatOptionsLabels::shared_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let gap = crate::theme::current(ctx).space.gap;
        let mut options = self.options;
        let shown = Dialog::new("jpeg-options", labels.jpeg_title.as_str()).stack(stack).footer(Footer::standard()).show(
            ctx,
            &mut self.state,
            &mut options,
            |ui, options| {
                let _ = SectionLabel::new(&labels.image_options).show(ui);
                FormGrid::new("jpeg-options").show(ui, |form| {
                    form.row(&labels.quality, |ui, _| {
                        let edit =
                            NumberField::new("quality", &labels.quality, options.quality, &numbers).range(0..=MAX_QUALITY).decimals(0..=0).show(ui);
                        apply_quality(edit.event.as_ref(), &mut options.quality);
                        let mut preset = QUALITY_PRESETS[preset_index(options.quality)];
                        let mut select = Select::new("quality-preset", &labels.preset, &mut preset);
                        for value in QUALITY_PRESETS {
                            select = select.option(value, jpeg_quality_label(value, &labels));
                        }
                        // The list shows the preset the quality falls in; only a pick changes it.
                        if select.show(ui).event.is_some() {
                            options.quality = preset;
                        }
                        edit.response
                    });
                    form.spanning(|ui| {
                        let edit = Slider::new("quality-slider", &labels.quality, f64::from(options.quality), 0.0..=f64::from(MAX_QUALITY))
                            .step(1.0)
                            .show(ui);
                        apply_quality(edit.event.as_ref(), &mut options.quality);
                        edit
                    });
                });
                ui.add_space(gap.stack);
                let _ = SectionLabel::new(&labels.format_options).show(ui);
                let _ = RadioGroup::new("jpeg-format", &labels.format_options, &mut options.format)
                    .item(JpegFormat::Baseline, &labels.baseline)
                    .item(JpegFormat::BaselineOptimized, &labels.baseline_optimized)
                    .item(JpegFormat::Progressive, &labels.progressive)
                    .show(ui);
            },
        );
        self.options = options;
        finished(shown.outcome, options)
    }
}

/// The label of a 0–12 quality: Low 0–4, Medium 5–7, High 8–9, Maximum 10–12.
// pc: crates/ui-egui/src/dialogs/jpeg_options_ui.rs:100 quality_label
pub fn jpeg_quality_label(quality: u8, labels: &FormatOptionsLabels) -> &str {
    match quality {
        0..=4 => &labels.low,
        5..=7 => &labels.medium,
        8..=9 => &labels.high,
        _ => &labels.maximum,
    }
}

/// The preset whose label [`jpeg_quality_label`] gives `quality`.
fn preset_index(quality: u8) -> usize {
    match quality {
        0..=4 => 0,
        5..=7 => 1,
        8..=9 => 2,
        _ => 3,
    }
}

/// Takes the value of a field or slider event as the quality.
fn apply_quality(event: Option<&EditEvent<f64>>, quality: &mut u8) {
    if let Some(value) = event.and_then(EditEvent::value) {
        *quality = value.round().clamp(0.0, f64::from(MAX_QUALITY)) as u8;
    }
}

/// The encoder's 1–100 quality for Photoshop's 0–12: `round(clamp(q, 0, 12) / 12 × 99 + 1)`.
// pc: crates/engine/src/cmd/file_cmds/save.rs:131 jpeg_quality
pub fn jpeg_quality_percent(quality: u8) -> u8 {
    (f64::from(quality.min(MAX_QUALITY)) / f64::from(MAX_QUALITY) * 99.0 + 1.0).round() as u8
}

/// What layered-TIFF Options returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TiffOptions {
    /// Save a flattened copy instead of the layers.
    pub discard_layers: bool,
    /// Don't ask again; the app clears its "ask before saving layered TIFF" preference.
    pub dont_show_again: bool,
}

/// TIFF Options, asked only for a layered TIFF while the app's "ask before saving layered TIFF"
/// preference is on: the note "Including layers will increase file size.", the "Discard Layers
/// and Save a Copy" box and "Don't show again". Result: [`TiffOptions`].
pub struct TiffOptionsDialog {
    id: DialogId,
    answer: TiffOptions,
    state: DialogState<TiffOptions>,
}

impl Default for TiffOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl TiffOptionsDialog {
    /// The dialog with layers kept and the "don't show again" box off.
    pub fn new() -> Self {
        Self { id: dialog_id("tiff-options"), answer: TiffOptions { discard_layers: false, dont_show_again: false }, state: DialogState::new() }
    }
}

impl StackDialog for TiffOptionsDialog {
    type Result = TiffOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/tiff_options_ui.rs:44
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<TiffOptions>> {
        let labels = FormatOptionsLabels::shared_ctx(ctx);
        let mut answer = self.answer;
        let shown = Dialog::new("tiff-options", labels.tiff_title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            &mut self.state,
            &mut answer,
            |ui, answer| {
                ui.label(&labels.layers_note);
                let _ = Checkbox::bool(&labels.discard_layers, &mut answer.discard_layers).show(ui);
                let _ = Checkbox::bool(&labels.dont_show, &mut answer.dont_show_again).show(ui);
            },
        );
        self.answer = answer;
        finished(shown.outcome, answer)
    }
}

/// What Targa Options returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargaOptions {
    /// Bits per pixel, 24 or 32.
    pub bits: u8,
}

/// Targa Options: Resolution as 24 or 32 bits/pixel radio buttons; not asked for grayscale.
/// Opens at `bits` (PhotoCraft: 32 when the image has alpha, else 24). Result: [`TargaOptions`].
pub struct TargaOptionsDialog {
    id: DialogId,
    bits: u8,
    state: DialogState<TargaOptions>,
}

impl TargaOptionsDialog {
    /// The dialog preset to `bits` per pixel: 32 when `bits` is 32, otherwise 24.
    pub fn new(bits: u8) -> Self {
        Self { id: dialog_id("targa-options"), bits: if bits == 32 { 32 } else { 24 }, state: DialogState::new() }
    }
}

impl StackDialog for TargaOptionsDialog {
    type Result = TargaOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/targa_options_ui.rs:58
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<TargaOptions>> {
        let labels = FormatOptionsLabels::shared_ctx(ctx);
        let mut options = TargaOptions { bits: self.bits };
        let shown = Dialog::new("targa-options", labels.targa_title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            &mut self.state,
            &mut options,
            |ui, options| {
                let _ = SectionLabel::new(&labels.resolution).show(ui);
                let _ =
                    RadioGroup::new("targa-bits", &labels.resolution, &mut options.bits).item(24, &labels.bits_24).item(32, &labels.bits_32).show(ui);
            },
        );
        self.bits = options.bits;
        finished(shown.outcome, options)
    }
}

/// DDS block compression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DdsCompression {
    #[default]
    /// No compression.
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
    /// The block compression.
    pub compression: DdsCompression,
    /// Generates mipmaps.
    pub mipmaps: bool,
}

/// DDS Options: Compression (None, BC1 / DXT1, BC2 / DXT3, BC3 / DXT5) and Generate Mipmaps;
/// opens at None and off. Result: [`DdsOptions`].
pub struct DdsOptionsDialog {
    id: DialogId,
    options: DdsOptions,
    state: DialogState<DdsOptions>,
}

impl Default for DdsOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl DdsOptionsDialog {
    /// The dialog with the default options (see [`DdsOptions::default`]).
    pub fn new() -> Self {
        Self { id: dialog_id("dds-options"), options: DdsOptions::default(), state: DialogState::new() }
    }
}

impl StackDialog for DdsOptionsDialog {
    type Result = DdsOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/dds_options_ui.rs:68
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DdsOptions>> {
        let labels = FormatOptionsLabels::shared_ctx(ctx);
        let gap = crate::theme::current(ctx).space.gap;
        let mut options = self.options;
        let shown = Dialog::new("dds-options", labels.dds_title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            &mut self.state,
            &mut options,
            |ui, options| {
                let _ = SectionLabel::new(&labels.compression).show(ui);
                let _ = RadioGroup::new("dds-compression", &labels.compression, &mut options.compression)
                    .item(DdsCompression::None, &labels.none)
                    .item(DdsCompression::Bc1, &labels.bc1)
                    .item(DdsCompression::Bc2, &labels.bc2)
                    .item(DdsCompression::Bc3, &labels.bc3)
                    .show(ui);
                ui.add_space(gap.stack);
                let _ = Checkbox::bool(&labels.mipmaps, &mut options.mipmaps).show(ui);
            },
        );
        self.options = options;
        finished(shown.outcome, options)
    }
}
