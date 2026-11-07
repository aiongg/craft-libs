//! View > Guides > New Guide, Edit Selected Guides and New Guide Layout: [`NewGuideDialog`] and [`GuideLayoutDialog`]
//! on `Dialog`, and their `DialogHost` forms.
//!
//! Positions and sizes are lengths: the fields open in the ruler unit ("0 cm") and take any unit Photoshop's fields
//! take ("50%", "2.5 cm", "1in", "12 pt", "3 pc", "100 px"); a bare number is in the ruler unit. Percent is of the
//! target's width for vertical guides, column widths and the left and right margins, and of its height for the rest.
//! [`GuideLayoutOptions::lines`] is the layout calculation, so the dialog's live preview and the app's command make
//! the same `canvas::Guide`s.

mod layout;
mod new_guide;

#[cfg(feature = "testing")]
mod stories;

pub use layout::{GuideGroup, GuideLayoutError, GuideLayoutOptions, GuideLayoutPreset, GuideLength, GuideRect, GuideTarget, GuideUnits, MAX_COUNT};
pub use new_guide::{HostedNewGuideDialog, NewGuideDialog, NewGuideOutput, NewGuideValues};

use super::{ColorRequest, HostRequests, ParamDialogWords, ResetFooter, ResetPick};
use crate::canvas::Guide;
use crate::choice::Checkbox;
use crate::contract::{Bind, EditResponse, Entry, NoValue, Outcome, Props as _, Remember as _, Show as _, ShowContainer as _};
use crate::dialog::{Dialog, DialogId, DialogOutput, DialogResult, DialogStack, StackDialog, dialog_id};
use crate::display::Text;
use crate::form::{Field, FieldGroup};
use crate::notices::{Notice, NoticeLayout};
use crate::pickers::Select;
use crate::runtime::env::{Env, Label, Units};
use crate::runtime::overlay::CloseReason;
use crate::runtime::words::CommonWords;
use crate::sections::{DialogRole, FooterButton, GroupBox};
use crate::text::NumberField;
use craft_behavior::{EditEvent, GestureId, Mixed, NumberCodec, Value};
use craft_theme::variant::Tone;
use craft_theme::{Rgba, Srgba};
use egui::{Response, Ui};
use std::cell::RefCell;
use std::sync::Arc;

/// The [`ColorRequest::key`] of New Guide's Custom colour.
pub const NEW_GUIDE_COLOR_KEY: &str = "guides.newGuide.color";
/// The [`ColorRequest::key`] of New Guide Layout's Custom colour.
pub const GUIDE_LAYOUT_COLOR_KEY: &str = "guides.layout.color";

/// How far from the origin a guide may be, in pixels.
const MAX_GUIDE_PX: f64 = 1_000_000.0;
/// The largest length a layout field takes, in pixels.
const MAX_LAYOUT_PX: f64 = 300_000.0;

craft_ui::words! {
    /// The guide dialogs' words, English by default (Photoshop's). OK, Cancel and Reset are the common words.
    pub struct GuideWords {
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
        picker_title: "guides.pickerTitle" = "Color Picker (Guides)";
        layout_title: "guides.layout" = "New Guide Layout";
        preset: "guides.preset" = "Preset";
        preset_custom: "guides.presetCustom" = "Custom";
        preset_five: "guides.presetFive" = "5 Columns, 3 Rows";
        preset_eight: "guides.presetEight" = "8 Columns";
        preset_twelve: "guides.presetTwelve" = "12 Columns";
        target: "guides.target" = "Target";
        document: "guides.document" = "Document";
        selected_artboards: "guides.selectedArtboards" = "Selected Artboards";
        columns: "guides.columns" = "Columns";
        rows: "guides.rows" = "Rows";
        number: "guides.number" = "Number";
        width: "guides.width" = "Width";
        height: "guides.height" = "Height";
        gutter: "guides.gutter" = "Gutter";
        automatic_size: "guides.automaticSize" = "Leave empty for automatic size";
        margin: "guides.margin" = "Margin";
        top: "guides.top" = "Top";
        left: "guides.left" = "Left";
        bottom: "guides.bottom" = "Bottom";
        right: "guides.right" = "Right";
        center_columns: "guides.centerColumns" = "Center Columns";
        clear_existing: "guides.clearExisting" = "Clear Existing Guides";
        invalid: "guides.invalid" = "Invalid guide layout";
        does_not_fit: "guides.doesNotFit" = "The columns or rows, their size and gutter must fit inside the margins.";
        margins_too_large: "guides.marginsTooLarge" = "The margins must leave space inside the target.";
        nothing: "guides.nothing" = "Turn on Columns, Rows or Margin.";
        too_many: "guides.tooMany" = "The layout would make too many guides.";
        no_artboards: "guides.noArtboards" = "Select an artboard or a layer inside one.";
    }
}

