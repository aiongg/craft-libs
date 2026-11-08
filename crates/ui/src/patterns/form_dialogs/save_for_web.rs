//! `SaveForWebDialog`: the Save for Web dialog on `Dialog`, and `HostedSaveForWeb`, the same dialog owned by a
//! `DialogHost`. The app encodes: the dialog asks for the panes it shows with a revision, and the app answers each
//! with a [`WebVariant`].

use super::form::FormDialog;
use super::host_requests::{HostRequests, PreviewRequest};
use super::preview::PreviewView;
use super::size_model::LinkedSize;
use super::sizing::{Resample, SizingWords};
use crate::runtime::requests::{ColorRequest, HostAnswer, PathFilter, PathRequest};

use crate::contract::{Bind, NoValue, Outcome, Props as _, ShowContainer as _};

use crate::dialog::{DialogId, DialogResult, DialogStack, StackDialog, dialog_id};

use crate::layout_components::{Row, Stack};

use crate::runtime::env::{Env, Label};

use crate::runtime::memory::{MemoryStage, Remember as _, SaveKey, unique};

use crate::runtime::motion::Animate;
use crate::runtime::overlay::CloseReason;
use craft_behavior::PixelSize;
use craft_keymap::{Chord, Key, Mods};

use craft_theme::{Rgba, Srgba};

use egui::load::SizedTexture;
use egui::{Response, Ui};

use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};

mod options;
mod panes;

#[cfg(feature = "testing")]
mod stories;

pub use panes::WebPane;

/// The key of the matte colour request the dialog files.
pub const MATTE_COLOR_KEY: &str = "web.matte";

/// The key of the path request Save… files.
pub const SAVE_PATH_KEY: &str = "web.save";

/// The resample methods the size row lists: Photoshop's Save for Web offers these five.
const RESAMPLE_METHODS: [Resample; 5] =
    [Resample::NearestNeighbor, Resample::Bilinear, Resample::Bicubic, Resample::BicubicSmoother, Resample::BicubicSharper];

/// The download speed of the time estimate in the captions, in kilobits per second.
const MODEM_KBPS: f64 = 56.6;

/// The frame rate of an animation whose timeline gives none.
const DEFAULT_FPS: f32 = 30.0;

/// The longest the next pass of a playing animation waits, in seconds.
const LONGEST_WAIT: f64 = 1.0;

/// The widest percent the Percent row and the Percent unit allow.
const MAX_PERCENT: f64 = 1000.0;

/// The zoom range of the panes, as fractions of the picture's size.
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.0001..=32.0;

