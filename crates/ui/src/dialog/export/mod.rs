//! Export dialogs. They take plain format descriptions and settings from the app and give
//! settings and a path back; encoding stays in the app.
//!
//! - [`ExportAsDialog`]: File › Export › Export As: format, size, quality, metadata, colour
//!   profile, a preview and a size estimate the app computes.
//! - [`SaveForWebDialog`]: Save for Web (Legacy): Original, Optimized, 2-Up and 4-Up views of
//!   encoded variants with size and download time.
//! - [`JpegOptionsDialog`], [`TiffOptionsDialog`], [`TargaOptionsDialog`], [`DdsOptionsDialog`]:
//!   the format options asked after the file name is chosen.
//! - [`WatermarkEditor`]: the watermark group of an export dialog.
//! - [`OutputPathRow`]: an output template list with the chosen file, for render queues.
//! - [`export_file_name`], [`numbered_paths`], [`resize_from_scale`], [`resize_from_side`]: the pure
//!   rules the dialogs share.

#[cfg(test)]
mod export_tests;
mod options;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod options_tests;
mod path_row;
mod watermark;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod watermark_tests;
mod web;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod web_tests;

use std::path::{Path, PathBuf};

use craft_host::{Host, PickRequest};
use craft_settings::Remembered;
use craft_theme::{Rgba, TypeRole};
use egui::text::LayoutJob;
use egui::{Context, Label, Rect, Sense, TextureHandle, Ui, vec2};
use serde::{Deserialize, Serialize};

use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::Width;
use super::sizing::{Resample, SizingLabels, human_bytes};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{FormGrid, SectionLabel};
use crate::controls::{Checkbox, NumberField, Select, Slider, Spinner};
use crate::edit::EditEvent;
use crate::localized::{LocaleNumbers, SharedLabels as _, Unit};
use crate::primitives::Content;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};

pub use options::{
    DdsCompression, DdsOptions, DdsOptionsDialog, FormatOptionsLabels, JpegFormat, JpegOptions, JpegOptionsDialog, TargaOptionsDialog, TiffAnswer,
    TiffOptionsDialog, jpeg_quality_label, jpeg_quality_percent,
};
pub use path_row::{OutputPathLabels, OutputPathResponse, OutputPathRow};
pub use watermark::{Watermark, WatermarkEditor, WatermarkLabels, WatermarkStyle};
pub use web::{
    ColorTable, Dither, MATTE_COLOR_KEY, SaveForWebDialog, SaveForWebResult, WebFormat, WebLabels, WebMetadata, WebSettings, WebVariant, WebView,
    download_seconds, four_up_variants,
};

crate::words! {
    /// The words of Export As, English by default.
    #[non_exhaustive]
    pub struct ExportLabels {
        title: "export.title" = "Export As";
        export: "export.export" = "Export…";
        format: "export.format" = "Format";
        quality: "export.quality" = "Quality";
        lossless: "export.lossless" = "Lossless";
        transparency: "export.transparency" = "Transparency";
        image_size: "export.imageSize" = "Image Size";
        scale: "export.scale" = "Scale";
        width: "export.width" = "Width";
        height: "export.height" = "Height";
        resample: "export.resample" = "Resample";
        metadata: "export.metadata" = "Metadata";
        metadata_none: "export.metadataNone" = "None";
        metadata_exif: "export.metadataExif" = "EXIF";
        metadata_all: "export.metadataAll" = "All";
        color_space: "export.colorSpace" = "Color Space";
        document_profile: "export.documentProfile" = "Document Profile";
        convert_srgb: "export.convertSrgb" = "Convert to sRGB";
        embed_profile: "export.embedProfile" = "Embed Color Profile";
        bits: "export.bits" = "Bits/Pixel";
        auto: "export.auto" = "Auto";
        /// `{size}`: the estimated file size ("1.2 MB"); shown while the exact one is computed.
        estimate: "export.estimate" = "≈ {size}";
        /// `{size}`: the exact file size.
        exact: "export.exact" = "{size}";
        ico_limit: "export.icoLimit" = "ICO is limited to 256 × 256 pixels";
        scale_range: "export.scaleRange" = "Scale must be within 1–1000 percent";
        pixel_range: "export.pixelRange" = "Export size must be within 1–300,000 pixels";
        /// The size shown before any estimate is in.
        no_estimate: "export.noEstimate" = "—";
        /// The file name suggested for a document with none.
        untitled: "export.untitled" = "Untitled";
    }
}

