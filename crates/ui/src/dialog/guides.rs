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

use std::cell::Cell;

use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{Rgba, Srgba, TypeRole};
use egui::{Color32, Context, Ui};

use super::ColorRequest;
use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::{DialogLabels, Footer};
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::canvas::{Guide, Orientation};
use crate::composites::{FormGrid, GroupBox, Notice, Tone};
use crate::controls::expr::ExprUnits;
use crate::controls::{Checkbox, NumberField, RadioGroup, Select, Swatch};
use crate::edit::EditEvent;
use crate::localized::units::{Length, LengthUnit, UnitContext};
use crate::localized::{LocaleNumbers, SharedLabels as _};
use crate::patterns::unit_number_field;
use crate::primitives::{Content, Gap, Text};
use crate::theme::{Tokens, TokensOf as _};

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

/// The `Remembered` scope of New Guide's last guide.
const NEW_GUIDE_SCOPE: &str = "guides.newGuide";
/// The `Remembered` scope of New Guide Layout's last layout.
const LAYOUT_SCOPE: &str = "guides.layout";
/// How far from the origin a guide may be (PhotoCraft's `MAX_GUIDE_PX`).
const MAX_GUIDE_PX: f64 = 1_000_000.0;
/// The largest length a layout field takes, in pixels (PhotoCraft's field range).
const MAX_LAYOUT_PX: f64 = 300_000.0;
/// The most columns or rows (PhotoCraft's `MAX_COUNT`).
const MAX_COUNT: u32 = 1000;
/// The most guide edges a layout makes over all targets (PhotoCraft's `MAX_EDGES`).
const MAX_EDGES: u64 = 16_000;
/// Guides closer than this are one guide.
const SAME_GUIDE: f64 = 0.01;

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
        match self {
            GuideColor::LightBlue => [0x33, 0x66, 0xff],
            GuideColor::LightRed => [0xff, 0x33, 0x33],
            GuideColor::Green => [0x33, 0xff, 0x33],
            GuideColor::MediumBlue => [0x33, 0x33, 0xff],
            GuideColor::Yellow => [0xff, 0xcc, 0x00],
            GuideColor::Magenta => [0xff, 0x00, 0xff],
            GuideColor::Cyan => [0x4a, 0xff, 0xff],
            GuideColor::LightGray => [0xe7, 0xe7, 0xe7],
            GuideColor::Black => [0x00, 0x00, 0x00],
            GuideColor::Custom(rgb) => rgb,
        }
    }

    /// The named colour with these values, else [`GuideColor::Custom`] (an edited guide's own
    /// colour shows as its name when it is one).
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:114 color_choice
    pub fn from_rgb(rgb: [u8; 3]) -> GuideColor {
        GuideColor::NAMED.into_iter().find(|named| named.rgb() == rgb).unwrap_or(GuideColor::Custom(rgb))
    }

    pub fn color32(self) -> Color32 {
        let [r, g, b] = self.rgb();
        Color32::from_rgb(r, g, b)
    }

    pub fn label(self, labels: &GuidesLabels) -> &str {
        match self {
            GuideColor::LightBlue => &labels.light_blue,
            GuideColor::LightRed => &labels.light_red,
            GuideColor::Green => &labels.green,
            GuideColor::MediumBlue => &labels.medium_blue,
            GuideColor::Yellow => &labels.yellow,
            GuideColor::Magenta => &labels.magenta,
            GuideColor::Cyan => &labels.cyan,
            GuideColor::LightGray => &labels.light_gray,
            GuideColor::Black => &labels.black,
            GuideColor::Custom(_) => &labels.custom_color,
        }
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
    /// In the ruler unit: the field converts anything typed with another unit ("2.5 cm" in a
    /// pixel ruler) into it.
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

/// `value` as the number a field shows in `unit`: rounded to the unit's places.
fn rounded(value: f64, unit: LengthUnit) -> f64 {
    let scale = 10f64.powi(i32::try_from(unit.decimals()).unwrap_or(0));
    (value * scale).round() / scale
}

/// The value a field's event changed it to, for a field showing `shown` in `unit`. A commit that
/// kept the value, and a value that rounds to what the field already showed, change nothing, so
/// a length the user didn't touch keeps its exact value.
fn edited(event: Option<&EditEvent<f64>>, shown: f64, unit: LengthUnit) -> Option<f64> {
    let value = *match event? {
        EditEvent::Commit { changed: false, .. } => return None,
        event => event.value()?,
    };
    (rounded(value, unit) != rounded(shown, unit)).then_some(value)
}

/// [`edited`] for a field that may be empty.
fn edited_optional(event: Option<&EditEvent<Option<f64>>>, shown: Option<f64>, unit: LengthUnit) -> Option<Option<f64>> {
    let value = *match event? {
        EditEvent::Commit { changed: false, .. } => return None,
        event => event.value()?,
    };
    match (value, shown) {
        (None, None) => None,
        (Some(new), Some(old)) if rounded(new, unit) == rounded(old, unit) => None,
        _ => Some(value),
    }
}

/// One length field in the ruler unit: `shown` is the value in `units.ruler`, typed text may
/// carry another unit or arithmetic, and the field accepts `range` (in the ruler unit).
fn length_field<'a>(
    salt: impl egui::AsIdSalt,
    caption: &str,
    shown: f64,
    numbers: &'a LocaleNumbers,
    units: &GuideUnits,
    extent: f64,
) -> NumberField<'a> {
    let ruler = units.ruler;
    let context = units.context(extent);
    let high = context.from_pixels(MAX_LAYOUT_PX, ruler).filter(|v| v.is_finite()).unwrap_or(MAX_LAYOUT_PX);
    unit_number_field(salt, caption, shown, numbers, &ExprUnits { unit: Some(ruler), suffix: ruler.suffix(), context }).range(0.0..=high.max(0.0))
}

