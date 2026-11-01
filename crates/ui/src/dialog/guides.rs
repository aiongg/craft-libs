#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! View › Guides › New Guide…, Edit Selected Guides and New Guide Layout… (F17.01), as in
//! Photoshop: [`NewGuideDialog`] and [`GuideLayoutDialog`] on the [`DialogHost`](super::DialogHost).
//!
//! Positions and sizes are [`Length`]s: the fields open in the ruler unit ("0 cm") and take any
//! unit Photoshop's fields take ("50%", "2.5 cm", "1in", "12 pt", "3 pc", "100 px"); a bare number
//! is in the ruler unit. Percent is of the target's width for vertical guides, column widths and
//! the left and right margins, and of its height for the rest.
//!
//! [`GuideLayoutOptions::lines`] is the layout calculation (PhotoCraft's engine `guide_layout`,
//! pure), so the dialog's live preview and the app's command draw the same guides: the app reads
//! [`GuideLayoutDialog::preview`] each pass and draws it with `canvas::Guides` until OK or Cancel.

use std::borrow::Cow;

use craft_theme::Srgba;
use egui::{Color32, Context};

use super::ColorRequest;
use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};
use crate::canvas::{Guide, Orientation};
use crate::localized::units::{Length, LengthUnit, UnitContext};

crate::words! {
    /// The guide dialogs' words, English by default (Photoshop's). OK, Cancel and Preview are
    /// [`DialogLabels`](super::DialogLabels)'.
    #[non_exhaustive]
    pub struct GuidesLabels {
        new_guide_title: "guides.newGuide" = "New Guide";
        edit_title: "guides.editSelected" = "Edit Selected Guides";
        orientation: "guides.orientation" = "Orientation";
        horizontal: "guides.horizontal" = "Horizontal";
        vertical: "guides.vertical" = "Vertical";
        position: "guides.position" = "Position";
        color: "guides.color" = "Color";
        custom_color: "guides.customColor" = "Custom…";
        light_blue: "guides.lightBlue" = "Light Blue";
        light_red: "guides.lightRed" = "Light Red";
        green: "guides.green" = "Green";
        medium_blue: "guides.mediumBlue" = "Medium Blue";
        yellow: "guides.yellow" = "Yellow";
        magenta: "guides.magenta" = "Magenta";
        cyan: "guides.cyan" = "Cyan";
        light_gray: "guides.lightGray" = "Light Gray";
        black: "guides.black" = "Black";
        /// New Guide's colour picker title ([`ColorRequest::title`]).
        picker_title: "guides.pickerTitle" = "Color Picker (Guides)";
        /// New Guide Layout's colour swatch name and its picker title.
        layout_color: "guides.layoutColor" = "Guide color";
        /// The position field's error for text that isn't a length.
        bad_position: "guides.badPosition" = "Enter a position such as 2 cm, 50% or 100 px.";
        layout_title: "guides.layout" = "New Guide Layout";
        preset: "guides.preset" = "Preset:";
        preset_custom: "guides.presetCustom" = "Custom";
        preset_five: "guides.presetFive" = "5 Columns, 3 Rows";
        preset_eight: "guides.presetEight" = "8 Columns";
        preset_twelve: "guides.presetTwelve" = "12 Columns";
        target: "guides.target" = "Target:";
        document: "guides.document" = "Document";
        selected_artboards: "guides.selectedArtboards" = "Selected Artboards";
        columns: "guides.columns" = "Columns";
        rows: "guides.rows" = "Rows";
        number: "guides.number" = "Number";
        width: "guides.width" = "Width";
        height: "guides.height" = "Height";
        gutter: "guides.gutter" = "Gutter";
        /// The Width and Height fields' tooltip.
        automatic_size: "guides.automaticSize" = "Leave empty for automatic size";
        margin: "guides.margin" = "Margin";
        top: "guides.top" = "Top:";
        left: "guides.left" = "Left:";
        bottom: "guides.bottom" = "Bottom:";
        right: "guides.right" = "Right:";
        center_columns: "guides.centerColumns" = "Center Columns";
        clear_existing: "guides.clearExisting" = "Clear Existing Guides";
        /// Under the fields while OK is disabled; the reason is its tooltip.
        invalid: "guides.invalid" = "Invalid guide layout";
        does_not_fit: "guides.doesNotFit" = "The columns or rows, their size and gutter must fit inside the margins.";
        margins_too_large: "guides.marginsTooLarge" = "The margins must leave space inside the target.";
        nothing: "guides.nothing" = "Turn on Columns, Rows or Margin.";
        too_many: "guides.tooMany" = "The layout would make too many guides.";
        no_artboards: "guides.noArtboards" = "Select an artboard or a layer inside one.";
    }
}

