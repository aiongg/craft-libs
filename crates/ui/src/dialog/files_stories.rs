//! Stories of the file dialogs: New Document, Export As, Save for Web, the format option
//! dialogs, the watermark group, the output path row and Recovery. Every dialog opens on a
//! [`DialogHost`](super::DialogHost) like the framework stories; pictures and size estimates the
//! app would compute are generated here, and the Host's file service answers every picker with
//! Cancelled, so nothing opens a picker or writes a file.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use craft_host::{AppInfo, FileError, Files, Host, PickAnswer, PickId, PickKind, PickRequest, Picked};
use craft_settings::Remembered;
use egui::{Color32, ColorImage, Context, Id, TextureHandle, TextureOptions, Ui};

use super::export::pdf::PdfExportDialog;
use super::export::{
    DdsOptionsDialog, ExportAsDialog, ExportOptions, ExportPreview, ExportSize, JpegOptionsDialog, OutputPathRow, SaveForWebDialog,
    SaveForWebOptions, SizeEstimate, TargaOptionsDialog, TiffOptionsDialog, Watermark, WatermarkEditor, WatermarkStyle, WebFormat, WebVariant,
};
use super::framework_stories::{claim, drive};
use super::new_document::{PRESETS_KEY, RECENT_KEY};
use super::{
    Anchor, BackgroundContents, BitDepth, ColorMode, DocumentOptions, NewDocumentDialog, PixelSize, RecoveredDocument, RecoveryDialog, SizeUnit,
};
use crate::edit::EditEvent;
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct NewDocumentArgs {
    open: bool,
    #[arg(name = "clipboard image")]
    clipboard: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ExportFormatArg {
    Png,
    Jpg,
    Webp,
    Tiff,
    Tga,
    Ico,
}

impl ExportFormatArg {
    fn id(self) -> &'static str {
        match self {
            ExportFormatArg::Png => "png",
            ExportFormatArg::Jpg => "jpg",
            ExportFormatArg::Webp => "webp",
            ExportFormatArg::Tiff => "tif",
            ExportFormatArg::Tga => "tga",
            ExportFormatArg::Ico => "ico",
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ExportAsArgs {
    open: bool,
    format: ExportFormatArg,
    #[arg(name = "preview ready")]
    preview: bool,
    #[arg(name = "exact size")]
    exact: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PdfPresetArg {
    Default,
    Press,
    PdfX1a,
    Smallest,
}

impl PdfPresetArg {
    fn name(self, app: &str) -> String {
        match self {
            PdfPresetArg::Default => craft_pdf::settings::default_preset_name(app),
            PdfPresetArg::Press => "Press Quality".to_owned(),
            PdfPresetArg::PdfX1a => "PDF/X-1a:2001".to_owned(),
            PdfPresetArg::Smallest => "Smallest File Size".to_owned(),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PdfExportArgs {
    open: bool,
    preset: PdfPresetArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SaveForWebArgs {
    open: bool,
    animation: bool,
    presets: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct JpegArgs {
    open: bool,
    #[arg(range = 0..=12)]
    quality: u8,
    #[arg(name = "file's own quality")]
    own: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TiffArgs {
    open: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TargaArgs {
    open: bool,
    alpha: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum StyleArg {
    Text,
    Graphic,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct WatermarkArgs {
    style: StyleArg,
    vertical: bool,
    #[arg(range = 240.0..=480.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct OutputPathArgs {
    templates: bool,
    #[arg(name = "file chosen")]
    chosen: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum AgeArg {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl AgeArg {
    fn duration(self) -> Duration {
        Duration::from_secs(match self {
            AgeArg::Seconds => 20,
            AgeArg::Minutes => 5 * 60,
            AgeArg::Hours => 3 * 3_600,
            AgeArg::Days => 2 * 86_400,
        })
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RecoveryArgs {
    open: bool,
    age: AgeArg,
    #[arg(name = "recover blocked")]
    blocked: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/new-document-dialog/dialog",
            Level::Patterns,
            "NewDocumentDialog",
            "Dialog",
            NewDocumentArgs { open: true, clipboard: false },
            show_new_document
        ),
        typed_story!(
            "patterns/export-as-dialog/dialog",
            Level::Patterns,
            "ExportAsDialog",
            "Dialog",
            ExportAsArgs { open: true, format: ExportFormatArg::Jpg, preview: true, exact: false },
            show_export_as
        ),
        typed_story!(
            "patterns/pdf-export-dialog/dialog",
            Level::Patterns,
            "PdfExportDialog",
            "Dialog",
            PdfExportArgs { open: true, preset: PdfPresetArg::Default },
            show_pdf_export
        ),
        typed_story!(
            "patterns/save-for-web-dialog/dialog",
            Level::Patterns,
            "SaveForWebDialog",
            "Dialog",
            SaveForWebArgs { open: true, animation: false, presets: true },
            show_save_for_web
        ),
        typed_story!(
            "patterns/jpeg-options-dialog/dialog",
            Level::Patterns,
            "JpegOptionsDialog",
            "Dialog",
            JpegArgs { open: true, quality: 8, own: false },
            show_jpeg
        ),
        typed_story!("patterns/tiff-options-dialog/dialog", Level::Patterns, "TiffOptionsDialog", "Dialog", TiffArgs { open: true }, show_tiff),
        typed_story!(
            "patterns/targa-options-dialog/dialog",
            Level::Patterns,
            "TargaOptionsDialog",
            "Dialog",
            TargaArgs { open: true, alpha: true },
            show_targa
        ),
        typed_story!("patterns/dds-options-dialog/dialog", Level::Patterns, "DdsOptionsDialog", "Dialog", TiffArgs { open: true }, show_dds),
        typed_story!(
            "composites/watermark-editor/group",
            Level::Composites,
            "WatermarkEditor",
            "Group",
            WatermarkArgs { style: StyleArg::Text, vertical: false, width: 360.0 },
            show_watermark
        ),
        typed_story!(
            "composites/output-path-row/row",
            Level::Composites,
            "OutputPathRow",
            "Row",
            OutputPathArgs { templates: true, chosen: false },
            show_output_path
        ),
        typed_story!(
            "patterns/recovery-dialog/dialog",
            Level::Patterns,
            "RecoveryDialog",
            "Dialog",
            RecoveryArgs { open: true, age: AgeArg::Minutes, blocked: false },
            show_recovery
        ),
    ]
}

/// A file service for stories: every picker answers at once with a path in a made-up
/// `/stories` folder (the suggested file name, else the purpose), so Save… and Browse… finish
/// the way they do in an app. Nothing touches the disk.
#[derive(Default)]
struct StoryFiles {
    state: std::sync::Mutex<StoryFilesState>,
}

#[derive(Default)]
struct StoryFilesState {
    next: u64,
    answered: Vec<PickAnswer>,
}

impl Files for StoryFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.next += 1;
        let id = PickId(state.next);
        let folder = PathBuf::from("/stories");
        let path = match request.kind {
            PickKind::Folder => folder,
            _ => folder.join(request.file_name.clone().unwrap_or_else(|| request.purpose.clone())),
        };
        state.answered.push(PickAnswer { id, request, picked: Picked::Paths(vec![path]) });
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let at = state.answered.iter().position(|answer| answer.id == id)?;
        Some(state.answered.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).answered.retain(|answer| answer.id != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

/// Installs a Host with the default in-memory settings and [`StoryFiles`], when the story runs
/// without one.
fn ensure_host(ctx: &Context) {
    if Host::try_of_ctx(ctx).is_none() {
        Host::builder(AppInfo { id: "craft-ui-stories", name: "Craft Stories", version: "0" })
            .files(std::sync::Arc::new(StoryFiles::default()))
            .install(ctx);
    }
}

/// A sky, a sun and hills, with each channel reduced to `levels` steps so a lower quality looks
/// coarser.
fn sample_image(width: usize, height: usize, levels: u32) -> ColorImage {
    let levels = levels.clamp(2, 256) as f32;
    let step = |v: f32| ((v / 255.0 * (levels - 1.0)).round() / (levels - 1.0) * 255.0) as u8;
    let (sun_x, sun_y, sun_r) = (width as f32 * 0.7, height as f32 * 0.38, height as f32 * 0.14);
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let t = y as f32 / height as f32;
        for x in 0..width {
            let (mut r, mut g, mut b) = (40.0 + 210.0 * t, 90.0 + 80.0 * t, 160.0 - 50.0 * t);
            let (dx, dy) = (x as f32 - sun_x, y as f32 - sun_y);
            if dx * dx + dy * dy < sun_r * sun_r {
                (r, g, b) = (255.0, 232.0, 150.0);
            }
            let ridge = height as f32 * (0.68 + 0.08 * (x as f32 / width as f32 * 9.0).sin());
            if y as f32 > ridge {
                let shade = 0.55 + 0.25 * ((y as f32 - ridge) / height as f32);
                (r, g, b) = (40.0 * shade, 90.0 * shade, 60.0 * shade);
            }
            pixels.push(Color32::from_rgb(step(r), step(g), step(b)));
        }
    }
    ColorImage::new([width, height], pixels)
}

/// The sample picture as a texture, loaded once per size and level count.
fn picture(ctx: &Context, width: usize, height: usize, levels: u32) -> TextureHandle {
    let key = Id::unique(("craft_ui::files_stories::picture", width, height, levels));
    if let Some(texture) = ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        return texture;
    }
    let texture = ctx.load_texture("story-picture", sample_image(width, height, levels), TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
    texture
}

/// Fits `size` to a long side of `side` pixels.
fn proxy_size(size: (u32, u32), side: u32) -> (usize, usize) {
    let long = size.0.max(size.1).max(1);
    let scale = f64::from(side) / f64::from(long);
    (((f64::from(size.0) * scale).round() as usize).max(1), ((f64::from(size.1) * scale).round() as usize).max(1))
}

/// Stores `value` under `key` in the Host's settings unless the key already has one.
fn seed<T: serde::Serialize + serde::de::DeserializeOwned>(ctx: &Context, key: &str, value: &T) {
    if let Some(host) = Host::try_of_ctx(ctx) {
        let remembered = Remembered::<T>::new(key);
        if remembered.recall(&host).is_none() {
            remembered.store(&host, value);
        }
    }
}

fn recent_settings(width: u32, height: u32, ppi: f64, unit: SizeUnit, mode: ColorMode, depth: BitDepth) -> DocumentOptions {
    DocumentOptions { size: PixelSize { width, height, pixels_per_inch: ppi }, unit, color_mode: mode, depth, ..DocumentOptions::default() }
}

/// New Document with recent documents and saved presets in the Host's memory; the clipboard
/// argument adds the Clipboard card.
fn show_new_document(ui: &mut Ui, args: &mut NewDocumentArgs) {
    if !claim(ui.ctx(), "new-document", ui.scope_id()) {
        return;
    }
    ensure_host(ui.ctx());
    seed(
        ui.ctx(),
        RECENT_KEY,
        &vec![
            recent_settings(3000, 2000, 300.0, SizeUnit::Inches, ColorMode::Rgb, BitDepth::Sixteen),
            recent_settings(1080, 1920, 72.0, SizeUnit::Pixels, ColorMode::Rgb, BitDepth::Eight),
        ],
    );
    let saved = |name: &str, width, height, ppi, unit, mode| DocumentOptions {
        name: name.to_owned(),
        ..recent_settings(width, height, ppi, unit, mode, BitDepth::Eight)
    };
    seed(
        ui.ctx(),
        PRESETS_KEY,
        &vec![
            saved("Postcard 6 x 4 in", 1800, 1200, 300.0, SizeUnit::Inches, ColorMode::Cmyk),
            saved("Thumbnail", 640, 360, 72.0, SizeUnit::Pixels, ColorMode::Rgb),
        ],
    );
    let clipboard = args.clipboard;
    drive(
        ui,
        "new-document",
        &mut args.open,
        clipboard,
        || {
            let mut dialog = NewDocumentDialog::new()
                .background_color([0.2, 0.4, 0.6])
                .profiles(ColorMode::Rgb, "sRGB IEC61966-2.1", vec!["Display P3".to_owned(), "Adobe RGB (1998)".to_owned()])
                .profiles(ColorMode::Cmyk, "U.S. Web Coated (SWOP) v2", vec!["Coated FOGRA39".to_owned()]);
            if clipboard {
                dialog = dialog.clipboard(PixelSize { width: 1920, height: 1080, pixels_per_inch: 72.0 });
            }
            dialog
        },
        |_| {},
        |settings| {
            let fill = match settings.background {
                BackgroundContents::Custom(_) => "custom background",
                _ => "preset background",
            };
            format!("OK, {} {} x {} px at {} ppi, {fill}", settings.name, settings.size.width, settings.size.height, settings.size.pixels_per_inch)
        },
    );
}

/// The file size the story's stand-in encoder reports for `settings`.
fn export_bytes(settings: &ExportOptions, source: (u32, u32)) -> u64 {
    let (width, height) = match settings.size {
        ExportSize::Scale(scale) => (f64::from(source.0) * scale / 100.0, f64::from(source.1) * scale / 100.0),
        ExportSize::Pixels { width, height } => (f64::from(width), f64::from(height)),
    };
    let per_pixel = match settings.format.as_str() {
        "jpg" => 0.04 + 0.003 * f64::from(settings.quality),
        "webp" if settings.lossless => 1.1,
        "webp" => 0.03 + 0.0018 * f64::from(settings.quality),
        "tif" => 3.0,
        "tga" => f64::from(if settings.bits == 24 { 24_u8 } else { 32 }) / 8.0,
        "ico" => 4.0,
        _ => 1.6,
    };
    (width * height * per_pixel) as u64
}

/// Export As of a 4000 x 3000 px document. The preview and the estimate answer each revision
/// of the settings, as the app's job would.
fn show_export_as(ui: &mut Ui, args: &mut ExportAsArgs) {
    if !claim(ui.ctx(), "export-as", ui.scope_id()) {
        return;
    }
    const SOURCE: (u32, u32) = (4000, 3000);
    let (format, ready, exact) = (args.format, args.preview, args.exact);
    let ctx = ui.ctx().clone();
    drive(
        ui,
        "export-as",
        &mut args.open,
        format,
        || {
            ExportAsDialog::new("poster.psd", SOURCE.0, SOURCE.1)
                .initial(ExportOptions { format: format.id().to_owned(), ..ExportOptions::default() })
        },
        |dialog| {
            if !ready {
                return;
            }
            if let Some((revision, settings)) = dialog.wanted_preview() {
                let (width, height) = match settings.size {
                    ExportSize::Scale(_) => SOURCE,
                    ExportSize::Pixels { width, height } => (width, height),
                };
                let side = if settings.format == "ico" { 256 } else { 512 };
                let (width, height) = proxy_size((width, height), side);
                let levels = if matches!(settings.format.as_str(), "jpg" | "webp") && !settings.lossless {
                    4 + u32::from(settings.quality) * 252 / 100
                } else {
                    256
                };
                let bytes = export_bytes(&settings, SOURCE);
                dialog.set_preview(ExportPreview { revision, texture: picture(&ctx, width, height, levels), estimate: SizeEstimate::Proxy(bytes) });
                if exact {
                    dialog.set_exact_estimate(revision, bytes);
                }
            }
        },
        |result| format!("OK, {} {} x {} px", result.options.format, result.width, result.height),
    );
}

/// The two colour profiles the Save PDF story knows.
struct StoryProfiles;

impl craft_pdf::settings::ProfileCatalog for StoryProfiles {
    fn kind(&self, name: &str) -> Option<craft_pdf::settings::ProfileKind> {
        match name {
            "sRGB IEC61966-2.1" => Some(craft_pdf::settings::ProfileKind::Rgb),
            "Coated FOGRA39" => Some(craft_pdf::settings::ProfileKind::Cmyk),
            _ => None,
        }
    }

    fn rgb(&self) -> &str {
        "sRGB IEC61966-2.1"
    }

    fn cmyk(&self) -> &str {
        "Coated FOGRA39"
    }
}

/// Save PDF of a poster in an RGB document, with two profiles. The file service answers every
/// picker with Cancelled, so the dialog stays open on Save PDF.
fn show_pdf_export(ui: &mut Ui, args: &mut PdfExportArgs) {
    if !claim(ui.ctx(), "pdf-export", ui.scope_id()) {
        return;
    }
    let words = StoryLabels::of(ui);
    let preset = args.preset;
    drive(
        ui,
        "pdf-export",
        &mut args.open,
        preset,
        move || {
            PdfExportDialog::new(words.app_name.as_str(), words.poster.as_str())
                .preset(preset.name(&words.app_name))
                .profiles(vec!["sRGB IEC61966-2.1".to_owned(), "Coated FOGRA39".to_owned()], std::sync::Arc::new(StoryProfiles))
                .document_bleed([9.0; 4])
        },
        |_| {},
        |result| format!("OK, {}", result.path.display()),
    );
}

fn web_settings(format: WebFormat, edit: impl FnOnce(&mut SaveForWebOptions)) -> SaveForWebOptions {
    let mut settings = SaveForWebOptions { format, ..SaveForWebOptions::default() };
    edit(&mut settings);
    settings
}

/// Save for Web of a 1200 x 800 px document with stand-in encodes for every pane the dialog asks for.
fn show_save_for_web(ui: &mut Ui, args: &mut SaveForWebArgs) {
    if !claim(ui.ctx(), "save-for-web", ui.scope_id()) {
        return;
    }
    const SIZE: (u32, u32) = (1200, 800);
    let (animation, presets) = (args.animation, args.presets);
    let ctx = ui.ctx().clone();
    drive(
        ui,
        "save-for-web",
        &mut args.open,
        (animation, presets),
        || {
            let mut dialog = SaveForWebDialog::new("poster.psd", SIZE.0, SIZE.1, picture(&ctx, 600, 400, 256));
            if presets {
                dialog = dialog.presets(vec![
                    ("JPEG High".to_owned(), web_settings(WebFormat::Jpeg, |s| s.quality = 80)),
                    ("JPEG Low".to_owned(), web_settings(WebFormat::Jpeg, |s| s.quality = 30)),
                    ("PNG-24".to_owned(), web_settings(WebFormat::Png24, |_| {})),
                    ("GIF 128 Dithered".to_owned(), web_settings(WebFormat::Gif, |s| s.colors = 128)),
                    ("PNG-8 64 No Dither".to_owned(), web_settings(WebFormat::Png8, |s| (s.colors, s.dither_amount) = (64, 0))),
                ]);
            }
            if animation {
                dialog = dialog.animation(12, 12.0);
            }
            dialog
        },
        |dialog| {
            for (index, revision, settings) in dialog.wanted_variants() {
                let lossy = matches!(settings.format, WebFormat::Jpeg | WebFormat::WebP) && !settings.lossless;
                let indexed = matches!(settings.format, WebFormat::Gif | WebFormat::Png8);
                let levels = if lossy {
                    4 + u32::from(settings.quality) * 252 / 100
                } else if indexed {
                    u32::from(settings.colors).clamp(2, 256).isqrt() * 4
                } else {
                    256
                };
                let pixels = f64::from(SIZE.0) * f64::from(SIZE.1) * (settings.percent / 100.0).powi(2);
                let per_pixel = match settings.format {
                    WebFormat::Jpeg => 0.03 + 0.004 * f64::from(settings.quality),
                    WebFormat::WebP => 0.02 + 0.002 * f64::from(settings.quality),
                    WebFormat::Png24 => 1.8,
                    WebFormat::Gif | WebFormat::Png8 => 0.2 + f64::from(settings.colors) / 800.0,
                    WebFormat::Wbmp => 0.125,
                };
                let count = usize::from(settings.colors.min(32));
                let palette =
                    if indexed { (0..count).map(|n| [(n * 8) as u8, 90 + (n * 4) as u8, 200 - (n * 6) as u8]).collect() } else { Vec::new() };
                dialog.set_variant(WebVariant {
                    index,
                    revision,
                    texture: picture(&ctx, 600, 400, levels),
                    bytes: (pixels * per_pixel) as u64,
                    colors: indexed.then_some(settings.colors),
                    palette,
                    proxy: false,
                });
            }
        },
        |result| format!("OK, {:?} {} x {} px", result.options.format, result.width, result.height),
    );
}

fn show_jpeg(ui: &mut Ui, args: &mut JpegArgs) {
    if !claim(ui.ctx(), "jpeg-options", ui.scope_id()) {
        return;
    }
    let (quality, own) = (args.quality, args.own);
    drive(
        ui,
        "jpeg-options",
        &mut args.open,
        (quality, own),
        || JpegOptionsDialog::new(quality).own_quality(own.then_some(9)),
        |_| {},
        |options| format!("OK, quality {}, {:?}", options.quality, options.format),
    );
}

fn show_tiff(ui: &mut Ui, args: &mut TiffArgs) {
    if !claim(ui.ctx(), "tiff-options", ui.scope_id()) {
        return;
    }
    drive(
        ui,
        "tiff-options",
        &mut args.open,
        (),
        TiffOptionsDialog::new,
        |_| {},
        |answer| {
            format!(
                "OK, {}{}",
                if answer.discard_layers { "discard layers" } else { "keep layers" },
                if answer.dont_show_again { ", don't show again" } else { "" }
            )
        },
    );
}

fn show_targa(ui: &mut Ui, args: &mut TargaArgs) {
    if !claim(ui.ctx(), "targa-options", ui.scope_id()) {
        return;
    }
    let bits = if args.alpha { 32 } else { 24 };
    drive(ui, "targa-options", &mut args.open, bits, || TargaOptionsDialog::new(bits), |_| {}, |options| format!("OK, {} bits/pixel", options.bits));
}

fn show_dds(ui: &mut Ui, args: &mut TiffArgs) {
    if !claim(ui.ctx(), "dds-options", ui.scope_id()) {
        return;
    }
    drive(
        ui,
        "dds-options",
        &mut args.open,
        (),
        DdsOptionsDialog::new,
        |_| {},
        |options| format!("OK, {:?}{}", options.compression, if options.mipmaps { " with mipmaps" } else { "" }),
    );
}

fn watermark_of(style: StyleArg, vertical: bool) -> Watermark {
    let style = match style {
        StyleArg::Text => WatermarkStyle::Text("\u{a9} Studio 2026".to_owned()),
        StyleArg::Graphic => WatermarkStyle::Graphic("logo.png".to_owned()),
    };
    Watermark { style, vertical, position: Anchor::BottomRight, ..Watermark::default() }
}

/// The watermark rows inline. The args pick the starting watermark; the rows edit it, and the
/// line under them reports the value.
fn show_watermark(ui: &mut Ui, args: &mut WatermarkArgs) {
    ensure_host(ui.ctx());
    let key = ui.make_persistent_id("craft_ui::files_stories::watermark");
    let start = (args.style, args.vertical);
    let (shown, mut watermark): (Option<(StyleArg, bool)>, Watermark) =
        ui.data(|d| d.get_temp(key)).unwrap_or_else(|| (None, watermark_of(start.0, start.1)));
    if shown != Some(start) {
        watermark = watermark_of(start.0, start.1);
    }
    ui.allocate_ui(egui::vec2(args.width.min(ui.available_width()), 0.0), |ui| {
        let _ = WatermarkEditor::new("story-watermark", &mut watermark).show(ui);
    });
    let text = match &watermark.style {
        WatermarkStyle::Text(text) | WatermarkStyle::Graphic(text) => text.as_str(),
    };
    ui.label(format!(
        "{text:?}, {:?}, {} degrees, size {} %, opacity {} %",
        watermark.position, watermark.rotation, watermark.size, watermark.opacity
    ));
    ui.data_mut(|d| d.insert_temp(key, (Some(start), watermark)));
}

/// The output template row; a picked template or a typed one replaces the path.
fn show_output_path(ui: &mut Ui, args: &mut OutputPathArgs) {
    ensure_host(ui.ctx());
    let key = ui.make_persistent_id("craft_ui::files_stories::output-path");
    let start = args.chosen;
    let (shown, mut path): (Option<bool>, String) = ui.data(|d| d.get_temp(key)).unwrap_or((None, String::new()));
    if shown != Some(start) {
        path = if start { "/renders/poster_final.mov".to_owned() } else { "[Comp Name].[fileExtension]".to_owned() };
    }
    let mut row = OutputPathRow::new(
        "story-output-path",
        &path,
        PickRequest::save("render").title("Output To").file_name("poster.mov").filter("QuickTime", &["mov"]),
    );
    if args.templates {
        row = row.templates(vec![
            ("[Comp Name].[fileExtension]".to_owned(), "Comp name".to_owned()),
            ("[Comp Name]_[compositionName]_[#####].[fileExtension]".to_owned(), "Comp name, numbered".to_owned()),
            ("[Date]/[Comp Name].[fileExtension]".to_owned(), "Dated folder".to_owned()),
        ]);
    }
    let response = row.show(ui);
    if let Some(EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = &response.edit.event {
        path.clone_from(value);
    }
    ui.data_mut(|d| d.insert_temp(key, (Some(start), path)));
}

/// Recovery for one autosaved document.
fn show_recovery(ui: &mut Ui, args: &mut RecoveryArgs) {
    if !claim(ui.ctx(), "recovery", ui.scope_id()) {
        return;
    }
    let (age, blocked) = (args.age, args.blocked);
    drive(
        ui,
        "recovery",
        &mut args.open,
        (age, blocked),
        || {
            RecoveryDialog::new(RecoveredDocument {
                key: "poster-2".to_owned(),
                name: "poster.psd".to_owned(),
                saved: SystemTime::now() - age.duration(),
                blocked: blocked.then(|| "Stop recording first.".to_owned()),
            })
        },
        |_| {},
        |action| format!("{action:?}"),
    );
}
