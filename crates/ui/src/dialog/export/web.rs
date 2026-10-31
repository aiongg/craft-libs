//! [`SaveForWebDialog`]: Save for Web (Legacy).

use std::ops::RangeInclusive;
use std::path::PathBuf;

use craft_host::{Host, PickId, PickRequest, Picked};
use craft_settings::Remembered;
use craft_theme::{Rgba, Srgba, TypeRole};
use egui::text::LayoutJob;
use egui::{Color32, Context, CornerRadius, Label, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2, vec2};
use serde::{Deserialize, Serialize};

use super::super::color_request::ColorRequest;
use super::super::dialog::{Dialog, Outcome};
use super::super::dialog_host::{DialogResult, StackDialog};
use super::super::footer::Footer;
use super::super::size::Width;
use super::super::sizing::{LinkedSizeFields, Resample, SizingLabels, human_bytes};
use super::super::stack::{DialogId, DialogStack, dialog_id};
use super::super::state::DialogState;
use super::resize_from_scale;
use crate::canvas::Checkerboard;
use crate::composites::{Form, FormGrid, SectionLabel};
use crate::controls::{Button, Checkbox, ControlSize, IconButton, NumberField, Segmented, Select, Swatch, TextField, checkerboard};
use crate::edit::{EditEvent, EditResponse};
use crate::icons::Icon;
use crate::localized::{LocaleNumbers, SharedLabels as _, Unit};
use crate::primitives::Content;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};

crate::words! {
    /// The words of Save for Web, English by default.
    #[non_exhaustive]
    pub struct WebLabels {
        title: "web.title" = "Save for Web (Legacy)";
        /// The accessible name of the view tabs.
        view: "web.view" = "Preview";
        format: "web.format" = "Format";
        /// The accessible name of the zoom field.
        zoom: "web.zoom" = "Zoom";
        /// The Preset list's entry for settings no preset has.
        unnamed: "web.unnamed" = "[Unnamed]";
        pan_hint: "web.panHint" = "Drag to pan · Scroll to zoom · Double-click to fit";
        save: "web.save" = "Save…";
        original: "web.original" = "Original";
        optimized: "web.optimized" = "Optimized";
        two_up: "web.twoUp" = "2-Up";
        four_up: "web.fourUp" = "4-Up";
        preset: "web.preset" = "Preset";
        color_table: "web.colorTable" = "Color Table";
        perceptual: "web.perceptual" = "Perceptual";
        selective: "web.selective" = "Selective";
        adaptive: "web.adaptive" = "Adaptive";
        restrictive: "web.restrictive" = "Restrictive (Web)";
        exact: "web.exact" = "Exact";
        colors: "web.colors" = "Colors";
        dither: "web.dither" = "Dither";
        no_dither: "web.noDither" = "No Dither";
        diffusion: "web.diffusion" = "Diffusion";
        pattern: "web.pattern" = "Pattern";
        noise: "web.noise" = "Noise";
        amount: "web.amount" = "Amount";
        transparency: "web.transparency" = "Transparency";
        interlaced: "web.interlaced" = "Interlaced";
        web_snap: "web.webSnap" = "Web Snap";
        quality: "web.quality" = "Quality";
        progressive: "web.progressive" = "Progressive";
        optimized_box: "web.optimizedBox" = "Optimized";
        lossless: "web.lossless" = "Lossless";
        embed_profile: "web.embedProfile" = "Embed Color Profile";
        matte: "web.matte" = "Matte";
        convert_srgb: "web.convertSrgb" = "Convert to sRGB";
        metadata: "web.metadata" = "Metadata";
        metadata_none: "web.metadataNone" = "None";
        metadata_copyright: "web.metadataCopyright" = "Copyright";
        metadata_contact: "web.metadataContact" = "Copyright and Contact Info";
        metadata_all: "web.metadataAll" = "All";
        image_size: "web.imageSize" = "Image Size";
        percent: "web.percent" = "Percent";
        fit: "web.fit" = "Fit";
        zoom_in: "web.zoomIn" = "Zoom In";
        zoom_out: "web.zoomOut" = "Zoom Out";
        actual: "web.actual" = "100%";
        play: "web.play" = "Play";
        pause: "web.pause" = "Pause";
        /// `{frame}` of `{total}` animation frames.
        frame: "web.frame" = "{frame} / {total}";
        /// `{name}`, `{width}` and `{height}`: the Original pane's second caption line (the first is
        /// "Original").
        original_caption: "web.originalCaption" = "{name}  ·  {width} × {height} px";
        /// `{format}` and `{size}`: an encoded pane's first caption line ("JPEG  48.2K").
        encoded_caption: "web.encodedCaption" = "{format}  {size}";
        /// `{seconds}`: the second caption line.
        download: "web.download" = "{seconds} sec @ 56.6 Kbps";
        /// `{count}`: appended to the second line for indexed formats.
        color_count: "web.colorCount" = "  ·  {count} colors";
        /// Appended to the first line when the pane shows a reduced proxy.
        proxy: "web.proxy" = "  ·  Proxy preview";
    }
}

/// The formats of Save for Web.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebFormat {
    Gif,
    Png8,
    Png24,
    #[default]
    Jpeg,
    WebP,
    Wbmp,
}