/// A format the dialog offers, described by the app (export dialogs never encode).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportFormat {
    /// Stable id, also the settings value: `"png"`.
    pub id: &'static str,
    /// The list's label: "PNG".
    pub name: &'static str,
    /// The extension a picked file gets, without the dot.
    pub extension: &'static str,
    /// Shows Quality (JPG, lossy WebP).
    pub quality: bool,
    /// Shows Lossless (WebP); Quality hides while it is checked.
    pub lossless: bool,
    /// Shows Transparency.
    pub transparency: bool,
    /// Shows Bits/Pixel (TGA: Auto, 24, 32).
    pub bits: &'static [u8],
    /// The largest side the format can hold (ICO: 256); the dialog shows the limit under the size.
    pub max_side: Option<u32>,
}

impl ExportFormat {
    /// PhotoCraft's list: PNG, JPG, WebP, TIFF, TGA, ICO.
    // pc: crates/ui-egui/src/dialogs/export_dialog.rs:18 FORMATS
    pub fn standard() -> Vec<ExportFormat> {
        let format = |id, name, quality, lossless, transparency, bits: &'static [u8], max_side| ExportFormat {
            id,
            name,
            extension: id,
            quality,
            lossless,
            transparency,
            bits,
            max_side,
        };
        vec![
            format("png", "PNG", false, false, true, &[], None),
            format("jpg", "JPG", true, false, false, &[], None),
            format("webp", "WebP", true, true, true, &[], None),
            format("tif", "TIFF", false, false, true, &[], None),
            format("tga", "TGA", false, false, true, &[24, 32], None),
            format("ico", "ICO", false, false, true, &[], Some(256)),
        ]
    }
}

/// How the output size is given.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExportSize {
    /// Percent of the document, 1–1000.
    Scale(f64),
    /// Pixel width and height (with the aspect lock off, or any size the lock allows).
    Pixels { width: u32, height: u32 },
}

/// What metadata goes into the file. `None` also drops the resolution (PhotoCraft).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportMetadata {
    #[default]
    None,
    Exif,
    All,
}

/// The colour profile of the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportProfile {
    #[default]
    Document,
    Srgb,
}

/// Export As's values: the dialog's result and what it remembers between opens
/// (`"export.exportAs"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportSettings {
    /// An [`ExportFormat::id`].
    pub format: String,
    /// 1–100; PhotoCraft's default 85 comes from its Export preference.
    pub quality: u8,
    pub lossless: bool,
    pub transparency: bool,
    pub size: ExportSize,
    pub lock_aspect: bool,
    pub resample: Resample,
    pub metadata: ExportMetadata,
    pub profile: ExportProfile,
    pub embed_profile: bool,
    /// 0 is Auto.
    pub bits: u8,
}

impl Default for ExportSettings {
    /// PNG at 100 %, aspect locked, quality 85, document profile embedded, no metadata.
    fn default() -> Self {
        Self {
            format: "png".to_owned(),
            quality: 85,
            lossless: false,
            transparency: true,
            size: ExportSize::Scale(100.0),
            lock_aspect: true,
            resample: Resample::Bicubic,
            metadata: ExportMetadata::None,
            profile: ExportProfile::Document,
            embed_profile: true,
            bits: 0,
        }
    }
}

/// The file size line under the preview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeEstimate {
    /// Scaled from an encode of the downscaled proxy: shown with "≈".
    Proxy(u64),
    /// From an encode of the full image.
    Exact(u64),
}

/// The quality and WebP mode a format takes when the user switches to it, from the app's Export
/// preferences (Quick Export shows the same values). Qualities are 1–100, as in the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportFormatDefaults {
    /// Quality when JPG is chosen.
    pub jpeg_quality: u8,
    /// Quality when WebP is chosen.
    pub webp_quality: u8,
    /// Lossless when WebP is chosen.
    pub webp_lossless: bool,
}