/// The [`ColorRequest::key`] of New Guide's Custom… colour.
pub const NEW_GUIDE_COLOR_KEY: &str = "guides.newGuide.color";
/// The [`ColorRequest::key`] of New Guide Layout's colour swatch.
pub const GUIDE_LAYOUT_COLOR_KEY: &str = "guides.layout.color";

/// A guide colour: Photoshop's list (PhotoCraft's `GUIDE_COLORS`, Cyan by default) or Custom….
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GuideColor {
    LightBlue,
    LightRed,
    Green,
    MediumBlue,
    Yellow,
    Magenta,
    #[default]
    Cyan,
    LightGray,
    Black,
    /// Chosen with Custom…'s swatch (the app's colour picker). Its `#rrggbb` is written with
    /// craft-theme's `Rgba::to_hex`, as every hex text in the suite.
    Custom([u8; 3]),
}

impl GuideColor {
    /// The named colours in Photoshop's menu order; Custom… is listed first in the menu, before
    /// these (PhotoCraft).
    pub const NAMED: [GuideColor; 9] = [
        GuideColor::LightBlue,
        GuideColor::LightRed,
        GuideColor::Green,
        GuideColor::MediumBlue,
        GuideColor::Yellow,
        GuideColor::Magenta,
        GuideColor::Cyan,
        GuideColor::LightGray,
        GuideColor::Black,
    ];

    /// sRGB: Light Blue `#3366ff`, Light Red `#ff3333`, Green `#33ff33`, Medium Blue `#3333ff`,
    /// Yellow `#ffcc00`, Magenta `#ff00ff`, Cyan `#4affff`, Light Gray `#e7e7e7`, Black `#000000`.
    // pc: crates/engine/src/cmd/edit_cmds.rs:856 GUIDE_COLORS
    pub fn rgb(self) -> [u8; 3] {
        todo!()
    }

    /// The named colour with these values, else [`GuideColor::Custom`] (an edited guide's own
    /// colour shows as its name when it is one).
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:114 color_choice
    pub fn from_rgb(rgb: [u8; 3]) -> GuideColor {
        todo!()
    }

    pub fn color32(self) -> Color32 {
        let [r, g, b] = self.rgb();
        Color32::from_rgb(r, g, b)
    }

    pub fn label(self, labels: &GuidesLabels) -> &str {
        todo!()
    }
}

/// The document a guide dialog measures in: its size in pixels (for percent), resolution and
/// ruler unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideUnits {
    pub ruler: LengthUnit,
    pub width_px: f64,
    pub height_px: f64,
    pub pixels_per_inch: f64,
    /// 72 (PostScript) or 72.27, Preferences › Units & Rulers › Point/Pica Size.
    pub points_per_inch: f64,
}

impl GuideUnits {
    /// The conversion for lengths across `extent` pixels (a target's width or height).
    pub fn context(&self, extent: f64) -> UnitContext {
        UnitContext { pixels_per_inch: self.pixels_per_inch, points_per_inch: self.points_per_inch, extent: Some(extent) }
    }
}

/// New Guide's values: what it opens with ([`NewGuideDialog::initial`]) and what it and Edit
/// Selected Guides answer.
#[derive(Clone, Debug, PartialEq)]
pub struct NewGuideOptions {
    pub orientation: Orientation,
    /// As typed; a bare number got the ruler unit.
    pub position: Length,
    /// [`position`](Self::position) in document pixels.
    pub position_px: f64,
    pub color: GuideColor,
    /// Edit Selected Guides: the colour was changed in the dialog. A guide drawn in the
    /// preference colour keeps using it unless this is true (PhotoCraft sends the colour only
    /// then).
    pub color_changed: bool,
    /// Edit Selected Guides: the key of the edited guide ([`Guide::key`]).
    pub edited: Option<u64>,
}

/// What New Guide remembers between uses: the last successful guide.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct NewGuideMemory {
    vertical: bool,
    position_px: f64,
    color: GuideColor,
}

