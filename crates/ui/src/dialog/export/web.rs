//! [`SaveForWebDialog`]: Save for Web (Legacy).

use std::path::PathBuf;

use egui::{Context, TextureHandle};
use serde::{Deserialize, Serialize};

use super::super::dialog_host::{DialogResult, StackDialog};
use super::super::sizing::{LinkedSizeFields, Resample};
use super::super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The words of Save for Web, English by default.
    #[non_exhaustive]
    pub struct WebLabels {
        title: "web.title" = "Save for Web (Legacy)";
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
/// "JPEG  48.2K" (the extension upper-cased and [`human_bytes`](super::super::human_bytes)) and
/// "9 sec @ 56.6 Kbps", with "  ·  128 colors" for indexed formats; "  ·  Proxy preview" follows
/// the first line when the pane is reduced (the proxy is more than 0.1 % smaller). The panes share one zoom and centre: the wheel zooms by
/// `exp(scroll × 0.005)` at the pointer, a pinch zooms, a drag pans, a double-click fits. The
/// zoom is kept between 0.0001 and 32.
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
    revision: u64,
    taken: u64,
    variants: [Option<WebVariant>; 3],
    original: Option<TextureHandle>,
    /// Shared pane zoom (`None` fits) and centre in image pixels; becomes the preview pane's
    /// state (F06.01) when that lands.
    zoom: Option<f32>,
    center: [f32; 2],
    picking: Option<craft_host::PickId>,
    recalled: bool,
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
            revision: 1,
            taken: 0,
            variants: [None, None, None],
            original: Some(original),
            zoom: None,
            center: [0.0, 0.0],
            picking: None,
            recalled: false,
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

    /// The panes to encode, `(index, revision, settings)`, when they changed since the last call:
    /// pane 0 in every view but Original, panes 1 and 2 in 4-Up ([`four_up_variants`]).
    pub fn wanted_variants(&mut self) -> Vec<(usize, u64, WebSettings)> {
        todo!()
    }

    /// An encoded pane; one for an older revision is dropped.
    pub fn set_variant(&mut self, variant: WebVariant) {
        todo!()
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
        todo!()
    }
}

/// 4-Up's two extra panes: JPEG and lossy WebP at half and a quarter of the quality; GIF and
/// PNG-8 at half and a quarter of the colours; any other format as PNG-8 at 128 colours and GIF
/// at 64 colours.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:61 variants
pub fn four_up_variants(settings: &WebSettings) -> [WebSettings; 2] {
    todo!()
}

/// Seconds to download `bytes` at `kbps` kilobits per second, rounded up: `ceil(bytes × 8 /
/// (kbps × 1000))`; the caption uses 56.6, Photoshop's modem estimate.
// pc: crates/ui-egui/src/dialogs/file_ui/web.rs:166 (download estimate in web_pane)
pub fn download_seconds(bytes: u64, kbps: f64) -> u64 {
    todo!()
}
