//! [`NewDocumentDialog`]: File › New. Category tabs over a grid of preset cards, a details
//! column, saved presets, and the document settings the app creates the document from.

use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{ButtonVariant, Srgba, TypeRole};
use egui::accesskit::{Action, Role};
use egui::text::{LayoutJob, TextWrapping};
use egui::{Align, Color32, Context, Id, Key, Label, Modifiers, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

use super::color_request::ColorRequest;
use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::Width;
use super::sizing::{Field, LinkedSizeFields, PixelSize, ResolutionUnit, SizeUnit, SizingLabels};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::behavior::{FieldTab, GridNav};
use crate::composites::SectionLabel;
use crate::controls::{Button, Checkbox, NumberField, Segmented, Select, Swatch, TextField};
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::units::{CM_PER_INCH, LengthUnitLabels, POINTS_PER_INCH};
use crate::localized::{LocaleNumbers, SharedLabels};
use crate::primitives::{Content, Corner, Divider, Gap, Interactive, Pad, SurfaceKind, Text};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_theme::Rgba;

/// The engine's greatest document side, in pixels.
const CARD: [f32; 2] = [164.0, 112.0];
const CARD_COLUMNS: usize = 3;
const CARD_ROWS: usize = 3;
const PAGE_ICON: f32 = 30.0;
const PAGE_BOX: f32 = 40.0;
const CARD_ICON_CENTER: f32 = 34.0;
const CARD_TITLE_CENTER: f32 = 70.0;
const CARD_LINE_CENTER: f32 = 97.0;
const SCROLL_ROOM: f32 = 16.0;
const SAVED_ROWS: f32 = 11.0;
/// The card grid's width: its columns, the gaps between them and room for the scroll bar.
fn grid_width(tokens: &craft_theme::Tokens) -> f32 {
    CARD_COLUMNS as f32 * CARD[0] + (CARD_COLUMNS - 1) as f32 * Gap::Row.points(tokens) + SCROLL_ROOM
}

/// The dialog's content width: the card grid, a details pane half a standard dialog wide, the gap
/// between them and the dialog margin on both sides.
fn dialog_width(tokens: &craft_theme::Tokens) -> f32 {
    let details = crate::style::dialog::DialogStyle::new(tokens).width_standard / 2.0;
    grid_width(tokens) + Gap::Section.points(tokens) + details + 2.0 * tokens.space.pad.dialog
}

const MAX_PIXELS: f64 = 300_000.0;
/// The least and greatest resolution, in pixels per inch.
const PPI_RANGE: (f64, f64) = (1.0, 30_000.0);
const MAX_RECENT: usize = 20;
const MAX_SAVED: usize = 256;
const MAX_NAME: usize = 255;
const RECENT_TAB: &str = "recent";
const SAVED_TAB: &str = "saved";
const CLIPBOARD_ID: &str = "clipboard";
/// The key of the colour request Custom… makes.
pub const BACKGROUND_COLOR_KEY: &str = "newDocument.backgroundColor";
/// The built-in "Default {app} Size" card.
const DEFAULT_ID: &str = "default";
const LAST_KEY: &str = "newDocument.last";
pub(super) const RECENT_KEY: &str = "newDocument.recent";
pub(super) const PRESETS_KEY: &str = "newDocument.presets";
/// The units of the Width and Height lists (Photoshop's New Document has no percent).
const UNITS: [SizeUnit; 6] = [SizeUnit::Pixels, SizeUnit::Inches, SizeUnit::Centimetres, SizeUnit::Millimetres, SizeUnit::Points, SizeUnit::Picas];

crate::words! {
    /// The words of New Document, English by default.
    #[non_exhaustive]
    pub struct NewDocumentLabels {
        title: "newDocument.title" = "New Document";
        create: "newDocument.create" = "Create";
        close: "newDocument.close" = "Close";
        recent: "newDocument.recent" = "Recent";
        saved: "newDocument.saved" = "Saved";
        photo: "newDocument.photo" = "Photo";
        print: "newDocument.print" = "Print";
        art: "newDocument.art" = "Art & Illustration";
        web: "newDocument.web" = "Web";
        mobile: "newDocument.mobile" = "Mobile";
        film: "newDocument.film" = "Film & Video";
        clipboard: "newDocument.clipboard" = "Clipboard";
        custom: "newDocument.custom" = "Custom";
        untitled: "newDocument.untitled" = "Untitled-1";
        name: "newDocument.name" = "Name";
        width: "newDocument.width" = "Width";
        height: "newDocument.height" = "Height";
        portrait: "newDocument.portrait" = "Portrait";
        landscape: "newDocument.landscape" = "Landscape";
        artboards: "newDocument.artboards" = "Artboards";
        resolution: "newDocument.resolution" = "Resolution";
        color_mode: "newDocument.colorMode" = "Color Mode";
        depth: "newDocument.depth" = "Depth";
        profile: "newDocument.profile" = "Color Profile";
        /// `{name}` is the working profile's name.
        working_profile: "newDocument.workingProfile" = "Working: {name}";
        background: "newDocument.background" = "Background Contents";
        white: "newDocument.white" = "White";
        black: "newDocument.black" = "Black";
        background_color: "newDocument.backgroundColor" = "Background Color";
        transparent: "newDocument.transparent" = "Transparent";
        custom_color: "newDocument.customColor" = "Custom…";
        save_preset: "newDocument.savePreset" = "Save Preset…";
        preset_name: "newDocument.presetName" = "Preset Name";
        save: "newDocument.save" = "Save";
        cancel: "newDocument.cancel" = "Cancel";
        delete: "newDocument.delete" = "Delete";
        no_saved: "newDocument.noSaved" = "No saved presets yet.";
        name_length: "newDocument.nameLength" = "Preset names must contain 1–255 characters after trimming";
        /// `{name}`: the name typed.
        name_taken: "newDocument.nameTaken" = "A preset named \"{name}\" already exists; choose another name";
        too_many: "newDocument.tooMany" = "At most 256 document presets can be saved";
        working: "newDocument.working" = "Working";
        grayscale: "newDocument.grayscale" = "Grayscale";
        rgb_color: "newDocument.rgbColor" = "RGB Color";
        cmyk_color: "newDocument.cmykColor" = "CMYK Color";
        lab_color: "newDocument.labColor" = "Lab Color";
        depth_8: "newDocument.depth8" = "8 bit";
        depth_16: "newDocument.depth16" = "16 bit";
        depth_32: "newDocument.depth32" = "32 bit (float)";
        orientation: "newDocument.orientation" = "Orientation";
        /// `{n}`: how many cards the open tab lists.
        presets_heading: "newDocument.presetsHeading" = "Blank Document Presets ({n})";
        preset_details: "newDocument.presetDetails" = "Preset Details";
        custom_swatch: "newDocument.customSwatch" = "Custom background color";
        /// `{width}`, `{height}`, `{unit}` and `{ppi}`: a card's size line.
        /// `{app}`: the application's name ([`Host::app`]).
        default_size: "newDocument.defaultSize" = "Default {app} Size";
        size_line: "newDocument.sizeLine" = "{width} × {height} {unit} @ {ppi} ppi";
    }
}

/// The document's colour mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorMode {
    #[allow(missing_docs)]
    Grayscale,
    #[default]
    #[allow(missing_docs)]
    Rgb,
    #[allow(missing_docs)]
    Cmyk,
    #[allow(missing_docs)]
    Lab,
}

/// Bits per channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitDepth {
    #[default]
    /// 8 bits per channel.
    Eight,
    /// 16 bits per channel.
    Sixteen,
    /// 32-bit floating point.
    ThirtyTwo,
}