/// How an indexed format's palette is chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorTable {
    Perceptual,
    #[default]
    Selective,
    Adaptive,
    /// The 216 web-safe colours.
    Restrictive,
    /// The image's own colours, when it has at most 256.
    Exact,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dither {
    None,
    #[default]
    Diffusion,
    Pattern,
    Noise,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebMetadata {
    None,
    #[default]
    Copyright,
    CopyrightAndContact,
    All,
}

/// The view tabs over the panes. 2-Up is the default; the last tab is remembered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebView {
    /// The document alone.
    Original,
    /// The current settings' encode alone.
    Optimized,
    /// Original beside the current settings' encode.
    #[default]
    TwoUp,
    /// Original, the current settings and two cheaper variants ([`four_up_variants`]).
    FourUp,
}

/// Save for Web's settings: the result, what is remembered (`"export.saveForWeb"`), and what the
/// app encodes each variant with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebSettings {
    pub format: WebFormat,
    /// 0–100 (JPEG, lossy WebP).
    pub quality: u8,
    pub color_table: ColorTable,
    /// 2–256 (GIF, PNG-8).
    pub colors: u16,
    pub dither: Dither,
    /// 0–100.
    pub dither_amount: u8,
    pub transparency: bool,
    pub interlaced: bool,
    /// 0–100.
    pub web_snap: u8,
    pub progressive: bool,
    pub optimized: bool,
    pub lossless: bool,
    pub embed_profile: bool,
    /// `#rrggbb`.
    pub matte: [u8; 3],
    pub convert_srgb: bool,
    pub metadata: WebMetadata,
    /// The output size in percent of the document, 1–1000 with two decimals.
    pub percent: f64,
    pub resample: Resample,
}

impl Default for WebSettings {
    /// PhotoCraft's opening values: JPEG quality 60, Selective 128 colours, Diffusion 88 %,
    /// transparency, white matte, Optimized, Convert to sRGB, Copyright, 100 %, Bicubic.
    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:10 open_web (opening values)
    fn default() -> Self {
        Self {
            format: WebFormat::Jpeg,
            quality: 60,
            color_table: ColorTable::Selective,
            colors: 128,
            dither: Dither::Diffusion,
            dither_amount: 88,
            transparency: true,
            interlaced: false,
            web_snap: 0,
            progressive: false,
            optimized: true,
            lossless: false,
            embed_profile: false,
            matte: [255, 255, 255],
            convert_srgb: true,
            metadata: WebMetadata::Copyright,
            percent: 100.0,
            resample: Resample::Bicubic,
        }
    }
}

/// One encoded pane the app hands back for a variant the dialog asked for.
pub struct WebVariant {
    /// Which pane: 0 is the current settings, 1 and 2 the 4-Up variants.
    pub index: usize,
    /// The revision [`SaveForWebDialog::wanted_variants`] gave; older answers are dropped.
    pub revision: u64,
    pub texture: TextureHandle,
    /// The encoded file size.
    pub bytes: u64,
    /// The palette size of an indexed format.
    pub colors: Option<u16>,
    /// The palette, shown as the Color Table swatches under the settings (GIF, PNG-8).
    pub palette: Vec<[u8; 3]>,
    /// The texture is a reduced proxy (the long side is capped at 2048 px).
    pub proxy: bool,
}

/// What Save for Web returns.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveForWebResult {
    pub settings: WebSettings,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

/// File › Export › Save for Web (Legacy) (spec Q-S4F-5). Width 900.
///
/// The user sees the view tabs (Original, Optimized, 2-Up, 4-Up; 2-Up first time, then the last
/// one used) over the panes, a zoom row under them (Fit, −, the percent field 0.01 %–3200 %, +,
/// 100%), the animation row when the document has two frames or more (Play/Pause, "3 / 12"), and
/// the settings column on the right: Preset (the app's named presets), the format list, the
/// format's options (GIF and PNG-8: Color Table, Colors 2–256, Dither with Amount, Transparency,
/// Interlaced, Web Snap, then the palette swatches; JPEG: Quality, Progressive, Optimized, Embed
/// Color Profile; WebP: Lossless, Quality when lossy, Transparency, Embed Color Profile; WBMP:
/// Dither; PNG-24: Transparency, Interlaced), Matte (hidden for PNG-24 and WebP with
/// transparency), Convert to sRGB, Metadata, and Image Size ([`LinkedSizeFields`] in pixels with
/// Percent and Resample: Nearest Neighbor, Bilinear, Bicubic, Bicubic Smoother, Bicubic Sharper).
/// Buttons: Save… and Cancel.
///
/// Each pane has two caption lines: "Original" and "poster  ·  1200 × 800 px"; an encoded pane
/// "JPG  48.2K" (the file extension upper-cased and [`human_bytes`](super::super::human_bytes))
/// and "9 sec @ 56.6 Kbps", with "  ·  128 colors" for indexed formats; "  ·  Proxy preview"
/// follows the first line when the pane is reduced ([`WebVariant::proxy`]). The panes share one
/// zoom and centre: the wheel zooms by `exp(scroll × 0.005)` at the pointer, a pinch zooms, a drag
/// pans, a double-click fits. The zoom is kept between 0.0001 and 32.
///
/// Encoding is the app's: [`wanted_variants`](Self::wanted_variants) lists the panes whose
/// settings changed with a new revision; the app encodes them on a job and returns
/// [`WebVariant`]s with [`set_variant`](Self::set_variant) through
/// `dialogs.get_mut::<SaveForWebDialog>()`. Save… queues a save picker through `host.files()`
/// (purpose `"export"`, the format's extension); a path finishes the dialog with
/// [`SaveForWebResult`].
pub struct SaveForWebDialog {
    id: DialogId,
    name: String,
    fields: LinkedSizeFields,
    settings: WebSettings,
    presets: Vec<(String, WebSettings)>,
    view: WebView,
    frames: u32,
    fps: f32,
    frame: u32,
    playing: bool,
    /// The clock time the frame last advanced at.
    clock: f64,
    revision: u64,
    taken: u64,
    variants: [Option<WebVariant>; 3],
    original: Option<TextureHandle>,
    /// Shared pane zoom and centre; becomes the preview pane's state (F06.01) when that lands.
    nav: Nav,
    picking: Option<PickId>,
    /// The request to hand out once.
    color_request: Option<ColorRequest>,
    recalled: bool,
    state: DialogState<()>,
}