craft_ui::words! {
    /// The words of Save for Web, beside the common OK and Cancel.
    pub struct SaveForWebWords {
        title: "saveForWeb.title" = "Save for Web (Legacy)";
        view: "saveForWeb.view" = "Preview";
        settings: "saveForWeb.settings" = "Settings";
        format: "saveForWeb.format" = "Format";
        zoom: "saveForWeb.zoom" = "Zoom";
        unnamed: "saveForWeb.unnamed" = "[Unnamed]";
        save: "saveForWeb.save" = "Save…";
        original: "saveForWeb.original" = "Original";
        optimized: "saveForWeb.optimized" = "Optimized";
        two_up: "saveForWeb.twoUp" = "2-Up";
        four_up: "saveForWeb.fourUp" = "4-Up";
        preset: "saveForWeb.preset" = "Preset";
        color_table: "saveForWeb.colorTable" = "Color Table";
        perceptual: "saveForWeb.perceptual" = "Perceptual";
        selective: "saveForWeb.selective" = "Selective";
        adaptive: "saveForWeb.adaptive" = "Adaptive";
        restrictive: "saveForWeb.restrictive" = "Restrictive (Web)";
        exact: "saveForWeb.exact" = "Exact";
        colors: "saveForWeb.colors" = "Colors";
        dither: "saveForWeb.dither" = "Dither";
        no_dither: "saveForWeb.noDither" = "No Dither";
        diffusion: "saveForWeb.diffusion" = "Diffusion";
        pattern: "saveForWeb.pattern" = "Pattern";
        noise: "saveForWeb.noise" = "Noise";
        amount: "saveForWeb.amount" = "Amount";
        transparency: "saveForWeb.transparency" = "Transparency";
        interlaced: "saveForWeb.interlaced" = "Interlaced";
        web_snap: "saveForWeb.webSnap" = "Web Snap";
        quality: "saveForWeb.quality" = "Quality";
        progressive: "saveForWeb.progressive" = "Progressive";
        optimized_box: "saveForWeb.optimizedBox" = "Optimized";
        lossless: "saveForWeb.lossless" = "Lossless";
        embed_profile: "saveForWeb.embedProfile" = "Embed Color Profile";
        matte: "saveForWeb.matte" = "Matte";
        matte_hex: "saveForWeb.matteHex" = "Matte color";
        convert_srgb: "saveForWeb.convertSrgb" = "Convert to sRGB";
        metadata: "saveForWeb.metadata" = "Metadata";
        metadata_none: "saveForWeb.metadataNone" = "None";
        metadata_copyright: "saveForWeb.metadataCopyright" = "Copyright";
        metadata_contact: "saveForWeb.metadataContact" = "Copyright and Contact Info";
        metadata_all: "saveForWeb.metadataAll" = "All";
        image_size: "saveForWeb.imageSize" = "Image Size";
        width: "saveForWeb.width" = "Width";
        height: "saveForWeb.height" = "Height";
        size_unit: "saveForWeb.sizeUnit" = "Unit";
        percent: "saveForWeb.percent" = "Percent";
        fit: "saveForWeb.fit" = "Fit";
        actual: "saveForWeb.actual" = "100%";
        /// `{modifier}`: the key held with the wheel to zoom (Ctrl, or Cmd on macOS).
        pan_hint: template "saveForWeb.panHint" = "Drag to pan · {modifier}+scroll to zoom · Double-click to fit";
        play: "saveForWeb.play" = "Play";
        pause: "saveForWeb.pause" = "Pause";
        /// `{frame}` of `{total}` animation frames.
        frame: template "saveForWeb.frame" = "{frame} / {total}";
        /// `{name}`, `{width}` and `{height}`: the Original pane's second caption line.
        original_caption: template "saveForWeb.originalCaption" = "{name}  ·  {width} × {height} px";
        /// `{format}` and `{size}`: an encoded pane's first caption line ("JPG  48.2K").
        encoded_caption: template "saveForWeb.encodedCaption" = "{format}  {size}";
        /// `{seconds}`: an encoded pane's second caption line.
        download: template "saveForWeb.download" = "{seconds} sec @ 56.6 Kbps";
        /// `{count}`: appended to the second line for indexed formats.
        color_count: template "saveForWeb.colorCount" = "  ·  {count} colors";
        /// Appended to the first line when the pane shows a reduced proxy.
        proxy: "saveForWeb.proxy" = "  ·  Proxy preview";
        /// `{name}` and `{number}`: the name of the 4-Up variant panes after the first.
        optimized_variant: template "saveForWeb.optimizedVariant" = "{name} {number}";
        /// `{pane}`, `{first}` and `{second}`: the accessible name of a pane.
        pane_name: template "saveForWeb.paneName" = "{pane}: {first}, {second}";
        /// The pane waits for the app's encode.
        pane_pending: "saveForWeb.panePending" = "Encoding";
    }
}

/// The formats of Save for Web.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebFormat {
    /// GIF, indexed colour.
    Gif,
    /// PNG with an indexed palette of at most 256 colours.
    Png8,
    /// PNG with 24-bit colour.
    Png24,
    /// JPEG.
    #[default]
    Jpeg,
    /// WebP, lossy or lossless.
    WebP,
    /// Wireless bitmap: one bit per pixel.
    Wbmp,
}

impl WebFormat {
    /// Every format, in the order of the format list.
    pub const ALL: [WebFormat; 6] = [WebFormat::Gif, WebFormat::Png8, WebFormat::Png24, WebFormat::Jpeg, WebFormat::WebP, WebFormat::Wbmp];

    /// The extension of a file in this format, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            WebFormat::Gif => "gif",
            WebFormat::Png8 | WebFormat::Png24 => "png",
            WebFormat::Jpeg => "jpg",
            WebFormat::WebP => "webp",
            WebFormat::Wbmp => "wbmp",
        }
    }

    /// The format list's entry.
    pub fn name(self) -> &'static str {
        match self {
            WebFormat::Gif => "GIF",
            WebFormat::Png8 => "PNG-8",
            WebFormat::Png24 => "PNG-24",
            WebFormat::Jpeg => "JPEG",
            WebFormat::WebP => "WebP",
            WebFormat::Wbmp => "WBMP",
        }
    }

    /// Whether the format has a palette.
    pub fn indexed(self) -> bool {
        matches!(self, WebFormat::Gif | WebFormat::Png8)
    }
}