/// A guide colour: Photoshop's list (Cyan by default) or a custom colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuideColor {
    /// `#3366ff`.
    LightBlue,
    /// `#ff3333`.
    LightRed,
    /// `#33ff33`.
    Green,
    /// `#3333ff`.
    MediumBlue,
    /// `#ffcc00`.
    Yellow,
    /// `#ff00ff`.
    Magenta,
    /// `#4affff`.
    #[default]
    Cyan,
    /// `#e7e7e7`.
    LightGray,
    /// `#000000`.
    Black,
    /// A colour the user chose in the app's colour picker.
    Custom(Rgba),
}

impl GuideColor {
    /// The named colours in Photoshop's menu order; Custom is listed first in the menu, before these.
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

    /// The colour.
    pub const fn rgba(self) -> Rgba {
        match self {
            GuideColor::LightBlue => Rgba::rgb(0x33, 0x66, 0xff),
            GuideColor::LightRed => Rgba::rgb(0xff, 0x33, 0x33),
            GuideColor::Green => Rgba::rgb(0x33, 0xff, 0x33),
            GuideColor::MediumBlue => Rgba::rgb(0x33, 0x33, 0xff),
            GuideColor::Yellow => Rgba::rgb(0xff, 0xcc, 0x00),
            GuideColor::Magenta => Rgba::rgb(0xff, 0x00, 0xff),
            GuideColor::Cyan => Rgba::rgb(0x4a, 0xff, 0xff),
            GuideColor::LightGray => Rgba::rgb(0xe7, 0xe7, 0xe7),
            GuideColor::Black => Rgba::rgb(0x00, 0x00, 0x00),
            GuideColor::Custom(rgba) => rgba,
        }
    }

    /// The named colour with these values, else [`GuideColor::Custom`] (an edited guide's own colour shows as its
    /// name when it is one).
    pub fn from_rgba(rgba: Rgba) -> GuideColor {
        GuideColor::NAMED.into_iter().find(|named| named.rgba() == rgba).unwrap_or(GuideColor::Custom(rgba))
    }

    fn label(self, words: &GuideWords) -> Label {
        match self {
            GuideColor::LightBlue => words.light_blue.clone(),
            GuideColor::LightRed => words.light_red.clone(),
            GuideColor::Green => words.green.clone(),
            GuideColor::MediumBlue => words.medium_blue.clone(),
            GuideColor::Yellow => words.yellow.clone(),
            GuideColor::Magenta => words.magenta.clone(),
            GuideColor::Cyan => words.cyan.clone(),
            GuideColor::LightGray => words.light_gray.clone(),
            GuideColor::Black => words.black.clone(),
            GuideColor::Custom(_) => words.custom_color.clone(),
        }
    }
}

/// The colour a guide dialog holds: the list's choice, and the custom colour the list's Custom entry stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideColorChoice {
    /// The chosen colour.
    pub color: GuideColor,
    /// The colour the Custom entry holds.
    pub custom: Rgba,
}

impl GuideColorChoice {
    /// The choice of `color`; a custom colour is also the Custom entry's.
    pub fn new(color: GuideColor) -> Self {
        let custom = match color {
            GuideColor::Custom(rgba) => rgba,
            named => named.rgba(),
        };
        GuideColorChoice { color, custom }
    }

    /// The host's answer to the colour request: the custom colour, chosen.
    pub fn answer(&mut self, rgba: Rgba) {
        self.custom = rgba;
        self.color = GuideColor::Custom(rgba);
    }

    fn entries(&self, words: &GuideWords) -> Vec<Entry<GuideColor>> {
        let custom = Entry::new(GuideColor::Custom(self.custom), words.custom_color.clone());
        std::iter::once(custom).chain(GuideColor::NAMED.into_iter().map(|named| Entry::new(named, named.label(words)))).collect()
    }