impl SaveForWebDialog {
    /// The dialog for the document `name` of `width` × `height` pixels, showing `original` in the
    /// Original pane.
    pub fn new(name: impl Into<String>, width: u32, height: u32, original: TextureHandle) -> Self {
        let size = super::super::sizing::PixelSize { width, height, pixels_per_inch: 72.0 };
        Self {
            id: dialog_id("save-for-web"),
            name: name.into(),
            fields: LinkedSizeFields::new(size),
            settings: WebSettings::default(),
            presets: Vec::new(),
            view: WebView::TwoUp,
            frames: 1,
            fps: 30.0,
            frame: 0,
            playing: false,
            clock: 0.0,
            revision: 1,
            taken: 0,
            variants: [None, None, None],
            original: Some(original),
            nav: Nav::FIT,
            picking: None,
            color_request: None,
            recalled: false,
            state: DialogState::new(),
        }
    }

    /// The Preset list: PhotoCraft's twelve ("JPEG High", "PNG-24", "GIF 128 Dithered", …) come
    /// from its engine.
    pub fn presets(mut self, presets: Vec<(String, WebSettings)>) -> Self {
        self.presets = presets;
        self
    }

    /// The document is an animation of `frames` frames at `fps` (30 when the timeline has none):
    /// the play row shows and [`frame`](Self::frame) advances while playing.
    pub fn animation(mut self, frames: u32, fps: f32) -> Self {
        self.frames = frames;
        self.fps = fps;
        self
    }

    /// The frame the panes show; the app encodes this frame.
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// The Original pane's picture, for the app to hand over the new frame's when
    /// [`frame`](Self::frame) changed.
    pub fn set_original(&mut self, original: TextureHandle) {
        self.original = Some(original);
    }

    /// The panes to encode, `(index, revision, settings)`, when they changed since the last call:
    /// pane 0 in every view but Original, panes 1 and 2 in 4-Up ([`four_up_variants`]).
    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:61 variants (which panes a view draws)
    pub fn wanted_variants(&mut self) -> Vec<(usize, u64, WebSettings)> {
        if self.revision == self.taken {
            return Vec::new();
        }
        self.taken = self.revision;
        let revision = self.revision;
        let mut wanted = Vec::new();
        if self.view != WebView::Original {
            wanted.push((0, revision, self.settings.clone()));
        }
        if self.view == WebView::FourUp {
            let [half, quarter] = four_up_variants(&self.settings);
            wanted.push((1, revision, half));
            wanted.push((2, revision, quarter));
        }
        wanted
    }

    /// The Matte swatch was pressed in the pass since the last call: the colour to ask the app's
    /// picker for, keyed [`MATTE_COLOR_KEY`]. Returned once; answer with
    /// [`set_color`](Self::set_color).
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: the matte, without alpha. Other keys are
    /// ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == MATTE_COLOR_KEY {
            let Rgba([r, g, b, _]) = color.to_rgba();
            self.settings.matte = [r, g, b];
            self.revision += 1;
        }
    }

    /// An encoded pane; one for an older revision is dropped.
    pub fn set_variant(&mut self, variant: WebVariant) {
        if variant.revision < self.taken || variant.index >= self.variants.len() {
            return;
        }
        let index = variant.index;
        self.variants[index] = Some(variant);
    }

    /// Loads what the last Save for Web remembered, once: the view and the settings.
    fn recall(&mut self, ctx: &Context) {
        if std::mem::replace(&mut self.recalled, true) {
            return;
        }
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let Some(last) = Remembered::<Last>::new(REMEMBER_KEY).recall(&host) else { return };
        self.view = last.view;
        self.settings = WebSettings { percent: 100.0, ..last.settings };
        self.revision += 1;
    }

    fn remember(&self, ctx: &Context, settings: Option<&WebSettings>) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let remembered = Remembered::<Last>::new(REMEMBER_KEY);
        let settings = settings.cloned().or_else(|| remembered.recall(&host).map(|last| last.settings)).unwrap_or_default();
        remembered.store(&host, &Last { view: self.view, settings });
    }

    /// The picker's answer, once it has come: the first path.
    fn picked(&mut self, ctx: &Context) -> Option<PathBuf> {
        let ticket = self.picking?;
        let host = Host::try_of_ctx(ctx)?;
        let answer = host.files().take_pick()?;
        if answer.id != ticket {
            host.files().return_pick(answer);
            return None;
        }
        self.picking = None;
        match answer.picked {
            Picked::Paths(paths) => paths.into_iter().next(),
            Picked::Contents(files) => files.into_iter().next().map(|(name, _)| PathBuf::from(name)),
            Picked::Cancelled => None,
        }
    }

    /// Queues the save picker for the current format.
    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:504 web_confirm
    fn queue_save(&mut self, ctx: &Context, title: &str) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let extension = self.settings.format.extension();
        let stem = std::path::Path::new(&self.name).file_stem().map_or_else(|| self.name.clone(), |stem| stem.to_string_lossy().into_owned());
        let request = PickRequest::save("export")
            .title(title)
            .file_name(format!("{stem}.{extension}"))
            .suggested_extension(extension)
            .filter(self.settings.format.name(), &[extension]);
        self.picking = Some(host.files().pick(request));
    }

    /// Moves the playing animation on by the time that has passed.
    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:207 web_playback
    fn advance(&mut self, ctx: &Context) {
        if self.frames < 2 {
            return;
        }
        let now = ctx.input(|i| i.time);
        self.frame = self.frame.min(self.frames - 1);
        if !self.playing {
            self.clock = now;
            return;
        }
        let (frame, clock) = advance_frame(self.frame, self.frames, self.fps, self.clock, now);
        self.frame = frame;
        self.clock = clock;
        let fps = if self.fps.is_finite() && self.fps > 0.0 { f64::from(self.fps) } else { 30.0 };
        ctx.request_repaint_after(std::time::Duration::from_secs_f64((1.0 / fps - (now - clock)).clamp(0.0, 1.0)));
    }
}