/// What fills the new document's background layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BackgroundContents {
    #[default]
    #[allow(missing_docs)]
    White,
    #[allow(missing_docs)]
    Black,
    /// The app's current background colour ([`NewDocumentDialog::background_color`]).
    BackgroundColor,
    #[allow(missing_docs)]
    Transparent,
    /// A colour the user picked with Custom…, sRGB, 0–1 per channel.
    Custom([f32; 3]),
}

/// Which way the page stands. The two buttons beside Height swap width and height and turn the
/// built-in cards with them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    /// Taller than wide, or square.
    Portrait,
    /// Wider than tall.
    Landscape,
}

impl Orientation {
    /// Landscape when wider than tall; a square is portrait.
    pub fn of(size: PixelSize) -> Self {
        if size.width > size.height { Orientation::Landscape } else { Orientation::Portrait }
    }
}

/// Everything New Document creates a document from: the dialog's result, a saved preset, a
/// recent card and the remembered last values are all this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocumentOptions {
    /// The document name ("Untitled-1"); a preset's own name when saved as one.
    pub name: String,
    /// Width and height in pixels, and the resolution in pixels per inch.
    pub size: PixelSize,
    /// The unit the size fields show; the size itself is always in pixels.
    pub unit: SizeUnit,
    /// The unit the resolution field shows; the resolution itself is stored in pixels per inch.
    pub resolution_unit: ResolutionUnit,
    /// The colour mode.
    pub color_mode: ColorMode,
    /// Bits per channel.
    pub depth: BitDepth,
    /// The colour profile's name; `None` is the working profile of the mode.
    pub profile: Option<String>,
    /// What fills the background layer.
    pub background: BackgroundContents,
    /// Artboards: the document starts with one artboard of this size.
    pub artboard: bool,
}

impl Default for DocumentOptions {
    /// PhotoCraft's default size: 2100 × 1500 px at 300 ppi (7 × 5 in), RGB, 8 bit, white.
    fn default() -> Self {
        Self {
            name: String::new(),
            size: PixelSize { width: 2100, height: 1500, pixels_per_inch: 300.0 },
            unit: SizeUnit::Pixels,
            resolution_unit: ResolutionUnit::PixelsPerInch,
            color_mode: ColorMode::Rgb,
            depth: BitDepth::Eight,
            profile: None,
            background: BackgroundContents::White,
            artboard: false,
        }
    }
}

/// One built-in card: a name and a page size.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentPreset {
    /// English name, shown through the catalog under `newDocument.preset.<id>`.
    pub name: &'static str,
    /// Stable id, the key of the preset's catalog entry (`newDocument.preset.<id>`).
    pub id: &'static str,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Resolution in pixels per inch.
    pub pixels_per_inch: f64,
}

impl DocumentPreset {
    /// A card named `name` (English, see [`name`](Self::name)) for a page of `width` × `height` pixels at `pixels_per_inch`.
    pub const fn new(id: &'static str, name: &'static str, width: u32, height: u32, pixels_per_inch: f64) -> Self {
        Self { name, id, width, height, pixels_per_inch }
    }

    /// Whether the preset is for print (300 ppi and up): its card and fields show inches at the
    /// app's print resolution; screen presets stay in pixels.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:172 is_print
    pub fn is_print(&self) -> bool {
        self.pixels_per_inch >= 300.0
    }

    /// The preset as settings: the unit (inches for print, pixels for screen, PhotoCraft
    /// `preset_unit`) and the resolution moved to the app's print or screen resolution, keeping
    /// a print preset's physical size.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:208 at_pref_resolutions, :192 preset_unit
    pub fn options(&self, print_ppi: f64, screen_ppi: f64) -> DocumentOptions {
        let print = self.is_print();
        let wanted = if print { print_ppi } else { screen_ppi };
        let (width, height, pixels_per_inch) = if !(wanted.is_finite() && wanted > 0.0) {
            (self.width, self.height, self.pixels_per_inch)
        } else if print {
            let scale = wanted / self.pixels_per_inch;
            let pixels = |side: u32| (f64::from(side) * scale).round().clamp(1.0, MAX_PIXELS) as u32;
            (pixels(self.width), pixels(self.height), wanted)
        } else {
            (self.width, self.height, wanted)
        };
        DocumentOptions {
            size: PixelSize { width, height, pixels_per_inch },
            unit: if print { SizeUnit::Inches } else { SizeUnit::Pixels },
            ..DocumentOptions::default()
        }
    }
}

/// A category tab and its cards.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetCategory {
    /// Stable id: `"photo"`.
    pub id: &'static str,
    /// English title, translated through [`NewDocumentLabels`] for the built-in ids.
    pub title: &'static str,
    /// The category's cards in display order.
    pub presets: Vec<DocumentPreset>,
}