/// New Guide (and Edit Selected Guides): an orientation, a position with a unit and a colour.
///
/// The user sees Orientation (Horizontal / Vertical radio buttons, one Tab stop, arrows pick),
/// Position (a text field, focused with its text selected on open), Color (a list: Custom… then
/// Photoshop's nine colours; with Custom… a swatch and its `#rrggbb` beside it), and OK and Cancel.
/// Enter is OK, Esc is Cancel. A position that isn't a length keeps OK disabled with
/// [`GuidesLabels::bad_position`] under the fields.
///
/// It opens with the last successful guide's orientation, colour and position (shown in the
/// current ruler unit), else horizontal at 0 in Cyan, as Photoshop does; the app records a
/// success with [`NewGuideDialog::remember`] after its command accepted the guide. A position the
/// app refuses reopens the dialog with what was typed and the app's message:
/// `dialogs.open(NewGuideDialog::new(units).initial(answer).error(message))`.
///
/// The Position field is a [`unit_number_field`](crate::patterns::unit_number_field) in the ruler
/// unit over the document's width (vertical) or height (horizontal), so "50%" and "2 cm" work and
/// the field keeps the library's number-field keys.
///
/// Custom…'s swatch asks the app for a colour, as New Document does: in the pass Custom… is
/// chosen (or the swatch clicked) [`pick_color`](Self::pick_color) returns a [`ColorRequest`]
/// keyed [`NEW_GUIDE_COLOR_KEY`], titled [`GuidesLabels::picker_title`]; the app opens its
/// colour picker as a child dialog and answers with [`set_color`](Self::set_color) or
/// [`cancel_color`](Self::cancel_color).
pub struct NewGuideDialog {
    id: DialogId,
    units: GuideUnits,
    orientation: Orientation,
    position: String,
    color: GuideColor,
    initial_color: GuideColor,
    edited: Option<u64>,
    error: Option<String>,
    /// The request to hand out once.
    color_request: Option<ColorRequest>,
    /// Set once the remembered guide was read (first pass, which has the Host).
    recalled: bool,
}

impl NewGuideDialog {
    /// New Guide, with the remembered guide (read on the first pass).
    pub fn new(units: GuideUnits) -> Self {
        Self {
            id: dialog_id("guides.newGuide"),
            units,
            orientation: Orientation::Horizontal,
            position: String::new(),
            color: GuideColor::Cyan,
            initial_color: GuideColor::Cyan,
            edited: None,
            error: None,
            color_request: None,
            recalled: false,
        }
    }

    /// Edit Selected Guides on `guide`: its orientation, its position in the ruler unit, and its
    /// own colour, or `preference_color` it is drawn in. Nothing is remembered.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:124 open_edit
    pub fn edit(guide: &Guide, preference_color: [u8; 3], units: GuideUnits) -> Self {
        todo!()
    }

    /// Opens with `options` instead of the remembered guide: the dialog again after the app
    /// refused an answer, with its values as typed and its edited guide (automation, tests).
    /// The position shows as typed, not converted to the ruler unit.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:235 confirm (reopen with __error)
    pub fn initial(self, options: NewGuideOptions) -> Self {
        todo!()
    }

    /// The app's message under the fields (red, `Content::Error`).
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// Records `answer` as the guide the next New Guide opens with, through the Host's settings
    /// (`Remembered` scope `"guides.newGuide"`). Edit answers are not recorded.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:215 confirm (the remembered value at :226)
    pub fn remember(ctx: &Context, answer: &NewGuideOptions) {
        todo!()
    }

    /// Custom… was chosen (or its swatch clicked) in the pass since the last call: the colour to
    /// ask the app's picker for, keyed [`NEW_GUIDE_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: Color becomes [`GuideColor::Custom`]
    /// (rounded to 8 bits). Other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        todo!()
    }

    /// The picker was cancelled: the colour stays what it was before Custom….
    pub fn cancel_color(&mut self) {
        todo!()
    }
}

impl StackDialog for NewGuideDialog {
    type Result = NewGuideOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:151 body, :194 params
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<NewGuideOptions>> {
        todo!()
    }
}

/// Guide positions in document pixels, sorted, without duplicates (positions closer than 0.01
/// are one guide).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuideLines {
    /// x positions.
    pub vertical: Vec<f64>,
    /// y positions.
    pub horizontal: Vec<f64>,
}

impl GuideLines {
    /// The lines as `canvas::Guide`s for drawing with `canvas::Guides`, keys from `first_key` up,
    /// in `color`.
    pub fn guides(&self, first_key: u64, color: Color32) -> Vec<Guide> {
        todo!()
    }
}

/// A rectangle a layout fills, in document pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// Where New Guide Layout puts its guides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GuideTarget {
    #[default]
    Document,
    /// Each selected artboard (listed only when the app passes some with
    /// [`GuideLayoutDialog::artboards`]).
    SelectedArtboards,
}