/// New Guide (and Edit Selected Guides): an orientation, a position with a unit and a colour.
///
/// The user sees Orientation (Horizontal / Vertical radio buttons, one Tab stop, arrows pick),
/// Position (a number field in the ruler unit; the first field, Orientation, has the focus on
/// open and Tab reaches Position), Color (a
/// list: Custom… then Photoshop's nine colours; with Custom… a swatch and its `#rrggbb` beside
/// it), and OK and Cancel. Enter is OK, Esc is Cancel.
///
/// It opens with the last successful guide's orientation, colour and position (shown in the
/// current ruler unit), else horizontal at 0 in Cyan, as Photoshop does; the app records a
/// success with [`NewGuideDialog::remember`] after its command accepted the guide. A position the
/// app refuses reopens the dialog with what was typed and the app's message:
/// `dialogs.open(NewGuideDialog::new(units).initial(answer).error(message))`.
///
/// The Position field is a [`unit_number_field`](crate::patterns::unit_number_field) in the ruler
/// unit over the document's width (vertical) or height (horizontal), so "50%" and "2 cm" work and
/// the field keeps the library's number-field keys. Text that isn't a length is not taken: the
/// field keeps its last value, so the dialog never holds a bad position.
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
    position_px: f64,
    color: GuideColor,
    /// The colour Custom… starts the picker at: the last custom colour.
    custom: [u8; 3],
    /// The colour the dialog opened with; `None` when it reopened after a changed colour.
    initial_color: Option<GuideColor>,
    edited: Option<u64>,
    error: Option<String>,
    /// The request to hand out once.
    color_request: Option<ColorRequest>,
    /// The colour before Custom… was chosen, while the picker is open.
    before_custom: Option<GuideColor>,
    /// Set once the remembered guide was read (first pass, which has the Host).
    recalled: bool,
    state: DialogState,
}

impl NewGuideDialog {
    /// New Guide, with the remembered guide (read on the first pass).
    pub fn new(units: GuideUnits) -> Self {
        Self {
            id: dialog_id("guides.newGuide"),
            units,
            orientation: Orientation::Horizontal,
            position_px: 0.0,
            color: GuideColor::Cyan,
            custom: GuideColor::Cyan.rgb(),
            initial_color: Some(GuideColor::Cyan),
            edited: None,
            error: None,
            color_request: None,
            before_custom: None,
            recalled: false,
            state: DialogState::new(),
        }
    }

    /// Edit Selected Guides on `guide`: its orientation, its position in the ruler unit, and its
    /// own colour, or `preference_color` it is drawn in. Nothing is remembered.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:124 open_edit
    pub fn edit(guide: &Guide, preference_color: [u8; 3], units: GuideUnits) -> Self {
        let rgb = guide.color.map_or(preference_color, |c| [c.r(), c.g(), c.b()]);
        let color = GuideColor::from_rgb(rgb);
        Self {
            orientation: guide.orientation,
            position_px: guide.position,
            color,
            custom: rgb,
            initial_color: Some(color),
            edited: Some(guide.key),
            recalled: true,
            ..Self::new(units)
        }
    }

    /// Opens with `options` instead of the remembered guide: the dialog again after the app
    /// refused an answer, with its values and its edited guide (automation, tests). The position
    /// shows in the ruler unit.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:235 confirm (reopen with __error)
    pub fn initial(mut self, options: NewGuideOptions) -> Self {
        self.orientation = options.orientation;
        self.position_px = options.position_px;
        self.color = options.color;
        if let GuideColor::Custom(rgb) = options.color {
            self.custom = rgb;
        }
        self.initial_color = (!options.color_changed).then_some(options.color);
        self.edited = options.edited;
        self.recalled = true;
        self
    }

    /// The app's message under the fields (a `Tone::Bad` notice).
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// Records `answer` as the guide the next New Guide opens with, through the Host's settings
    /// (`Remembered` scope `"guides.newGuide"`). Edit answers are not recorded.
    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:215 confirm (the remembered value at :226)
    pub fn remember(ctx: &Context, answer: &NewGuideOptions) {
        if answer.edited.is_some() {
            return;
        }
        if let Some(host) = Host::try_of_ctx(ctx) {
            let memory =
                NewGuideMemory { vertical: answer.orientation == Orientation::Vertical, position_px: answer.position_px, color: answer.color };
            Remembered::<NewGuideMemory>::new(NEW_GUIDE_SCOPE).store(&host, &memory);
        }
    }