impl PresetCategory {
    /// PhotoCraft's categories in tab order: Photo, Print, Art & Illustration, Web, Mobile, Film &
    /// Video, with their presets (Photoshop's tabs; Recent and Saved are not in this list, the
    /// dialog adds them first and last). Recent's built-in defaults (Default {app} Size,
    /// HDTV 1080p) are [`NewDocumentDialog::default_recent`].
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:19 CATEGORIES
    pub fn standard() -> Vec<PresetCategory> {
        let category = |id, title, presets: &[DocumentPreset]| PresetCategory { id, title, presets: presets.to_vec() };
        let p = DocumentPreset::new;
        vec![
            category(
                "photo",
                "Photo",
                &[
                    p("landscape-6x4", "Landscape, 6 x 4", 1800, 1200, 300.0),
                    p("landscape-7x5", "Landscape, 7 x 5", 2100, 1500, 300.0),
                    p("landscape-10x8", "Landscape, 10 x 8", 3000, 2400, 300.0),
                    p("portrait-4x6", "Portrait, 4 x 6", 1200, 1800, 300.0),
                    p("portrait-5x7", "Portrait, 5 x 7", 1500, 2100, 300.0),
                    p("square-5x5", "Square, 5 x 5", 1500, 1500, 300.0),
                ],
            ),
            category(
                "print",
                "Print",
                &[
                    p("letter", "Letter", 2550, 3300, 300.0),
                    p("legal", "Legal", 2550, 4200, 300.0),
                    p("tabloid", "Tabloid", 3300, 5100, 300.0),
                    p("a4", "A4", 2480, 3508, 300.0),
                    p("a3", "A3", 3508, 4961, 300.0),
                    p("a5", "A5", 1748, 2480, 300.0),
                    p("b3", "B3", 4169, 5906, 300.0),
                    p("b4", "B4", 2953, 4169, 300.0),
                ],
            ),
            category(
                "art",
                "Art & Illustration",
                &[
                    p("poster", "Poster", 5400, 7200, 300.0),
                    p("postcard", "Postcard", 1800, 1200, 300.0),
                    p("comic-book", "Comic Book", 1988, 3075, 300.0),
                    p("square-12x12", "Square, 12 x 12", 3600, 3600, 300.0),
                ],
            ),
            category(
                "web",
                "Web",
                &[
                    p("web-most-common", "Web Most Common", 1366, 768, 72.0),
                    p("web-minimum", "Web Minimum", 1024, 768, 72.0),
                    p("web-large", "Web Large", 1920, 1080, 72.0),
                    p("macbook-pro-16", "MacBook Pro 16\"", 3456, 2234, 72.0),
                    p("imac-24", "iMac 24\"", 4480, 2520, 72.0),
                ],
            ),
            category(
                "mobile",
                "Mobile",
                &[
                    p("iphone-16", "iPhone 16", 1179, 2556, 72.0),
                    p("iphone-16-pro-max", "iPhone 16 Pro Max", 1320, 2868, 72.0),
                    p("ipad-pro-13", "iPad Pro 13\"", 2064, 2752, 72.0),
                    p("android-1080p", "Android 1080p", 1080, 1920, 72.0),
                    p("apple-watch-45", "Apple Watch 45mm", 396, 484, 72.0),
                ],
            ),
            category(
                "film",
                "Film & Video",
                &[
                    p("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0),
                    p("hdtv-720p", "HDTV 720p", 1280, 720, 72.0),
                    p("uhd-4k", "UHD 4K", 3840, 2160, 72.0),
                    p("dci-4k", "DCI 4K", 4096, 2160, 72.0),
                    p("uhd-8k", "UHD 8K", 7680, 4320, 72.0),
                ],
            ),
        ]
    }
}

/// File › New, after Photoshop's New Document window and PhotoCraft's port of it (spec Q-S4F-3).
///
/// The user sees category tabs (Recent, Photo, Print, Art & Illustration, Web, Mobile, Film &
/// Video, Saved; built as DS tabs) over a grid of cards three wide, at most three rows tall and
/// scrolling; each card shows the page outline at its aspect (longer side 30 px), the name (two
/// lines, elided) and its size line ("1800 × 1200 px @ 300 ppi"). The selected card has the
/// accent border. Recent lists the clipboard's image size first when the app passed one, then
/// recent documents newest first (up to 20, remembered across runs under
/// `"newDocument.recent"`), then the built-in defaults not yet in it. Saved lists the user's
/// presets with Delete per row and "No saved presets yet." when there are none.
///
/// The details column on the right: Name (focused with its text selected on open), Width with the
/// unit list, Height with the portrait and landscape buttons, Artboards, Resolution with its unit
/// list, Color Mode, Depth, Color Profile ("Working: sRGB IEC61966-2.1" first) and Background
/// Contents with a swatch. Save Preset… opens a name row (Preset Name, Save, Cancel) under the
/// column; Enter there saves the preset and does not create the document. A name is trimmed and
/// must have 1–255 characters ([`NewDocumentLabels::name_length`]); a name already saved
/// (ignoring ASCII case) is refused ([`NewDocumentLabels::name_taken`]); at most 256 presets are
/// kept ([`NewDocumentLabels::too_many`]).
///
/// Keys: Tab moves Name → Width → Height → Resolution and on through the lists; arrows move in the
/// card grid ([`GridNav`](crate::behavior::GridNav)); a click on a card fills the details; a
/// double-click creates. Enter is Create when no list is open; Esc is Close. The buttons are
/// "Create" and "Close" (Photoshop's words). Width 800 px.
///
/// Custom… under Background Contents asks the app for a colour: for one pass
/// [`pick_color`](NewDocumentDialog::pick_color) returns a [`ColorRequest`] keyed
/// [`BACKGROUND_COLOR_KEY`]; the app opens its colour picker as a child dialog and hands the
/// answer back with [`set_color`](NewDocumentDialog::set_color) (or
/// [`cancel_color`](NewDocumentDialog::cancel_color)) through `dialogs.get_mut::<NewDocumentDialog>()`.
///
/// The dialog opens with the last created settings (`"newDocument.last"`, PhotoCraft's
/// `file.new` memory) and stores them, and a recent entry, on Create. Result:
/// [`DocumentOptions`].
///
/// ```ignore
/// dialogs.open(NewDocumentDialog::new().clipboard(clip_size).background_color(bg));
/// if let Some(DialogResult::Ok(settings)) = dialogs.take::<NewDocumentDialog>() { app.new_document(settings); }
/// ```
pub struct NewDocumentDialog {
    id: DialogId,
    categories: Vec<PresetCategory>,
    recent_defaults: Vec<DocumentPreset>,
    clipboard: Option<PixelSize>,
    background_color: [f32; 3],
    print_ppi: f64,
    screen_ppi: f64,
    profiles: Vec<(ColorMode, String)>,
    working: Vec<(ColorMode, String)>,
    settings: DocumentOptions,
    /// The open tab's id.
    tab: String,
    /// The card the user picked in the open tab (the saved row in Saved).
    selected_card: Option<usize>,
    /// The Save Preset name row, when open.
    preset_name: Option<String>,
    /// A pick was asked for and is unanswered.
    awaiting_color: bool,
    /// The request to hand out once.
    color_request: Option<ColorRequest>,
    /// Set once the remembered values were read (first pass, which has the Host).
    recalled: bool,
    state: DialogState,
    recent: Vec<DocumentOptions>,
    saved: Vec<DocumentOptions>,
    /// The page orientation picked with the two buttons; built-in cards turn with it.
    orientation: Option<Orientation>,
    /// The background before Custom… was chosen, for [`cancel_color`](NewDocumentDialog::cancel_color).
    before_custom: Option<BackgroundContents>,
    /// Why the last Save Preset was refused.
    error: Option<String>,
    /// A double-click on a card or saved row asked for Create.
    create_now: bool,
    focus_preset_name: bool,
}

impl Default for NewDocumentDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl NewDocumentDialog {
    /// The dialog with [`PresetCategory::standard`], print resolution 300 and screen resolution
    /// 72 (PhotoCraft's preference defaults).
    pub fn new() -> Self {
        Self {
            id: dialog_id("new-document"),
            categories: PresetCategory::standard(),
            recent_defaults: Self::default_recent(),
            clipboard: None,
            background_color: [1.0, 1.0, 1.0],
            print_ppi: 300.0,
            screen_ppi: 72.0,
            profiles: Vec::new(),
            working: Vec::new(),
            settings: DocumentOptions::default(),
            tab: "recent".to_owned(),
            selected_card: None,
            preset_name: None,
            awaiting_color: false,
            color_request: None,
            recalled: false,
            state: DialogState::new(),
            recent: Vec::new(),
            saved: Vec::new(),
            orientation: None,
            before_custom: None,
            error: None,
            create_now: false,
            focus_preset_name: false,
        }
    }

    /// Replaces the built-in categories (an app with other page sizes: DesignCraft's picas).
    pub fn categories(mut self, categories: Vec<PresetCategory>) -> Self {
        self.categories = categories;
        self
    }

    /// The clipboard holds an image of this size: Recent shows a Clipboard card first and the
    /// dialog opens on it (Photoshop, PhotoCraft).
    pub fn clipboard(mut self, size: PixelSize) -> Self {
        self.clipboard = Some(size);
        self.settings.size = size;
        self.settings.unit = SizeUnit::Pixels;
        self.selected_card = Some(0);
        self
    }

    /// The app's background colour, drawn on the Background Color choice's swatch.
    pub fn background_color(mut self, rgb: [f32; 3]) -> Self {
        self.background_color = rgb;
        self
    }

    /// The resolutions print and screen presets open at (PhotoCraft's Units & Rulers › New
    /// Document Preset Resolutions).
    pub fn resolutions(mut self, print_ppi: f64, screen_ppi: f64) -> Self {
        self.print_ppi = print_ppi;
        self.screen_ppi = screen_ppi;
        self
    }

    /// The profiles the Color Profile list offers for `mode`, with the working one first.
    pub fn profiles(mut self, mode: ColorMode, working: impl Into<String>, others: Vec<String>) -> Self {
        let working = working.into();
        self.working.push((mode, working));
        self.profiles.extend(others.into_iter().map(|p| (mode, p)));
        self
    }

    /// Recent's built-in cards after the recent documents, when [`recent_defaults`](Self::recent_defaults)
    /// is not called: "Default {app} Size" (2100 × 1500 px at 300 ppi, the name from
    /// [`Host::app`]) and HDTV 1080p.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:20 CATEGORIES "Recent"
    pub fn default_recent() -> Vec<DocumentPreset> {
        vec![DocumentPreset::new(DEFAULT_ID, "Default Size", 2100, 1500, 300.0), DocumentPreset::new("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0)]
    }

    /// Recent's built-in cards after the recent documents, in place of
    /// [`default_recent`](Self::default_recent). A card with the id `"default"` is named
    /// "Default {app} Size".
    pub fn recent_defaults(mut self, presets: Vec<DocumentPreset>) -> Self {
        self.recent_defaults = presets;
        self
    }

    /// The settings shown in the details column.
    pub fn options(&self) -> &DocumentOptions {
        &self.settings
    }

    /// Custom… was chosen (or the swatch clicked) in the pass since the last call: the colour to
    /// ask the app's picker for, keyed [`BACKGROUND_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: Background Contents becomes
    /// [`BackgroundContents::Custom`]. Other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key != BACKGROUND_COLOR_KEY {
            return;
        }
        self.settings.background = BackgroundContents::Custom(color.to_rgb_array());
        self.before_custom = None;
        self.awaiting_color = false;
        self.color_request = None;
        self.selected_card = None;
    }