    /// The colour row. Returns whether the user asked for the colour picker.
    fn row(&mut self, ui: &mut Ui, words: &GuideWords) -> bool {
        let entries = self.entries(words);
        let mut asks = false;
        let label = words.color.clone();
        let current = self.color;
        Field::new(label.clone()).show(ui, |ui, _| {
            if let Some(picked) = edited(&Select::new(label.clone(), Bind::given(current), &entries).show(ui)) {
                match picked {
                    GuideColor::Custom(_) => asks = true,
                    named => self.color = named,
                }
            }
        });
        if asks {
            self.color = GuideColor::Custom(self.custom);
        }
        asks
    }

    fn request(&self, key: &str, title: Label) -> ColorRequest {
        ColorRequest::new(key, Srgba::from(self.color.rgba())).title(title)
    }
}

/// The value a control's event carries.
pub(super) fn edited<T: Clone>(response: &EditResponse<T>) -> Option<T> {
    response.event.as_ref().and_then(|event| event.value().cloned())
}

/// `value` rounded to `places` decimals, for telling a typed value from the one the field already showed.
fn rounded(value: f64, places: usize) -> f64 {
    let scale = 10f64.powi(i32::try_from(places).unwrap_or(0));
    (value * scale).round() / scale
}

/// The form values of a guide dialog, as the caller's binding holds them.
pub(super) struct Held<'a, V>(Bind<'a, V>);

impl<'a, V: Value> Held<'a, V> {
    pub(super) fn new(bind: Bind<'a, V>) -> Self {
        Held(bind)
    }

    fn read(&mut self, cx: &mut crate::contract::BodyCx<'_>) -> Option<V> {
        match self.0.reborrow().read(cx) {
            Some(Mixed::Same(held)) => Some(held),
            _ => None,
        }
    }

    fn write(&mut self, cx: &mut crate::contract::BodyCx<'_>, before: &V, after: V, name: &Arc<str>) {
        let _ = self.0.reborrow().write(cx, &EditEvent::set(GestureId::default(), Mixed::Same(before.clone()), after, name.clone(), None));
    }

    fn apply(&mut self, before: &V, after: V, name: &Arc<str>) {
        let _ = self.0.reborrow().apply(&EditEvent::set(GestureId::default(), Mixed::Same(before.clone()), after, name.clone(), None));
    }
}

/// The values the dialog opened with.
#[derive(Clone)]
struct Opened<V>(Option<V>);

impl<V> Default for Opened<V> {
    fn default() -> Self {
        Opened(None)
    }
}

/// What the footer reads: whether the values differ from the opening ones (Reset) and whether they can be accepted.
#[derive(Clone, Copy)]
struct Footing {
    changed: bool,
    ok: bool,
}

impl Default for Footing {
    fn default() -> Self {
        Footing { changed: false, ok: true }
    }
}

/// What a form pass reports.
struct Shown<V> {
    output: DialogOutput,
    values: V,
}

/// Shows `dialog` over the values in `held`: `rows` edits the values and says whether they can be accepted. Alt makes
/// Cancel a Reset that restores the opening values.
fn run_form<V: Value>(
    ui: &mut Ui,
    dialog: Dialog<'_>,
    mut held: Held<'_, V>,
    fallback: V,
    name: Arc<str>,
    reset: &ResetFooter,
    mut rows: impl FnMut(&mut Ui, &mut crate::contract::BodyCx<'_>, &mut V) -> bool,
) -> Shown<V> {
    let work = RefCell::new(fallback.clone());
    let start = RefCell::new(None::<V>);
    let dialog = dialog.settle_buttons({
        let reset = reset.clone();
        move |cx, buttons| {
            let footing = *cx.memory::<Footing>();
            reset.settle(buttons, footing.changed);
            if let Some(accept) = buttons.first_mut() {
                accept.set_enabled(footing.ok);
            }
        }
    });
    let shown = dialog.show(ui, |ui, cx| {
        let before = held.read(cx).unwrap_or_else(|| fallback.clone());
        if cx.appeared() {
            cx.memory::<Opened<V>>().0 = Some(before.clone());
        }
        *start.borrow_mut() = cx.memory::<Opened<V>>().0.clone();
        let mut values = before.clone();
        let ok = rows(ui, cx, &mut values);
        let changed = start.borrow().as_ref().is_some_and(|opened| *opened != values);
        *cx.memory::<Footing>() = Footing { changed, ok };
        if values != before {
            held.write(cx, &before, values.clone(), &name);
        }
        *work.borrow_mut() = values;
    });
    let mut values = work.into_inner();
    if reset.pick(shown.output.picked) == Some(ResetPick::Reset)
        && let Some(opened) = start.into_inner()
    {
        held.apply(&values, opened.clone(), &name);
        values = opened;
        crate::runtime::frame::request_repaint(ui.ctx());
    }
    Shown { output: shown.output, values }
}

/// Whether a dialog that closed with `closed` was accepted.
fn accepted(closed: Option<CloseReason>) -> bool {
    matches!(closed, Some(CloseReason::Enter | CloseReason::Activated))
}

/// What the length fields of a guide dialog read with.
struct Measure<'a> {
    words: &'a GuideWords,
    numbers: &'a Units,
    units: &'a GuideUnits,
}