    /// The values shown now: what OK answers.
    pub fn options(&self) -> NewGuideOptions {
        let ruler = self.units.ruler;
        let value = self.context().from_pixels(self.position_px, ruler).unwrap_or(0.0);
        NewGuideOptions {
            orientation: self.orientation,
            position: Length { value, unit: ruler },
            position_px: self.position_px,
            color: self.color,
            color_changed: self.edited.is_some() && self.initial_color != Some(self.color),
            edited: self.edited,
        }
    }

    /// Lengths here are across the document's width for a vertical guide, its height otherwise.
    fn context(&self) -> UnitContext {
        self.units.context(if self.orientation == Orientation::Vertical { self.units.width_px } else { self.units.height_px })
    }

    /// Custom… was chosen (or its swatch clicked) in the pass since the last call: the colour to
    /// ask the app's picker for, keyed [`NEW_GUIDE_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: Color becomes [`GuideColor::Custom`]
    /// (rounded to 8 bits). Other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key != NEW_GUIDE_COLOR_KEY {
            return;
        }
        let [r, g, b, _] = color.to_rgba().0;
        self.custom = [r, g, b];
        self.color = GuideColor::Custom(self.custom);
        self.before_custom = None;
        self.color_request = None;
    }

    /// The picker was cancelled: the colour stays what it was before Custom….
    pub fn cancel_color(&mut self) {
        if let Some(before) = self.before_custom.take() {
            self.color = before;
        }
        self.color_request = None;
    }

    /// Chooses Custom…: the request for the picker, opening at the last custom colour (or the
    /// colour already custom).
    fn ask_color(&mut self, title: &str) {
        if self.before_custom.is_none() {
            self.before_custom = Some(self.color);
        }
        let rgb = match self.color {
            GuideColor::Custom(rgb) => rgb,
            _ => self.custom,
        };
        self.color = GuideColor::Custom(rgb);
        let [r, g, b] = rgb;
        self.color_request = Some(ColorRequest::new(NEW_GUIDE_COLOR_KEY, Rgba::rgb(r, g, b).into()).title(title));
    }

    /// Reads the remembered guide: its orientation, colour and position in document pixels.
    fn recall(&mut self, ctx: &Context) {
        self.recalled = true;
        let Some(memory) = Host::try_of_ctx(ctx).and_then(|host| Remembered::<NewGuideMemory>::new(NEW_GUIDE_SCOPE).recall(&host)) else { return };
        self.orientation = if memory.vertical { Orientation::Vertical } else { Orientation::Horizontal };
        self.position_px = if memory.position_px.is_finite() { memory.position_px.clamp(-MAX_GUIDE_PX, MAX_GUIDE_PX) } else { 0.0 };
        self.color = memory.color;
        if let GuideColor::Custom(rgb) = memory.color {
            self.custom = rgb;
        }
        self.initial_color = Some(memory.color);
    }

    fn body(&mut self, ui: &mut Ui, labels: &GuidesLabels, numbers: &LocaleNumbers) {
        if let Some(error) = &self.error {
            Notice::new(error.as_str()).tone(Tone::Bad).id_salt("guides-error").show(ui);
            ui.add_space(Gap::Row.points(&Tokens::of(ui)));
        }
        FormGrid::new("new-guide").show(ui, |form| {
            form.row(labels.orientation.as_str(), |ui, _| {
                RadioGroup::new("new-guide-orientation", labels.orientation.as_str(), &mut self.orientation)
                    .item(Orientation::Horizontal, labels.horizontal.as_str())
                    .item(Orientation::Vertical, labels.vertical.as_str())
                    .show(ui)
            });
            form.row(labels.position.as_str(), |ui, _| {
                let ruler = self.units.ruler;
                let context = self.context();
                let shown = context.from_pixels(self.position_px, ruler).unwrap_or(0.0);
                let high = context.from_pixels(MAX_GUIDE_PX, ruler).filter(|v| v.is_finite()).unwrap_or(MAX_GUIDE_PX).abs();
                let units = ExprUnits { unit: Some(ruler), suffix: ruler.suffix(), context };
                let edit = unit_number_field("new-guide-position", labels.position.as_str(), shown, numbers, &units).range(-high..=high).show(ui);
                if let Some(value) = edited(edit.event.as_ref(), shown, ruler)
                    && let Some(px) = context.to_pixels(value, ruler)
                {
                    self.position_px = px.clamp(-MAX_GUIDE_PX, MAX_GUIDE_PX);
                }
                edit
            });
            form.row(labels.color.as_str(), |ui, _| self.color_row(ui, labels));
        });
    }

    fn color_row(&mut self, ui: &mut Ui, labels: &GuidesLabels) -> egui::Response {
        let mut choice = self.color;
        let mut select =
            Select::new("new-guide-color", labels.color.as_str(), &mut choice).option(GuideColor::Custom(self.custom), labels.custom_color.as_str());
        for named in GuideColor::NAMED {
            select = select.option(named, named.label(labels));
        }
        let response = select.show(ui).response;
        let mut asks = false;
        if choice != self.color {
            match choice {
                GuideColor::Custom(_) => asks = true,
                named => self.color = named,
            }
        }
        if let GuideColor::Custom(rgb) = self.color {
            let [r, g, b] = rgb;
            let color = Rgba::rgb(r, g, b);
            // A click on the swatch asks again, also while the choice is already Custom….
            asks |= ui.add(Swatch::new("new-guide-swatch", labels.custom_color.as_str(), color)).clicked();
            ui.add(Text::new(color.to_hex()).role(TypeRole::Mono));
        }
        if asks {
            self.ask_color(&labels.picker_title);
        }
        response
    }
}

