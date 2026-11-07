//! The data of New Document: the document settings, the built-in presets and categories, the lists the dialog
//! remembers and the rules on them.

use crate::patterns::form_dialogs::{LinkedSize, MAX_DOCUMENT_SIDE};
use crate::runtime::env::{Label, UnitKind, Units};
use craft_behavior::PixelSize;
use serde::{Deserialize, Serialize};

use super::NewDocumentWords;

/// The most documents Recent keeps.
pub(super) const MAX_RECENT: usize = 20;
/// The most presets a user can save.
pub(super) const MAX_SAVED: usize = 256;
/// The most characters of a document or preset name.
pub(super) const MAX_NAME: usize = 255;
/// The id of the Recent card that stands for the application's own default size.
pub(super) const DEFAULT_ID: &str = "default";
/// The resolution a document without one opens at, in pixels per inch.
const DEFAULT_RESOLUTION: f64 = 300.0;
/// The size a document without one opens at: PhotoCraft's default, 7 x 5 in at 300 ppi.
const DEFAULT_SIZE: PixelSize = PixelSize::new(2100, 1500);
/// The least and greatest resolution, in pixels per inch.
const RESOLUTION_LIMIT: f64 = 30_000.0;

/// The units the Width and Height lists offer (Photoshop's New Document has no percent).
pub(super) const SIZE_UNITS: [UnitKind; 6] =
    [UnitKind::Pixels, UnitKind::Inches, UnitKind::Centimetres, UnitKind::Millimetres, UnitKind::Points, UnitKind::Picas];
/// The units the Resolution list offers.
pub(super) const RESOLUTION_UNITS: [UnitKind; 2] = [UnitKind::PixelsPerInch, UnitKind::PixelsPerCentimetre];

/// The document's colour mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ColorMode {
    /// One grey channel.
    Grayscale,
    /// Red, green and blue.
    #[default]
    Rgb,
    /// Cyan, magenta, yellow and black.
    Cmyk,
    /// Lightness and the two colour-opponent channels.
    Lab,
}

/// Bits per channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BitDepth {
    /// 8 bits per channel.
    #[default]
    Eight,
    /// 16 bits per channel.
    Sixteen,
    /// 32-bit floating point.
    ThirtyTwo,
}

/// What fills the new document's background layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BackgroundContents {
    /// White.
    #[default]
    White,
    /// Black.
    Black,
    /// The application's current background colour (`NewDocumentDialog::background_color`).
    BackgroundColor,
    /// No fill.
    Transparent,
    /// A colour the user picked with Custom, sRGB, 0 to 1 per channel.
    Custom([f32; 3]),
}

/// Which way the page stands. The orientation control swaps width and height, and the built-in cards turn with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Orientation {
    /// Taller than wide, or square.
    #[default]
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

/// Everything New Document creates a document from: the dialog's result, a saved preset, a recent card and the
/// remembered last values are all this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "Stored", into = "Stored")]
#[non_exhaustive]
pub struct DocumentOptions {
    /// The document name ("Untitled-1"); a preset's own name when saved as one.
    pub name: String,
    /// Width and height in pixels.
    pub size: PixelSize,
    /// The resolution in pixels per inch.
    pub pixels_per_inch: f64,
    /// The unit the size fields show; the size itself is always in pixels.
    pub unit: UnitKind,
    /// The unit the resolution field shows; the resolution itself is stored in pixels per inch.
    pub resolution_unit: UnitKind,
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
    /// 2100 x 1500 px at 300 ppi (7 x 5 in), RGB, 8 bit, white.
    fn default() -> Self {
        Self {
            name: String::new(),
            size: DEFAULT_SIZE,
            pixels_per_inch: DEFAULT_RESOLUTION,
            unit: UnitKind::Pixels,
            resolution_unit: UnitKind::PixelsPerInch,
            color_mode: ColorMode::Rgb,
            depth: BitDepth::Eight,
            profile: None,
            background: BackgroundContents::White,
            artboard: false,
        }
    }
}

impl DocumentOptions {
    /// Whether two settings make the same document (the name and the display units apart).
    pub fn same_document(&self, other: &DocumentOptions) -> bool {
        self.size == other.size
            && self.pixels_per_inch == other.pixels_per_inch
            && self.color_mode == other.color_mode
            && self.depth == other.depth
            && self.background == other.background
            && self.profile == other.profile
    }