/// How an indexed format's palette is chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ColorTable {
    /// A palette weighted toward colours the eye notices.
    Perceptual,
    /// As Perceptual, favouring broad areas of colour and web-safe colours.
    #[default]
    Selective,
    /// A palette sampled from the colours most common in the image.
    Adaptive,
    /// The 216 web-safe colours.
    Restrictive,
    /// The image's own colours, when it has at most 256.
    Exact,
}

/// How an indexed format spreads colour error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Dither {
    /// Nearest palette colour, no dither.
    None,
    /// Error diffusion; the amount sets its strength.
    #[default]
    Diffusion,
    /// A regular halftone-like pattern.
    Pattern,
    /// Random noise.
    Noise,
}

/// What metadata goes into a web file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebMetadata {
    /// No metadata.
    None,
    /// The copyright notice only.
    #[default]
    Copyright,
    /// The copyright notice and contact details.
    CopyrightAndContact,
    /// All metadata.
    All,
}

/// The view tabs over the panes. 2-Up is the default; the last tab is remembered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
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

impl WebView {
    /// The panes this view shows, in reading order.
    pub fn panes(self) -> &'static [WebPane] {
        match self {
            WebView::Original => &[WebPane::Original],
            WebView::Optimized => &[WebPane::Encoded(0)],
            WebView::TwoUp => &[WebPane::Original, WebPane::Encoded(0)],
            WebView::FourUp => &[WebPane::Original, WebPane::Encoded(0), WebPane::Encoded(1), WebPane::Encoded(2)],
        }
    }
}

/// Save for Web's settings: the result, what is remembered, and what the app encodes each variant with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SaveForWebOptions {
    /// The file format.
    pub format: WebFormat,
    /// 0 to 100 (JPEG, lossy WebP).
    pub quality: u8,
    /// The palette method; used by indexed formats.
    pub color_table: ColorTable,
    /// 2 to 256 (GIF, PNG-8).
    pub colors: u16,
    /// How colour error is spread; used by indexed formats.
    pub dither: Dither,
    /// 0 to 100.
    pub dither_amount: u8,
    /// Keeps transparency where the format allows it; otherwise the matte fills it.
    pub transparency: bool,
    /// Writes an interlaced file (GIF, PNG).
    pub interlaced: bool,
    /// 0 to 100.
    pub web_snap: u8,
    /// Writes a progressive JPEG.
    pub progressive: bool,
    /// Optimizes the JPEG encoding for a smaller file.
    pub optimized: bool,
    /// Encodes WebP losslessly; the quality is then ignored.
    pub lossless: bool,
    /// Embeds the colour profile in the file.
    pub embed_profile: bool,
    /// The matte, red, green and blue.
    pub matte: [u8; 3],
    /// Converts the colours to sRGB before encoding.
    pub convert_srgb: bool,
    /// What metadata goes into the file.
    pub metadata: WebMetadata,
    /// The output size in percent of the document, 1 to 1000 with two decimals.
    pub percent: f64,
    /// How the pixels are made when the size differs from the document's.
    pub resample: Resample,
}

impl Default for SaveForWebOptions {
    /// The opening values: JPEG quality 60, Selective 128 colours, Diffusion 88 %, transparency, white matte,
    /// Optimized, Convert to sRGB, Copyright, 100 %.
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

impl SaveForWebOptions {
    /// Whether the matte row shows: the matte fills the transparency of formats that cannot keep it.
    pub fn has_matte(&self) -> bool {
        !matches!(self.format, WebFormat::Png24 | WebFormat::WebP) || !self.transparency
    }

    /// Whether the quality row shows.
    pub fn has_quality(&self) -> bool {
        match self.format {
            WebFormat::Jpeg => true,
            WebFormat::WebP => !self.lossless,
            _ => false,
        }
    }
}

/// A named set of options in the Preset list.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WebPreset {
    /// The name in the list.
    pub name: String,
    /// The options the preset sets.
    pub options: SaveForWebOptions,
}

impl WebPreset {
    /// A preset named `name`.
    pub fn new(name: impl Into<String>, options: SaveForWebOptions) -> Self {
        WebPreset { name: name.into(), options }
    }

    /// Whether the preset is what `current` shows apart from the size.
    pub fn is_set_in(&self, current: &SaveForWebOptions) -> bool {
        self.options == SaveForWebOptions { percent: self.options.percent, ..current.clone() }
    }
}