impl Default for ExportFormatDefaults {
    /// PhotoCraft's Export preferences: 85, 85, lossless WebP.
    fn default() -> Self {
        Self { jpeg_quality: 85, webp_quality: 85, webp_lossless: true }
    }
}

/// The preview and estimate the app computes for one revision of the settings.
pub struct ExportPreview {
    /// The revision of [`ExportAsDialog::wanted_preview`] this answers; older answers are ignored.
    pub revision: u64,
    pub texture: TextureHandle,
    pub estimate: SizeEstimate,
}

/// What Export As returns: the settings and the path the user chose.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportAsResult {
    pub settings: ExportSettings,
    pub path: PathBuf,
    /// The output size in pixels.
    pub width: u32,
    pub height: u32,
}

/// File › Export › Export As (spec Q-S4F-4).
///
/// The user sees the settings on the left: Format (the app's [`ExportFormat`] list), Quality slider 1–100 (JPG, lossy WebP),
/// Lossless (WebP), Transparency, Bits/Pixel (TGA), Image Size (Scale percent, or Width and
/// Height with the aspect lock, then Resample), Metadata (None, EXIF, All), Color Space
/// (Document Profile, Convert to sRGB) with Embed Color Profile. Buttons: Export… and Cancel;
/// Enter is Export…, Esc cancels. The preview is on the right, over a checkerboard, with the
/// format and size line under it ("PNG  ≈ 1.2M", then "PNG  1.2M" once exact). The app loads the
/// preview texture with `TextureOptions::NEAREST` when Resample is Nearest Neighbor.
///
/// Export… queues a save picker through `host.files()` (purpose `"export"`, the format's
/// filter and extension, the name from [`file_name`](Self::file_name)); the dialog stays open
/// until the answer: a path finishes it with [`ExportAsResult`], a cancelled pick keeps it open.
///
/// Previews: whenever the settings change, [`wanted_preview`](Self::wanted_preview) returns the
/// settings and a new revision; the app encodes a proxy (long side 512, ICO 256) on a job and
/// hands the result back with [`set_preview`](Self::set_preview) through
/// `dialogs.get_mut::<ExportAsDialog>()`. A proxy estimate shows at once ("≈ 1.2 MB"); the app
/// may follow with an exact one (PhotoCraft encodes the full image when it has at most
/// 40,000,000 px). The dialog opens with the last settings.
pub struct ExportAsDialog {
    id: DialogId,
    formats: Vec<ExportFormat>,
    source: (u32, u32),
    file_name: String,
    settings: ExportSettings,
    revision: u64,
    /// The revision the app last took with `wanted_preview`.
    taken: u64,
    preview: Option<ExportPreview>,
    picking: Option<craft_host::PickId>,
    recalled: bool,
    /// The size edit the last pass refused, shown under the size fields.
    size_error: Option<SizeError>,
    defaults: ExportFormatDefaults,
    state: DialogState<()>,
}

impl ExportAsDialog {
    /// The dialog for a document of `width` × `height` pixels named `file_name` (no extension).
    pub fn new(width: u32, height: u32, file_name: impl Into<String>) -> Self {
        Self {
            id: dialog_id("export-as"),
            formats: ExportFormat::standard(),
            source: (width, height),
            file_name: file_name.into(),
            settings: ExportSettings::default(),
            revision: 1,
            taken: 0,
            preview: None,
            picking: None,
            recalled: false,
            size_error: None,
            defaults: ExportFormatDefaults::default(),
            state: DialogState::new(),
        }
    }

    /// Replaces [`ExportFormat::standard`].
    pub fn formats(mut self, formats: Vec<ExportFormat>) -> Self {
        self.formats = formats;
        self.fit();
        self
    }

    /// The quality and WebP mode applied when the user switches to JPG or WebP (the app's Export
    /// preferences), and the ones the dialog opens with unless it recalls or is given settings.
    pub fn format_defaults(mut self, defaults: ExportFormatDefaults) -> Self {
        self.defaults = defaults;
        if !self.recalled {
            self.settings.quality = defaults.jpeg_quality.clamp(1, 100);
            self.settings.lossless = defaults.webp_lossless;
        }
        self
    }

    /// Opens with these settings instead of the remembered ones (a command run with arguments).
    pub fn settings(mut self, settings: ExportSettings) -> Self {
        self.settings = settings;
        self.recalled = true;
        self.fit();
        self
    }