    /// The picker was cancelled: Background Contents goes back to what it was before Custom….
    pub fn cancel_color(&mut self) {
        if let Some(before) = self.before_custom.take() {
            self.settings.background = before;
        }
        self.awaiting_color = false;
        self.color_request = None;
    }
}

/// Everything one pass reads besides the dialog's own state.
struct Env<'a> {
    ctx: &'a Context,
    labels: &'a NewDocumentLabels,
    sizing: &'a SizingLabels,
    units: &'a LengthUnitLabels,
    numbers: &'a LocaleNumbers,
}

/// A card of the grid, ready to draw and to apply.
struct Card {
    title: String,
    /// What the card sets: the size and unit, or (a recent document) everything but the name.
    settings: DocumentOptions,
    restores_all: bool,
    /// The hover text of a recent document, which names everything it restores.
    detail: Option<String>,
    /// The aspect the page outline has.
    aspect: (u32, u32),
}

impl StackDialog for NewDocumentDialog {
    type Result = DocumentOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    /// One pass: recalls the remembered values on the first pass, draws tabs, cards, saved
    /// presets and the details column on [`Dialog`](super::Dialog) with Create and Close; on
    /// Create stores the last values and the recent entry through
    /// [`Remembered`](craft_settings::Remembered) and returns the settings. Saving and deleting a
    /// preset write `"newDocument.presets"` at once (they outlive a Close).
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:528 body (tabs :528, cards :578, details :643, saved :440, save_preset :482);
    // pc: crates/ui-egui/src/dialogs.rs:997 confirm, crates/ui-egui/src/app/files.rs:653 remember_new_document
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DocumentOptions>> {
        let labels = NewDocumentLabels::shared_ctx(ctx);
        if !self.recalled {
            self.recall(ctx, &labels);
        }
        let sizing = SizingLabels::shared_ctx(ctx);
        let units = LengthUnitLabels::shared_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let env = Env { ctx, labels: &labels, sizing: &sizing, units: &units, numbers: &numbers };
        let mut state = std::mem::take(&mut self.state);
        let footer = Footer::standard().ok(labels.create.as_str()).cancel(labels.close.as_str());
        let shown = Dialog::new("new-document", labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(dialog_width(&crate::theme::current(ctx))))
            .footer(footer)
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &env));
        self.state = state;
        let outcome = match shown.outcome {
            Some(outcome) => Some(outcome),
            None if self.create_now => {
                self.state.close();
                Some(Outcome::Ok)
            }
            None => None,
        };
        self.create_now = false;
        match outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.create(ctx, &labels))),
            Some(_) => Some(DialogResult::Cancel),
            None => None,
        }
    }

    /// Create with the dialog's values.
    fn confirm(&mut self, ctx: &Context) -> Option<DocumentOptions> {
        let labels = NewDocumentLabels::shared_ctx(ctx);
        if !self.recalled {
            self.recall(ctx, &labels);
        }
        Some(self.create(ctx, &labels))
    }
}

impl NewDocumentDialog {
    /// Reads the remembered values: the last settings (not with a clipboard image, which opens
    /// the dialog on its own card), Recent and the saved presets.
    fn recall(&mut self, ctx: &Context, labels: &NewDocumentLabels) {
        self.recalled = true;
        if let Some(host) = Host::try_of_ctx(ctx) {
            let list = |key: &str, max: usize| -> Vec<DocumentOptions> {
                let mut list = Remembered::<Vec<DocumentOptions>>::new(key).recall(&host).unwrap_or_default();
                list.retain(fits);
                list.truncate(max);
                list
            };
            self.recent = list(RECENT_KEY, MAX_RECENT);
            self.saved = list(PRESETS_KEY, MAX_SAVED);
            if self.clipboard.is_none()
                && let Some(last) = Remembered::<DocumentOptions>::new(LAST_KEY).recall(&host).filter(fits)
            {
                self.settings = last;
            }
        }
        self.settings.name = labels.untitled.clone();
    }

    /// The settings Create returns, remembered as the last values and the newest recent entry.
    fn create(&mut self, ctx: &Context, labels: &NewDocumentLabels) -> DocumentOptions {
        let mut settings = self.settings.clone();
        settings.name = settings.name.trim().to_owned();
        if settings.name.is_empty() {
            settings.name.clone_from(&labels.untitled);
        }
        let mut remembered = settings.clone();
        remembered.name.clear();
        push_recent(&mut self.recent, remembered.clone(), MAX_RECENT);
        if let Some(host) = Host::try_of_ctx(ctx) {
            Remembered::<DocumentOptions>::new(LAST_KEY).store(&host, &remembered);
            Remembered::<Vec<DocumentOptions>>::new(RECENT_KEY).store(&host, &self.recent);
        }
        settings
    }

    fn store_presets(&self, ctx: &Context) {
        if let Some(host) = Host::try_of_ctx(ctx) {
            Remembered::<Vec<DocumentOptions>>::new(PRESETS_KEY).store(&host, &self.saved);
        }
    }