impl StackDialog for SaveForWebDialog {
    type Result = SaveForWebResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:89 web_pane, :207 web_playback, :246 web_body (settings column), :439 web_size, :504 web_confirm;
    // pc: crates/ui-egui/src/dialogs/file_ui/web_preview.rs:169 Navigation
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<SaveForWebResult>> {
        self.recall(ctx);
        if let Some(path) = self.picked(ctx) {
            self.remember(ctx, Some(&self.settings));
            self.state.close();
            let size = self.fields.current;
            return Some(DialogResult::Ok(SaveForWebResult { settings: self.settings.clone(), path, width: size.width, height: size.height }));
        }
        self.advance(ctx);
        let labels = WebLabels::shared_ctx(ctx);
        let sizing = SizingLabels::shared_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let before = (self.settings.clone(), self.fields.current, self.view, self.frame);
        let saving = self.picking.is_some();
        let tokens = crate::theme::current(ctx);
        let height = tokens.component.dialog.preview_pane[1] * 2.0;
        let (width, settings_width) = (tokens.component.export.web_width, tokens.component.export.web_settings_width);
        let Self { name, fields, settings, presets, view, frames, frame, playing, variants, original, nav, state, color_request, .. } = self;
        let shown = Dialog::with_id(self.id, labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(width))
            .footer(Footer::standard().ok(labels.save.as_str()).ok_enabled(!saving))
            .show(ctx, state, &mut (), |ui, ()| {
                let mut preview = Preview { name, fields, view, frames: *frames, frame, playing, variants, original: original.as_ref(), nav, height };
                preview.controls(ui, &labels);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - settings_width - ui.spacing().item_spacing.x);
                        preview.panes(ui, settings, &labels);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(settings_width);
                        let palette = preview.variants[0].as_ref().map_or(&[][..], |variant| variant.palette.as_slice());
                        egui::ScrollArea::vertical().id_salt("web-settings").max_height(preview.height).show(ui, |ui| {
                            settings_column(ui, preview.fields, settings, presets, palette, &labels, &sizing, &numbers, color_request);
                        });
                    });
                });
            });
        let size_changed = before.1 != self.fields.current;
        if size_changed {
            let original = f64::from(self.fields.original.width.max(1));
            self.settings.percent = (f64::from(self.fields.current.width) / original * 10_000.0).round() / 100.0;
        }
        if before.0 != self.settings || before.1 != self.fields.current || before.2 != self.view || before.3 != self.frame {
            self.revision += 1;
        }
        if before.2 != self.view {
            self.remember(ctx, None);
        }
        match shown.outcome {
            Some(Outcome::Ok) => {
                self.queue_save(ctx, &labels.title);
                None
            }
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

/// What Save for Web remembers under [`REMEMBER_KEY`].
#[derive(Serialize, Deserialize)]
struct Last {
    view: WebView,
    settings: WebSettings,
}

const REMEMBER_KEY: &str = "export.saveForWeb";

/// The zoom limits of the panes.
const ZOOM_RANGE: (f32, f32) = (0.0001, 32.0);

impl WebFormat {
    fn extension(self) -> &'static str {
        match self {
            WebFormat::Gif => "gif",
            WebFormat::Png8 | WebFormat::Png24 => "png",
            WebFormat::Jpeg => "jpg",
            WebFormat::WebP => "webp",
            WebFormat::Wbmp => "wbmp",
        }
    }

    /// The format list's entry.
    fn name(self) -> &'static str {
        match self {
            WebFormat::Gif => "GIF",
            WebFormat::Png8 => "PNG-8",
            WebFormat::Png24 => "PNG-24",
            WebFormat::Jpeg => "JPEG",
            WebFormat::WebP => "WebP",
            WebFormat::Wbmp => "WBMP",
        }
    }

    fn indexed(self) -> bool {
        matches!(self, WebFormat::Gif | WebFormat::Png8)
    }
}

/// The panes' shared view: the zoom (`None` fits) and the centre as a fraction of the image.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Nav {
    zoom: Option<f32>,
    center: Vec2,
    /// The scale the panes drew at last pass, in device pixels per image pixel.
    effective: f32,
}

impl Nav {
    const FIT: Nav = Nav { zoom: None, center: vec2(0.5, 0.5), effective: 1.0 };

    /// Points per image pixel.
    fn scale(self, viewport: Vec2, source: Vec2, ppp: f32) -> f32 {
        self.zoom.map_or((viewport.x / source.x).min(viewport.y / source.y), |zoom| zoom / ppp)
    }

    fn clamp(&mut self, viewport: Vec2, image: Vec2) {
        let half = (viewport / image * 0.5).min(vec2(0.5, 0.5));
        self.center = self.center.clamp(half, Vec2::splat(1.0) - half);
    }