    /// The file name the picker suggests: [`export_file_name`] of the document name plus the
    /// format's extension.
    pub fn file_name(&self, labels: &ExportLabels) -> String {
        let extension = self.format().map_or("png", |format| format.extension);
        format!("{}.{extension}", export_file_name(&self.file_name, &labels.untitled))
    }

    /// The settings and their revision when they changed since the last call; `None` otherwise.
    pub fn wanted_preview(&mut self) -> Option<(u64, ExportSettings)> {
        if self.revision == self.taken {
            return None;
        }
        self.taken = self.revision;
        Some((self.revision, self.settings.clone()))
    }

    /// The app's preview for a revision; one older than the newest answer is dropped.
    pub fn set_preview(&mut self, preview: ExportPreview) {
        let newest = self.preview.as_ref().map_or(0, |current| current.revision);
        if preview.revision < self.taken || preview.revision < newest {
            return;
        }
        self.preview = Some(preview);
    }

    /// Replaces the estimate of the current revision with the exact size.
    pub fn set_exact_estimate(&mut self, revision: u64, bytes: u64) {
        if let Some(preview) = self.preview.as_mut().filter(|preview| preview.revision == revision && revision >= self.taken) {
            preview.estimate = SizeEstimate::Exact(bytes);
        }
    }

    fn format(&self) -> Option<&ExportFormat> {
        self.formats.iter().find(|format| format.id == self.settings.format)
    }

    /// The output size in pixels.
    fn output_size(&self) -> (u32, u32) {
        match self.settings.size {
            ExportSize::Scale(scale) => resize_from_scale(self.source, scale).unwrap_or(self.source),
            ExportSize::Pixels { width, height } => (width, height),
        }
    }

    /// Makes the settings something the controls can hold for this document: a format the list
    /// has, a quality of 1–100, and a size the document can take. A linked pixel width keeps its
    /// width and takes the height from this document (the last export's proportions would
    /// stretch another shape); unlinked pixels stay as asked; anything out of range becomes 100 %.
    // pc: crates/ui-egui/src/dialogs/export_dialog.rs:100 seed_remembered, :150 fit_size
    fn fit(&mut self) {
        if let Some(first) = self.formats.first()
            && self.format().is_none()
        {
            self.settings.format = first.id.to_owned();
        }
        self.settings.quality = self.settings.quality.clamp(1, 100);
        let source = self.source;
        let pixels_ok = |width: u32, height: u32| (1..=MAX_PIXELS as u32).contains(&width) && (1..=MAX_PIXELS as u32).contains(&height);
        self.settings.size = match self.settings.size {
            ExportSize::Pixels { width, height } if !self.settings.lock_aspect && pixels_ok(width, height) => self.settings.size,
            ExportSize::Pixels { width, .. } => match resize_from_side(source, f64::from(width), 0.0, true, true) {
                Ok(((width, height), scale)) if (1.0..=1000.0).contains(&scale) => ExportSize::Pixels { width, height },
                _ => ExportSize::Scale(100.0),
            },
            ExportSize::Scale(scale) if resize_from_scale(source, scale).is_ok() => ExportSize::Scale(scale),
            ExportSize::Scale(_) => ExportSize::Scale(100.0),
        };
    }

    /// Loads what the last Export As remembered, once.
    fn recall(&mut self, ctx: &Context) {
        if std::mem::replace(&mut self.recalled, true) {
            return;
        }
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let Some(last) = Remembered::<ExportSettings>::new(REMEMBER_KEY).recall(&host) else { return };
        self.settings = last;
        self.fit();
        self.revision += 1;
    }

    /// The picker's answer, once it has come: the first path.
    fn picked(&mut self, ctx: &Context) -> Option<PathBuf> {
        let ticket = self.picking?;
        let host = Host::try_of_ctx(ctx)?;
        let picked = host.files().take_answer(ticket)?;
        self.picking = None;
        picked.first_path()
    }

    /// Drops the outstanding pick when the dialog closes without waiting for its answer.
    fn forget_pick(&mut self, ctx: &Context) {
        if let Some(ticket) = self.picking.take()
            && let Some(host) = Host::try_of_ctx(ctx)
        {
            host.files().forget(ticket);
        }
    }