/// One encoded pane the app hands back for a variant the dialog asked for.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WebVariant {
    /// Which pane: 0 is the current settings, 1 and 2 the 4-Up variants.
    pub index: usize,
    /// The revision [`WantedVariant::revision`] gave; older answers are dropped.
    pub revision: u64,
    /// The encoded picture, drawn at `size` in document pixels (a reduced proxy texture is drawn at the full size),
    /// or why the encode failed.
    pub picture: Result<SizedTexture, Label>,
    /// The encoded file size in bytes.
    pub bytes: u64,
    /// The palette size of an indexed format.
    pub colors: Option<u16>,
    /// The texture is a reduced proxy (the long side is capped at 2048 px).
    pub proxy: bool,
}

impl WebVariant {
    /// The encoded pane `index` of `revision`, shown from `texture`.
    pub fn new(index: usize, revision: u64, texture: SizedTexture, bytes: u64) -> Self {
        WebVariant { index, revision, picture: Ok(texture), bytes, colors: None, proxy: false }
    }

    /// The pane `index` of `revision` whose encode failed with `message`.
    pub fn failed(index: usize, revision: u64, message: impl Into<Label>) -> Self {
        WebVariant { index, revision, picture: Err(message.into()), bytes: 0, colors: None, proxy: false }
    }

    /// The palette size of an indexed format.
    #[must_use]
    pub fn colors(mut self, colors: u16) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Marks the texture as a reduced proxy.
    #[must_use]
    pub fn proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }
}

/// A pane the dialog wants encoded.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WantedVariant {
    /// Which pane: 0 is the current settings, 1 and 2 the 4-Up variants.
    pub index: usize,
    /// The options to encode with and the revision to answer with.
    pub request: PreviewRequest<SaveForWebOptions>,
}

/// What Save for Web returns.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SaveForWebResult {
    /// The settings the user confirmed.
    pub options: SaveForWebOptions,
    /// The file the user chose to write.
    pub path: PathBuf,
    /// The output width in pixels.
    pub width: u32,
    /// The output height in pixels.
    pub height: u32,
    remember: Option<(SaveKey, Last)>,
}

impl SaveForWebResult {
    /// Stores the settings and the view for the next open of a dialog built with
    /// [`SaveForWebDialog::remember`]. The app calls it after the file is written; without a remember key it does
    /// nothing.
    pub fn remember(&self, ui: &Ui) {
        if let Some((key, last)) = &self.remember {
            let mut stage = MemoryStage::open(ui.ctx(), unique(("craft_ui::save_for_web_result", key.as_str())));
            *stage.saved::<Option<Last>>(key) = Some(last.clone());
            stage.write_back();
        }
    }
}

/// An animation: the panes show one frame, and Play steps through them.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Animation {
    /// The number of frames; fewer than two shows no play row.
    pub frames: u32,
    /// Frames per second; 30 when it is not positive.
    pub fps: f32,
}

impl Animation {
    /// An animation of `frames` frames at `fps`.
    pub fn new(frames: u32, fps: f32) -> Self {
        Animation { frames, fps }
    }
}

/// What the dialog remembers under its key: the last view and the last confirmed settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Last {
    view: WebView,
    options: SaveForWebOptions,
}

/// What the dialog keeps in its root container's slot besides the settings.
#[derive(Clone)]
struct Live {
    started: bool,
    link: LinkedSize,
    view: WebView,
    shared: PreviewView,
    selected: WebPane,
    frame: u32,
    playing: bool,
    clock: f64,
    revision: u64,
    reported: u64,
}

impl Default for Live {
    fn default() -> Self {
        Live {
            started: false,
            link: LinkedSize::new(PixelSize::new(1, 1)),
            view: WebView::default(),
            shared: PreviewView::fitting().zoom_range(ZOOM_RANGE),
            selected: WebPane::Encoded(0),
            frame: 0,
            playing: false,
            clock: 0.0,
            revision: 1,
            reported: 0,
        }
    }
}

impl Live {
    /// The state of a dialog that opens now: the document `size` and, when saved, the last view.
    fn open(size: PixelSize, saved: Option<&Last>) -> Self {
        Live { started: true, link: LinkedSize::new(size), view: saved.map_or_else(WebView::default, |last| last.view), ..Live::default() }
    }

    /// Puts back the size from when the dialog opened and asks for new encodes.
    fn reset(&mut self) {
        self.link = LinkedSize::new(self.link.original());
        self.revision += 1;
    }