    fn body(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let tokens = Tokens::of(ui);
        let gap = Gap::Section.points(&tokens);
        ui.horizontal_top(|ui| {
            let grid_width = grid_width(&tokens);
            ui.vertical(|ui| {
                ui.set_width(grid_width);
                self.tab_strip(ui, env);
                ui.add_space(Gap::Row.points(&tokens));
                Divider::horizontal().show(ui);
                ui.add_space(Gap::Row.points(&tokens));
                if self.tab == SAVED_TAB {
                    self.saved_list(ui, env);
                } else {
                    self.card_grid(ui, env);
                }
            });
            ui.add_space(gap);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                self.details(ui, env);
            });
        });
    }

    fn tab_strip(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let tokens = Tokens::of(ui);
        let mut tabs: Vec<(String, String)> = vec![(RECENT_TAB.to_owned(), env.labels.recent.clone())];
        tabs.extend(self.categories.iter().map(|c| (c.id.to_owned(), category_title(env.labels, c))));
        tabs.push((SAVED_TAB.to_owned(), env.labels.saved.clone()));
        let ids: Vec<Id> = tabs.iter().map(|(key, _)| Interactive::id_in(ui, ("new-document-tab", key.clone()))).collect();
        let pad = Pad::Control.points(&tokens);
        let mut picked = None;
        let open = tabs.iter().position(|(key, _)| *key == self.tab);
        let stop = GridNav::roving_stop(ui, &ids, open, true);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Gap::Section.points(&tokens);
            for (i, (key, label)) in tabs.iter().enumerate() {
                let on = *key == self.tab;
                let content = if on { Content::Normal } else { Content::Muted };
                let format = Text::format(&tokens, TypeRole::Label, content);
                let galley = ui.painter().layout_job(LayoutJob::single_section(label.clone(), format.clone()));
                let (_, rect) = ui.allocate_space(vec2(galley.size().x, galley.size().y + 2.0 * pad));
                let mut element = Interactive::with_id(ids[i], Role::Tab, label.as_str()).selected(on).position_in_set(i);
                if i != stop {
                    element = element.sense(Sense::CLICK);
                }
                let response = element.interact_at(ui, rect).finish(ui);
                if ui.is_rect_visible(rect) {
                    let top = rect.top() + pad;
                    ui.painter().galley(pos2(rect.left(), top), galley, format.color);
                    if on {
                        let y = rect.bottom();
                        ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(2.0, format.color));
                    }
                }
                if response.clicked() {
                    picked = Some(key.clone());
                }
                if response.gained_focus() {
                    picked = Some(key.clone());
                }
            }
        });
        if let Some(key) = picked.filter(|key| *key != self.tab) {
            self.tab = key;
            self.selected_card = None;
        }
    }

    /// The built-in preset turned to the orientation picked with the buttons. Square presets
    /// stay as they are.
    fn oriented(&self, preset: &DocumentPreset) -> DocumentPreset {
        let Some(orientation) = self.orientation else { return preset.clone() };
        let portrait = orientation == Orientation::Portrait;
        if preset.width == preset.height || (preset.height > preset.width) == portrait {
            return preset.clone();
        }
        DocumentPreset { width: preset.height, height: preset.width, ..preset.clone() }
    }

    /// A card for a built-in preset, in the unit it applies: a print preset in the physical unit
    /// picked for Width and Height (inches otherwise), a screen preset in pixels.
    fn preset_card(&self, env: &Env<'_>, preset: &DocumentPreset) -> Card {
        let preset = self.oriented(preset);
        let mut settings = preset.options(self.print_ppi, self.screen_ppi);
        if preset.is_print() && !self.settings.unit.is_pixel_unit() {
            settings.unit = self.settings.unit;
        }
        let aspect = (settings.size.width, settings.size.height);
        Card { title: preset_title(env.ctx, env.labels, &preset), settings, restores_all: false, detail: None, aspect }
    }

    fn cards(&self, env: &Env<'_>) -> Vec<Card> {
        if self.tab != RECENT_TAB {
            return self
                .categories
                .iter()
                .find(|c| c.id == self.tab)
                .map_or_else(Vec::new, |c| c.presets.iter().map(|p| self.preset_card(env, p)).collect());
        }
        let mut cards = Vec::new();
        if let Some(size) = self.clipboard {
            let preset = DocumentPreset::new(CLIPBOARD_ID, "Clipboard", size.width, size.height, size.pixels_per_inch);
            let settings = DocumentOptions { size, unit: SizeUnit::Pixels, ..DocumentOptions::default() };
            cards.push(Card {
                title: preset_title(env.ctx, env.labels, &preset),
                settings,
                restores_all: false,
                detail: None,
                aspect: (size.width, size.height),
            });
        }
        for recent in &self.recent {
            let title = self.recent_title(env, recent);
            let line = size_line(recent, env.labels, env.numbers);
            let detail =
                format!("{title} · {line} · {} · {}", mode_label(env.labels, recent.color_mode), background_label(env.labels, recent.background));
            cards.push(Card {
                title,
                settings: recent.clone(),
                restores_all: true,
                detail: Some(detail),
                aspect: (recent.size.width, recent.size.height),
            });
        }
        for preset in self.recent_defaults.clone() {
            let card = self.preset_card(env, &preset);
            if !self.recent.iter().any(|r| r.size == card.settings.size) {
                cards.push(card);
            }
        }
        cards
    }

    /// A recent document takes the name of the built-in preset it matches.
    fn recent_title(&self, env: &Env<'_>, recent: &DocumentOptions) -> String {
        let listed =
            |p: &&DocumentPreset| (p.width, p.height, p.pixels_per_inch) == (recent.size.width, recent.size.height, recent.size.pixels_per_inch);
        let defaults = &self.recent_defaults;
        let found = self.categories.iter().flat_map(|c| c.presets.iter()).chain(defaults.iter()).find(listed);
        match found {
            Some(preset) => preset_title(env.ctx, env.labels, preset),
            None => env.labels.custom.clone(),
        }
    }

    fn apply_card(&mut self, card: &Card) {
        if card.restores_all {
            let name = std::mem::take(&mut self.settings.name);
            self.settings = DocumentOptions { name, ..card.settings.clone() };
            self.before_custom = None;
            self.awaiting_color = false;
            self.color_request = None;
        } else {
            self.settings.size = card.settings.size;
            self.settings.unit = card.settings.unit;
        }
    }

    fn card_grid(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let tokens = Tokens::of(ui);
        let cards = self.cards(env);
        let count = craft_i18n::fmt(&env.labels.presets_heading, &[("n", &env.numbers.format(cards.len() as f64, 0..=0))]);
        SectionLabel::new(count).show(ui);
        ui.add_space(Gap::Row.points(&tokens));
        let gap = Gap::Row.points(&tokens);
        let inset = Pad::Control.points(&tokens);
        let title_format = Text::format(&tokens, TypeRole::Body, Content::Normal);
        let line_format = Text::format(&tokens, TypeRole::Caption, Content::Faint);
        let outline = Content::Muted.color(&tokens).to_egui();
        let accent = Content::Accent.color(&tokens).to_egui();
        let (card_size, columns, max_rows) = (Vec2::from(CARD), CARD_COLUMNS, CARD_ROWS);
        let rows = cards.len().div_ceil(columns);
        let ids: Vec<Id> = (0..cards.len()).map(|i| Interactive::id_in(ui, ("new-document-card", self.tab.clone(), i))).collect();
        let mut clicked = None;
        let mut double = false;
        ScrollArea::vertical()
            .id_salt(("new-document-cards", self.tab.clone()))
            .max_height(max_rows as f32 * card_size.y + (max_rows - 1) as f32 * gap)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let width = columns as f32 * card_size.x + (columns - 1) as f32 * gap;
                let (_, area) = ui.allocate_space(vec2(width, rows as f32 * card_size.y + rows.saturating_sub(1) as f32 * gap));
                let selected = self.selected_card.filter(|i| *i < cards.len());
                let stop = GridNav::roving_stop(ui, &ids, selected, true);
                for (i, card) in cards.iter().enumerate() {
                    let (row, column) = (i / columns, i % columns);
                    let rect = Rect::from_min_size(area.min + vec2(column as f32 * (card_size.x + gap), row as f32 * (card_size.y + gap)), card_size);
                    let on = selected == Some(i)
                        || (selected.is_none() && self.selected_card.is_none() && card.restores_all && same_document(&card.settings, &self.settings));
                    let mut element = Interactive::with_id(ids[i], Role::Button, card.title.as_str())
                        .surface(SurfaceKind::Raised)
                        .corner(Corner::Control)
                        .selected(on);
                    if i != stop {
                        element = element.sense(Sense::CLICK);
                    }
                    let line = size_line(&card.settings, env.labels, env.numbers);
                    let response = element.interact_at(ui, rect).finish(ui);
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_description(line.clone());
                        if i != stop {
                            node.add_action(Action::Focus);
                        }
                    });
                    if response.gained_focus() {
                        ui.scroll_to_rect(rect, None);
                    }
                    let mut elided = false;
                    if ui.is_rect_visible(rect) {
                        if on {
                            ui.painter().rect_stroke(rect, Corner::Control.radius(&tokens), Stroke::new(1.5, accent), StrokeKind::Inside);
                        }
                        let center = pos2(rect.center().x, rect.top() + CARD_ICON_CENTER);
                        paint_page(ui, Rect::from_center_size(center, Vec2::splat(PAGE_BOX)), card.aspect, PAGE_ICON, outline);
                        let mut job = LayoutJob::single_section(card.title.clone(), title_format.clone());
                        job.wrap.max_width = card_size.x - 2.0 * inset;
                        job.wrap.max_rows = 2;
                        job.halign = Align::Center;
                        let title = ui.painter().layout_job(job);
                        elided = title.elided;
                        let top = rect.top() + CARD_TITLE_CENTER - title.size().y / 2.0;
                        ui.painter().galley(pos2(rect.center().x, top), title, title_format.color);
                        let mut job = LayoutJob::single_section(line, line_format.clone());
                        job.wrap.max_width = card_size.x - 2.0 * inset;
                        job.wrap.max_rows = 1;
                        job.halign = Align::Center;
                        let galley = ui.painter().layout_job(job);
                        let top = rect.top() + CARD_LINE_CENTER - galley.size().y / 2.0;
                        ui.painter().galley(pos2(rect.center().x, top), galley, line_format.color);
                    }
                    let response = match (&card.detail, elided) {
                        (Some(detail), _) => response.on_hover_text(detail.as_str()),
                        (None, true) => response.on_hover_text(card.title.as_str()),
                        (None, false) => response,
                    };
                    if response.clicked() {
                        clicked = Some(i);
                    }
                    if response.double_clicked() {
                        double = true;
                    }
                }
            });
        if let Some(i) = clicked {
            self.apply_card(&cards[i]);
            self.selected_card = Some(i);
            self.create_now |= double;
        }
        if let Some(stop) = ids.get(self.selected_card.unwrap_or(0)).or(ids.first()) {
            FieldTab::register_stop(ui, *stop);
        }
    }

    fn saved_list(&mut self, ui: &mut Ui, env: &Env<'_>) {
        if self.saved.is_empty() {
            ui.add(Text::new(env.labels.no_saved.as_str()).content(Content::Muted));
            return;
        }
        let tokens = Tokens::of(ui);
        let (saved_height, saved_row) = (SAVED_ROWS * tokens.size.control.md, tokens.size.control.md);
        let mut apply = None;
        let mut delete = None;
        let mut double = false;
        ScrollArea::vertical().id_salt("new-document-saved").max_height(saved_height).auto_shrink([false, true]).show(ui, |ui| {
            for (i, preset) in self.saved.iter().enumerate() {
                ui.push_id(i, |ui| {
                    ui.horizontal(|ui| {
                        let delete_width = Button::new(env.labels.delete.as_str()).natural_width(ui);
                        let width = ui.available_width() - delete_width - ui.spacing().item_spacing.x;
                        let (_, rect) = ui.allocate_space(vec2(width, saved_row));
                        let on = self.selected_card == Some(i);
                        let response = Interactive::new(("new-document-saved", i), Role::Button, preset.name.as_str())
                            .surface(SurfaceKind::Raised)
                            .corner(Corner::Control)
                            .selected(on)
                            .interact_at(ui, rect)
                            .finish(ui);
                        if ui.is_rect_visible(rect) {
                            let format = Text::format(&tokens, TypeRole::Body, Content::Normal);
                            let mut job = LayoutJob::single_section(preset.name.clone(), format.clone());
                            job.wrap = TextWrapping::truncate_at_width(width - 2.0 * Pad::Control.points(&tokens));
                            let galley = ui.painter().layout_job(job);
                            let pos = pos2(rect.left() + Pad::Control.points(&tokens), rect.center().y - galley.size().y / 2.0);
                            ui.painter().galley(pos, galley, format.color);
                        }
                        let response = response.on_hover_text(preset.name.as_str());
                        if response.clicked() {
                            apply = Some(i);
                        }
                        double |= response.double_clicked();
                        if ui.add(Button::new(env.labels.delete.as_str()).id_salt(("new-document-delete", i))).clicked() {
                            delete = Some(i);
                        }
                    });
                    let line = format!(
                        "{} · {}, {}",
                        size_line(preset, env.labels, env.numbers),
                        mode_label(env.labels, preset.color_mode),
                        depth_label(env.labels, preset.depth)
                    );
                    ui.add(Text::new(line).role(TypeRole::Caption).content(Content::Muted));
                    ui.add_space(Gap::Row.points(&tokens));
                });
            }
        });
        if let Some(i) = apply {
            let preset = self.saved[i].clone();
            self.apply_card(&Card { title: String::new(), aspect: (0, 0), detail: None, restores_all: true, settings: preset });
            self.selected_card = Some(i);
            self.create_now |= double;
        }
        if let Some(i) = delete {
            self.saved.remove(i);
            self.selected_card = None;
            self.store_presets(env.ctx);
        }
    }

    fn details(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let tokens = Tokens::of(ui);
        let labels = env.labels;
        let before = self.settings.clone();
        let mut keep_selection = false;
        SectionLabel::new(labels.preset_details.as_str()).show(ui);
        ui.add_space(Gap::Row.points(&tokens));
        let edit = TextField::new("new-document-name", labels.name.as_str(), &self.settings.name).char_limit(MAX_NAME).show(ui);
        if let Some(name) = edit.event.as_ref().and_then(EditEvent::value) {
            self.settings.name.clone_from(name);
        }
        let mut fields = LinkedSizeFields::new(self.settings.size);
        fields.constrain = false;
        fields.chain = false;
        fields.unit = self.settings.unit;
        fields.resolution_unit = self.settings.resolution_unit;
        caption(ui, &labels.width);
        size_row(ui, env, &mut fields, Field::Width);
        caption(ui, &labels.height);
        size_row(ui, env, &mut fields, Field::Height);
        caption(ui, &labels.orientation);
        let mut orientation = Orientation::of(fields.current);
        let turned = Segmented::new("new-document-orientation", labels.orientation.as_str(), &mut orientation)
            .icon_item(Orientation::Portrait, Icon::OrientationPortrait, labels.portrait.as_str())
            .icon_item(Orientation::Landscape, Icon::OrientationLandscape, labels.landscape.as_str())
            .show(ui);
        if turned.changed() {
            self.orientation = Some(orientation);
            if Orientation::of(fields.current) != orientation {
                let size = fields.current;
                fields.current = PixelSize { width: size.height, height: size.width, ..size };
            }
            keep_selection = true;
        }
        ui.add_space(Gap::Row.points(&tokens));
        let _ = Checkbox::bool(labels.artboards.as_str(), &mut self.settings.artboard).show(ui);
        caption(ui, &labels.resolution);
        resolution_row(ui, env, &mut fields);
        self.settings.size = fields.current;
        self.settings.unit = fields.unit;
        self.settings.resolution_unit = fields.resolution_unit;

        caption(ui, &labels.color_mode);
        ui.horizontal(|ui| {
            let mut mode = self.settings.color_mode;
            let _ = Select::new("new-document-mode", labels.color_mode.as_str(), &mut mode)
                .option(ColorMode::Grayscale, mode_label(labels, ColorMode::Grayscale))
                .option(ColorMode::Rgb, mode_label(labels, ColorMode::Rgb))
                .option(ColorMode::Cmyk, mode_label(labels, ColorMode::Cmyk))
                .option(ColorMode::Lab, mode_label(labels, ColorMode::Lab))
                .show(ui);
            if mode != self.settings.color_mode {
                self.settings.color_mode = mode;
                // A profile of the old mode's colour space does not suit the new one.
                self.settings.profile = None;
            }
            let mut depth = self.settings.depth;
            let _ = Select::new("new-document-depth", labels.depth.as_str(), &mut depth)
                .option(BitDepth::Eight, depth_label(labels, BitDepth::Eight))
                .option(BitDepth::Sixteen, depth_label(labels, BitDepth::Sixteen))
                .option(BitDepth::ThirtyTwo, depth_label(labels, BitDepth::ThirtyTwo))
                .show(ui);
            self.settings.depth = depth;
        });
        caption(ui, &labels.profile);
        self.profile_row(ui, env);
        caption(ui, &labels.background);
        self.background_row(ui, env);
        self.save_preset(ui, env);

        let mut same = self.settings.clone();
        same.name.clone_from(&before.name);
        same.unit = before.unit;
        same.resolution_unit = before.resolution_unit;
        if same != before && !keep_selection {
            self.selected_card = None;
        }
    }

    fn profile_row(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let labels = env.labels;
        let mode = self.settings.color_mode;
        let working = self.working.iter().find(|(m, _)| *m == mode).map(|(_, name)| name.as_str());
        let working_label = working.map_or_else(|| labels.working.clone(), |name| craft_i18n::fmt(&labels.working_profile, &[("name", name)]));
        let others: Vec<&str> = self.profiles.iter().filter(|(m, _)| *m == mode).map(|(_, name)| name.as_str()).collect();
        let mut profile = self.settings.profile.clone().filter(|p| others.contains(&p.as_str()));
        let mut select = Select::new("new-document-profile", labels.profile.as_str(), &mut profile).option(None, working_label);
        for name in &others {
            select = select.option(Some((*name).to_owned()), *name);
        }
        let _ = select.show(ui);
        self.settings.profile = profile;
    }

    fn background_row(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let labels = env.labels;
        let shown = self.settings.background;
        let mut kind = BackgroundKind::of(shown);
        let mut picks = false;
        ui.horizontal(|ui| {
            let _ = Select::new("new-document-background", labels.background.as_str(), &mut kind)
                .option(BackgroundKind::White, labels.white.as_str())
                .option(BackgroundKind::Black, labels.black.as_str())
                .option(BackgroundKind::BackgroundColor, labels.background_color.as_str())
                .option(BackgroundKind::Transparent, labels.transparent.as_str())
                .option(BackgroundKind::Custom, labels.custom_color.as_str())
                .show(ui);
            let color = background_rgba(shown, self.background_color);
            let swatch = ui.add(Swatch::new("new-document-swatch", labels.custom_swatch.as_str(), color));
            // A click on the swatch asks for a colour again, also while the choice is Custom.
            picks |= swatch.clicked();
        });
        if picks {
            kind = BackgroundKind::Custom;
        }
        if picks || kind != BackgroundKind::of(shown) {
            self.selected_card = None;
            match kind {
                BackgroundKind::White => self.settings.background = BackgroundContents::White,
                BackgroundKind::Black => self.settings.background = BackgroundContents::Black,
                BackgroundKind::BackgroundColor => self.settings.background = BackgroundContents::BackgroundColor,
                BackgroundKind::Transparent => self.settings.background = BackgroundContents::Transparent,
                BackgroundKind::Custom => {
                    if !self.awaiting_color {
                        self.before_custom = Some(shown);
                    }
                    self.awaiting_color = true;
                    let current = Srgba::from_rgb_array(match shown {
                        BackgroundContents::Custom(c) => c,
                        BackgroundContents::White => [1.0; 3],
                        BackgroundContents::Black => [0.0; 3],
                        BackgroundContents::BackgroundColor => self.background_color,
                        BackgroundContents::Transparent => [1.0; 3],
                    });
                    self.color_request = Some(ColorRequest::new(BACKGROUND_COLOR_KEY, current).title(labels.custom_swatch.as_str()));
                }
            }
        }
    }

    fn save_preset(&mut self, ui: &mut Ui, env: &Env<'_>) {
        let labels = env.labels;
        let tokens = Tokens::of(ui);
        ui.add_space(Gap::Stack.points(&tokens));
        let Some(mut name) = self.preset_name.take() else {
            if ui.add(Button::new(labels.save_preset.as_str()).variant(ButtonVariant::Secondary)).clicked() {
                self.preset_name = Some(String::new());
                self.error = None;
                self.focus_preset_name = true;
            }
            self.show_error(ui);
            return;
        };
        let field = ui.make_persistent_id("new-document-preset-name");
        // Taken before the dialog sees it, so naming a preset never creates the document.
        let enter = ui.memory(|m| m.has_focus(field)) && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
        if std::mem::take(&mut self.focus_preset_name) {
            ui.memory_mut(|m| m.request_focus(field));
        }
        caption(ui, &labels.preset_name);
        let edit = TextField::new("new-document-preset-name", labels.preset_name.as_str(), &name).id(field).char_limit(MAX_NAME).show(ui);
        if let Some(typed) = edit.event.as_ref().and_then(EditEvent::value) {
            name.clone_from(typed);
        }
        let mut save = enter;
        let mut cancel = false;
        ui.horizontal(|ui| {
            save |= ui.add(Button::new(labels.save.as_str()).variant(ButtonVariant::Secondary)).clicked();
            cancel = ui.add(Button::new(labels.cancel.as_str()).variant(ButtonVariant::Secondary)).clicked();
        });
        if cancel {
            self.error = None;
            self.show_error(ui);
            return;
        }
        if save {
            match self.try_save(labels, &name) {
                Ok(()) => {
                    self.store_presets(env.ctx);
                    self.tab = SAVED_TAB.to_owned();
                    self.selected_card = Some(self.saved.len() - 1);
                    self.error = None;
                    return;
                }
                Err(error) => self.error = Some(error),
            }
        }
        self.preset_name = Some(name);
        self.show_error(ui);
    }

    /// Adds the current settings as a saved preset named `name`, or says why not.
    fn try_save(&mut self, labels: &NewDocumentLabels, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME {
            return Err(labels.name_length.clone());
        }
        if self.saved.iter().any(|preset| preset.name.eq_ignore_ascii_case(name)) {
            return Err(craft_i18n::fmt(&labels.name_taken, &[("name", name)]));
        }
        if self.saved.len() >= MAX_SAVED {
            return Err(labels.too_many.clone());
        }
        self.saved.push(DocumentOptions { name: name.to_owned(), ..self.settings.clone() });
        Ok(())
    }

    fn show_error(&self, ui: &mut Ui) {
        if let Some(error) = &self.error {
            ui.add(Label::new(error.as_str()).wrap().selectable(true));
        }
    }
}