    /// Queues the save picker for the current format.
    // pc: crates/ui-egui/src/dialogs/export_dialog.rs:1038 confirm
    fn queue_save(&mut self, ctx: &Context, labels: &ExportLabels) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let Some(format) = self.format() else { return };
        let request = PickRequest::save("export")
            .title(labels.title.as_str())
            .file_name(self.file_name(labels))
            .suggested_extension(format.extension)
            .filter(format.name, &[format.extension]);
        self.picking = Some(host.files().pick(request));
    }

    /// The size line under the preview: the format, then "≈ 1.2M" until the exact size is in.
    fn size_line(&self, labels: &ExportLabels) -> String {
        let name = self.format().map_or("", |format| format.name).to_uppercase();
        match self.preview.as_ref().map(|preview| preview.estimate) {
            Some(SizeEstimate::Proxy(bytes)) => format!("{name}  {}", craft_i18n::fmt(&labels.estimate, &[("size", &human_bytes(bytes as f64))])),
            Some(SizeEstimate::Exact(bytes)) => format!("{name}  {}", craft_i18n::fmt(&labels.exact, &[("size", &human_bytes(bytes as f64))])),
            None => format!("{name}  {}", craft_i18n::fmt(&labels.estimate, &[("size", labels.no_estimate.as_str())])),
        }
    }
}

impl StackDialog for ExportAsDialog {
    type Result = ExportAsResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/export_dialog.rs:869 body, :100 open (seed_remembered), :1038 write_export
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ExportAsResult>> {
        self.recall(ctx);
        if let Some(path) = self.picked(ctx) {
            if let Some(host) = Host::try_of_ctx(ctx) {
                Remembered::<ExportSettings>::new(REMEMBER_KEY).store(&host, &self.settings);
            }
            self.state.close();
            let (width, height) = self.output_size();
            return Some(DialogResult::Ok(ExportAsResult { settings: self.settings.clone(), path, width, height }));
        }
        let labels = ExportLabels::shared_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let before = self.settings.clone();
        let saving = self.picking.is_some();
        let tokens = crate::theme::current(ctx);
        let side = tokens.component.dialog.preview_pane[1] * 2.0;
        let (settings_width, pane_gap) = (tokens.component.export.settings_width, tokens.component.export.pane_gap);
        let line = self.size_line(&labels);
        let size = self.output_size();
        let Self { formats, source, settings, preview, size_error, defaults, state, .. } = self;
        let shown = Dialog::with_id(self.id, labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(settings_width + side + pane_gap))
            .footer(Footer::standard().ok(labels.export.as_str()).ok_enabled(!saving))
            .show(ctx, state, &mut (), |ui, ()| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(settings_width);
                        settings_column(ui, formats, *source, size, settings, size_error, *defaults, &labels, &numbers);
                    });
                    ui.add_space(pane_gap);
                    ui.vertical(|ui| preview_column(ui, preview.as_ref(), side, &line));
                });
            });
        if before != self.settings {
            self.revision += 1;
        }
        match shown.outcome {
            Some(Outcome::Ok) => {
                self.queue_save(ctx, &labels);
                None
            }
            Some(Outcome::Cancel | Outcome::Close) => {
                self.forget_pick(ctx);
                Some(DialogResult::Cancel)
            }
            _ => None,
        }
    }
}

const REMEMBER_KEY: &str = "export.exportAs";

/// The preview with the size line under it.
fn preview_column(ui: &mut Ui, preview: Option<&ExportPreview>, side: f32, line: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
    let texture = preview.map(|preview| &preview.texture);
    let image = texture.map_or(rect, |texture| {
        let size = texture.size_vec2();
        Rect::from_center_size(rect.center(), size * (side / size.x.max(size.y).max(1.0)))
    });
    web::paint_picture(ui, rect, texture, image);
    if preview.is_none() {
        ui.put(Rect::from_center_size(rect.center(), vec2(24.0, 24.0)), Spinner::new());
    }
    web::caption(ui, line.to_owned(), Content::Muted);
}

/// A size edit this pass.
enum Edit {
    Scale(f64),
    /// The typed side's value, and whether it is the width.
    Side(f64, bool),
}