    /// Zooms by `factor` keeping the image point under `at` (from the viewport's centre) still.
    fn zoom_at(&mut self, factor: f32, at: Vec2, viewport: Vec2, source: Vec2, ppp: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let old = self.scale(viewport, source, ppp);
        self.zoom = Some((old * ppp * factor).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1));
        let new = self.scale(viewport, source, ppp);
        self.center += at / source * (1.0 / old - 1.0 / new);
        self.clamp(viewport, source * new);
    }
}

/// `frame` after the time from `last` to `now`, and the clock to carry on from; the fraction of a
/// frame that has not yet passed is kept.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:171 advance_preview
fn advance_frame(frame: u32, frames: u32, fps: f32, last: f64, now: f64) -> (u32, f64) {
    let frames = frames.max(1);
    let frame = frame.min(frames - 1);
    if !now.is_finite() {
        return (frame, last);
    }
    if !last.is_finite() || now < last {
        return (frame, now);
    }
    let fps = if fps.is_finite() && fps > 0.0 { f64::from(fps) } else { 30.0 };
    let elapsed = (now - last) * fps;
    if !elapsed.is_finite() {
        return (frame, now);
    }
    let steps = elapsed.floor();
    if steps < 1.0 {
        return (frame, last);
    }
    let delta = ((steps % f64::from(frames)) as u32) % frames;
    (((frame + delta) % frames), now - (elapsed - steps) / fps)
}

/// A caption line in the caption role.
pub(super) fn caption(ui: &mut Ui, text: String, content: Content) {
    let tokens = Tokens::of(ui);
    let format = role_format(&tokens, TypeRole::Caption, content.color(&tokens).to_egui());
    ui.add(Label::new(LayoutJob::single_section(text, format)).truncate().selectable(false));
}

/// The part of the dialog that shows the document: tabs, play and zoom rows, and the panes.
struct Preview<'a> {
    name: &'a str,
    fields: &'a mut LinkedSizeFields,
    view: &'a mut WebView,
    frames: u32,
    frame: &'a mut u32,
    playing: &'a mut bool,
    variants: &'a [Option<WebVariant>; 3],
    original: Option<&'a TextureHandle>,
    nav: &'a mut Nav,
    /// The panes' total height.
    height: f32,
}

impl Preview<'_> {
    fn controls(&mut self, ui: &mut Ui, labels: &WebLabels) {
        let gap = crate::theme::current(ui.ctx()).space.gap;
        Segmented::new("web-view", labels.view.as_str(), self.view)
            .item(WebView::Original, labels.original.as_str())
            .item(WebView::Optimized, labels.optimized.as_str())
            .item(WebView::TwoUp, labels.two_up.as_str())
            .item(WebView::FourUp, labels.four_up.as_str())
            .show(ui);
        ui.add_space(gap.row);
        if self.frames >= 2 {
            ui.horizontal(|ui| {
                let text = if *self.playing { &labels.pause } else { &labels.play };
                if ui.add(Button::new(text.as_str())).clicked() {
                    *self.playing = !*self.playing;
                }
                let (frame, total) = ((*self.frame + 1).to_string(), self.frames.to_string());
                ui.label(craft_i18n::fmt(&labels.frame, &[("frame", frame.as_str()), ("total", total.as_str())]));
            });
        }
        let numbers = LocaleNumbers::of(ui);
        ui.horizontal(|ui| {
            if ui.add(Button::new(labels.fit.as_str())).clicked() {
                *self.nav = Nav { effective: self.nav.effective, ..Nav::FIT };
            }
            if ui.add(IconButton::new(Icon::Minus, labels.zoom_out.as_str())).clicked() {
                self.nav.zoom = Some((self.nav.zoom.unwrap_or(self.nav.effective) / 1.25).max(ZOOM_RANGE.0));
            }
            if ui.add(IconButton::new(Icon::Plus, labels.zoom_in.as_str())).clicked() {
                self.nav.zoom = Some((self.nav.zoom.unwrap_or(self.nav.effective) * 1.25).min(ZOOM_RANGE.1));
            }
            let shown = f64::from(self.nav.zoom.unwrap_or(self.nav.effective)) * 100.0;
            let edit = NumberField::new("web-zoom", labels.zoom.as_str(), shown, &numbers)
                .range(0.01..=3200.0)
                .decimals(0..=2)
                .unit(Unit::new("{n}%"))
                .show(ui);
            if let Some(percent) = edit.event.as_ref().and_then(EditEvent::value) {
                self.nav.zoom = Some((*percent as f32 / 100.0).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1));
            }
            if ui.add(Button::new(labels.actual.as_str())).clicked() {
                self.nav.zoom = Some(1.0);
            }
            caption(ui, labels.pan_hint.clone(), Content::Faint);
        });
        ui.add_space(gap.row);
    }

    fn panes(&mut self, ui: &mut Ui, settings: &WebSettings, labels: &WebLabels) {
        let width = ui.available_width();
        let gap = ui.spacing().item_spacing;
        let full = vec2(width, self.height);
        let half = vec2((width - gap.x) / 2.0, (self.height - gap.y) / 2.0);
        let original = Pane::Original;
        match *self.view {
            WebView::Original => self.pane(ui, full, original, settings, labels),
            WebView::Optimized => self.pane(ui, full, Pane::Encoded(0), settings, labels),
            WebView::TwoUp => {
                let size = vec2(half.x, self.height);
                ui.horizontal(|ui| {
                    self.pane(ui, size, original, settings, labels);
                    self.pane(ui, size, Pane::Encoded(0), settings, labels);
                });
            }
            WebView::FourUp => {
                ui.horizontal(|ui| {
                    self.pane(ui, half, original, settings, labels);
                    self.pane(ui, half, Pane::Encoded(0), settings, labels);
                });
                ui.horizontal(|ui| {
                    self.pane(ui, half, Pane::Encoded(1), settings, labels);
                    self.pane(ui, half, Pane::Encoded(2), settings, labels);
                });
            }
        }
    }

    /// One pane of `size` with its two caption lines under the picture.
    // pc: crates/ui-egui/src/dialogs/file_ui/web.rs:89 web_pane (captions :148), web_preview.rs:197 paint
    fn pane(&mut self, ui: &mut Ui, size: Vec2, pane: Pane, settings: &WebSettings, labels: &WebLabels) {
        let tokens = Tokens::of(ui);
        let line = tokens.typography.caption.line_height;
        let (texture, lines) = match pane {
            Pane::Original => {
                let (width, height) = (self.fields.original.width.to_string(), self.fields.original.height.to_string());
                let second = craft_i18n::fmt(&labels.original_caption, &[("name", self.name), ("width", &width), ("height", &height)]);
                (self.original, [labels.original.clone(), second])
            }
            Pane::Encoded(index) => match &self.variants[index] {
                Some(variant) => {
                    let format = if index == 0 { settings.format } else { four_up_variants(settings)[index - 1].format };
                    let size_text = human_bytes(variant.bytes as f64);
                    let extension = format.extension().to_uppercase();
                    let mut first = craft_i18n::fmt(&labels.encoded_caption, &[("format", &extension), ("size", &size_text)]);
                    if variant.proxy {
                        first.push_str(&labels.proxy);
                    }
                    let seconds = download_seconds(variant.bytes, DOWNLOAD_KBPS).to_string();
                    let mut second = craft_i18n::fmt(&labels.download, &[("seconds", &seconds)]);
                    if let Some(colors) = variant.colors {
                        second.push_str(&craft_i18n::fmt(&labels.color_count, &[("count", &colors.to_string())]));
                    }
                    (Some(&variant.texture), [first, second])
                }
                None => (None, [String::new(), String::new()]),
            },
        };
        let source = vec2(self.fields.current.width as f32, self.fields.current.height as f32);
        ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(size.x);
            let picture = vec2(size.x, (size.y - 2.0 * line).max(1.0));
            let (rect, response) = ui.allocate_exact_size(picture, Sense::drag());
            let response = response.on_hover_cursor(egui::CursorIcon::Grab);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, lines.join(", ")));
            paint_pane(ui, rect, &response, texture, source, self.nav);
            let [first, second] = lines;
            caption(ui, first, Content::Normal);
            caption(ui, second, Content::Muted);
        });
    }
}