    /// Moves the playing animation on by the time that has passed and asks for the pass of the next frame.
    fn advance(&mut self, cx: &impl Animate, animation: Option<Animation>) {
        let Some(Animation { frames, fps }) = animation.filter(|animation| animation.frames >= 2) else { return };
        let now = cx.now();
        self.frame = self.frame.min(frames - 1);
        if !self.playing {
            self.clock = now;
            return;
        }
        let (frame, clock) = advance_frame(self.frame, frames, fps, self.clock, now);
        self.frame = frame;
        self.clock = clock;
        let rate = f64::from(frame_rate(fps));
        cx.wake_at(now + (1.0 / rate - (now - clock)).clamp(0.0, LONGEST_WAIT));
    }
}

/// What the dialog reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct SaveForWebOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open. A path answer closes the dialog as
    /// `CloseReason::Activated`.
    pub closed: Option<CloseReason>,
    /// The answer, in the pass the host answered the save request with a path.
    pub result: Option<SaveForWebResult>,
    /// The panes to encode, when the settings, the size, the view or the frame changed: every pane the view shows,
    /// with the revision to answer with.
    pub wanted_variants: Vec<WantedVariant>,
    /// What the dialog asks of the host: the colour picker for the matte (keyed [`MATTE_COLOR_KEY`]) and, after
    /// Save…, the file dialog (keyed [`SAVE_PATH_KEY`]). The host answers with
    /// [`HostRequests::answer_color`] and [`HostRequests::answer_path`] and gives the answers back through
    /// [`SaveForWebDialog::answers`].
    pub requests: HostRequests,
    /// The settings after the pass.
    pub options: SaveForWebOptions,
    /// The output size after the pass.
    pub size: PixelSize,
    /// The frame the panes show; the app encodes this frame.
    pub frame: u32,
    /// The pane last clicked.
    pub selected: WebPane,
}

impl Outcome for SaveForWebOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The picture the Original pane shows and the answers of the app for the encoded panes.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct WebPictures {
    /// The Original pane's picture, drawn at the document's size in pixels.
    pub original: Option<SizedTexture>,
    /// The encoded panes by index.
    pub variants: [Option<WebVariant>; 3],
}

impl WebPictures {
    /// No pictures yet: every pane waits.
    pub fn new() -> Self {
        Self::default()
    }

    /// The picture of the Original pane.
    #[must_use]
    pub fn original(mut self, original: SizedTexture) -> Self {
        self.original = Some(original);
        self
    }

    /// Takes `variant` for its pane; an index past the third pane is ignored.
    pub fn set(&mut self, variant: WebVariant) {
        if let Some(slot) = self.variants.get_mut(variant.index) {
            *slot = Some(variant);
        }
    }
}

/// The Save for Web dialog: the view tabs over a row of preview panes that share one view, the zoom row, the
/// animation row, and the settings column. It is a [`FormDialog`] over [`SaveForWebOptions`].
///
/// - Keys: Tab walks the controls, Enter is Save… unless a field owns it, Escape cancels. Holding Alt turns Cancel
///   into Reset, which restores the settings and the size from when the dialog opened. Ctrl+Z steps the dialog's own
///   settings.
/// - Panes: a drag pans every pane, Ctrl with the wheel zooms about the pointer, a double click fits, a click selects
///   the pane. The zoom row steps through the preview's zoom levels and has Fit and 100%.
/// - Encoding is the app's: [`SaveForWebOutput::wanted_variants`] lists the panes to encode; the app answers with
///   [`WebVariant`]s in `pictures`, and the dialog drops none: the owner of the pictures drops stale ones
///   ([`HostedSaveForWeb::set_variant`]).
/// - Save… files a path request in [`SaveForWebOutput::requests`] (keyed [`SAVE_PATH_KEY`]); the dialog stays open
///   while the host's file dialog is up (`saving` disables Save…). The host's path answer, given through `answers`,
///   closes the dialog as `CloseReason::Activated` and makes `result` the answer.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct SaveForWebDialog<'a> {
    name: String,
    size: PixelSize,
    pictures: WebPictures,
    options: Bind<'a, SaveForWebOptions>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// The Preset list.
    #[prop]
    presets: Vec<WebPreset>,
    /// The document is an animation: the play row shows and the frame advances while playing.
    #[prop]
    animation: Option<Animation>,
    /// Remembers the last view and, through [`SaveForWebResult::remember`], the last settings under `key`. A
    /// remembered set replaces the bound settings when the dialog opens.
    #[prop]
    remember: Option<SaveKey>,
    /// Whether the host's file dialog is open: Save… waits.
    #[prop]
    saving: bool,
    /// The host's answers to earlier requests: the matte colour and the path of the file to write.
    #[prop]
    answers: Vec<HostAnswer>,
}