    /// Whether the settings fit what the dialog can show: a damaged remembered value does not.
    pub fn fits(&self) -> bool {
        let side = |n: u32| (1..=MAX_DOCUMENT_SIDE as u32).contains(&n);
        side(self.size.width)
            && side(self.size.height)
            && self.pixels_per_inch.is_finite()
            && self.pixels_per_inch > 0.0
            && self.pixels_per_inch <= RESOLUTION_LIMIT
            && SIZE_UNITS.contains(&self.unit)
            && RESOLUTION_UNITS.contains(&self.resolution_unit)
    }

    /// The fields that show these settings: the size, resolution and units.
    pub(super) fn linked(&self) -> LinkedSize {
        let mut linked = LinkedSize::new(self.size).opening_resolution(self.pixels_per_inch).chain(false);
        linked.set_unit(self.unit);
        linked.set_resolution_unit(self.resolution_unit);
        linked
    }

    /// Takes the size, resolution and units from `linked`.
    pub(super) fn take_linked(&mut self, linked: &LinkedSize) {
        self.size = linked.pixels();
        self.pixels_per_inch = linked.pixels_per_inch();
        self.unit = linked.unit();
        self.resolution_unit = linked.resolution_unit();
    }
}

/// The saved form of `DocumentOptions`: the units are named by a stable key, so a catalog or enum change elsewhere
/// does not change what was saved.
#[derive(Clone, Serialize, Deserialize)]
struct Stored {
    name: String,
    width: u32,
    height: u32,
    pixels_per_inch: f64,
    unit: StoredUnit,
    resolution_unit: StoredResolution,
    color_mode: ColorMode,
    depth: BitDepth,
    profile: Option<String>,
    background: BackgroundContents,
    artboard: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum StoredUnit {
    Pixels,
    Inches,
    Centimetres,
    Millimetres,
    Points,
    Picas,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum StoredResolution {
    PixelsPerInch,
    PixelsPerCentimetre,
}

impl From<DocumentOptions> for Stored {
    fn from(options: DocumentOptions) -> Self {
        let unit = match options.unit {
            UnitKind::Inches => StoredUnit::Inches,
            UnitKind::Centimetres => StoredUnit::Centimetres,
            UnitKind::Millimetres => StoredUnit::Millimetres,
            UnitKind::Points => StoredUnit::Points,
            UnitKind::Picas => StoredUnit::Picas,
            _ => StoredUnit::Pixels,
        };
        let resolution_unit = match options.resolution_unit {
            UnitKind::PixelsPerCentimetre => StoredResolution::PixelsPerCentimetre,
            _ => StoredResolution::PixelsPerInch,
        };
        Stored {
            name: options.name,
            width: options.size.width,
            height: options.size.height,
            pixels_per_inch: options.pixels_per_inch,
            unit,
            resolution_unit,
            color_mode: options.color_mode,
            depth: options.depth,
            profile: options.profile,
            background: options.background,
            artboard: options.artboard,
        }
    }
}

impl From<Stored> for DocumentOptions {
    fn from(stored: Stored) -> Self {
        let unit = match stored.unit {
            StoredUnit::Pixels => UnitKind::Pixels,
            StoredUnit::Inches => UnitKind::Inches,
            StoredUnit::Centimetres => UnitKind::Centimetres,
            StoredUnit::Millimetres => UnitKind::Millimetres,
            StoredUnit::Points => UnitKind::Points,
            StoredUnit::Picas => UnitKind::Picas,
        };
        let resolution_unit = match stored.resolution_unit {
            StoredResolution::PixelsPerInch => UnitKind::PixelsPerInch,
            StoredResolution::PixelsPerCentimetre => UnitKind::PixelsPerCentimetre,
        };
        DocumentOptions {
            name: stored.name,
            size: PixelSize::new(stored.width, stored.height),
            pixels_per_inch: stored.pixels_per_inch,
            unit,
            resolution_unit,
            color_mode: stored.color_mode,
            depth: stored.depth,
            profile: stored.profile,
            background: stored.background,
            artboard: stored.artboard,
        }
    }
}

/// One built-in card: a name and a page size.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DocumentPreset {
    /// Stable id. The name shows through the word set under it for the built-in presets.
    pub id: &'static str,
    /// The English name, shown when no word set names the id.
    pub name: &'static str,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Resolution in pixels per inch.
    pub pixels_per_inch: f64,
}

impl DocumentPreset {
    /// The card `id`, named `name` in English, for a page of `width` x `height` pixels at `pixels_per_inch`.
    pub const fn new(id: &'static str, name: &'static str, width: u32, height: u32, pixels_per_inch: f64) -> Self {
        Self { id, name, width, height, pixels_per_inch }
    }