#[derive(Clone, Copy)]
enum Pane {
    Original,
    /// The encoded pane, by [`WebVariant::index`].
    Encoded(usize),
}

/// Photoshop's modem estimate for the download time.
const DOWNLOAD_KBPS: f64 = 56.6;

/// Draws the picture of a pane and applies the pointer's pan and zoom to `nav`.
// pc: crates/ui-egui/src/dialogs/file_ui/web_preview.rs:197 paint
fn paint_pane(ui: &mut Ui, rect: Rect, response: &Response, texture: Option<&TextureHandle>, source: Vec2, nav: &mut Nav) {
    let ppp = ui.ctx().pixels_per_point();
    if response.double_clicked() {
        *nav = Nav { effective: nav.effective, ..Nav::FIT };
    }
    let mut scale = nav.scale(rect.size(), source, ppp);
    if response.dragged() {
        nav.zoom = Some(scale * ppp);
        nav.center -= response.drag_delta() / (source * scale);
    }
    if response.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = if pinch != 1.0 { pinch } else { (scroll * 0.005).exp() };
        if factor != 1.0 {
            let at = response.hover_pos().unwrap_or(rect.center()) - rect.center();
            nav.zoom_at(factor, at, rect.size(), source, ppp);
            scale = nav.scale(rect.size(), source, ppp);
            ui.ctx().request_repaint();
        }
    }
    nav.clamp(rect.size(), source * scale);
    nav.effective = scale * ppp;
    let image = Rect::from_min_size(rect.center() - nav.center * source * scale, source * scale);
    paint_picture(ui, rect, texture, image);
}