/// The Preset list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GuideLayoutPreset {
    #[default]
    Custom,
    /// 5 columns, 3 rows, no gutter.
    FiveColumnsThreeRows,
    /// 8 columns, gutter 20 px.
    EightColumns,
    /// 12 columns, gutter 24 px.
    TwelveColumns,
}

impl GuideLayoutPreset {
    pub const ALL: [GuideLayoutPreset; 4] =
        [GuideLayoutPreset::Custom, GuideLayoutPreset::FiveColumnsThreeRows, GuideLayoutPreset::EightColumns, GuideLayoutPreset::TwelveColumns];

    /// `layout` with this preset's columns, rows and gutters, no margin and Center Columns off;
    /// target, colour and Clear Existing Guides stay. Custom changes nothing.
    // pc: crates/ui-egui/src/dialogs/guide_layout_ui.rs:261 main_fields (the preset values at :278)
    pub fn apply(self, layout: &mut GuideLayoutOptions) {
        todo!()
    }
}

/// Columns or rows of a layout.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuideAxis {
    /// The group's check box.
    pub enabled: bool,
    /// 1 to 1000 (PhotoCraft's `MAX_COUNT`; Photoshop stops at 32).
    pub number: u32,
    /// Width of a column or height of a row; `None` shares the room out evenly.
    pub size: Option<Length>,
    pub gutter: Length,
}

/// New Guide Layout's values: what it opens with ([`GuideLayoutDialog::initial`]), what it
/// answers, and the calculation ([`lines`](Self::lines)).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuideLayoutOptions {
    pub preset: GuideLayoutPreset,
    pub target: GuideTarget,
    pub columns: GuideAxis,
    pub rows: GuideAxis,
    pub margin_enabled: bool,
    /// Top, left, bottom, right.
    pub margin: [Length; 4],
    /// With a fixed column width, centre the columns between the margins instead of packing
    /// them from the left.
    pub center_columns: bool,
    pub clear_existing: bool,
}

impl Default for GuideLayoutOptions {
    /// PhotoCraft's factory layout (Alt+Cancel resets to it): 8 columns with a 20 px gutter,
    /// rows off, no margin.
    // pc: crates/ui-egui/src/menus/view_cmds.rs:123 default_guide_layout
    fn default() -> Self {
        todo!()
    }
}

/// Why a layout can't be made; the dialog shows it under the fields with OK disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuideLayoutError {
    /// Columns or rows with their size and gutters are wider than the room inside the margins.
    DoesNotFit,
    /// The margins leave no room inside a target.
    MarginsTooLarge,
    /// Columns, Rows and Margin are all off (or empty).
    Nothing,
    /// More than 16 000 guide edges over all targets (PhotoCraft's `MAX_EDGES`).
    TooMany,
    /// [`GuideTarget::SelectedArtboards`] with none selected.
    NoArtboards,
}

impl GuideLayoutError {
    pub fn message(self, labels: &GuidesLabels) -> &str {
        todo!()
    }
}

/// The guides a layout makes: `added` are its own lines, `all` the set OK commits (the existing
/// guides, unless Clear Existing Guides, plus `added`).
#[derive(Clone, Debug, PartialEq)]
pub struct GuideLayoutLines {
    pub added: GuideLines,
    pub all: GuideLines,
}

impl GuideLayoutOptions {
    /// The guides for `targets` (the document, or each selected artboard) over `existing`.
    /// Per target: the margin edges when any margin is not zero, then both edges of every
    /// column and row, Photoshop-style. Without a size, columns share the room after the
    /// gutters: `(room - (n - 1) × gutter) / n`; with one, they pack from the left margin, or
    /// centre with Center Columns. Clear Existing Guides with artboards removes only the guides
    /// inside the selected artboards.
    // pc: crates/engine/src/guide_layout.rs:146 calculate, :118 axis
    pub fn lines(&self, targets: &[GuideRect], existing: &GuideLines, units: &GuideUnits) -> Result<GuideLayoutLines, GuideLayoutError> {
        todo!()
    }
}

/// What the canvas shows while New Guide Layout is open with Preview on: the full set OK would
/// commit, in the dialog's colour.
#[derive(Clone, Debug, PartialEq)]
pub struct GuidePreview {
    pub lines: GuideLayoutLines,
    pub color: Color32,
}