impl<'a> SaveForWebDialog<'a> {
    /// A dialog for the document `name` of `size` pixels with the app's `pictures`, editing the settings in `options`.
    pub fn new(name: impl Into<String>, size: PixelSize, pictures: WebPictures, options: Bind<'a, SaveForWebOptions>) -> Self {
        SaveForWebDialog {
            name: name.into(),
            size,
            pictures,
            options,
            open: None,
            presets: Vec::new(),
            animation: None,
            remember: None,
            saving: false,
            answers: Vec::new(),
        }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> SaveForWebOutput {
        let SaveForWebDialog { name, size, pictures, options, open, presets, animation, remember, saving, answers } = self;
        let env = Env::of(ui);
        let words = env.words::<SaveForWebWords>();
        let sizing = env.words::<SizingWords>();
        let units = env.units();
        let hint = zoom_hint(&env, &words);

        let mut form = FormDialog::new(words.title.clone(), options)
            .ok_label(words.save.clone())
            .ok_enabled(!saving)
            .outer_gesture(true)
            .answers(answers)
            .on_reset(|cx| cx.memory::<Live>().reset())
            .accept_stays_open(true);
        if let Some(open) = open {
            form = form.opened(open);
        }

        let mut after = Live::default();
        let mut wanted = Vec::new();
        let mut chosen = None;
        let mut finished = SaveForWebOptions::default();
        let shown = form.show(ui, |ui, cx, options| {
            let mut live = cx.memory::<Live>().clone();
            if !live.started {
                let saved = remember.as_ref().and_then(|key| cx.saved::<Option<Last>>(key).clone());
                live = Live::open(size, saved.as_ref());
                if let Some(last) = &saved {
                    *options = SaveForWebOptions { percent: 100.0, ..last.options.clone() };
                }
            }
            for answer in cx.take_answers() {
                match answer {
                    HostAnswer::Color { key, color } if key == MATTE_COLOR_KEY => {
                        let Rgba([r, g, b, _]) = color.to_rgba();
                        options.matte = [r, g, b];
                    }
                    HostAnswer::Path { key, path } if key == SAVE_PATH_KEY => {
                        chosen = Some(path);
                        cx.request_close_as(CloseReason::Activated);
                    }
                    _ => {}
                }
            }
            live.advance(&**cx, animation);
            let before = (options.clone(), live.link.pixels(), live.view, live.frame);
            let view_before = live.view;

            let mut ask = None;
            Row::new().show(ui, |ui, _| {
                Stack::new().id_salt("preview").show(ui, |ui, _| {
                    let reads = panes::Panes {
                        words: &words,
                        units: &units,
                        name: &name,
                        size,
                        pictures: &pictures,
                        animation,
                        hint: &hint,
                        options: &*options,
                    };
                    panes::show(ui, &reads, &mut live)
                });
                Stack::new().id_salt("settings").show(ui, |ui, _| {
                    let kit = options::Kit { words: &words, sizing: &sizing, units: &units, presets: &presets };
                    options::show(ui, &kit, options, &mut live.link, &mut ask)
                });
            });

            if live.link.pixels() != before.1 {
                options.percent = percent_of(live.link.pixels(), live.link.original());
            }
            if (options.clone(), live.link.pixels(), live.view, live.frame) != before {
                live.revision += 1;
            }
            if let Some(key) = &remember
                && live.view != view_before
            {
                let saved = cx.saved::<Option<Last>>(key);
                let kept = saved.as_ref().map_or_else(|| options.clone(), |last| last.options.clone());
                *saved = Some(Last { view: live.view, options: kept });
            }
            if let Some(request) = ask {
                cx.requests().request_color(request);
            }
            wanted = Self::wanted(&live, options);
            live.reported = live.revision;
            *cx.memory::<Live>() = live.clone();
            after = live;
            finished = options.clone();
            true
        });

        let live = after;
        let pixels = live.link.pixels();
        let mut requests = shown.requests;
        if shown.picked == Some(0) {
            requests.request_path(save_request(&name, finished.format));
        }
        let result = shown.accepted.zip(chosen).map(|(accepted, path)| SaveForWebResult {
            options: accepted.clone(),
            path,
            width: pixels.width,
            height: pixels.height,
            remember: remember.map(|key| (key, Last { view: live.view, options: accepted })),
        });
        SaveForWebOutput {
            response: shown.response,
            closed: shown.closed,
            result,
            wanted_variants: wanted,
            requests,
            options: finished,
            size: pixels,
            frame: live.frame,
            selected: live.selected,
        }
    }
}

/// The percent `size` is of `original`, rounded to two decimals.
fn percent_of(size: PixelSize, original: PixelSize) -> f64 {
    (f64::from(size.width) / f64::from(original.width.max(1)) * 10_000.0).round() / 100.0
}

/// The file dialog request for `name` in `format`: the document's name without its extension, with the format's.
fn save_request(name: &str, format: WebFormat) -> PathRequest {
    let stem = Path::new(name).file_stem().map_or_else(|| name.to_owned(), |stem| stem.to_string_lossy().into_owned());
    let extension = format.extension();
    PathRequest::save(SAVE_PATH_KEY, format!("{stem}.{extension}"), [PathFilter::new(format.name(), &[extension])])
}

/// The pan and zoom hint, naming the key the wheel zooms with on this platform.
fn zoom_hint(env: &Env, words: &SaveForWebWords) -> Label {
    let view = env.view();
    let mut keys = Chord::new(Mods::CMD, Key::Plus).parts_for(view.shell().os, view.keyboard_layout());
    keys.pop();
    Label::from(words.pan_hint.fill(&[("modifier", keys.join("+").as_str())]))
}

/// The frame rate to use: `fps`, or the default for one that is not a positive number.
fn frame_rate(fps: f32) -> f32 {
    if fps.is_finite() && fps > 0.0 { fps } else { DEFAULT_FPS }
}

/// `frame` after the time from `last` to `now`, and the clock to carry on from; the fraction of a frame that has not
/// yet passed is kept.
pub(crate) fn advance_frame(frame: u32, frames: u32, fps: f32, last: f64, now: f64) -> (u32, f64) {
    let frames = frames.max(1);
    let frame = frame.min(frames - 1);
    if !now.is_finite() {
        return (frame, last);
    }
    if !last.is_finite() || now < last {
        return (frame, now);
    }
    let fps = f64::from(frame_rate(fps));
    let elapsed = (now - last) * fps;
    if !elapsed.is_finite() {
        return (frame, now);
    }
    let steps = elapsed.floor();
    if steps < 1.0 {
        return (frame, last);
    }
    let delta = ((steps % f64::from(frames)) as u32) % frames;
    ((frame + delta) % frames, now - (elapsed - steps) / fps)
}

/// 4-Up's two extra panes: JPEG and lossy WebP at half and a quarter of the quality; GIF and PNG-8 at half and a
/// quarter of the colours; any other format as PNG-8 at 128 colours and GIF at 64 colours.
pub(crate) fn four_up_variants(settings: &SaveForWebOptions) -> [SaveForWebOptions; 2] {
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

/// Seconds to download `bytes` at `kbps` kilobits per second, rounded up: the caption uses 56.6, the modem estimate.
pub(crate) fn download_seconds(bytes: u64, kbps: f64) -> u64 {
    (bytes as f64 * 8.0 / (kbps * 1000.0)).ceil() as u64
}

impl SaveForWebDialog<'_> {
    /// The panes to encode when the revision changed since the last report.
    fn wanted(live: &Live, options: &SaveForWebOptions) -> Vec<WantedVariant> {
        if live.revision == live.reported {
            return Vec::new();
        }
        let revision = live.revision;
        let mut wanted = Vec::new();
        if live.view != WebView::Original {
            wanted.push(WantedVariant { index: 0, request: PreviewRequest::new(revision, options.clone()) });
        }
        if live.view == WebView::FourUp {
            let [half, quarter] = four_up_variants(options);
            wanted.push(WantedVariant { index: 1, request: PreviewRequest::new(revision, half) });
            wanted.push(WantedVariant { index: 2, request: PreviewRequest::new(revision, quarter) });
        }
        wanted
    }
}

/// A [`SaveForWebDialog`] the [`DialogHost`](crate::dialog::DialogHost) owns. It keeps the settings and what the app
/// answers (the pictures), drops stale answers, keeps the colour and path requests until the app takes them, and
/// finishes with the result when the file dialog's path comes back.
///
/// ```ignore
/// let mut web = HostedSaveForWeb::new("poster.psd", PixelSize::new(1200, 800), original).animation(Animation::new(12, 24.0));
/// // each pass: for want in web.wanted_variants() { encode(want) -> web.set_variant(variant) }
/// // pick_color() -> picker -> answer_color(MATTE_COLOR_KEY, color); pick_path() -> file dialog -> answer_path(key, path)
/// ```
pub struct HostedSaveForWeb {
    id: DialogId,
    name: String,
    size: PixelSize,
    options: SaveForWebOptions,
    presets: Vec<WebPreset>,
    animation: Option<Animation>,
    remember: Option<SaveKey>,
    pictures: WebPictures,
    taken: u64,
    wanted: Vec<WantedVariant>,
    frame: u32,
    requests: HostRequests,
    saving: bool,
}

impl HostedSaveForWeb {
    /// A hosted dialog for the document `name` of `size` pixels showing `original` in the Original pane.
    pub fn new(name: impl Into<String>, size: PixelSize, original: SizedTexture) -> Self {
        HostedSaveForWeb {
            id: dialog_id("save-for-web"),
            name: name.into(),
            size,
            options: SaveForWebOptions::default(),
            presets: Vec::new(),
            animation: None,
            remember: None,
            pictures: WebPictures::new().original(original),
            taken: 0,
            wanted: Vec::new(),
            frame: 0,
            requests: HostRequests::new(),
            saving: false,
        }
    }