/// A picture over the checkerboard in `rect`, with the pane's edge; `image` is where the
/// texture's whole picture goes.
pub(super) fn paint_picture(ui: &mut Ui, rect: Rect, texture: Option<&TextureHandle>, image: Rect) {
    let ppp = ui.ctx().pixels_per_point();
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(rect);
    let board = Checkerboard::default();
    match board.size.square() {
        Some(square) => painter.extend(checkerboard(rect, CornerRadius::ZERO, board.colors.colors(), square)),
        None => {
            painter.rect_filled(rect, CornerRadius::ZERO, Color32::WHITE);
        }
    }
    if let Some(texture) = texture {
        painter.image(texture.id(), image, Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
    let edge = Stroke::new(craft_theme::Stroke::Hairline.width(ppp), tokens.color.line.divider.to_egui());
    painter.rect_stroke(rect, CornerRadius::ZERO, edge, egui::StrokeKind::Inside);
}

/// The value a number field reported this pass.
pub(super) fn typed(edit: &EditResponse<f64>) -> Option<f64> {
    edit.event.as_ref().and_then(EditEvent::value).copied()
}

/// A whole-number row with `range`; returns the typed value.
pub(super) fn number_row(form: &mut Form<'_>, id: &str, label: &str, numbers: &LocaleNumbers, value: f64, range: RangeInclusive<f64>) -> Option<f64> {
    let mut out = None;
    form.row(label, |ui, _| {
        let edit = NumberField::new(id, label, value, numbers).range(range).decimals(0..=0).show(ui);
        out = typed(&edit);
        edit.response
    });
    out
}

pub(super) fn check_row(form: &mut Form<'_>, label: &str, value: &mut bool) {
    form.unlabelled(|ui| Checkbox::bool(label, value).show(ui));
}

pub(super) fn clamp_number(value: f64, range: RangeInclusive<f64>) -> f64 {
    value.round().clamp(*range.start(), *range.end())
}

/// Whether `preset` is what the dialog shows apart from the size.
fn is_preset(preset: &WebSettings, settings: &WebSettings) -> bool {
    *preset == WebSettings { percent: preset.percent, resample: preset.resample, ..settings.clone() }
}

/// The key of the colour request the Matte swatch makes.
pub const MATTE_COLOR_KEY: &str = "web.matte";

/// `#rrggbb` as three bytes.
fn parse_matte(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// The settings column: Preset, Format, the format's options, Matte, Convert to sRGB, Metadata,
/// the palette and Image Size.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:246 web_body (settings column), :439 web_size
#[expect(clippy::too_many_arguments, reason = "the column's inputs are separate borrows of the dialog")]
fn settings_column(
    ui: &mut Ui,
    size: &mut LinkedSizeFields,
    settings: &mut WebSettings,
    presets: &[(String, WebSettings)],
    palette: &[[u8; 3]],
    labels: &WebLabels,
    sizing: &SizingLabels,
    numbers: &LocaleNumbers,
    ask: &mut Option<ColorRequest>,
) {
    let gap = crate::theme::current(ui.ctx()).space.gap;
    FormGrid::new("web-settings").show(ui, |form| {
        let mut preset = presets.iter().position(|(_, candidate)| is_preset(candidate, settings));
        let chosen = preset;
        let mut select = Select::new("web-preset", labels.preset.as_str(), &mut preset).option(None, labels.unnamed.as_str());
        for (index, (name, _)) in presets.iter().enumerate() {
            select = select.option(Some(index), name.as_str());
        }
        form.row(&labels.preset, |ui, _| select.show(ui).response);
        if preset != chosen
            && let Some(index) = preset
        {
            let (percent, resample) = (settings.percent, settings.resample);
            *settings = WebSettings { percent, resample, ..presets[index].1.clone() };
        }

        form.row(&labels.format, |ui, _| {
            let mut select = Select::new("web-format", labels.format.as_str(), &mut settings.format);
            for format in [WebFormat::Gif, WebFormat::Png8, WebFormat::Png24, WebFormat::Jpeg, WebFormat::WebP, WebFormat::Wbmp] {
                select = select.option(format, format.name());
            }
            select.show(ui).response
        });
        form.space(gap.row);

        match settings.format {
            WebFormat::Gif | WebFormat::Png8 => {
                form.row(&labels.color_table, |ui, _| {
                    let mut select = Select::new("web-color-table", labels.color_table.as_str(), &mut settings.color_table);
                    for (value, text) in [
                        (ColorTable::Perceptual, &labels.perceptual),
                        (ColorTable::Selective, &labels.selective),
                        (ColorTable::Adaptive, &labels.adaptive),
                        (ColorTable::Restrictive, &labels.restrictive),
                        (ColorTable::Exact, &labels.exact),
                    ] {
                        select = select.option(value, text.as_str());
                    }
                    select.show(ui).response
                });
                if let Some(value) = number_row(form, "web-colors", &labels.colors, numbers, f64::from(settings.colors), 2.0..=256.0) {
                    settings.colors = clamp_number(value, 2.0..=256.0) as u16;
                }
                dither_row(form, settings, labels, true);
                if let Some(value) = number_row(form, "web-dither-amount", &labels.amount, numbers, f64::from(settings.dither_amount), 0.0..=100.0) {
                    settings.dither_amount = clamp_number(value, 0.0..=100.0) as u8;
                }
                check_row(form, &labels.transparency, &mut settings.transparency);
                check_row(form, &labels.interlaced, &mut settings.interlaced);
                if let Some(value) = number_row(form, "web-snap", &labels.web_snap, numbers, f64::from(settings.web_snap), 0.0..=100.0) {
                    settings.web_snap = clamp_number(value, 0.0..=100.0) as u8;
                }
            }
            WebFormat::Jpeg => {
                quality_row(form, settings, labels, numbers);
                check_row(form, &labels.progressive, &mut settings.progressive);
                check_row(form, &labels.optimized_box, &mut settings.optimized);
                check_row(form, &labels.embed_profile, &mut settings.embed_profile);
            }
            WebFormat::WebP => {
                check_row(form, &labels.lossless, &mut settings.lossless);
                if !settings.lossless {
                    quality_row(form, settings, labels, numbers);
                }
                check_row(form, &labels.transparency, &mut settings.transparency);
                check_row(form, &labels.embed_profile, &mut settings.embed_profile);
            }
            WebFormat::Wbmp => dither_row(form, settings, labels, false),
            WebFormat::Png24 => {
                check_row(form, &labels.transparency, &mut settings.transparency);
                check_row(form, &labels.interlaced, &mut settings.interlaced);
            }
        }

        if !matches!(settings.format, WebFormat::Png24 | WebFormat::WebP) || !settings.transparency {
            form.row(&labels.matte, |ui, _| {
                let [r, g, b] = settings.matte;
                let hex = format!("#{r:02x}{g:02x}{b:02x}");
                ui.horizontal(|ui| {
                    if ui.add(Swatch::new("web-matte-swatch", labels.matte.as_str(), Rgba([r, g, b, 255]))).clicked() {
                        *ask = Some(ColorRequest::new(MATTE_COLOR_KEY, Srgba::from(Rgba([r, g, b, 255]))).title(labels.matte.as_str()));
                    }
                    let edit = TextField::new("web-matte", labels.matte.as_str(), &hex)
                        .width(crate::theme::current(ui.ctx()).component.export.web_field_width)
                        .show(ui);
                    if let Some(matte) = edit.committed().and_then(|text| parse_matte(text)) {
                        settings.matte = matte;
                    }
                    edit.response
                })
                .inner
            });
        }
        form.space(gap.row);
        check_row(form, &labels.convert_srgb, &mut settings.convert_srgb);
        form.row(&labels.metadata, |ui, _| {
            let mut select = Select::new("web-metadata", labels.metadata.as_str(), &mut settings.metadata);
            for (value, text) in [
                (WebMetadata::None, &labels.metadata_none),
                (WebMetadata::Copyright, &labels.metadata_copyright),
                (WebMetadata::CopyrightAndContact, &labels.metadata_contact),
                (WebMetadata::All, &labels.metadata_all),
            ] {
                select = select.option(value, text.as_str());
            }
            select.show(ui).response
        });
    });

    if settings.format.indexed() && !palette.is_empty() {
        ui.add_space(gap.row);
        let _ = SectionLabel::new(&labels.color_table).show(ui);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let side = Swatch::style(&Tokens::of(ui), ControlSize::Sm).side;
            for (index, [r, g, b]) in palette.iter().enumerate() {
                let label = format!("#{r:02x}{g:02x}{b:02x}");
                ui.add(Swatch::new(("web-palette", index), label, Rgba::rgb(*r, *g, *b)).table_cell(side).tab_stop(false));
            }
        });
    }

    ui.add_space(gap.section);
    let _ = SectionLabel::new(&labels.image_size).show(ui);
    FormGrid::new("web-size").show(ui, |form| {
        let _ = size.rows(form, sizing, false);
        let (width, height) = (size.original.width, size.original.height);
        let percent = f64::from(size.current.width) / f64::from(width.max(1)) * 100.0;
        form.row(&labels.percent, |ui, _| {
            let edit = NumberField::new("web-percent", labels.percent.as_str(), percent, numbers).range(1.0..=1000.0).decimals(0..=2).show(ui);
            if let Some(value) = typed(&edit)
                && let Ok((w, h)) = resize_from_scale((width, height), value)
            {
                size.current.width = w;
                size.current.height = h;
            }
            edit.response
        });
        form.row(&sizing.resample_method, |ui, _| {
            let mut select = Select::new("web-resample", sizing.resample_method.as_str(), &mut settings.resample);
            for (value, text) in [
                (Resample::NearestNeighbor, &sizing.nearest_neighbor),
                (Resample::Bilinear, &sizing.bilinear),
                (Resample::Bicubic, &sizing.bicubic),
                (Resample::BicubicSmoother, &sizing.bicubic_smoother),
                (Resample::BicubicSharper, &sizing.bicubic_sharper),
            ] {
                select = select.option(value, text.as_str());
            }
            select.show(ui).response
        });
    });
}

fn quality_row(form: &mut Form<'_>, settings: &mut WebSettings, labels: &WebLabels, numbers: &LocaleNumbers) {
    if let Some(value) = number_row(form, "web-quality", &labels.quality, numbers, f64::from(settings.quality), 0.0..=100.0) {
        settings.quality = clamp_number(value, 0.0..=100.0) as u8;
    }
}

/// The Dither list; WBMP has no Noise.
fn dither_row(form: &mut Form<'_>, settings: &mut WebSettings, labels: &WebLabels, noise: bool) {
    form.row(&labels.dither, |ui, _| {
        let mut select = Select::new("web-dither", labels.dither.as_str(), &mut settings.dither)
            .option(Dither::None, labels.no_dither.as_str())
            .option(Dither::Diffusion, labels.diffusion.as_str())
            .option(Dither::Pattern, labels.pattern.as_str());
        if noise {
            select = select.option(Dither::Noise, labels.noise.as_str());
        }
        select.show(ui).response
    });
}

/// 4-Up's two extra panes: JPEG and lossy WebP at half and a quarter of the quality; GIF and
/// PNG-8 at half and a quarter of the colours; any other format as PNG-8 at 128 colours and GIF
/// at 64 colours.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:61 variants
pub fn four_up_variants(settings: &WebSettings) -> [WebSettings; 2] {
    [2u16, 4].map(|divisor| {
        let mut variant = settings.clone();
        match settings.format {
            WebFormat::Jpeg | WebFormat::WebP if !(settings.format == WebFormat::WebP && settings.lossless) => {
                variant.quality = (f64::from(settings.quality) / f64::from(divisor)).round().max(1.0) as u8;
            }
            WebFormat::Gif | WebFormat::Png8 => {
                variant.colors = (f64::from(settings.colors) / f64::from(divisor)).round().max(2.0) as u16;
            }
            _ => {
                variant.format = if divisor == 2 { WebFormat::Png8 } else { WebFormat::Gif };
                variant.colors = 256 / divisor;
            }
        }
        variant
    })
}

/// Seconds to download `bytes` at `kbps` kilobits per second, rounded up: `ceil(bytes × 8 /
/// (kbps × 1000))`; the caption uses 56.6, Photoshop's modem estimate.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:166 (download estimate in web_pane)
pub fn download_seconds(bytes: u64, kbps: f64) -> u64 {
    (bytes as f64 * 8.0 / (kbps * 1000.0)).ceil() as u64
}