impl StackDialog for NewGuideDialog {
    type Result = NewGuideOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/new_guide_ui.rs:151 body, :194 params
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<NewGuideOptions>> {
        let labels = GuidesLabels::shared_ctx(ctx);
        if !self.recalled {
            self.recall(ctx);
        }
        let numbers = LocaleNumbers::of_ctx(ctx);
        let title = if self.edited.is_some() { &labels.edit_title } else { &labels.new_guide_title };
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id, title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            &mut state,
            &mut (),
            |ui, ()| self.body(ui, &labels, &numbers),
        );
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.options())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
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
        let lines = self.vertical.iter().map(|x| (Orientation::Vertical, *x)).chain(self.horizontal.iter().map(|y| (Orientation::Horizontal, *y)));
        lines.zip(first_key..).map(|((orientation, position), key)| Guide { color: Some(color), ..Guide::new(key, orientation, position) }).collect()
    }

    /// Adds `value` unless a line within [`SAME_GUIDE`] is there. Fails for a value that is not
    /// finite.
    fn add(lines: &mut Vec<f64>, value: f64) -> Result<(), GuideLayoutError> {
        if !value.is_finite() {
            return Err(GuideLayoutError::DoesNotFit);
        }
        if !lines.iter().any(|x| (x - value).abs() < SAME_GUIDE) {
            lines.push(value);
        }
        Ok(())
    }

    fn sorted(mut self) -> Self {
        self.vertical.sort_by(f64::total_cmp);
        self.horizontal.sort_by(f64::total_cmp);
        self
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
        let (columns, rows, gutter) = match self {
            GuideLayoutPreset::Custom => return,
            GuideLayoutPreset::FiveColumnsThreeRows => (5, 3, 0.0),
            GuideLayoutPreset::EightColumns => (8, 0, 20.0),
            GuideLayoutPreset::TwelveColumns => (12, 0, 24.0),
        };
        let pixels = |value| Length { value, unit: LengthUnit::Pixels };
        layout.preset = self;
        layout.columns = GuideAxis { enabled: true, number: columns, size: None, gutter: pixels(gutter) };
        layout.rows = GuideAxis { enabled: rows > 0, number: rows.max(1), size: None, gutter: pixels(0.0) };
        layout.margin_enabled = true;
        layout.margin = [pixels(0.0); 4];
        layout.center_columns = false;
    }

    fn label(self, labels: &GuidesLabels) -> &str {
        match self {
            GuideLayoutPreset::Custom => &labels.preset_custom,
            GuideLayoutPreset::FiveColumnsThreeRows => &labels.preset_five,
            GuideLayoutPreset::EightColumns => &labels.preset_eight,
            GuideLayoutPreset::TwelveColumns => &labels.preset_twelve,
        }
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
    /// With a fixed column width, centre the columns (and with a fixed row height the rows)
    /// between the margins instead of packing them from the start (PhotoCraft's engine centres
    /// both).
    pub center_columns: bool,
    pub clear_existing: bool,
}

impl Default for GuideLayoutOptions {
    /// PhotoCraft's factory layout (Alt+Cancel resets to it): 8 columns with a 20 px gutter,
    /// rows off, no margin.
    // pc: crates/ui-egui/src/menus/view_cmds.rs:123 default_guide_layout
    fn default() -> Self {
        let pixels = |value| Length { value, unit: LengthUnit::Pixels };
        GuideLayoutOptions {
            preset: GuideLayoutPreset::Custom,
            target: GuideTarget::Document,
            columns: GuideAxis { enabled: true, number: 8, size: None, gutter: pixels(20.0) },
            rows: GuideAxis { enabled: false, number: 1, size: None, gutter: pixels(0.0) },
            margin_enabled: true,
            margin: [pixels(0.0); 4],
            center_columns: false,
            clear_existing: false,
        }
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
        match self {
            GuideLayoutError::DoesNotFit => &labels.does_not_fit,
            GuideLayoutError::MarginsTooLarge => &labels.margins_too_large,
            GuideLayoutError::Nothing => &labels.nothing,
            GuideLayoutError::TooMany => &labels.too_many,
            GuideLayoutError::NoArtboards => &labels.no_artboards,
        }
    }
}