/// Format, its options, Metadata, Color Space and Image Size.
#[expect(clippy::too_many_arguments, reason = "the column's inputs are separate borrows of the dialog")]
fn settings_column(
    ui: &mut Ui,
    formats: &[ExportFormat],
    source: (u32, u32),
    size: (u32, u32),
    settings: &mut ExportSettings,
    size_error: &mut Option<SizeError>,
    defaults: ExportFormatDefaults,
    labels: &ExportLabels,
    numbers: &LocaleNumbers,
) {
    let sizing = SizingLabels::shared_ctx(ui.ctx());
    let gap = crate::theme::current(ui.ctx()).space.gap;
    let mut chosen = formats.iter().position(|format| format.id == settings.format).unwrap_or(0);
    FormGrid::new("export-settings").show(ui, |form| {
        let before = chosen;
        form.row(&labels.format, |ui, _| {
            let mut select = Select::new("export-format", labels.format.as_str(), &mut chosen);
            for (index, format) in formats.iter().enumerate() {
                select = select.option(index, format.name);
            }
            select.show(ui).response
        });
        if chosen != before {
            settings.format = formats[chosen].id.to_owned();
            match settings.format.as_str() {
                "jpg" => settings.quality = defaults.jpeg_quality.clamp(1, 100),
                "webp" => {
                    settings.quality = defaults.webp_quality.clamp(1, 100);
                    settings.lossless = defaults.webp_lossless;
                }
                _ => {}
            }
        }
        let Some(format) = formats.get(chosen) else { return };
        if format.lossless {
            web::check_row(form, &labels.lossless, &mut settings.lossless);
        }
        if format.quality && !(format.lossless && settings.lossless) {
            if let Some(value) = web::number_row(form, "export-quality", &labels.quality, numbers, f64::from(settings.quality), 1.0..=100.0) {
                settings.quality = web::clamp_number(value, 1.0..=100.0) as u8;
            }
            form.spanning(|ui| {
                let edit = Slider::new("export-quality-slider", &labels.quality, f64::from(settings.quality), 1.0..=100.0).step(1.0).show(ui);
                if let Some(value) = edit.event.as_ref().and_then(EditEvent::value) {
                    settings.quality = web::clamp_number(*value, 1.0..=100.0) as u8;
                }
                edit
            });
        }
        if !format.bits.is_empty() {
            form.row(&labels.bits, |ui, _| {
                let mut select = Select::new("export-bits", labels.bits.as_str(), &mut settings.bits).option(0, labels.auto.as_str());
                for bits in format.bits {
                    select = select.option(*bits, bits.to_string());
                }
                select.show(ui).response
            });
        }
        if format.transparency {
            web::check_row(form, &labels.transparency, &mut settings.transparency);
        }
        form.space(gap.row);
        form.row(&labels.metadata, |ui, _| {
            let mut select = Select::new("export-metadata", labels.metadata.as_str(), &mut settings.metadata);
            for (value, text) in [
                (ExportMetadata::None, &labels.metadata_none),
                (ExportMetadata::Exif, &labels.metadata_exif),
                (ExportMetadata::All, &labels.metadata_all),
            ] {
                select = select.option(value, text.as_str());
            }
            select.show(ui).response
        });
        form.row(&labels.color_space, |ui, _| {
            Select::new("export-profile", labels.color_space.as_str(), &mut settings.profile)
                .option(ExportProfile::Document, labels.document_profile.as_str())
                .option(ExportProfile::Srgb, labels.convert_srgb.as_str())
                .show(ui)
                .response
        });
        web::check_row(form, &labels.embed_profile, &mut settings.embed_profile);
    });

    ui.add_space(gap.section);
    let _ = SectionLabel::new(&labels.image_size).show(ui);
    let mut edited = None;
    FormGrid::new("export-size").show(ui, |form| {
        let scale = match settings.size {
            ExportSize::Scale(scale) => scale,
            ExportSize::Pixels { width, .. } => f64::from(width) / f64::from(source.0.max(1)) * 100.0,
        };
        form.row(&labels.scale, |ui, _| {
            let edit = NumberField::new("export-scale", labels.scale.as_str(), scale.round(), numbers)
                .range(1.0..=1000.0)
                .decimals(0..=0)
                .unit(Unit::new("{n}%"))
                .show(ui);
            if let Some(value) = web::typed(&edit) {
                edited = Some(Edit::Scale(value));
            }
            edit.response
        });
        for (id, label, value, width) in [("export-width", &labels.width, size.0, true), ("export-height", &labels.height, size.1, false)] {
            form.row(label, |ui, _| {
                let edit = NumberField::new(id, label.as_str(), f64::from(value), numbers)
                    .range(1.0..=MAX_PIXELS)
                    .decimals(0..=0)
                    .unit(Unit::new("{n} px"))
                    .show(ui);
                if let Some(value) = web::typed(&edit) {
                    edited = Some(Edit::Side(value, width));
                }
                edit.response
            });
        }
        let mut lock = settings.lock_aspect;
        form.unlabelled(|ui| Checkbox::bool(sizing.constrain.as_str(), &mut lock).show(ui));
        if lock != settings.lock_aspect {
            settings.lock_aspect = lock;
            if lock {
                edited = Some(Edit::Side(f64::from(size.0), true));
            }
        }
        form.row(&labels.resample, |ui, _| {
            let mut select = Select::new("export-resample", labels.resample.as_str(), &mut settings.resample);
            for (value, text) in [
                (Resample::Automatic, &sizing.automatic),
                (Resample::Bicubic, &sizing.bicubic),
                (Resample::BicubicSmoother, &sizing.bicubic_smoother),
                (Resample::BicubicSharper, &sizing.bicubic_sharper),
                (Resample::Bilinear, &sizing.bilinear),
                (Resample::NearestNeighbor, &sizing.nearest_neighbor),
            ] {
                select = select.option(value, text.as_str());
            }
            select.show(ui).response
        });
    });
    if let Some(edit) = edited {
        let result = match edit {
            Edit::Scale(scale) => resize_from_scale(source, scale).map(|_| ExportSize::Scale(scale)),
            Edit::Side(value, width) => {
                let (w, h) = if width { (value, f64::from(size.1)) } else { (f64::from(size.0), value) };
                resize_from_side(source, w, h, settings.lock_aspect, width).map(|((width, height), _)| ExportSize::Pixels { width, height })
            }
        };
        match result {
            Ok(new) => {
                settings.size = new;
                *size_error = None;
            }
            Err(error) => *size_error = Some(error),
        }
    }
    let tokens = Tokens::of(ui);
    let dimensions = craft_i18n::fmt(&sizing.dimensions_value, &[("width", &size.0.to_string()), ("height", &size.1.to_string())]);
    web::caption(ui, dimensions, Content::Muted);
    let status = |ui: &mut Ui, text: &str, color: Rgba| {
        let format = role_format(&tokens, TypeRole::Caption, color.to_egui());
        ui.add(Label::new(LayoutJob::single_section(text.to_owned(), format)).selectable(false));
    };
    if let Some(error) = *size_error {
        let text = match error {
            SizeError::Scale => &labels.scale_range,
            SizeError::Pixels => &labels.pixel_range,
        };
        status(ui, text, tokens.color.status.bad.fg);
    }
    if let Some(limit) = formats.iter().find(|format| format.id == settings.format).and_then(|format| format.max_side)
        && (size.0 > limit || size.1 > limit)
    {
        status(ui, &labels.ico_limit, tokens.color.status.warn.fg);
    }
}