impl Measure<'_> {
    /// A number field `label` over the ruler unit showing `shown`, for lengths across `extent` pixels: the value the
    /// user typed if it differs from the one shown (`Some(None)` is an emptied field).
    fn input(&self, ui: &mut Ui, label: Label, shown: Option<f64>, extent: f64) -> Option<Option<f64>> {
        let ruler = self.units.ruler;
        let places = *ruler.decimals().end();
        let high = self.units.from_pixels(MAX_LAYOUT_PX, ruler, extent).filter(|v| v.is_finite()).unwrap_or(MAX_LAYOUT_PX).max(0.0);
        let codec = NumberCodec::new(self.units.reader(self.numbers, extent), places);
        let field = NumberField::new(label, Bind::given(shown)).codec(codec).range(0.0..=high).step(ruler.step()).unit(self.numbers.symbol(ruler));
        let typed = edited(&field.show(ui).edit)?;
        match (typed, shown) {
            (Some(new), Some(old)) if rounded(new, places) == rounded(old, places) => None,
            (None, None) => None,
            (typed, _) => Some(typed),
        }
    }

    /// A required length row in the ruler unit.
    fn length(&self, ui: &mut Ui, label: Label, length: &mut GuideLength, extent: f64) {
        let ruler = self.units.ruler;
        let shown = self.units.to_pixels(*length, extent).and_then(|px| self.units.from_pixels(px, ruler, extent)).unwrap_or(length.value);
        Field::new(label.clone()).show(ui, |ui, _| {
            if let Some(Some(value)) = self.input(ui, label.clone(), Some(shown), extent) {
                *length = GuideLength::new(value, ruler);
            }
        });
    }
}

/// The values of New Guide Layout.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideLayoutValues {
    /// The layout's settings.
    pub options: GuideLayoutOptions,
    /// The guides' colour.
    pub color: GuideColorChoice,
    /// Whether the app draws the layout while the dialog is open.
    pub preview: bool,
}

impl GuideLayoutValues {
    /// The values for `options` and `color`, with Preview on.
    pub fn new(options: GuideLayoutOptions, color: GuideColor) -> Self {
        GuideLayoutValues { options, color: GuideColorChoice::new(color), preview: true }
    }
}

impl Default for GuideLayoutValues {
    fn default() -> Self {
        GuideLayoutValues::new(GuideLayoutOptions::default(), GuideColor::default())
    }
}

/// What New Guide Layout answers.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideLayoutResult {
    /// The settings the user confirmed.
    pub options: GuideLayoutOptions,
    /// The guides OK commits: the existing guides (unless Clear Existing Guides) and the layout's.
    pub guides: Vec<Guide>,
    /// The colour chosen; the app may keep it as the guide colour preference.
    pub color: Rgba,
}

/// What New Guide Layout reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct GuideLayoutOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The answer, the pass the dialog closed as OK.
    pub result: Option<GuideLayoutResult>,
    /// The guides the app shows on the canvas in place of the document's while the dialog is open: the full set OK
    /// would commit. `None` with Preview off; the last valid set while a field is invalid.
    pub preview: Option<Vec<Guide>>,
    /// The Custom colour was asked for: the app opens its colour picker and answers into the values.
    pub pick_color: Option<ColorRequest>,
}

impl Outcome for GuideLayoutOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The last layout that was valid, for the preview while a field is invalid.
#[derive(Clone, Default)]
struct LastValid(Option<Vec<Guide>>);