/// The guides a layout makes: `added` are its own lines, `all` the set OK commits (the existing
/// guides, unless Clear Existing Guides, plus `added`).
#[derive(Clone, Debug, PartialEq)]
pub struct GuideLayoutLines {
    pub added: GuideLines,
    pub all: GuideLines,
}

/// `length` in pixels across `extent`, when it is a finite length of at least zero.
fn pixels_of(length: Length, units: &GuideUnits, extent: f64) -> Option<f64> {
    units.context(extent).to_pixels(length.value, length.unit).filter(|v| v.is_finite() && *v >= 0.0)
}

/// The edges of `count` columns (or rows) across `start..end`, Photoshop-style: both edges of
/// every one. A fixed `size` packs them from `start`, or centres them; otherwise they share the
/// room left by the gutters.
// pc: crates/engine/src/guide_layout.rs:118 axis
fn axis_edges(start: f64, end: f64, count: u64, fixed: f64, gap: f64, center: bool, out: &mut Vec<f64>) -> Result<(), GuideLayoutError> {
    if count == 0 {
        return Ok(());
    }
    let available = end - start;
    let n = count as f64;
    let size = if fixed > 0.0 { fixed } else { (available - (n - 1.0) * gap) / n };
    let total = n * size + (n - 1.0) * gap;
    if size <= 0.0 || !total.is_finite() || total > available + 0.001 {
        return Err(GuideLayoutError::DoesNotFit);
    }
    let origin = if center && fixed > 0.0 { start + (available - total) / 2.0 } else { start };
    for i in 0..count {
        let a = origin + i as f64 * (size + gap);
        GuideLines::add(out, a)?;
        GuideLines::add(out, a + size)?;
    }
    Ok(())
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
        let count = |axis: &GuideAxis| if axis.enabled { u64::from(axis.number) } else { 0 };
        let (columns, rows) = (count(&self.columns), count(&self.rows));
        if columns.max(rows) > u64::from(MAX_COUNT) {
            return Err(GuideLayoutError::TooMany);
        }
        if targets.is_empty() {
            return Err(GuideLayoutError::NoArtboards);
        }
        if (columns + rows + 4).saturating_mul(2).saturating_mul(targets.len() as u64) > MAX_EDGES {
            return Err(GuideLayoutError::TooMany);
        }
        let zero = Length { value: 0.0, unit: LengthUnit::Pixels };
        let margin = if self.margin_enabled { self.margin } else { [zero; 4] };
        let mut added = GuideLines::default();
        for rect in targets {
            let (across, down) = (rect.x1 - rect.x0, rect.y1 - rect.y0);
            let sides = [(margin[0], down), (margin[1], across), (margin[2], down), (margin[3], across)];
            let [top, left, bottom, right] = sides.map(|(length, extent)| pixels_of(length, units, extent).ok_or(GuideLayoutError::MarginsTooLarge));
            let (top, left, bottom, right) = (top?, left?, bottom?, right?);
            let (x0, y0, x1, y1) = (rect.x0 + left, rect.y0 + top, rect.x1 - right, rect.y1 - bottom);
            if x0 >= x1 || y0 >= y1 {
                return Err(GuideLayoutError::MarginsTooLarge);
            }
            if [top, left, bottom, right].iter().any(|m| *m != 0.0) {
                GuideLines::add(&mut added.vertical, x0)?;
                GuideLines::add(&mut added.vertical, x1)?;
                GuideLines::add(&mut added.horizontal, y0)?;
                GuideLines::add(&mut added.horizontal, y1)?;
            }
            // A disabled group's size and gutter are not read, so stale values can't fail it.
            for (axis, number, extent, from, to, out) in
                [(&self.columns, columns, across, x0, x1, &mut added.vertical), (&self.rows, rows, down, y0, y1, &mut added.horizontal)]
            {
                if number == 0 {
                    continue;
                }
                let size = match axis.size {
                    Some(size) => pixels_of(size, units, extent).ok_or(GuideLayoutError::DoesNotFit)?,
                    None => 0.0,
                };
                let gap = pixels_of(axis.gutter, units, extent).ok_or(GuideLayoutError::DoesNotFit)?;
                axis_edges(from, to, number, size, gap, self.center_columns, out)?;
            }
        }
        if added.vertical.is_empty() && added.horizontal.is_empty() {
            return Err(GuideLayoutError::Nothing);
        }
        let mut all = existing.clone();
        if self.clear_existing {
            if self.target == GuideTarget::SelectedArtboards {
                // Guides are document-wide axes: only those inside a selected artboard go.
                all.vertical.retain(|v| !targets.iter().any(|r| *v >= r.x0 && *v <= r.x1));
                all.horizontal.retain(|v| !targets.iter().any(|r| *v >= r.y0 && *v <= r.y1));
            } else {
                all = GuideLines::default();
            }
        }
        for x in &added.vertical {
            GuideLines::add(&mut all.vertical, *x)?;
        }
        for y in &added.horizontal {
            GuideLines::add(&mut all.horizontal, *y)?;
        }
        Ok(GuideLayoutLines { added: added.sorted(), all: all.sorted() })
    }

    /// Whether `other` has the same layout fields: everything but the preset, the target and Clear
    /// Existing Guides (a typed edit to one of those fields turns the preset to Custom).
    fn same_fields(&self, other: &GuideLayoutOptions) -> bool {
        (self.columns, self.rows, self.margin_enabled, self.margin, self.center_columns)
            == (other.columns, other.rows, other.margin_enabled, other.margin, other.center_columns)
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
/// Width and Height are optional number fields (`NumberField::show_optional`): left empty they
/// mean automatic size, and show the tooltip "Leave empty for automatic size". Typing in a layout
/// field turns the preset to Custom; changing target, colour or Preview does not. A group whose
/// box is off greys its fields and keeps their values.
///
/// Live preview: while Preview is on (the default) and the layout is valid, the app draws
/// [`preview`](Self::preview) on the canvas each pass, in place of the document's guides; the
/// dialog asks for a repaint when a field changes. An invalid layout keeps OK disabled and shows
/// [`GuidesLabels::invalid`] with the reason as its tooltip; the canvas keeps the last valid
/// preview. Enter is OK when valid; Esc cancels; Alt turns Cancel into Reset, which restores
/// [`GuideLayoutOptions::default`] (a Cancel pressed with Alt held resets, as the side footer has
/// no Reset of its own). The last committed layout opens next time (`Remembered`
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
    /// The guide colour preference the dialog opened in, which Reset returns to.
    preference_color: [u8; 3],
    color: [u8; 3],
    preview_on: bool,
    /// The last calculation, kept while the layout is unchanged.
    calculated: Option<(GuideLayoutOptions, Result<GuideLayoutLines, GuideLayoutError>)>,
    /// The last valid calculation, kept while a field is invalid.
    last_valid: Option<GuideLayoutLines>,
    color_request: Option<ColorRequest>,
    recalled: bool,
    state: DialogState,
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
            preference_color: color,
            color,
            preview_on: true,
            calculated: None,
            last_valid: None,
            color_request: None,
            recalled: false,
            state: DialogState::new(),
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
        if !self.preview_on {
            return None;
        }
        let [r, g, b] = self.color;
        self.last_valid.clone().map(|lines| GuidePreview { lines, color: Color32::from_rgb(r, g, b) })
    }

    /// The colour swatch was clicked in the pass since the last call: the colour to ask the
    /// app's picker for, keyed [`GUIDE_LAYOUT_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: the layout's colour (the preview redraws
    /// in it). Other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key != GUIDE_LAYOUT_COLOR_KEY {
            return;
        }
        let [r, g, b, _] = color.to_rgba().0;
        self.color = [r, g, b];
        self.color_request = None;
    }

    /// The picker was cancelled: the colour stays.
    pub fn cancel_color(&mut self) {
        self.color_request = None;
    }

    /// The layout shown now.
    pub fn options(&self) -> GuideLayoutOptions {
        self.layout
    }

    /// The rectangles the layout fills.
    fn targets(&self) -> Vec<GuideRect> {
        match self.layout.target {
            GuideTarget::Document => vec![self.document],
            GuideTarget::SelectedArtboards => self.artboards.clone(),
        }
    }

    /// The width and height a field's percent refers to: the first target's.
    fn extents(&self) -> (f64, f64) {
        let rect = self.targets().first().copied().unwrap_or(self.document);
        (rect.x1 - rect.x0, rect.y1 - rect.y0)
    }

    /// The layout's lines, calculated again only when the layout changed.
    fn calculate(&mut self) -> Result<GuideLayoutLines, GuideLayoutError> {
        if let Some((options, result)) = &self.calculated
            && *options == self.layout
        {
            return result.clone();
        }
        let result = self.layout.lines(&self.targets(), &self.existing, &self.units);
        self.calculated = Some((self.layout, result.clone()));
        if let Ok(lines) = &result {
            self.last_valid = Some(lines.clone());
        }
        result
    }

    /// Reads the remembered layout, unless the app gave one with [`initial`](Self::initial); a
    /// remembered artboard target without artboards is the document.
    fn recall(&mut self, ctx: &Context) {
        if !self.recalled
            && let Some(layout) = Host::try_of_ctx(ctx).and_then(|host| Remembered::<GuideLayoutOptions>::new(LAYOUT_SCOPE).recall(&host))
        {
            self.layout = layout;
        }
        self.recalled = true;
        for axis in [&mut self.layout.columns, &mut self.layout.rows] {
            axis.number = axis.number.clamp(1, MAX_COUNT);
        }
        if self.layout.target == GuideTarget::SelectedArtboards && self.artboards.is_empty() {
            self.layout.target = GuideTarget::Document;
        }
    }

    /// Alt+Cancel: the factory layout in the preference colour, Preview on.
    fn reset(&mut self) {
        self.layout = GuideLayoutOptions::default();
        self.color = self.preference_color;
        self.preview_on = true;
        self.color_request = None;
    }
}