    /// The settings the dialog opens with.
    #[must_use]
    pub fn options(mut self, options: SaveForWebOptions) -> Self {
        self.options = options;
        self
    }

    /// The Preset list.
    #[must_use]
    pub fn presets(mut self, presets: Vec<WebPreset>) -> Self {
        self.presets = presets;
        self
    }

    /// The document is an animation of `animation.frames` frames.
    #[must_use]
    pub fn animation(mut self, animation: Animation) -> Self {
        self.animation = Some(animation);
        self
    }

    /// Remembers the last view and settings under `key`.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.remember = Some(key);
        self
    }

    /// The frame the panes show; the app encodes this frame.
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// The Original pane's picture, for the app to hand over the new frame's when [`frame`](Self::frame) changed.
    pub fn set_original(&mut self, original: SizedTexture) {
        self.pictures.original = Some(original);
    }

    /// The panes to encode when they changed since the last call.
    pub fn wanted_variants(&mut self) -> Vec<WantedVariant> {
        std::mem::take(&mut self.wanted)
    }

    /// An encoded pane; one for an older revision than the last asked for is dropped.
    pub fn set_variant(&mut self, variant: WebVariant) {
        if variant.revision >= self.taken {
            self.pictures.set(variant);
        }
    }

    /// The Matte button was pressed in the pass since the last call: the colour to ask the app's picker for, keyed
    /// [`MATTE_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.requests.take_color()
    }

    /// Save… was pressed in the pass since the last call: the file dialog to open, keyed [`SAVE_PATH_KEY`].
    /// Returned once; answer with [`answer_path`](Self::answer_path).
    pub fn pick_path(&mut self) -> Option<PathRequest> {
        self.requests.take_path()
    }

    /// Answers the colour request `key`: the matte, without alpha. Other keys are ignored.
    pub fn answer_color(&mut self, key: impl Into<String>, color: Srgba) {
        self.requests.answer_color(key, color);
    }

    /// Answers the file dialog request `key` with the chosen path, which finishes the dialog.
    pub fn answer_path(&mut self, key: impl Into<String>, path: PathBuf) {
        self.saving = false;
        self.requests.answer_path(key, path);
    }

    /// The host cancelled its picker or file dialog: the requests not taken yet are dropped and Save… works again.
    pub fn cancel_requests(&mut self) {
        self.saving = false;
        self.requests.cancel();
    }
}

impl StackDialog for HostedSaveForWeb {
    type Result = SaveForWebResult;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<SaveForWebResult>> {
        let dialog = SaveForWebDialog::new(self.name.clone(), self.size, self.pictures.clone(), Bind::bound(&mut self.options))
            .presets(self.presets.clone())
            .saving(self.saving)
            .answers(self.requests.take_answers());
        let dialog = SaveForWebDialog { animation: self.animation, remember: self.remember.clone(), ..dialog };
        let mut shown = dialog.show(ui);
        self.frame = shown.frame;
        if let Some(last) = shown.wanted_variants.first() {
            self.taken = last.request.revision;
            self.wanted = shown.wanted_variants;
        }
        if let Some(request) = shown.requests.take_color() {
            self.requests.request_color(request);
        }
        if let Some(request) = shown.requests.take_path() {
            self.requests.request_path(request);
            self.saving = true;
        }
        match (shown.result, shown.closed) {
            (Some(result), _) => Some(DialogResult::Ok(result)),
            (None, Some(_)) => {
                self.saving = false;
                self.requests.cancel();
                Some(DialogResult::Cancel)
            }
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some("save-for-web")
    }

    fn cancelled(&mut self, _: &egui::Context) {
        self.cancel_requests();
    }
}