/// A file name safe on every platform: at most 80 characters, control characters and
/// `< > : " / \ | ? *` replaced by `_`, leading and trailing dots and spaces trimmed; empty
/// becomes `fallback` ("layer" for per-layer exports, the document name otherwise).
// pc: crates/ui-egui/src/dialogs/export_dialog.rs:231 export_name
pub fn export_file_name(name: &str, fallback: &str) -> String {
    let name: String = name.chars().take(MAX_NAME_CHARS).map(|c| if c.is_control() || RESERVED.contains(c) { '_' } else { c }).collect();
    let name = name.trim().trim_matches('.');
    if name.is_empty() { fallback.to_owned() } else { name.to_owned() }
}

const MAX_NAME_CHARS: usize = 80;
const RESERVED: &str = "<>:\"/\\|?*";
/// The largest side an export can have, in pixels.
const MAX_PIXELS: f64 = 300_000.0;

/// The paths of a multi-file export: the first is `first` (its overwrite is the save picker's
/// question); file `i` (0-based) after it is `{stem}-{i + 1}-{name}.{ext}` beside it, `name`
/// through [`export_file_name`], stem "layers" and extension "png" when `first` has none. A later
/// path that `exists` gets `-1`, `-2`, … after the name (`{stem}-{i + 1}-{name}-{k}.{ext}`) so a
/// sibling file is never overwritten silently; on wasm nothing exists (the browser names
/// downloads).
// pc: crates/ui-egui/src/dialogs/export_dialog.rs:239 batch_paths
pub fn numbered_paths(first: &Path, names: &[&str], exists: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let parent = first.parent().unwrap_or_else(|| Path::new(""));
    let stem = first.file_stem().and_then(|s| s.to_str()).unwrap_or("layers");
    let ext = first.extension().and_then(|e| e.to_str()).unwrap_or("png");
    names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == 0 {
                return first.to_path_buf();
            }
            let name = export_file_name(name, "layer");
            let mut candidate = parent.join(format!("{stem}-{}-{name}.{ext}", i + 1));
            if !cfg!(target_arch = "wasm32") {
                let mut suffix = 1u64;
                while exists(&candidate) {
                    candidate = parent.join(format!("{stem}-{}-{name}-{suffix}.{ext}", i + 1));
                    suffix = suffix.saturating_add(1);
                }
            }
            candidate
        })
        .collect()
}