    /// Whether the preset is for print (300 ppi and up): its card and fields show inches at the app's print resolution;
    /// screen presets stay in pixels.
    pub fn is_print(&self) -> bool {
        self.pixels_per_inch >= 300.0
    }

    /// The preset turned to `orientation`; a square preset stays as it is.
    #[must_use]
    pub fn turned(&self, orientation: Orientation) -> DocumentPreset {
        let portrait = orientation == Orientation::Portrait;
        if self.width == self.height || (self.height > self.width) == portrait {
            return self.clone();
        }
        DocumentPreset { width: self.height, height: self.width, ..self.clone() }
    }

    /// The preset as settings: the unit (inches for print, pixels for screen) and the resolution moved to the app's
    /// print or screen resolution, keeping a print preset's physical size.
    pub fn options(&self, print_ppi: f64, screen_ppi: f64) -> DocumentOptions {
        let print = self.is_print();
        let wanted = if print { print_ppi } else { screen_ppi };
        let (width, height, pixels_per_inch) = if !(wanted.is_finite() && wanted > 0.0) {
            (self.width, self.height, self.pixels_per_inch)
        } else if print {
            let scale = wanted / self.pixels_per_inch;
            let pixels = |side: u32| (f64::from(side) * scale).round().clamp(1.0, MAX_DOCUMENT_SIDE) as u32;
            (pixels(self.width), pixels(self.height), wanted)
        } else {
            (self.width, self.height, wanted)
        };
        DocumentOptions {
            size: PixelSize::new(width, height),
            pixels_per_inch,
            unit: if print { UnitKind::Inches } else { UnitKind::Pixels },
            ..DocumentOptions::default()
        }
    }
}

/// A category tab and its cards.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PresetCategory {
    /// Stable id: `"photo"`.
    pub id: &'static str,
    /// The English title, shown when no word names the id.
    pub title: &'static str,
    /// The category's cards in display order.
    pub presets: Vec<DocumentPreset>,
}

impl PresetCategory {
    /// The category `id` titled `title` in English, with its `presets`.
    pub fn new(id: &'static str, title: &'static str, presets: Vec<DocumentPreset>) -> Self {
        Self { id, title, presets }
    }

