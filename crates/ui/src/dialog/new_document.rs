#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`NewDocumentDialog`]: File › New. Category tabs over a grid of preset cards, a details
//! column, saved presets, and the document settings the app creates the document from.

use egui::Context;
use serde::{Deserialize, Serialize};

use super::dialog_host::{DialogResult, StackDialog};
use super::sizing::{PixelSize, ResolutionUnit, SizeUnit};
use super::stack::{DialogId, DialogStack, dialog_id};

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
        /// `{width}`, `{height}`, `{unit}` and `{ppi}`: a card's size line.
        size_line: "newDocument.sizeLine" = "{width} × {height} {unit} @ {ppi} ppi";
    }
}

/// The document's colour mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorMode {
    Grayscale,
    #[default]
    Rgb,
    Cmyk,
    Lab,
}

/// Bits per channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitDepth {
    #[default]
    Eight,
    Sixteen,
    /// 32-bit floating point.
    ThirtyTwo,
}

/// What fills the new document's background layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BackgroundContents {
    #[default]
    White,
    Black,
    /// The app's current background colour ([`NewDocumentDialog::background_color`]).
    BackgroundColor,
    Transparent,
    /// A colour the user picked with Custom…, sRGB, 0–1 per channel.
    Custom([f32; 3]),
}

/// Which way the page stands. The two buttons beside Height swap width and height and turn the
/// built-in cards with them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    Portrait,
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
pub struct DocumentSettings {
    /// The document name ("Untitled-1"); a preset's own name when saved as one.
    pub name: String,
    /// Width and height in pixels, and the resolution in pixels per inch.
    pub size: PixelSize,
    /// The unit the size fields show; the size itself is always in pixels.
    pub unit: SizeUnit,
    pub resolution_unit: ResolutionUnit,
    pub color_mode: ColorMode,
    pub depth: BitDepth,
    /// The colour profile's name; `None` is the working profile of the mode.
    pub profile: Option<String>,
    pub background: BackgroundContents,
    /// Artboards: the document starts with one artboard of this size.
    pub artboard: bool,
}

impl Default for DocumentSettings {
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
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
    pub pixels_per_inch: f64,
}

impl DocumentPreset {
    /// Whether the preset is for print (300 ppi and up): its card and fields show inches at the
    /// app's print resolution; screen presets stay in pixels.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:172 is_print
    pub fn is_print(&self) -> bool {
        todo!()
    }

    /// The preset as settings: the unit (inches for print, pixels for screen, PhotoCraft
    /// `preset_unit`) and the resolution moved to the app's print or screen resolution, keeping
    /// a print preset's physical size.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:208 at_pref_resolutions, :192 preset_unit
    pub fn settings(&self, print_ppi: f64, screen_ppi: f64) -> DocumentSettings {
        todo!()
    }
}

/// A category tab and its cards.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetCategory {
    /// Stable id: `"photo"`.
    pub id: &'static str,
    /// English title, translated through [`NewDocumentLabels`] for the built-in ids.
    pub title: &'static str,
    pub presets: Vec<DocumentPreset>,
}

impl PresetCategory {
    /// PhotoCraft's categories in tab order: Photo, Print, Art & Illustration, Web, Mobile, Film &
    /// Video, with their presets (Photoshop's tabs; Recent and Saved are not in this list, the
    /// dialog adds them first and last). Recent's built-in defaults (Default PhotoCraft Size,
    /// HDTV 1080p) are [`NewDocumentDialog::recent_defaults`].
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:19 CATEGORIES
    pub fn standard() -> Vec<PresetCategory> {
        todo!()
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
/// Custom… under Background Contents asks the app for a colour: the dialog sets
/// [`wants_color`](NewDocumentDialog::wants_color); the app opens its colour picker as a child
/// dialog and hands the answer back with [`set_custom_color`](NewDocumentDialog::set_custom_color)
/// through `dialogs.get_mut::<NewDocumentDialog>()`.
///
/// The dialog opens with the last created settings (`"newDocument.last"`, PhotoCraft's
/// `file.new` memory) and stores them, and a recent entry, on Create. Result:
/// [`DocumentSettings`].
///
/// ```ignore
/// dialogs.open(NewDocumentDialog::new().clipboard(clip_size).background_color(bg));
/// if let Some(DialogResult::Ok(settings)) = dialogs.take::<NewDocumentDialog>() { app.new_document(settings); }
/// ```
pub struct NewDocumentDialog {
    id: DialogId,
    categories: Vec<PresetCategory>,
    clipboard: Option<PixelSize>,
    background_color: [f32; 3],
    print_ppi: f64,
    screen_ppi: f64,
    profiles: Vec<(ColorMode, String)>,
    working: Vec<(ColorMode, String)>,
    settings: DocumentSettings,
    /// The open tab's id.
    tab: String,
    selected_card: Option<usize>,
    /// The Save Preset name row, when open.
    preset_name: Option<String>,
    wants_color: bool,
    /// Set once the remembered values were read (first pass, which has the Host).
    recalled: bool,
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
            categories: Vec::new(),
            clipboard: None,
            background_color: [1.0, 1.0, 1.0],
            print_ppi: 300.0,
            screen_ppi: 72.0,
            profiles: Vec::new(),
            working: Vec::new(),
            settings: DocumentSettings::default(),
            tab: "recent".to_owned(),
            selected_card: None,
            preset_name: None,
            wants_color: false,
            recalled: false,
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

    /// Recent's built-in cards after the recent documents: Default PhotoCraft Size (2100 × 1500 px
    /// at 300 ppi) and HDTV 1080p.
    // pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:20 CATEGORIES "Recent"
    pub fn recent_defaults() -> Vec<DocumentPreset> {
        todo!()
    }

    /// The settings shown in the details column.
    pub fn settings(&self) -> &DocumentSettings {
        &self.settings
    }

    /// Custom… was chosen under Background Contents and the dialog waits for a colour. Reading
    /// it does not clear it; [`set_custom_color`](Self::set_custom_color) or a cancelled pick
    /// ([`cancel_color`](Self::cancel_color)) does.
    pub fn wants_color(&self) -> bool {
        self.wants_color
    }

    /// The colour the app's picker returned for Custom…: Background Contents becomes
    /// [`BackgroundContents::Custom`].
    pub fn set_custom_color(&mut self, rgb: [f32; 3]) {
        todo!()
    }

    /// The picker was cancelled: Background Contents goes back to what it was before Custom….
    pub fn cancel_color(&mut self) {
        todo!()
    }
}

impl StackDialog for NewDocumentDialog {
    type Result = DocumentSettings;

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
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DocumentSettings>> {
        todo!()
    }
}

/// The size line of a card or a saved preset: "1800 × 1200 px @ 300 ppi", or inches for a print
/// preset ("6 × 4 in @ 300 ppi"), numbers in the locale's format with at most two decimals.
// pc: crates/ui-egui/src/dialogs/new_doc_ui.rs:412 size_line
pub fn size_line(settings: &DocumentSettings, labels: &NewDocumentLabels) -> String {
    todo!()
}

/// The recent list after creating `settings`: equal settings move to the front instead of
/// repeating, and the list keeps at most `max` entries (20 in the dialog).
// pc: crates/engine/src/cmd/document_preset_cmds.rs:244 add_recent (MAX_RECENT :19)
#[expect(clippy::ptr_arg, reason = "skeleton: the body inserts and truncates")]
pub fn push_recent(recent: &mut Vec<DocumentSettings>, settings: DocumentSettings, max: usize) {
    todo!()
}