impl StackDialog for GuideLayoutDialog {
    type Result = GuideLayoutResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/guide_layout_ui.rs:369 body, :261 main_fields, :233 axis
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<GuideLayoutResult>> {
        let labels = GuidesLabels::shared_ctx(ctx);
        let words = DialogLabels::shared_ctx(ctx);
        if !self.recalled {
            self.recall(ctx);
        }
        let numbers = LocaleNumbers::of_ctx(ctx);
        let before = (self.layout, self.color, self.preview_on);
        let alt = ctx.input(|i| i.modifiers.alt);
        let valid = Cell::new(false);
        let mut preview = self.preview_on;
        let mut state = std::mem::take(&mut self.state);
        let footer =
            Footer::side().cancel(if alt { words.reset.as_str() } else { words.cancel.as_str() }).ok_enabled_with(|| valid.get()).below(|ui| {
                let _ = Checkbox::bool(words.preview.as_str(), &mut preview).id_salt("guide-layout-preview").show(ui);
            });
        let shown = Dialog::with_id(self.id, labels.layout_title.as_str()).stack(stack).width(Width::Standard).footer(footer).show(
            ctx,
            &mut state,
            &mut (),
            |ui, ()| self.body(ui, &labels, &numbers, &valid),
        );
        self.state = state;
        self.preview_on = preview;
        let outcome = match shown.outcome {
            Some(Outcome::Cancel) if alt => {
                self.reset();
                None
            }
            outcome => outcome,
        };
        if before != (self.layout, self.color, self.preview_on) {
            ctx.request_repaint();
        }
        match outcome {
            Some(Outcome::Ok) => {
                let lines = self.calculate().ok()?;
                let options = self.layout;
                if let Some(host) = Host::try_of_ctx(ctx) {
                    Remembered::<GuideLayoutOptions>::new(LAYOUT_SCOPE).store(&host, &options);
                }
                Some(DialogResult::Ok(GuideLayoutResult { options, lines, color: self.color }))
            }
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

impl GuideLayoutDialog {
    fn body(&mut self, ui: &mut Ui, labels: &GuidesLabels, numbers: &LocaleNumbers, valid: &Cell<bool>) {
        let gap = Gap::Section.points(&Tokens::of(ui));
        let mut layout = self.layout;
        self.header(ui, labels, &mut layout);
        ui.add_space(gap);
        self.color_group(ui, labels);
        ui.add_space(gap);
        let (across, down) = self.extents();
        let units = self.units;
        ui.horizontal_top(|ui| {
            axis_group(ui, labels, numbers, &units, &mut layout.columns, Axis::Columns, across);
            axis_group(ui, labels, numbers, &units, &mut layout.rows, Axis::Rows, down);
        });
        ui.add_space(gap);
        let sides = [&labels.top, &labels.left, &labels.bottom, &labels.right];
        let mut margin_enabled = layout.margin_enabled;
        GroupBox::new(labels.margin.as_str()).checkbox(&mut margin_enabled).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, caption) in sides.into_iter().enumerate() {
                    // Top and bottom run along the height, left and right along the width.
                    let extent = if index % 2 == 0 { down } else { across };
                    ui.vertical(|ui| {
                        ui.add(Text::new(caption.as_str()).role(TypeRole::Label));
                        length_cell(ui, ("guide-layout-margin", index), caption, &mut layout.margin[index], numbers, &units, extent);
                    });
                }
            });
        });
        layout.margin_enabled = margin_enabled;
        ui.add_space(gap);
        let _ = Checkbox::bool(labels.center_columns.as_str(), &mut layout.center_columns).show(ui);
        let _ = Checkbox::bool(labels.clear_existing.as_str(), &mut layout.clear_existing).show(ui);
        // A typed edit to a layout field turns a named preset to Custom; target, colour and
        // Preview don't.
        if !layout.same_fields(&self.layout) && layout.preset == self.layout.preset {
            layout.preset = GuideLayoutPreset::Custom;
        }
        self.layout = layout;
        match self.calculate() {
            Ok(_) => valid.set(true),
            Err(error) => {
                valid.set(false);
                let text = Text::new(labels.invalid.as_str()).content(Content::Muted);
                ui.add(text).on_hover_text(error.message(labels));
            }
        }
    }

    /// Preset and Target.
    fn header(&mut self, ui: &mut Ui, labels: &GuidesLabels, layout: &mut GuideLayoutOptions) {
        FormGrid::new("guide-layout-header").show(ui, |form| {
            form.row(labels.preset.as_str(), |ui, _| {
                let mut preset = layout.preset;
                let mut select = Select::new("guide-layout-preset", labels.preset.as_str(), &mut preset);
                for option in GuideLayoutPreset::ALL {
                    select = select.option(option, option.label(labels));
                }
                let response = select.show(ui).response;
                if preset != layout.preset {
                    preset.apply(layout);
                    layout.preset = preset;
                }
                response
            });
            form.row(labels.target.as_str(), |ui, _| {
                let mut select = Select::new("guide-layout-target", labels.target.as_str(), &mut layout.target)
                    .option(GuideTarget::Document, labels.document.as_str());
                if !self.artboards.is_empty() {
                    select = select.option(GuideTarget::SelectedArtboards, labels.selected_artboards.as_str());
                }
                select.show(ui).response
            });
        });
    }

    /// Color: the list of Photoshop's colours (and the colour itself when it is none of them)
    /// with the swatch that asks the app's picker.
    fn color_group(&mut self, ui: &mut Ui, labels: &GuidesLabels) {
        GroupBox::new(labels.color.as_str()).show(ui, |ui| {
            ui.horizontal(|ui| {
                let current = GuideColor::from_rgb(self.color);
                let mut choice = current;
                let mut select = Select::new("guide-layout-color", labels.color.as_str(), &mut choice);
                if let GuideColor::Custom(rgb) = current {
                    select = select.option(GuideColor::Custom(rgb), labels.custom_color.as_str());
                }
                for named in GuideColor::NAMED {
                    select = select.option(named, named.label(labels));
                }
                let _ = select.show(ui);
                self.color = choice.rgb();
                let [r, g, b] = self.color;
                let swatch = ui.add(Swatch::new("guide-layout-swatch", labels.layout_color.as_str(), Rgba::rgb(r, g, b)));
                if swatch.clicked() {
                    self.color_request =
                        Some(ColorRequest::new(GUIDE_LAYOUT_COLOR_KEY, Rgba::rgb(r, g, b).into()).title(labels.layout_color.as_str()));
                }
            });
        });
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Columns,
    Rows,
}