/// Why a size was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeError {
    /// The scale is not within 1–1000 %: [`ExportLabels::scale_range`].
    Scale,
    /// A side is not within 1–300,000 px: [`ExportLabels::pixel_range`].
    Pixels,
}

/// The output size at `scale` percent (1–1000) of a `source` size: each side scaled and rounded,
/// at least 1 px (a source side under 1 counts as 1); a side over 300,000 px is
/// [`SizeError::Pixels`].
// pc: crates/ui-egui/src/dialogs/export_dialog.rs:353 resize_from_scale (MAX_PX :335)
pub fn resize_from_scale(source: (u32, u32), scale: f64) -> Result<(u32, u32), SizeError> {
    if !scale.is_finite() || !(1.0..=1000.0).contains(&scale) {
        return Err(SizeError::Scale);
    }
    let side = |source: u32| (f64::from(source.max(1)) * scale / 100.0).round().max(1.0);
    let (width, height) = (side(source.0), side(source.1));
    if width > MAX_PIXELS || height > MAX_PIXELS {
        return Err(SizeError::Pixels);
    }
    Ok((width as u32, height as u32))
}

/// The output size after the user typed `width` (`edited_width`) or `height`: the typed side is
/// rounded and must be 1–300,000 px; with `lock`, the other side follows the source's aspect
/// (at least 1 px, same range), otherwise it keeps its value (same range). Returns the size and
/// the scale it amounts to, in percent of the edited side.
// pc: crates/ui-egui/src/dialogs/export_dialog.rs:366 resize_from_side, :345 finite_px
pub fn resize_from_side(source: (u32, u32), width: f64, height: f64, lock: bool, edited_width: bool) -> Result<((u32, u32), f64), SizeError> {
    let (source_width, source_height) = (f64::from(source.0.max(1)), f64::from(source.1.max(1)));
    let (width, height) = if edited_width {
        let width = finite_pixels(width)?;
        let height = if lock { finite_pixels((source_height * width / source_width).max(1.0))? } else { finite_pixels(height)? };
        (width, height)
    } else {
        let height = finite_pixels(height)?;
        let width = if lock { finite_pixels((source_width * height / source_height).max(1.0))? } else { finite_pixels(width)? };
        (width, height)
    };
    let scale = if edited_width { width / source_width * 100.0 } else { height / source_height * 100.0 };
    Ok(((width as u32, height as u32), scale))
}

fn finite_pixels(value: f64) -> Result<f64, SizeError> {
    if value.is_finite() && (1.0..=MAX_PIXELS).contains(&value) { Ok(value.round()) } else { Err(SizeError::Pixels) }
}