/// What New Guide Layout answers.
#[derive(Clone, Debug, PartialEq)]
pub struct GuideLayoutResult {
    pub options: GuideLayoutOptions,
    pub lines: GuideLayoutLines,
    /// The guide colour chosen; PhotoCraft stores it as the guide colour preference.
    pub color: [u8; 3],
}

/// New Guide Layout, Photoshop's dialog: Preset and Target lists, Color, the Columns and Rows
/// groups (each a check box with Number, Width or Height, and Gutter), the Margin group (a check
/// box and Top, Left, Bottom, Right), Center Columns, Clear Existing Guides; OK, Cancel and
/// Preview in a column at the right (`Footer::side()`).
///
/// Every field takes a [`Length`] in any unit and shows the ruler unit: Number is a
/// [`NumberField`](crate::controls::NumberField) (1 to 1000), the lengths are
/// [`unit_number_field`](crate::patterns::unit_number_field)s over the target's width or height.
/// Width and Height may be left empty (tooltip "Leave empty for automatic size"), which needs a
/// number field that takes an empty value (`// needs: controls NumberField empty value`). Typing in a layout field turns the
/// preset to Custom; changing target, colour or Preview does not. A group whose box is off greys
/// its fields and keeps their values; turning Columns or Rows on with Number 0 makes it 1.
///
/// Live preview: while Preview is on (the default) and the layout is valid, the app draws
/// [`preview`](Self::preview) on the canvas each pass, in place of the document's guides; the
/// dialog asks for a repaint when a field changes. An invalid layout keeps OK disabled and shows
/// [`GuidesLabels::invalid`] with the reason as its tooltip; the canvas keeps the last valid
/// preview. Enter is OK when valid; Esc cancels; Alt turns Cancel into Reset, which restores
/// [`GuideLayoutOptions::default`]. The last committed layout opens next time (`Remembered`
/// `"guides.layout"`).
///
/// The colour swatch asks the app for a colour: [`pick_color`](Self::pick_color) returns a
/// [`ColorRequest`] keyed [`GUIDE_LAYOUT_COLOR_KEY`], titled [`GuidesLabels::layout_color`]; the
/// app answers with [`set_color`](Self::set_color) or [`cancel_color`](Self::cancel_color).
pub struct GuideLayoutDialog {
    id: DialogId,
    layout: GuideLayoutOptions,
    units: GuideUnits,
    document: GuideRect,
    artboards: Vec<GuideRect>,
    existing: GuideLines,
    color: [u8; 3],
    preview_on: bool,
    /// The field texts as typed, by field.
    texts: Vec<(Cow<'static, str>, String)>,
    /// The last valid calculation, kept while a field is invalid.
    last_valid: Option<GuideLayoutLines>,
    color_request: Option<ColorRequest>,
    recalled: bool,
}

impl GuideLayoutDialog {
    /// The dialog over the document's bounds and its current guides, in the guide colour
    /// preference `color`.
    pub fn new(document: GuideRect, existing: GuideLines, units: GuideUnits, color: [u8; 3]) -> Self {
        Self {
            id: dialog_id("guides.layout"),
            layout: GuideLayoutOptions::default(),
            units,
            document,
            artboards: Vec::new(),
            existing,
            color,
            preview_on: true,
            texts: Vec::new(),
            last_valid: None,
            color_request: None,
            recalled: false,
        }
    }

    /// The selected artboards; with any, Target lists Selected Artboards.
    pub fn artboards(mut self, artboards: Vec<GuideRect>) -> Self {
        self.artboards = artboards;
        self
    }

    /// Opens with `options` instead of the remembered layout (automation, tests).
    pub fn initial(mut self, options: GuideLayoutOptions) -> Self {
        self.layout = options;
        self.recalled = true;
        self
    }

    /// The guides to draw now: `None` with Preview off, before the first valid layout, or once
    /// the dialog is gone.
    // pc: crates/ui-egui/src/dialogs/guide_layout_ui.rs:174 preview
    pub fn preview(&self) -> Option<GuidePreview> {
        todo!()
    }

    /// The colour swatch was clicked in the pass since the last call: the colour to ask the
    /// app's picker for, keyed [`GUIDE_LAYOUT_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: the layout's colour (the preview redraws
    /// in it). Other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        todo!()
    }

    /// The picker was cancelled: the colour stays.
    pub fn cancel_color(&mut self) {
        todo!()
    }
}

impl StackDialog for GuideLayoutDialog {
    type Result = GuideLayoutResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/guide_layout_ui.rs:369 body, :261 main_fields, :233 axis
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<GuideLayoutResult>> {
        todo!()
    }
}