/// One length of a layout in the ruler unit: edits convert into `length`'s own unit, the ruler.
fn length_cell(ui: &mut Ui, salt: impl egui::AsIdSalt, caption: &str, length: &mut Length, numbers: &LocaleNumbers, units: &GuideUnits, extent: f64) {
    let ruler = units.ruler;
    let shown = units.context(extent).convert(*length, ruler).unwrap_or(length.value);
    let edit = length_field(salt, caption, shown, numbers, units, extent).show(ui);
    if let Some(value) = edited(edit.event.as_ref(), shown, ruler) {
        *length = Length { value, unit: ruler };
    }
}

/// The Columns or Rows group: its check box, Number, Width or Height (empty is automatic) and
/// Gutter.
fn axis_group(ui: &mut Ui, labels: &GuidesLabels, numbers: &LocaleNumbers, units: &GuideUnits, axis: &mut GuideAxis, kind: Axis, extent: f64) {
    let (title, size_caption, salt) = match kind {
        Axis::Columns => (&labels.columns, &labels.width, "guide-layout-columns"),
        Axis::Rows => (&labels.rows, &labels.height, "guide-layout-rows"),
    };
    let ruler = units.ruler;
    let mut enabled = axis.enabled;
    GroupBox::new(title.as_str()).checkbox(&mut enabled).id_salt(salt).fill(false).show(ui, |ui| {
        FormGrid::new((salt, "fields")).show(ui, |form| {
            form.row(labels.number.as_str(), |ui, _| {
                let edit = NumberField::new((salt, "number"), labels.number.as_str(), axis.number, numbers)
                    .range(1.0..=f64::from(MAX_COUNT))
                    .decimals(0..=0)
                    .show(ui);
                if let Some(value) =
                    edit.event.as_ref().and_then(|event| if matches!(event, EditEvent::Commit { changed: false, .. }) { None } else { event.value() })
                {
                    axis.number = (value.round() as u32).clamp(1, MAX_COUNT);
                }
                edit
            });
            form.row(size_caption.as_str(), |ui, _| {
                let shown = axis.size.and_then(|size| units.context(extent).convert(size, ruler));
                let edit = length_field((salt, "size"), size_caption, shown.unwrap_or(f64::NAN), numbers, units, extent).show_optional(ui);
                if let Some(value) = edited_optional(edit.event.as_ref(), shown, ruler) {
                    axis.size = value.map(|value| Length { value, unit: ruler });
                }
                edit.response.clone().on_hover_text(labels.automatic_size.as_str());
                edit
            });
            form.row(labels.gutter.as_str(), |ui, _| {
                let shown = units.context(extent).convert(axis.gutter, ruler).unwrap_or(axis.gutter.value);
                let edit = length_field((salt, "gutter"), labels.gutter.as_str(), shown, numbers, units, extent).show(ui);
                if let Some(value) = edited(edit.event.as_ref(), shown, ruler) {
                    axis.gutter = Length { value, unit: ruler };
                }
                edit
            });
        });
    });
    axis.enabled = enabled;
}