    /// PhotoCraft's categories in tab order: Photo, Print, Art & Illustration, Web, Mobile, Film & Video, with their
    /// presets. Recent and Saved are not in this list: the dialog adds them first and last.
    pub fn standard() -> Vec<PresetCategory> {
        vec![
            PresetCategory::new(
                "photo",
                "Photo",
                vec![
                    DocumentPreset::new("landscape-6x4", "Landscape, 6 x 4", 1800, 1200, 300.0),
                    DocumentPreset::new("landscape-7x5", "Landscape, 7 x 5", 2100, 1500, 300.0),
                    DocumentPreset::new("landscape-10x8", "Landscape, 10 x 8", 3000, 2400, 300.0),
                    DocumentPreset::new("portrait-4x6", "Portrait, 4 x 6", 1200, 1800, 300.0),
                    DocumentPreset::new("portrait-5x7", "Portrait, 5 x 7", 1500, 2100, 300.0),
                    DocumentPreset::new("square-5x5", "Square, 5 x 5", 1500, 1500, 300.0),
                ],
            ),
            PresetCategory::new(
                "print",
                "Print",
                vec![
                    DocumentPreset::new("letter", "Letter", 2550, 3300, 300.0),
                    DocumentPreset::new("legal", "Legal", 2550, 4200, 300.0),
                    DocumentPreset::new("tabloid", "Tabloid", 3300, 5100, 300.0),
                    DocumentPreset::new("a4", "A4", 2480, 3508, 300.0),
                    DocumentPreset::new("a3", "A3", 3508, 4961, 300.0),
                    DocumentPreset::new("a5", "A5", 1748, 2480, 300.0),
                    DocumentPreset::new("b3", "B3", 4169, 5906, 300.0),
                    DocumentPreset::new("b4", "B4", 2953, 4169, 300.0),
                ],
            ),
            PresetCategory::new(
                "art",
                "Art & Illustration",
                vec![
                    DocumentPreset::new("poster", "Poster", 5400, 7200, 300.0),
                    DocumentPreset::new("postcard", "Postcard", 1800, 1200, 300.0),
                    DocumentPreset::new("comic-book", "Comic Book", 1988, 3075, 300.0),
                    DocumentPreset::new("square-12x12", "Square, 12 x 12", 3600, 3600, 300.0),
                ],
            ),
            PresetCategory::new(
                "web",
                "Web",
                vec![
                    DocumentPreset::new("web-most-common", "Web Most Common", 1366, 768, 72.0),
                    DocumentPreset::new("web-minimum", "Web Minimum", 1024, 768, 72.0),
                    DocumentPreset::new("web-large", "Web Large", 1920, 1080, 72.0),
                    DocumentPreset::new("macbook-pro-16", "MacBook Pro 16\"", 3456, 2234, 72.0),
                    DocumentPreset::new("imac-24", "iMac 24\"", 4480, 2520, 72.0),
                ],
            ),
            PresetCategory::new(
                "mobile",
                "Mobile",
                vec![
                    DocumentPreset::new("iphone-16", "iPhone 16", 1179, 2556, 72.0),
                    DocumentPreset::new("iphone-16-pro-max", "iPhone 16 Pro Max", 1320, 2868, 72.0),
                    DocumentPreset::new("ipad-pro-13", "iPad Pro 13\"", 2064, 2752, 72.0),
                    DocumentPreset::new("android-1080p", "Android 1080p", 1080, 1920, 72.0),
                    DocumentPreset::new("apple-watch-45", "Apple Watch 45mm", 396, 484, 72.0),
                ],
            ),
            PresetCategory::new(
                "film",
                "Film & Video",
                vec![
                    DocumentPreset::new("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0),
                    DocumentPreset::new("hdtv-720p", "HDTV 720p", 1280, 720, 72.0),
                    DocumentPreset::new("uhd-4k", "UHD 4K", 3840, 2160, 72.0),
                    DocumentPreset::new("dci-4k", "DCI 4K", 4096, 2160, 72.0),
                    DocumentPreset::new("uhd-8k", "UHD 8K", 7680, 4320, 72.0),
                ],
            ),
        ]
    }

    /// The cards Recent shows after the recent documents when the app gives none: "Default {app} Size" and HDTV 1080p.
    pub fn default_recent() -> Vec<DocumentPreset> {
        vec![DocumentPreset::new(DEFAULT_ID, "Default Size", 2100, 1500, 300.0), DocumentPreset::new("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0)]
    }
}

/// The colour profiles one colour mode offers.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorProfiles {
    /// The mode the profiles are for.
    pub mode: ColorMode,
    /// The working profile's name, listed first as "Working: name".
    pub working: String,
    /// The other profiles.
    pub others: Vec<String>,
}

impl ColorProfiles {
    /// The profiles of `mode`: the `working` one and the `others`.
    pub fn new(mode: ColorMode, working: impl Into<String>, others: Vec<String>) -> Self {
        Self { mode, working: working.into(), others }
    }
}

/// The recent list after creating `settings`: equal settings move to the front instead of repeating, and the list
/// keeps at most `max` entries (20 in the dialog).
pub fn push_recent(recent: &mut Vec<DocumentOptions>, settings: DocumentOptions, max: usize) {
    recent.retain(|held| !held.same_document(&settings));
    recent.insert(0, settings);
    recent.truncate(max);
}

/// Why a preset name is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameError {
    /// The name has no characters after trimming, or more than 255.
    Length,
    /// A preset of this name (ignoring ASCII case) is saved already.
    Taken(String),
    /// 256 presets are saved already.
    TooMany,
}

/// Whether `name` can be saved as a preset beside the `saved` ones.
pub fn preset_name_error(saved: &[DocumentOptions], name: &str) -> Option<NameError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Some(NameError::Length);
    }
    if saved.iter().any(|preset| preset.name.eq_ignore_ascii_case(name)) {
        return Some(NameError::Taken(name.to_owned()));
    }
    (saved.len() >= MAX_SAVED).then_some(NameError::TooMany)
}