/// New Guide Layout, Photoshop's dialog: Preset and Target lists, Color, the Columns and Rows groups (each a check
/// box with Number, Width or Height, and Gutter), the Margin group (a check box and Top, Left, Bottom, Right), Center
/// Columns and Clear Existing Guides; OK, Cancel and Preview.
///
/// - Keys: Tab walks the fields, Enter is OK when the layout is valid, Escape cancels. Holding Alt turns Cancel
///   into Reset, which restores the values from when the dialog opened and keeps the dialog open.
/// - Fields: every length takes any unit and shows the ruler unit; Width and Height left empty mean automatic size.
///   Typing in a layout field turns the preset to Custom; changing target, colour or Preview does not. A group whose
///   box is off greys its fields and keeps their values.
/// - Preview: while Preview is on and the layout is valid, [`GuideLayoutOutput::preview`] is the set the app draws.
///   An invalid layout disables OK and shows the reason under the fields.
/// - Values: the `values` binding holds them; the app keeps them for the next open.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct GuideLayoutDialog<'a> {
    units: GuideUnits,
    document: GuideRect,
    values: Held<'a, GuideLayoutValues>,
    /// The selected artboards the Target list offers.
    #[prop]
    artboards: Vec<GuideRect>,
    /// The guides the document has; the layout is added to them.
    #[prop]
    existing: Vec<Guide>,
    /// The key of the first guide the layout makes.
    #[prop]
    first_key: u64,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
}

impl<'a> GuideLayoutDialog<'a> {
    /// A dialog over `values` for the `document` measured in `units`.
    pub fn new(units: GuideUnits, document: GuideRect, values: Bind<'a, GuideLayoutValues>) -> Self {
        GuideLayoutDialog { units, document, values: Held::new(values), artboards: Vec::new(), existing: Vec::new(), first_key: 0, open: None }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> GuideLayoutOutput {
        let GuideLayoutDialog { units, document, values, artboards, existing, first_key, open } = self;
        let env = Env::of(ui);
        let words = env.words::<GuideWords>();
        let common = env.words::<CommonWords>();
        let preview_word = env.words::<ParamDialogWords>().preview.clone();
        let number_units = env.units();
        let reset = ResetFooter::new(ui, None, true);
        let accept = FooterButton::new(common.ok.clone(), DialogRole::Accept);
        let mut dialog = Dialog::new(words.layout_title.clone()).close_on_enter(true).outer_gesture(true).buttons(reset.buttons(accept, false));
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        let ask = std::cell::Cell::new(false);
        let valid = RefCell::new(None::<Vec<Guide>>);
        let last = RefCell::new(None::<Vec<Guide>>);
        let measure = Measure { words: &words, numbers: &number_units, units: &units };
        let form = Form { document: &document, artboards: &artboards, preview_word: &preview_word };
        let shown = run_form(ui, dialog, values, GuideLayoutValues::default(), Arc::from(words.layout_title.as_str()), &reset, |ui, cx, values| {
            if layout_rows(ui, &measure, &form, values) {
                ask.set(true);
            }
            let rects = targets(&values.options, &document, &artboards);
            let made = values.options.lines(&rects, &existing, &units, first_key, Some(values.color.color.rgba()));
            let remembered = cx.memory::<LastValid>();
            let ok = match made {
                Ok(guides) => {
                    remembered.0 = Some(guides.clone());
                    *valid.borrow_mut() = Some(guides);
                    true
                }
                Err(error) => {
                    *valid.borrow_mut() = None;
                    Notice::new(Tone::Bad, words.invalid.clone()).layout(NoticeLayout::Inline).show(ui);
                    Text::new(error.message(&words)).show(ui);
                    false
                }
            };
            *last.borrow_mut() = remembered.0.clone();
            ok
        });
        let Shown { output, values } = shown;
        let DialogOutput { response, closed, .. } = output;
        let result = accepted(closed).then(|| valid.borrow_mut().take()).flatten().map(|guides| GuideLayoutResult {
            options: values.options,
            guides,
            color: values.color.color.rgba(),
        });
        let preview = values.preview.then(|| last.into_inner()).flatten();
        let pick_color = ask.get().then(|| values.color.request(GUIDE_LAYOUT_COLOR_KEY, words.picker_title.clone()));
        GuideLayoutOutput { response, closed, result, preview, pick_color }
    }
}

/// The rectangles the layout fills for its target.
fn targets(options: &GuideLayoutOptions, document: &GuideRect, artboards: &[GuideRect]) -> Vec<GuideRect> {
    match options.target {
        GuideTarget::Document => vec![*document],
        GuideTarget::SelectedArtboards => artboards.to_vec(),
    }
}

/// The widths and heights percentages are of: the first target's.
fn extents(options: &GuideLayoutOptions, document: &GuideRect, artboards: &[GuideRect]) -> (f64, f64) {
    let rect = targets(options, document, artboards).first().copied().unwrap_or(*document);
    (rect.width(), rect.height())
}

/// The fields of New Guide Layout.
fn layout_rows(ui: &mut Ui, measure: &Measure<'_>, form: &Form<'_>, values: &mut GuideLayoutValues) -> bool {
    let words = measure.words;
    let before = values.options;
    let (across, down) = extents(&values.options, form.document, form.artboards);
    let mut asks = false;
    FieldGroup::new(words.layout_title.clone()).show(ui, |ui, _| {
        preset_row(ui, words, &mut values.options);
        target_row(ui, words, &mut values.options, !form.artboards.is_empty());
        asks = values.color.row(ui, words);
        group_box(ui, measure, &mut values.options.columns, GroupKind::Columns, across);
        group_box(ui, measure, &mut values.options.rows, GroupKind::Rows, down);
        margin_box(ui, measure, &mut values.options, across, down);
        let center = values.options.center_columns;
        if let Some(on) = edited(&Checkbox::new(Bind::given(center)).label(words.center_columns.clone()).show(ui)) {
            values.options.center_columns = on;
        }
        let clear = values.options.clear_existing;
        if let Some(on) = edited(&Checkbox::new(Bind::given(clear)).label(words.clear_existing.clone()).show(ui)) {
            values.options.clear_existing = on;
        }
        if let Some(on) = edited(&Checkbox::new(Bind::given(values.preview)).label(form.preview_word.clone()).show(ui)) {
            values.preview = on;
        }
    });
    if !values.options.same_fields(&before) && values.options.preset == before.preset {
        values.options.preset = GuideLayoutPreset::Custom;
    }
    asks
}

/// What the layout rows read besides the values.
struct Form<'a> {
    document: &'a GuideRect,
    artboards: &'a [GuideRect],
    preview_word: &'a Label,
}