/// The Background Contents list's choices; Custom stands for any colour of the user's own.
#[derive(Clone, Copy, PartialEq)]
enum BackgroundKind {
    White,
    Black,
    BackgroundColor,
    Transparent,
    Custom,
}

impl BackgroundKind {
    fn of(background: BackgroundContents) -> Self {
        match background {
            BackgroundContents::White => Self::White,
            BackgroundContents::Black => Self::Black,
            BackgroundContents::BackgroundColor => Self::BackgroundColor,
            BackgroundContents::Transparent => Self::Transparent,
            BackgroundContents::Custom(_) => Self::Custom,
        }
    }
}

/// The colour the new document's background starts with, as the swatch shows it.
fn background_rgba(background: BackgroundContents, app_background: [f32; 3]) -> Rgba {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    let rgb = |c: [f32; 3]| Rgba([byte(c[0]), byte(c[1]), byte(c[2]), 255]);
    match background {
        BackgroundContents::White => rgb([1.0; 3]),
        BackgroundContents::Black => rgb([0.0; 3]),
        BackgroundContents::BackgroundColor => rgb(app_background),
        BackgroundContents::Transparent => Rgba([0, 0, 0, 0]),
        BackgroundContents::Custom(c) => rgb(c),
    }
}

/// A field's caption above its control, as Photoshop's details pane.
fn caption(ui: &mut Ui, text: &str) {
    ui.add(Text::new(text).role(TypeRole::Caption).content(Content::Muted));
}