/// The size line of a card or a saved preset: "1800 x 1200 px @ 300 ppi", or inches for a print preset
/// ("6 x 4 in @ 300 ppi"), numbers in the user's format with at most two decimals.
pub fn size_line(options: &DocumentOptions, words: &NewDocumentWords, units: &Units) -> String {
    let unit = if SIZE_UNITS.contains(&options.unit) { options.unit } else { UnitKind::Pixels };
    let mut linked = options.linked();
    linked.set_unit(unit);
    let (width, height) = (units.number_ungrouped(linked.width(), 0..=2), units.number_ungrouped(linked.height(), 0..=2));
    let resolution = units.show(UnitKind::PixelsPerInch, options.pixels_per_inch);
    words.size_line.fill(&[("width", &width), ("height", &height), ("unit", &units.symbol(unit)), ("resolution", &resolution)])
}

/// The recent documents, remembered across runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Recents(pub(super) Vec<DocumentOptions>);

/// The presets the user saved, remembered across runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SavedPresets(pub(super) Vec<DocumentOptions>);

/// The settings of the last created document, remembered across runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct LastOptions(pub(super) Option<DocumentOptions>);

craft_ui::words! {
    /// The names of the built-in presets, in the order of `PRESET_IDS`.
    pub struct PresetWords {
        names: array [
            "newDocument.presetLandscape6x4" = "Landscape, 6 x 4",
            "newDocument.presetLandscape7x5" = "Landscape, 7 x 5",
            "newDocument.presetLandscape10x8" = "Landscape, 10 x 8",
            "newDocument.presetPortrait4x6" = "Portrait, 4 x 6",
            "newDocument.presetPortrait5x7" = "Portrait, 5 x 7",
            "newDocument.presetSquare5x5" = "Square, 5 x 5",
            "newDocument.presetLetter" = "Letter",
            "newDocument.presetLegal" = "Legal",
            "newDocument.presetTabloid" = "Tabloid",
            "newDocument.presetA4" = "A4",
            "newDocument.presetA3" = "A3",
            "newDocument.presetA5" = "A5",
            "newDocument.presetB3" = "B3",
            "newDocument.presetB4" = "B4",
            "newDocument.presetPoster" = "Poster",
            "newDocument.presetPostcard" = "Postcard",
            "newDocument.presetComicBook" = "Comic Book",
            "newDocument.presetSquare12x12" = "Square, 12 x 12",
            "newDocument.presetWebMostCommon" = "Web Most Common",
            "newDocument.presetWebMinimum" = "Web Minimum",
            "newDocument.presetWebLarge" = "Web Large",
            "newDocument.presetMacbookPro16" = "MacBook Pro 16\"",
            "newDocument.presetImac24" = "iMac 24\"",
            "newDocument.presetIphone16" = "iPhone 16",
            "newDocument.presetIphone16ProMax" = "iPhone 16 Pro Max",
            "newDocument.presetIpadPro13" = "iPad Pro 13\"",
            "newDocument.presetAndroid1080p" = "Android 1080p",
            "newDocument.presetAppleWatch45" = "Apple Watch 45mm",
            "newDocument.presetHdtv1080p" = "HDTV 1080p",
            "newDocument.presetHdtv720p" = "HDTV 720p",
            "newDocument.presetUhd4k" = "UHD 4K",
            "newDocument.presetDci4k" = "DCI 4K",
            "newDocument.presetUhd8k" = "UHD 8K",
        ];
    }
}

/// The ids of the built-in presets, in the order of `PresetWords::names`.
const PRESET_IDS: [&str; 33] = [
    "landscape-6x4",
    "landscape-7x5",
    "landscape-10x8",
    "portrait-4x6",
    "portrait-5x7",
    "square-5x5",
    "letter",
    "legal",
    "tabloid",
    "a4",
    "a3",
    "a5",
    "b3",
    "b4",
    "poster",
    "postcard",
    "comic-book",
    "square-12x12",
    "web-most-common",
    "web-minimum",
    "web-large",
    "macbook-pro-16",
    "imac-24",
    "iphone-16",
    "iphone-16-pro-max",
    "ipad-pro-13",
    "android-1080p",
    "apple-watch-45",
    "hdtv-1080p",
    "hdtv-720p",
    "uhd-4k",
    "dci-4k",
    "uhd-8k",
];

impl PresetWords {
    /// The translated name of the built-in preset `id`; `None` for an id the set does not name.
    pub fn name_of(&self, id: &str) -> Option<&Label> {
        PRESET_IDS.iter().position(|listed| *listed == id).and_then(|at| self.names.get(at))
    }
}