fn preset_row(ui: &mut Ui, words: &GuideWords, options: &mut GuideLayoutOptions) {
    let entries: Vec<Entry<GuideLayoutPreset>> = GuideLayoutPreset::ALL.into_iter().map(|preset| Entry::new(preset, preset.label(words))).collect();
    let current = options.preset;
    Field::new(words.preset.clone()).show(ui, |ui, _| {
        if let Some(picked) = edited(&Select::new(words.preset.clone(), Bind::given(current), &entries).show(ui)) {
            picked.apply(options);
            options.preset = picked;
        }
    });
}

fn target_row(ui: &mut Ui, words: &GuideWords, options: &mut GuideLayoutOptions, artboards: bool) {
    let mut entries = vec![Entry::new(GuideTarget::Document, words.document.clone())];
    if artboards {
        entries.push(Entry::new(GuideTarget::SelectedArtboards, words.selected_artboards.clone()));
    }
    let current = options.target;
    Field::new(words.target.clone()).show(ui, |ui, _| {
        if let Some(picked) = edited(&Select::new(words.target.clone(), Bind::given(current), &entries).show(ui)) {
            options.target = picked;
        }
    });
}

#[derive(Clone, Copy, PartialEq)]
enum GroupKind {
    Columns,
    Rows,
}

fn group_box(ui: &mut Ui, measure: &Measure<'_>, group: &mut GuideGroup, kind: GroupKind, extent: f64) {
    let words = measure.words;
    let (title, size_label) = match kind {
        GroupKind::Columns => (words.columns.clone(), words.width.clone()),
        GroupKind::Rows => (words.rows.clone(), words.height.clone()),
    };
    let mut enabled = group.enabled;
    let on = group.enabled;
    GroupBox::new(title, Some(Bind::bound(&mut enabled))).fill(false).show(ui, |ui, _| {
        Field::new(words.number.clone()).enabled(on).show(ui, |ui, _| {
            let number = Some(f64::from(group.number));
            let field = NumberField::new(words.number.clone(), Bind::given(number)).range(1.0..=f64::from(MAX_COUNT)).step(1.0);
            if let Some(Some(value)) = edited(&field.show(ui).edit) {
                group.number = (value.round() as u32).clamp(1, MAX_COUNT);
            }
        });
        let ruler = measure.units.ruler;
        let shown = group.size.and_then(|size| measure.units.to_pixels(size, extent)).and_then(|px| measure.units.from_pixels(px, ruler, extent));
        Field::new(size_label.clone()).enabled(on).tooltip(words.automatic_size.clone()).show(ui, |ui, _| {
            if let Some(typed) = measure.input(ui, size_label.clone(), shown, extent) {
                group.size = typed.map(|value| GuideLength::new(value, ruler));
            }
        });
        let mut gutter = group.gutter;
        measure.length(ui, words.gutter.clone(), &mut gutter, extent);
        group.gutter = gutter;
    });
    group.enabled = enabled;
}