/// The least and greatest value of a size field in `unit`, and how it parses typed text.
fn size_row(ui: &mut Ui, env: &Env<'_>, fields: &mut LinkedSizeFields, field: Field) {
    let (id, name, unit_name) = match field {
        Field::Width => ("width", &env.labels.width, &env.sizing.width_unit),
        _ => ("height", &env.labels.height, &env.sizing.height_unit),
    };
    ui.horizontal(|ui| {
        let (unit, ppi, points) = (fields.unit, fields.current.pixels_per_inch, fields.points_per_inch);
        let original = f64::from(if field == Field::Width { fields.original.width } else { fields.original.height });
        let shown = fields.shown(field);
        let limit = unit.from_pixels(MAX_PIXELS, original, ppi, points);
        let least = unit.from_pixels(1.0, original, ppi, points).min(shown);
        let edit = NumberField::new(("new-document", id), name.as_str(), shown, env.numbers)
            .range(least..=limit)
            .decimals(unit.decimals()..=unit.decimals())
            .or_parse(move |text| unit.parse_typed(text, original, ppi, points))
            .show(ui);
        apply_edit(ui, fields, field, edit.event);
        let mut picked = fields.unit;
        let mut select = Select::new(("new-document", id, "unit"), unit_name.as_str(), &mut picked);
        for unit in UNITS {
            select = select.option(unit, unit.label(env.units));
        }
        let _ = select.show(ui);
        fields.unit = picked;
    });
}