fn margin_box(ui: &mut Ui, measure: &Measure<'_>, options: &mut GuideLayoutOptions, across: f64, down: f64) {
    let words = measure.words;
    let sides = [words.top.clone(), words.left.clone(), words.bottom.clone(), words.right.clone()];
    let mut enabled = options.margin_enabled;
    GroupBox::new(words.margin.clone(), Some(Bind::bound(&mut enabled))).fill(false).show(ui, |ui, _| {
        for (index, label) in sides.iter().enumerate() {
            let extent = if index % 2 == 0 { down } else { across };
            let mut length = options.margin[index];
            measure.length(ui, label.clone(), &mut length, extent);
            options.margin[index] = length;
        }
    });
    options.margin_enabled = enabled;
}

/// A hosted New Guide Layout: the `DialogHost` owns the values and the app's colour answer goes into them.
pub struct HostedGuideLayoutDialog {
    id: DialogId,
    units: GuideUnits,
    document: GuideRect,
    artboards: Vec<GuideRect>,
    existing: Vec<Guide>,
    first_key: u64,
    values: GuideLayoutValues,
    preview: Option<Vec<Guide>>,
    requests: HostRequests,
}

impl HostedGuideLayoutDialog {
    /// A dialog for the `document` measured in `units`, opening with `values`.
    pub fn new(units: GuideUnits, document: GuideRect, values: GuideLayoutValues) -> Self {
        HostedGuideLayoutDialog {
            id: dialog_id("guides.layout"),
            units,
            document,
            artboards: Vec::new(),
            existing: Vec::new(),
            first_key: 0,
            values,
            preview: None,
            requests: HostRequests::new(),
        }
    }

    /// The selected artboards the Target list offers.
    #[must_use]
    pub fn artboards(mut self, artboards: Vec<GuideRect>) -> Self {
        self.artboards = artboards;
        self
    }

    /// The guides the document has.
    #[must_use]
    pub fn existing(mut self, existing: Vec<Guide>) -> Self {
        self.existing = existing;
        self
    }

    /// The key of the first guide the layout makes.
    #[must_use]
    pub fn first_key(mut self, first_key: u64) -> Self {
        self.first_key = first_key;
        self
    }

    /// The values the dialog shows.
    pub fn values(&self) -> &GuideLayoutValues {
        &self.values
    }

    /// The guides to draw on the canvas while the dialog is open, as of the last pass.
    pub fn preview(&self) -> Option<&[Guide]> {
        self.preview.as_deref()
    }

    /// The colour request, once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.requests.take_color()
    }

    /// The colour picker's answer.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == GUIDE_LAYOUT_COLOR_KEY {
            self.values.color.answer(color.to_rgba());
        }
    }
}

impl StackDialog for HostedGuideLayoutDialog {
    type Result = GuideLayoutResult;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<GuideLayoutResult>> {
        let shown = GuideLayoutDialog::new(self.units, self.document, Bind::bound(&mut self.values))
            .artboards(self.artboards.clone())
            .existing(self.existing.clone())
            .first_key(self.first_key)
            .show(ui);
        self.preview = shown.preview;
        if let Some(request) = shown.pick_color {
            self.requests.request_color(request);
        }
        match (shown.result, shown.closed) {
            (Some(result), _) => Some(DialogResult::Ok(result)),
            (None, Some(_)) => Some(DialogResult::Cancel),
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some("guides.layout")
    }
}

/// The most a guide's position can be from the origin, in pixels.
pub(super) const GUIDE_LIMIT: f64 = MAX_GUIDE_PX;