fn resolution_row(ui: &mut Ui, env: &Env<'_>, fields: &mut LinkedSizeFields) {
    ui.horizontal(|ui| {
        let per = if fields.resolution_unit == ResolutionUnit::PixelsPerInch { 1.0 } else { CM_PER_INCH };
        let places = if fields.resolution_unit == ResolutionUnit::PixelsPerInch { 0 } else { 2 };
        let edit = NumberField::new("new-document-resolution", env.labels.resolution.as_str(), fields.current.pixels_per_inch / per, env.numbers)
            .range(PPI_RANGE.0 / per..=PPI_RANGE.1 / per)
            .decimals(places..=places)
            .show(ui);
        apply_edit(ui, fields, Field::Resolution, edit.event);
        let mut picked = fields.resolution_unit;
        let _ = Select::new("new-document-resolution-unit", env.sizing.resolution_unit.as_str(), &mut picked)
            .option(ResolutionUnit::PixelsPerInch, env.sizing.pixels_per_inch.as_str())
            .option(ResolutionUnit::PixelsPerCentimetre, env.sizing.pixels_per_centimetre.as_str())
            .show(ui);
        fields.resolution_unit = picked;
    });
}

/// Applies a field's edit and puts back the size from before a cancelled gesture.
fn apply_edit(ui: &Ui, fields: &mut LinkedSizeFields, field: Field, event: Option<EditEvent<f64>>) {
    let Some(event) = event else { return };
    let before = ui.make_persistent_id(("new-document-size-before", field as u8));
    match &event {
        EditEvent::Begin { .. } => {
            let current = fields.current;
            ui.data_mut(|d| d.insert_temp(before, current));
        }
        EditEvent::Cancel { .. } => {
            if let Some(size) = ui.data(|d| d.get_temp::<PixelSize>(before)) {
                fields.current = size;
            }
        }
        _ => {}
    }
    if let Some(value) = event.value() {
        fields.edit(field, *value);
    }
}

/// The page outline of a card: the preset's aspect with its longer side `longer` long,
/// centred in `rect`.
fn paint_page(ui: &Ui, rect: Rect, (width, height): (u32, u32), longer: f32, color: Color32) {
    let scale = longer / width.max(height).max(1) as f32;
    let page = Rect::from_center_size(rect.center(), vec2(width as f32 * scale, height as f32 * scale));
    ui.painter().rect_stroke(page, 1.0, Stroke::new(1.2, color), StrokeKind::Inside);
}

/// Whether `settings` fit what the dialog can show (a damaged remembered value does not).
fn fits(settings: &DocumentOptions) -> bool {
    let side = |n: u32| (1..=MAX_PIXELS as u32).contains(&n);
    let ppi = settings.size.pixels_per_inch;
    side(settings.size.width)
        && side(settings.size.height)
        && ppi.is_finite()
        && ppi > 0.0
        && ppi <= PPI_RANGE.1
        && settings.unit != SizeUnit::Percent
}

/// Whether two settings make the same document (the name and the display units apart).
fn same_document(a: &DocumentOptions, b: &DocumentOptions) -> bool {
    a.size == b.size && a.color_mode == b.color_mode && a.depth == b.depth && a.background == b.background && a.profile == b.profile
}

fn category_title(labels: &NewDocumentLabels, category: &PresetCategory) -> String {
    match category.id {
        "photo" => labels.photo.clone(),
        "print" => labels.print.clone(),
        "art" => labels.art.clone(),
        "web" => labels.web.clone(),
        "mobile" => labels.mobile.clone(),
        "film" => labels.film.clone(),
        _ => category.title.to_owned(),
    }
}

/// A preset's name in the Host's language: the catalog row `newDocument.preset.<id>`, else the
/// English name.
fn preset_title(ctx: &Context, labels: &NewDocumentLabels, preset: &DocumentPreset) -> String {
    if preset.id == CLIPBOARD_ID {
        return labels.clipboard.clone();
    }
    if preset.id == DEFAULT_ID
        && let Some(host) = Host::try_of_ctx(ctx)
    {
        return craft_i18n::fmt(&labels.default_size, &[("app", host.app().name)]);
    }
    match Host::try_of_ctx(ctx) {
        Some(host) => {
            host.i18n().map_or(preset.name, |i18n| i18n.tid(host.lang(), &format!("newDocument.preset.{}", preset.id), preset.name)).to_owned()
        }
        None => preset.name.to_owned(),
    }
}

fn mode_label(labels: &NewDocumentLabels, mode: ColorMode) -> &str {
    match mode {
        ColorMode::Grayscale => &labels.grayscale,
        ColorMode::Rgb => &labels.rgb_color,
        ColorMode::Cmyk => &labels.cmyk_color,
        ColorMode::Lab => &labels.lab_color,
    }
}

fn depth_label(labels: &NewDocumentLabels, depth: BitDepth) -> &str {
    match depth {
        BitDepth::Eight => &labels.depth_8,
        BitDepth::Sixteen => &labels.depth_16,
        BitDepth::ThirtyTwo => &labels.depth_32,
    }
}

fn background_label(labels: &NewDocumentLabels, background: BackgroundContents) -> &str {
    match background {
        BackgroundContents::White => &labels.white,
        BackgroundContents::Black => &labels.black,
        BackgroundContents::BackgroundColor => &labels.background_color,
        BackgroundContents::Transparent => &labels.transparent,
        BackgroundContents::Custom(_) => &labels.custom_color,
    }
}

/// The size line of a card or a saved preset: "1800 × 1200 px @ 300 ppi", or inches for a print
/// preset ("6 × 4 in @ 300 ppi"), numbers in the locale's format with at most two decimals.
// pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:412 size_line
pub fn size_line(settings: &DocumentOptions, labels: &NewDocumentLabels, numbers: &LocaleNumbers) -> String {
    let numbers = numbers.clone().ungrouped();
    let size = settings.size;
    let unit = if settings.unit == SizeUnit::Percent { SizeUnit::Pixels } else { settings.unit };
    let ppi = size.pixels_per_inch;
    let value = |pixels: u32| {
        let pixels = f64::from(pixels);
        numbers.format(unit.from_pixels(pixels, pixels, ppi, POINTS_PER_INCH), 0..=2)
    };
    craft_i18n::fmt(
        &labels.size_line,
        &[("width", &value(size.width)), ("height", &value(size.height)), ("unit", unit.suffix()), ("ppi", &numbers.format(ppi, 0..=2))],
    )
}

/// The recent list after creating `settings`: equal settings move to the front instead of
/// repeating, and the list keeps at most `max` entries (20 in the dialog).
// pc: crates/engine/src/cmd/document_preset_cmds.rs:244 add_recent (MAX_RECENT :19)
pub fn push_recent(recent: &mut Vec<DocumentOptions>, settings: DocumentOptions, max: usize) {
    recent.retain(|r| !same_document(r, &settings));
    recent.insert(0, settings);
    recent.truncate(max);
}
