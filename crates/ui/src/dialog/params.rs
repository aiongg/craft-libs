//! [`ParamDialog`]: a filter or command dialog built from a parameter list, with Preview and
//! Reset. [`Param`] and [`ParamKind`] are the list; [`CommandForm`](super::CommandForm) shares them.

use std::fmt;
use std::sync::Arc;

use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{ControlSize, Rgba, Srgba};
use egui::load::SizedTexture;
use egui::{Color32, Context, CornerRadius, Id, Label, Rect, Response, Sense, Ui, Vec2, pos2, vec2};

use super::color_request::ColorRequest;
use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::fields::{DialogFields, FieldValue};
use super::footer::{DialogLabels, Footer, Reset};
use super::size::{Body, Width};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{FieldControl as _, Form, FormGrid, SectionHeader};
use crate::controls::text_field::{BoxState, TextField, field_box};
use crate::controls::{
    AngleDial, Button, Checkbox, IconButton, NumberField, PointPicker, Select, Slider, SliderScale, Swatch, TooltipExt as _, checkerboard,
};
use crate::direction;
use crate::edit::{self, EditResponse};
use crate::icons::Icon;
use crate::localized::{Labels as _, LocaleNumbers, Unit, Units, focus_first_field};
use crate::theme::{Tokens, TokensOf};

/// What a parameter is and which control edits it. Defaults are the values a dialog opens with
/// when nothing is remembered and the host gave no [`values`](ParamDialog::values).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ParamKind {
    /// A slider with a number field. [`SliderScale::Log`] for ranges over three decades
    /// (PhotoCraft's rule for `0.1..1000`).
    Range { min: f64, max: f64, default: f64, scale: SliderScale },
    /// A whole-number field with steppers; `None` bounds are unbounded.
    Int { min: Option<i64>, max: Option<i64>, default: i64 },
    /// A bare [`NumberField`] without a slider (Photoshop's Merge to HDR Pro has sliders; a
    /// canvas size or a count does not). A `None` bound is open on that side. Scrub and arrow
    /// keys move by `step`; `decimals` is the most digits shown after the decimal mark, so an
    /// integer is `decimals: 0`. The field holds a [`FieldValue::Number`].
    Number { min: Option<f64>, max: Option<f64>, default: f64, step: f64, decimals: u32 },
    /// A check box.
    Bool { default: bool },
    /// A [`Select`] of `(id, label)` options; the field holds the id.
    Choice { options: Vec<(String, String)>, default: Option<String> },
    /// An [`AngleDial`] with its field, in degrees.
    Angle { default: f64 },
    /// A [`Swatch`] that opens the host's colour picker; `None`: unset
    /// until picked.
    Color { default: Option<[f32; 3]> },
    /// A grid of `cells` integer fields, [`side`](ParamKind::grid_side) per row (Custom filter:
    /// `int[25]` is 5×5). A `cells` that is not square wraps every `round(sqrt(cells))` fields
    /// and the last row is short, as PhotoCraft lays it out.
    Grid { cells: usize, default: i64 },
    /// A [`PointPicker`] over the preview image plus X and Y fields,
    /// in percent of the image.
    Point { default: [f64; 2] },
    /// One of the document's channels by index, shown by name.
    Channel { names: Vec<String>, default: usize },
    /// A [`TextField`].
    Text { default: String },
    /// A number that may be unset: a "Custom" check box switches the field on. Unset the key is
    /// absent from the [`DialogFields`] (PhotoCraft's Batch quality: `null` keeps the command's
    /// own default). `initial` is what the field shows the first time the box is checked;
    /// `decimals` is the most digits shown after the decimal mark (`0` for an integer, as in
    /// [`ParamKind::Number`]).
    OptionalNumber { min: f64, max: f64, default: Option<f64>, initial: f64, decimals: u32 },
    /// A row the host draws: see [`Param::custom`]. It has no value of its own.
    Custom(CustomRow),
    /// A section heading with a hairline over it: see [`Param::section`]. It has no value.
    Section,
}

/// What draws a [`ParamKind::Custom`] row: it gets the row's control cell, the dialog's values and
/// the row's [`CustomContext`], and returns the response the row label names and centres on.
/// Equal only to itself.
#[derive(Clone)]
pub struct CustomRow(Arc<CustomDraw>);

type CustomDraw = dyn Fn(&mut Ui, &mut DialogFields, &CustomContext) -> Response + Send + Sync;

impl CustomRow {
    pub fn new(draw: impl Fn(&mut Ui, &mut DialogFields, &CustomContext) -> Response + Send + Sync + 'static) -> Self {
        Self(Arc::new(draw))
    }
}

/// What a [`Param::custom`] row can ask of the host besides editing the values.
pub struct CustomContext {
    ctx: Context,
    dialog: DialogId,
    title: String,
    colors: DialogFields,
}

impl CustomContext {
    /// Asks the host for its colour picker for the value under `key`, which needs no row of its
    /// own. The picker opens at the colour the dialog holds under `key` (black without one), titled
    /// with the row's label. The request is reported once, as a Colour row's press is
    /// ([`ParamDialog::pick_color`], [`ParamRowsResponse::color`]); the host answers with
    /// [`ParamDialog::set_color`] for `key`.
    pub fn pick_color(&self, key: &str) {
        let color = Srgba::from_rgb_array(self.colors.color(key).unwrap_or([0.0; 3]));
        let request = ColorRequest::new(key, color).title(self.title.as_str());
        self.ctx.data_mut(|d| d.insert_temp(color_press_id(self.dialog), request));
    }
}

impl fmt::Debug for CustomRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CustomRow")
    }
}

impl PartialEq for CustomRow {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// What a text row's Browse button picks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PathKind {
    #[default]
    Folder,
    /// An existing file. `extensions` are the ones the file dialog offers, without the dot and in
    /// any letter case (`["cube", "3dl"]`); empty offers every file. The host passes them to its
    /// file dialog as one filter (craft-dialogs' `Filter`).
    OpenFile {
        extensions: Vec<String>,
    },
    SaveFile,
}

impl PathKind {
    /// An existing file of one of `extensions`.
    pub fn open_file(extensions: &[&str]) -> Self {
        Self::OpenFile { extensions: extensions.iter().map(|e| (*e).to_owned()).collect() }
    }
}

/// A pressed Browse button: the host opens its file dialog for `kind` and writes the answer with
/// `fields_mut().set(key, FieldValue::Text(path))`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathRequest {
    pub key: String,
    pub kind: PathKind,
}

/// The eyedropper toggle of a [`Param::eyedropper`] row after a press: the host arms its dropper
/// for the colour parameter `key` when `armed`, and disarms it otherwise; the picked colour is
/// written with [`ParamDialog::set_color`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EyedropperRequest {
    pub key: String,
    pub armed: bool,
}

impl ParamKind {
    /// Fields per row of a [`ParamKind::Grid`] of `cells`: `round(sqrt(cells))`, at least 1.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:973 (`Kind::Grid` draw).
    pub fn grid_side(cells: usize) -> usize {
        ((cells as f32).sqrt().round() as usize).max(1)
    }

    /// The value a field of this kind starts with.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:386 `default_value`.
    pub fn default_value(&self) -> Option<FieldValue> {
        Some(match self {
            ParamKind::Range { default, .. } | ParamKind::Angle { default } => FieldValue::Number(*default),
            ParamKind::Int { default, .. } => FieldValue::Int(*default),
            ParamKind::Number { default, .. } => FieldValue::Number(*default),
            ParamKind::Bool { default } => FieldValue::Bool(*default),
            ParamKind::Choice { options, default } => {
                let initial = default.as_deref().or_else(|| options.first().map(|(id, _)| id.as_str()));
                FieldValue::Text(initial.unwrap_or_default().to_owned())
            }
            ParamKind::Color { default } => FieldValue::Color((*default)?),
            ParamKind::Grid { cells: count, default } => {
                let mut cells = vec![*default; *count];
                // A grid that starts at 0 is a kernel: the identity, 1 in the centre.
                if *default == 0
                    && let Some(centre) = cells.get_mut(count / 2)
                {
                    *centre = 1;
                }
                FieldValue::Grid(cells)
            }
            ParamKind::Point { default } => FieldValue::Point(*default),
            ParamKind::Channel { default, .. } => FieldValue::Int(i64::try_from(*default).unwrap_or(0)),
            ParamKind::Text { default } => FieldValue::Text(default.clone()),
            ParamKind::OptionalNumber { default, .. } => FieldValue::Number((*default)?),
            ParamKind::Custom(_) | ParamKind::Section => return None,
        })
    }

    /// Whether `value` is one this kind can hold (a remembered value must, or it is dropped).
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:335 `rememberable`.
    pub fn fits(&self, value: &FieldValue) -> bool {
        let number = |value: &FieldValue| match value {
            FieldValue::Number(n) => Some(*n),
            FieldValue::Int(i) => Some(*i as f64),
            _ => None,
        };
        match (self, value) {
            (ParamKind::Range { min, max, .. }, v) => number(v).is_some_and(|n| n.is_finite() && n >= *min && n <= *max),
            (ParamKind::Angle { .. }, v) => number(v).is_some_and(f64::is_finite),
            (ParamKind::Int { min, max, .. }, FieldValue::Int(i)) => min.is_none_or(|m| *i >= m) && max.is_none_or(|m| *i <= m),
            (ParamKind::Number { min, max, .. }, v) => {
                number(v).is_some_and(|n| n.is_finite() && min.is_none_or(|m| n >= m) && max.is_none_or(|m| n <= m))
            }
            (ParamKind::Bool { .. }, FieldValue::Bool(_)) => true,
            (ParamKind::Choice { options, .. }, FieldValue::Text(id)) => options.iter().any(|(option, _)| option == id),
            (ParamKind::Color { .. }, FieldValue::Color(c)) => c.iter().all(|v| v.is_finite()),
            (ParamKind::Grid { cells: count, .. }, FieldValue::Grid(cells)) => cells.len() == *count,
            (ParamKind::Point { .. }, FieldValue::Point(p)) => p.iter().all(|v| v.is_finite()),
            (ParamKind::Channel { names, .. }, FieldValue::Int(i)) => usize::try_from(*i).is_ok_and(|i| i < names.len()),
            (ParamKind::Text { .. }, FieldValue::Text(_)) => true,
            (ParamKind::OptionalNumber { min, max, .. }, v) => number(v).is_some_and(|n| n.is_finite() && n >= *min && n <= *max),
            _ => false,
        }
    }
}

/// One row of a [`ParamDialog`] or [`CommandForm`](super::CommandForm).
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// The key in [`DialogFields`] and in the command the app runs (`"radius"`).
    pub key: String,
    /// The label without its colon, in English (`"Radius"`); the dialog translates it and adds the
    /// colon (a trailing colon given here is dropped first).
    pub label: String,
    pub kind: ParamKind,
    /// Shown after the number (`px`, `%`, `°`); formats through the locale.
    pub unit: Option<Unit>,
    /// The row takes input only while `fields[key]` equals the value (Replace Color's Range is
    /// enabled only when Localized Color Clusters is checked).
    pub enabled_when: Option<(String, FieldValue)>,
    /// The row is drawn only while `fields[key]` equals the value ([`Param::visible_when`]). A
    /// hidden row takes no space; its value stays in the fields and in the result.
    pub visible_when: Option<(String, FieldValue)>,
    /// A Browse button after a [`ParamKind::Text`] field.
    pub browse: Option<PathKind>,
    /// The hint and tooltip ([`Param::hint`], [`Param::tooltip`]), boxed so rows without them stay small.
    texts: Option<Box<RowTexts>>,
    /// The named options ([`Param::decimals`], [`Param::randomize`], [`Param::eyedropper`],
    /// [`Param::full_width`]), boxed for the same reason.
    options: Option<Box<RowOptions>>,
}

/// Options a row may carry beyond its kind.
#[derive(Clone, Debug, Default, PartialEq)]
struct RowOptions {
    range_decimals: Option<u32>,
    randomize: bool,
    eyedropper: bool,
    full_width: bool,
    units: Option<UnitChoices>,
}

/// The units of a number row ([`Param::number_with_units`]).
#[derive(Clone, Debug, PartialEq)]
struct UnitChoices {
    units: Vec<NumberUnit>,
    /// The field key the chosen unit's id is stored under.
    key: String,
    initial: usize,
}

/// One unit of a number row with units ([`Param::number_with_units`]).
#[derive(Clone, Debug, PartialEq)]
pub struct NumberUnit {
    /// What the dialog result carries as the chosen unit (`"ppi"`).
    pub id: String,
    /// The name in the unit menu, in English (`"pixels/inch"`); the dialog translates it.
    pub label: String,
    /// How many base units one of this unit is: the base value is the shown value times `factor`
    /// (pixels/cm is `2.54` over a base of pixels/inch). Must be positive and finite.
    pub factor: f64,
}

impl NumberUnit {
    /// A unit `id` named `label` that is `factor` base units.
    pub fn new(id: impl Into<String>, label: impl Into<String>, factor: f64) -> Self {
        Self { id: id.into(), label: label.into(), factor }
    }
}

/// Texts a row may carry beyond its label.
#[derive(Clone, Debug, Default, PartialEq)]
struct RowTexts {
    hint: Option<String>,
    tooltip: Option<String>,
}

impl Param {
    pub fn new(key: impl Into<String>, label: impl Into<String>, kind: ParamKind) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind,
            unit: None,
            enabled_when: None,
            visible_when: None,
            browse: None,
            texts: None,
            options: None,
        }
    }

    /// A bare number row: a [`ParamKind::Number`] open on both sides. Bound it with
    /// [`Param::bounds`].
    pub fn number(key: impl Into<String>, label: impl Into<String>, default: f64, step: f64, decimals: u32) -> Self {
        Self::new(key, label, ParamKind::Number { min: None, max: None, default, step, decimals })
    }

    /// Bounds a [`ParamKind::Number`] row (`None`: open on that side); other kinds are unchanged.
    pub fn bounds(mut self, lo: Option<f64>, hi: Option<f64>) -> Self {
        if let ParamKind::Number { min, max, .. } = &mut self.kind {
            (*min, *max) = (lo, hi);
        }
        self
    }

    /// A row the host draws (PhotoCraft's Zoom center picker, Indexed Color's color count). `draw`
    /// gets the control cell, the dialog's values and the row's [`CustomContext`] (to ask for the
    /// host's colour picker); the row label names the response it returns.
    /// The row has no default and nothing remembered; `draw` reads and writes
    /// [`DialogFields`] under whatever keys it likes.
    pub fn custom(
        key: impl Into<String>,
        label: impl Into<String>,
        draw: impl Fn(&mut Ui, &mut DialogFields, &CustomContext) -> Response + Send + Sync + 'static,
    ) -> Self {
        Self::new(key, label, ParamKind::Custom(CustomRow::new(draw)))
    }

    /// A section heading: the label in the form's section style (the bold title of a
    /// non-collapsible [`SectionHeader`]) under a hairline across the form. It has no value and
    /// no control, and the label takes no colon.
    pub fn section(label: impl Into<String>) -> Self {
        let label = label.into();
        Self::new(format!("section:{label}"), label, ParamKind::Section)
    }

    /// A number row with a unit menu after the field. `base` is the value in base units; the
    /// field shows `base / factor` of the chosen unit, and an edit stores `shown * factor`.
    /// The first of `units` is chosen at first (see [`Param::default_unit`]).
    ///
    /// The result holds the base-unit value under the row's key and the chosen unit's
    /// [`id`](NumberUnit::id) as text under [`Param::unit_key`] (`"{key}.unit"`).
    /// [`Param::bounds`] are in base units. Both keys are remembered with the dialog.
    pub fn number_with_units(key: impl Into<String>, label: impl Into<String>, base: f64, step: f64, decimals: u32, units: Vec<NumberUnit>) -> Self {
        let key = key.into();
        let mut param = Self::number(key.clone(), label, base, step, decimals);
        param.options.get_or_insert_default().units = Some(UnitChoices { units, key: format!("{key}.unit"), initial: 0 });
        param
    }

    /// The unit of a [`Param::number_with_units`] row that is chosen when the dialog opens; an id
    /// the row does not have leaves the first.
    pub fn default_unit(mut self, id: &str) -> Self {
        if let Some(choices) = self.options.get_or_insert_default().units.as_mut()
            && let Some(at) = choices.units.iter().position(|unit| unit.id == id)
        {
            choices.initial = at;
        }
        self
    }

    /// The key a [`Param::number_with_units`] row stores its chosen unit's id under: `"{key}.unit"`.
    pub fn unit_key(&self) -> Option<&str> {
        self.options.as_deref()?.units.as_ref().map(|units| units.key.as_str())
    }

    /// The row is drawn only while `fields[key]` equals `value`, as [`enabled_when`](Param::enabled_when)
    /// enables it; otherwise it takes no space. Its value stays in the fields and in the result.
    pub fn visible_when(mut self, key: impl Into<String>, value: impl Into<FieldValue>) -> Self {
        self.visible_when = Some((key.into(), value.into()));
        self
    }

    /// The row takes input only while `fields[key]` equals `value`; otherwise it is drawn disabled.
    pub fn enabled_when(mut self, key: impl Into<String>, value: impl Into<FieldValue>) -> Self {
        self.enabled_when = Some((key.into(), value.into()));
        self
    }

    /// A Browse button after the field of a [`ParamKind::Text`] row. The press is reported by
    /// [`ParamDialog::pick_path`] / [`CommandForm::pick_path`](super::CommandForm::pick_path).
    pub fn browse(mut self, kind: PathKind) -> Self {
        self.browse = Some(kind);
        self
    }

    /// The text shown while a [`ParamKind::Text`] field is empty (`{name}`); other kinds are
    /// unchanged.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.texts.get_or_insert_default().hint = Some(text.into());
        self
    }

    /// Hover text for the whole row, label and control, of any kind.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.texts.get_or_insert_default().tooltip = Some(text.into());
        self
    }

    /// The most digits shown after the decimal mark on a [`ParamKind::OptionalNumber`] or
    /// [`ParamKind::Range`] row (`0` for an integer such as Batch quality); other kinds are
    /// unchanged. A Range row stores its value rounded to these digits, and `0` stores a
    /// [`FieldValue::Int`].
    pub fn decimals(mut self, places: u32) -> Self {
        match &mut self.kind {
            ParamKind::OptionalNumber { decimals, .. } => *decimals = places,
            ParamKind::Range { .. } => self.options.get_or_insert_default().range_decimals = Some(places),
            _ => {}
        }
        self
    }

    /// A Random button after the field of a [`ParamKind::Int`] row (a seed): it sets a random
    /// value within the row's bounds (an open side is `0` or `i32::MAX`); other kinds are unchanged.
    pub fn randomize(mut self) -> Self {
        self.options.get_or_insert_default().randomize = true;
        self
    }

    /// A pipette toggle after the swatch of a [`ParamKind::Color`] row. A press is reported by
    /// [`ParamDialog::pick_eyedropper`] / [`CommandForm::pick_eyedropper`](super::CommandForm::pick_eyedropper)
    /// / [`ParamRowsResponse::eyedropper`], and the host arms or disarms its own dropper from it;
    /// other kinds are unchanged.
    pub fn eyedropper(mut self) -> Self {
        self.options.get_or_insert_default().eyedropper = true;
        self
    }

    /// For a [`Param::custom`] row: no label, and the body spans both columns of the form; other
    /// kinds are unchanged.
    pub fn full_width(mut self) -> Self {
        self.options.get_or_insert_default().full_width = true;
        self
    }

    /// The value the row opens with: its kind's default, stored by the same rule as an edit
    /// (a Range row's [`decimals`](Param::decimals)).
    pub(super) fn default_value(&self) -> Option<FieldValue> {
        match (&self.kind, self.option().range_decimals) {
            (ParamKind::Range { default, .. }, Some(places)) => Some(rounded(*default, Some(places))),
            _ => self.kind.default_value(),
        }
    }

    fn option(&self) -> RowOptions {
        self.options.as_deref().cloned().unwrap_or_default()
    }

    /// Whether `value` is one the field `key` of this row can hold: the row's own value, or the
    /// unit id of a row with units.
    fn accepts(&self, key: &str, value: &FieldValue) -> bool {
        if self.key == key {
            return self.kind.fits(value);
        }
        match (self.options.as_deref().and_then(|o| o.units.as_ref()), value) {
            (Some(units), FieldValue::Text(id)) => units.key == key && units.units.iter().any(|u| &u.id == id),
            _ => false,
        }
    }

    fn visible(&self, fields: &DialogFields) -> bool {
        self.visible_when.as_ref().is_none_or(|(key, value)| fields.get(key) == Some(value))
    }

    fn enabled(&self, fields: &DialogFields) -> bool {
        self.enabled_when.as_ref().is_none_or(|(key, value)| fields.get(key) == Some(value))
    }

    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Parses PhotoCraft's registry notation into a list, labels from the keys in Title Case with
    /// PhotoCraft's fixed names (`cellSize` is `Cell Size`, `bpc` is `Black Point Compensation`;
    /// `{"radius":0.1..1000=1,"method":"spin|zoom","monochromatic":bool,"seed":u32=0}`). Apps
    /// with their own command registry build [`Param`]s directly.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:55 `parse_spec` and :616 `source_label`.
    pub fn parse_list(spec: &str) -> Vec<Param> {
        split_fields(object_body(spec))
            .iter()
            .filter_map(|part| {
                let (key, value) = part.split_once(':')?;
                let key = key.trim().trim_matches('"');
                if key.is_empty() || key == "layer" {
                    return None;
                }
                let kind = parse_kind(value.trim())?;
                Some(Param::new(key, source_label(key), kind))
            })
            .collect()
    }

    /// Draws this parameter's row in a form: the label in the label column, the control in the
    /// field column, writing `fields[key]`. Shared by both dialogs.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:810 `body` (the match on `Kind`), with
    // the library controls in place of PhotoCraft's widgets.
    pub(super) fn row(&self, form: &mut Form<'_>, fields: &mut DialogFields, preview: Option<&SizedTexture>) {
        self.row_in(dialog_id("param-row"), form, fields, preview);
    }

    /// [`Param::row`] for a row of the dialog `dialog`: a colour swatch press is kept under it.
    pub(super) fn row_in(&self, dialog: DialogId, form: &mut Form<'_>, fields: &mut DialogFields, preview: Option<&SizedTexture>) {
        if !self.visible(fields) {
            return;
        }
        form.set_disabled(!self.enabled(fields));
        self.row_body(dialog, form, fields, preview);
        if let (Some(tip), Some(row)) = (self.texts.as_ref().and_then(|t| t.tooltip.as_ref()), form.last_row()) {
            let _ = row.clone().tooltip(translated(form.ctx(), tip));
        }
        form.set_disabled(false);
    }

    fn row_body(&self, dialog: DialogId, form: &mut Form<'_>, fields: &mut DialogFields, preview: Option<&SizedTexture>) {
        let ctx = form.ctx().clone();
        let numbers = LocaleNumbers::of_ctx(&ctx);
        let numbers = &*numbers;
        let name = translated(&ctx, self.label.trim_end_matches([':', '：']));
        let label = format!("{name}:");
        let key = self.key.as_str();
        let default = self.kind.default_value();
        let unit = &self.unit;
        match &self.kind {
            ParamKind::Range { min, max, scale, .. } => {
                let value = fields.number(key).or_else(|| number_of(default.as_ref())).unwrap_or(*min);
                let shown = self.option().range_decimals;
                let (places, step) = match shown {
                    Some(n) => (0..=n as usize, 10f64.powi(-(n as i32))),
                    None if (max - min).abs() <= 10.0 => (0..=2, 0.01),
                    None => (0..=1, 1.0),
                };
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let slider = Slider::new((key, "slider"), &name, value, *min..=*max).scale(*scale).show(ui);
                        let field = NumberField::new((key, "field"), &name, value, numbers).range(*min..=*max).step(step).decimals(places);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field).or_else(|| edited(&slider)) {
                            fields.set(key, rounded(v, shown));
                        }
                        (slider, field)
                    })
                    .inner
                });
            }
            ParamKind::Number { min, max, step, decimals, .. } if self.option().units.is_some() => {
                let choices = self.option().units.unwrap_or_else(|| unreachable!("guarded"));
                let base = fields.number(key).or_else(|| number_of(default.as_ref())).unwrap_or(0.0);
                let chosen = fields.text(&choices.key).and_then(|id| choices.units.iter().position(|u| u.id == id)).unwrap_or(choices.initial);
                let mut unit_id = choices.units.get(chosen).map(|u| u.id.clone()).unwrap_or_default();
                let factor = choices.units.get(chosen).map_or(1.0, |u| u.factor);
                let words = DialogLabels::of_ctx(&ctx);
                let range = min.map_or(f64::NEG_INFINITY, |m| m / factor)..=max.map_or(f64::INFINITY, |m| m / factor);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let places = 0..=*decimals as usize;
                        let field = NumberField::new((key, "field"), &name, base / factor, numbers).range(range).step(*step).decimals(places);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field) {
                            fields.set(key, FieldValue::Number(v * factor));
                        }
                        let mut select = Select::new((key, "unit"), words.unit.as_str(), &mut unit_id);
                        for unit in &choices.units {
                            select = select.option(unit.id.clone(), translated(&ctx, &unit.label));
                        }
                        let select = select.show(ui);
                        if let Some(v) = edited(&select) {
                            fields.set(choices.key.as_str(), FieldValue::Text(v));
                        }
                        (field, select)
                    })
                    .inner
                });
            }
            ParamKind::Number { min, max, step, decimals, .. } => {
                let value = fields.number(key).or_else(|| number_of(default.as_ref())).unwrap_or(0.0);
                let range = min.unwrap_or(f64::NEG_INFINITY)..=max.unwrap_or(f64::INFINITY);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let places = 0..=*decimals as usize;
                        let field = NumberField::new((key, "field"), &name, value, numbers).range(range).step(*step).decimals(places);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field) {
                            fields.set(key, FieldValue::Number(v));
                        }
                        field
                    })
                    .inner
                });
            }
            ParamKind::Int { min, max, .. } => {
                let value = fields.int(key).or_else(|| number_of(default.as_ref()).map(|n| n as i64)).unwrap_or(0);
                let range = min.map_or(f64::NEG_INFINITY, |m| m as f64)..=max.map_or(f64::INFINITY, |m| m as f64);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let field = NumberField::new((key, "field"), &name, value as f64, numbers).range(range).decimals(0..=0);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field) {
                            fields.set(key, FieldValue::Int(v.round() as i64));
                        }
                        let random = self.option().randomize.then(|| {
                            let words = DialogLabels::of_ctx(&ctx);
                            let button = ui.add(Button::new(words.randomize.as_str()).id_salt((key, "random")));
                            if button.clicked() {
                                let (lo, hi) = (min.unwrap_or(0), max.unwrap_or(i32::MAX as i64));
                                fields.set(key, FieldValue::Int(random_in(&ctx, lo, hi)));
                            }
                            button
                        });
                        (field, random)
                    })
                    .inner
                });
            }
            ParamKind::Bool { default } => {
                let mut value = fields.bool(key).unwrap_or(*default);
                let text = name.as_str();
                form.unlabelled(|ui| {
                    edit::scope(ui, |ui| {
                        let check = Checkbox::bool(text, &mut value).id_salt(key).show(ui);
                        if let Some(v) = edited(&check) {
                            fields.set(key, FieldValue::Bool(v));
                        }
                        check
                    })
                    .inner
                });
            }
            ParamKind::Choice { options, .. } => {
                let mut value = fields.text(key).map(str::to_owned).or_else(|| text_of(default.as_ref())).unwrap_or_default();
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let mut select = Select::new((key, "select"), &name, &mut value);
                        for (id, text) in options {
                            select = select.option(id.clone(), translated(&ctx, text));
                        }
                        let select = select.show(ui);
                        if let Some(v) = edited(&select) {
                            fields.set(key, FieldValue::Text(v));
                        }
                        select
                    })
                    .inner
                });
            }
            ParamKind::Angle { default } => {
                let value = fields.number(key).unwrap_or(*default);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let dial = AngleDial::new((key, "angle"), &name, value, numbers).show(ui);
                        if let Some(v) = edited(&dial) {
                            fields.set(key, FieldValue::Number(v));
                        }
                        dial
                    })
                    .inner
                });
            }
            ParamKind::Color { .. } => {
                let color = fields.color(key);
                let shown = color.map_or(Rgba::BLACK, |c| Rgba::rgb(encode(c[0]), encode(c[1]), encode(c[2])));
                form.row(label.as_str(), |ui, _| {
                    let swatch = ui.add(Swatch::new((key, "swatch"), &name, shown).mixed(color.is_none()));
                    if swatch.clicked() {
                        let request = ColorRequest::new(key, Srgba::from_rgb_array(color.unwrap_or([0.0; 3]))).title(&name);
                        ui.ctx().data_mut(|d| d.insert_temp(color_press_id(dialog), request));
                    }
                    if self.option().eyedropper {
                        let name = DialogLabels::of_ctx(&ctx).eyedropper.clone();
                        let state = eyedropper_state_id(dialog, key);
                        let mut armed = ui.ctx().data(|d| d.get_temp::<bool>(state)).unwrap_or(false);
                        let button = ui.add(IconButton::new(Icon::Eyedropper, name).selected(armed).id_salt((key, "eyedropper")));
                        if button.clicked() {
                            armed = !armed;
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(state, armed);
                                d.insert_temp(eyedropper_press_id(dialog), EyedropperRequest { key: key.to_owned(), armed });
                            });
                        }
                    }
                    swatch
                });
            }
            ParamKind::Grid { cells: count, .. } => {
                let side = ParamKind::grid_side(*count);
                let cells = match fields.grid(key) {
                    Some(cells) if cells.len() == *count => cells.to_vec(),
                    _ => match default {
                        Some(FieldValue::Grid(cells)) => cells,
                        _ => vec![0; *count],
                    },
                };
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let mut next = cells.clone();
                        let mut first = None;
                        ui.vertical(|ui| {
                            for row in 0..count.div_ceil(side) {
                                ui.horizontal(|ui| {
                                    for column in 0..side.min(count - row * side) {
                                        let i = row * side + column;
                                        let name = format!("{name} {}, {}", row + 1, column + 1);
                                        let cell = NumberField::new((key, i), name, cells[i] as f64, numbers).decimals(0..=0).show(ui);
                                        if let Some(v) = edited(&cell) {
                                            next[i] = v.round() as i64;
                                        }
                                        first.get_or_insert_with(|| cell.response.clone());
                                    }
                                });
                            }
                        });
                        if next != cells {
                            fields.set(key, FieldValue::Grid(next));
                        }
                        first.unwrap_or_else(|| ui.label(""))
                    })
                    .inner
                });
            }
            ParamKind::Point { default } => {
                let value = fields.point(key).unwrap_or(*default);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let picker = PointPicker::new((key, "point"), &name, value, preview.copied()).show(ui);
                        if let Some(v) = edited(&picker) {
                            fields.set(key, FieldValue::Point(v));
                        }
                        picker
                    })
                    .inner
                });
            }
            ParamKind::Channel { names, default } => {
                let value = fields.int(key).and_then(|i| usize::try_from(i).ok()).unwrap_or(*default);
                let mut index = value;
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let mut select = Select::new((key, "channel"), &name, &mut index);
                        for (i, name) in names.iter().enumerate() {
                            select = select.option(i, name.as_str());
                        }
                        let select = select.show(ui);
                        if let Some(v) = edited(&select) {
                            fields.set(key, FieldValue::Int(i64::try_from(v).unwrap_or(0)));
                        }
                        select
                    })
                    .inner
                });
            }
            ParamKind::Text { default } => {
                let value = fields.text(key).unwrap_or(default).to_owned();
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let mut text = TextField::new((key, "text"), &name, &value);
                        if let Some(hint) = self.texts.as_ref().and_then(|t| t.hint.as_ref()) {
                            text = text.placeholder(translated(&ctx, hint));
                        }
                        let text = text.show(ui);
                        if let Some(v) = edited(&text) {
                            fields.set(key, FieldValue::Text(v));
                        }
                        let browse = self.browse.clone().map(|kind| {
                            let words = DialogLabels::of_ctx(&ctx);
                            let button = ui.add(Button::new(words.browse.as_str()).id_salt((key, "browse")));
                            if button.clicked() {
                                let request = PathRequest { key: key.to_owned(), kind };
                                ui.ctx().data_mut(|d| d.insert_temp(path_press_id(dialog), request));
                            }
                            button
                        });
                        (text, browse)
                    })
                    .inner
                });
            }
            ParamKind::OptionalNumber { min, max, initial, decimals, .. } => {
                let stored = fields.number(key);
                let last_id = Id::unique(("craft_ui::dialog::params::optional_last", dialog, key));
                let last = stored.or_else(|| ctx.data(|d| d.get_temp::<f64>(last_id))).unwrap_or(*initial);
                let mut on = stored.is_some();
                let words = DialogLabels::of_ctx(&ctx);
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let check = Checkbox::bool(words.custom.as_str(), &mut on).id_salt((key, "custom")).show(ui);
                        match edited(&check) {
                            Some(true) => fields.set(key, FieldValue::Number(last)),
                            Some(false) => {
                                ui.ctx().data_mut(|d| d.insert_temp(last_id, last));
                                fields.remove(key);
                            }
                            None => {}
                        }
                        let field = ui
                            .add_enabled_ui(fields.number(key).is_some(), |ui| {
                                let field =
                                    NumberField::new((key, "field"), &name, last, numbers).range(*min..=*max).decimals(0..=*decimals as usize);
                                with_unit(field, unit).show(ui)
                            })
                            .inner;
                        if let Some(v) = edited(&field) {
                            fields.set(key, FieldValue::Number(v));
                        }
                        (check.own_name(), field)
                    })
                    .inner
                });
            }
            ParamKind::Section => {
                form.spanning(|ui| SectionHeader::new((key, "section"), name.as_str()).collapsible(false).show(ui, |_| {}).response);
            }
            ParamKind::Custom(row) => {
                let custom = CustomContext { ctx: ctx.clone(), dialog, title: name.clone(), colors: fields.clone() };
                if self.option().full_width {
                    form.spanning(|ui| (row.0)(ui, fields, &custom));
                } else {
                    form.row(label.as_str(), |ui, _| (row.0)(ui, fields, &custom));
                }
            }
        }
    }
}

/// `value` rounded to `places` digits (`None`: as is); no digits stores an integer.
fn rounded(value: f64, places: Option<u32>) -> FieldValue {
    match places {
        None => FieldValue::Number(value),
        Some(0) => FieldValue::Int(value.round() as i64),
        Some(n) => {
            let scale = 10f64.powi(n as i32);
            FieldValue::Number((value * scale).round() / scale)
        }
    }
}

/// A pseudo-random integer in `lo..=hi` (SplitMix64 over the frame time and a per-context
/// counter), for a Random button; a seed needs no better source.
fn random_in(ctx: &Context, lo: i64, hi: i64) -> i64 {
    let counter_id = Id::unique("craft_ui::dialog::params::random");
    let count = ctx.data_mut(|d| {
        let count = d.get_temp::<u64>(counter_id).unwrap_or(0).wrapping_add(1);
        d.insert_temp(counter_id, count);
        count
    });
    let mut z = ctx.input(|i| i.time).to_bits() ^ count.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let span = (hi as i128 - lo as i128 + 1) as u128;
    (lo as i128 + (z as u128 % span) as i128) as i64
}

fn with_unit<'a>(field: NumberField<'a>, unit: &Option<Unit>) -> NumberField<'a> {
    match unit {
        Some(unit) => field.unit(unit.clone()),
        None => field,
    }
}

/// The value a control's event carries (inside [`edit::scope`] every event is an update).
fn edited<T: Clone>(response: &EditResponse<T>) -> Option<T> {
    response.event.as_ref().and_then(|event| event.value().cloned())
}

fn number_of(value: Option<&FieldValue>) -> Option<f64> {
    match value? {
        FieldValue::Number(n) => Some(*n),
        FieldValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}

fn text_of(value: Option<&FieldValue>) -> Option<String> {
    match value? {
        FieldValue::Text(t) => Some(t.clone()),
        _ => None,
    }
}

/// An sRGB component in 0..=1 as a byte.
pub(super) fn encode(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// `text` in the installed Host's language: the catalog row of exactly this text, else the text.
/// A parameter's label and a choice's option label are runtime strings (the English fallback of
/// [`Param::parse_list`], or the app's own words), which `Words` cannot look up (its words are
/// `&'static`), so this reads the Host's `I18n` with the call `Words::from_i18n` makes for a word
/// looked up by its English text.
pub fn translated(ctx: &Context, text: &str) -> String {
    match Host::try_of_ctx(ctx) {
        Some(host) => host.i18n().map_or(text, |i18n| i18n.t(host.lang(), text)).to_owned(),
        None => text.to_owned(),
    }
}

fn color_press_id(dialog: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::color_press", dialog))
}

/// The colour request of the parameter whose swatch of `dialog` was pressed this pass. Keyed by
/// dialog, so a parent and its child form never take each other's press.
pub(super) fn take_color_press(ctx: &Context, dialog: DialogId) -> Option<ColorRequest> {
    ctx.data_mut(|d| {
        let id = color_press_id(dialog);
        let request = d.get_temp::<ColorRequest>(id);
        d.remove::<ColorRequest>(id);
        request
    })
}

/// Writes `color` into the colour parameter `key` of `params`, or into a key no parameter has when
/// a [`Param::custom`] row can have asked for it; other keys are ignored.
pub(super) fn set_color_field(params: &[Param], fields: &mut DialogFields, key: &str, color: Srgba) {
    let colour_row = params.iter().any(|param| param.key == key && matches!(param.kind, ParamKind::Color { .. }));
    let custom_request = params.iter().any(|param| matches!(param.kind, ParamKind::Custom(_))) && params.iter().all(|param| param.key != key);
    if colour_row || custom_request {
        fields.set(key, FieldValue::Color(color.to_rgb_array()));
    }
}

fn eyedropper_press_id(dialog: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::eyedropper_press", dialog))
}

fn eyedropper_state_id(dialog: DialogId, key: &str) -> Id {
    Id::unique(("craft_ui::dialog::params::eyedropper_state", dialog, key))
}

/// The eyedropper toggle's press, once.
pub(super) fn take_eyedropper_press(ctx: &Context, dialog: DialogId) -> Option<EyedropperRequest> {
    ctx.data_mut(|d| d.remove_temp::<EyedropperRequest>(eyedropper_press_id(dialog)))
}

/// Sets the eyedropper toggle of the Color row `key` of `dialog` (the host disarms it when its
/// dropper ends).
pub(super) fn set_eyedropper_state(ctx: &Context, dialog: DialogId, key: &str, armed: bool) {
    ctx.data_mut(|d| d.insert_temp(eyedropper_state_id(dialog, key), armed));
}

fn path_press_id(dialog: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::path_press", dialog))
}

/// The Browse button of `dialog` pressed this pass.
pub(super) fn take_path_press(ctx: &Context, dialog: DialogId) -> Option<PathRequest> {
    ctx.data_mut(|d| d.remove_temp::<PathRequest>(path_press_id(dialog)))
}

/// Fills the keys `params` have defaults for and the fields lack, and takes the opening snapshot.
pub(super) fn open_fields(ctx: &Context, id: DialogId, params: &[Param], fields: &mut DialogFields) {
    for param in params {
        if !fields.contains(&param.key)
            && let Some(value) = param.default_value()
        {
            fields.set(param.key.as_str(), value);
        }
        if let Some(units) = param.options.as_deref().and_then(|o| o.units.as_ref())
            && !fields.contains(&units.key)
            && let Some(unit) = units.units.get(units.initial)
        {
            fields.set(units.key.as_str(), FieldValue::Text(unit.id.clone()));
        }
    }
    fields.snapshot();
    ctx.data_mut(|d| d.insert_temp(first_field_due_id(id), true));
}

fn first_field_due_id(id: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::first_field_due", id))
}

/// Draws a body so that the first number field it draws, on the first laid-out pass after the
/// dialog opened, takes the focus with its text selected: typing replaces it, Enter applies it.
// pc: crates/ui-egui/src/dialogs.rs:156 `first_field_focused` (used by `filter_body` :404 and the
// command forms), whichever parameter comes first.
pub(super) fn first_field_focused<R>(ui: &mut Ui, id: DialogId, body: impl FnOnce(&mut Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let due = !ui.is_sizing_pass() && ctx.data_mut(|d| d.remove_temp::<bool>(first_field_due_id(id))).unwrap_or(false);
    focus_first_field(&ctx, due);
    let result = body(ui);
    focus_first_field(&ctx, false);
    result
}

/// Draws one row per parameter.
pub(super) fn draw_rows(params: &[Param], dialog: DialogId, form: &mut Form<'_>, fields: &mut DialogFields, preview: Option<&SizedTexture>) {
    for param in params {
        param.row_in(dialog, form, fields, preview);
    }
}

/// What [`ParamRows::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ParamRowsResponse {
    /// The rows' form.
    pub response: Response,
    /// A row changed `fields` this pass (each pass of a slider drag, a keystroke that parsed).
    pub changed: bool,
    /// The colour parameter whose swatch was pressed this pass: the host opens its colour picker
    /// for the request and writes the answer into the request's key as a
    /// [`FieldValue::Color`].
    pub color: Option<ColorRequest>,
    /// The Browse button pressed this pass: the host picks a path and writes it into the
    /// request's key as a [`FieldValue::Text`].
    pub path: Option<PathRequest>,
    /// The eyedropper toggle pressed this pass ([`Param::eyedropper`]): the host arms or disarms
    /// its dropper for the request's key.
    pub eyedropper: Option<EyedropperRequest>,
}

/// The rows of a parameter list outside a [`ParamDialog`]: one form row per [`Param`], as the
/// dialog draws them, in a panel the host lays out (Filter Gallery's effect settings in its side
/// column).
///
/// ```ignore
/// let rows = ParamRows::new("gallery-settings", catalog.params(effect.filter)).show(ui, &mut effect.fields);
/// if let Some(request) = rows.color { app.pick_color(request) }
/// ```
///
/// - Every [`ParamKind`] draws as in [`ParamDialog`] (labels translated through the host's
///   `I18n`, [`enabled_when`](Param::enabled_when), hints and tooltips), in a
///   [`FormGrid`] whose label column fits the panel.
/// - Values live in the host's [`DialogFields`]; the rows fill a missing key with its parameter's
///   default before drawing it. The host keeps its own history of `fields` (a
///   [`LocalHistory`](super::LocalHistory) settled while no drag is in progress) and its own
///   Reset.
/// - No first-field focus: a panel opens without taking the keyboard.
#[must_use = "show the rows with `show`"]
pub struct ParamRows<'a> {
    id_salt: egui::IdSalt,
    params: &'a [Param],
    preview: Option<SizedTexture>,
}

impl<'a> ParamRows<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, params: &'a [Param]) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), params, preview: None }
    }

    /// The image a [`ParamKind::Point`] row picks over.
    pub fn preview(mut self, image: SizedTexture) -> Self {
        self.preview = Some(image);
        self
    }

    /// Draws the rows over `fields`.
    pub fn show(self, ui: &mut Ui, fields: &mut DialogFields) -> ParamRowsResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:782-786 (the selected effect's parameters
        // drawn from its spec in a scrolling column).
        let id = ui.make_persistent_id(self.id_salt);
        for param in self.params {
            if !fields.contains(&param.key)
                && let Some(value) = param.default_value()
            {
                fields.set(param.key.as_str(), value);
            }
        }
        let before = fields.clone();
        let preview = self.preview.as_ref();
        let shown = FormGrid::new(id).show(ui, |form| draw_rows(self.params, id, form, fields, preview));
        let ctx = ui.ctx();
        ParamRowsResponse {
            response: shown.response,
            changed: *fields != before,
            color: take_color_press(ctx, id),
            path: take_path_press(ctx, id),
            eyedropper: take_eyedropper_press(ctx, id),
        }
    }
}

/// PhotoCraft's `uses_logarithmic_slider`: a range whose max is over 500 times its min.
// pc: crates/ui-egui/src/dialogs/filter_dialog.rs:652.
const LOG_SLIDER_RATIO: f64 = 500.0;

/// The kind one `"key":value` entry of the notation names. Values the dialog does not edit
/// (documents, structured JSON, references) are `None`.
fn parse_kind(v: &str) -> Option<ParamKind> {
    if let Some(choices) = v.strip_prefix('"') {
        let (body, rest) = choices.split_once('"').unwrap_or((choices, ""));
        let ids: Vec<&str> = body.split('|').collect();
        let default = rest.split_once('=').and_then(|(_, def)| {
            let def = def.trim();
            let def = if let Some(quoted) = def.strip_prefix('"') {
                quoted.split('"').next().unwrap_or_default()
            } else if let Some(quoted) = def.strip_prefix('\'') {
                quoted.split('\'').next().unwrap_or_default()
            } else {
                def.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ',' | '}')).next().unwrap_or_default()
            };
            ids.contains(&def).then(|| def.to_owned())
        });
        let options = ids.iter().map(|id| ((*id).to_owned(), source_label(id))).collect();
        Some(ParamKind::Choice { options, default })
    } else if let Some(rest) = v.strip_prefix("bool") {
        Some(ParamKind::Bool { default: rest.trim_start_matches('=').trim() == "true" })
    } else if v == "text" {
        Some(ParamKind::Text { default: String::new() })
    } else if v == "color" {
        Some(ParamKind::Color { default: None })
    } else if let Some(hex) = v.strip_prefix("color=") {
        Some(ParamKind::Color { default: Srgba::from_hex(hex).map(Srgba::to_rgb_array) })
    } else if v == "doc" || v == "json" || v.starts_with("layer id") || v.starts_with(['[', '{']) || v.trim_end_matches('?') == "index|name" {
        None
    } else if let Some(cells) = v.strip_prefix("int[").and_then(|r| r.strip_suffix(']')).and_then(|n| n.parse::<usize>().ok()) {
        Some(ParamKind::Grid { cells, default: 0 })
    } else {
        let (range, default) = v.split_once('=').map_or((v, ""), |(a, b)| (a, b.trim()));
        if let Some((lo, hi)) = range.split_once("..") {
            let min = leading_number(lo).unwrap_or(0.0);
            let max = leading_number(hi).unwrap_or(100.0);
            let default = default.parse().unwrap_or(min);
            // Over `LOG_SLIDER_RATIO` a linear track leaves the small values no room.
            let scale = if min > 0.0 && max / min > LOG_SLIDER_RATIO { SliderScale::Log } else { SliderScale::Linear };
            Some(ParamKind::Range { min, max, default, scale })
        } else {
            Some(ParamKind::Int { min: None, max: None, default: default.parse().unwrap_or(0) })
        }
    }
}

/// Splits on the commas that separate entries: not inside a string or brackets.
fn split_fields(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let (mut in_str, mut depth) = (false, 0i32);
    for ch in inner.chars() {
        if ch == '"' {
            in_str = !in_str;
        }
        if !in_str {
            match ch {
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        if ch == ',' && !in_str && depth == 0 {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

/// The number a range bound starts with, ignoring a unit or marker after it: `5000 px?` is 5000,
/// `1000 %` is 1000.
fn leading_number(bound: &str) -> Option<f64> {
    let b = bound.trim();
    if let Ok(v) = b.parse() {
        return Some(v);
    }
    let end = b.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))).unwrap_or(b.len());
    b.get(..end)?.parse().ok()
}

/// The inside of the spec's leading `{...}` object. Notes after it (`-> {...}` results,
/// parenthesised remarks) are documentation, not parameters.
fn object_body(spec: &str) -> &str {
    let s = spec.trim();
    let Some(body) = s.strip_prefix('{') else { return s };
    let (mut depth, mut in_str) = (0i32, false);
    for (i, ch) in body.char_indices() {
        match ch {
            '"' => in_str = !in_str,
            '[' | '{' if !in_str => depth += 1,
            ']' | '}' if !in_str => {
                if depth == 0 {
                    return body.get(..i).unwrap_or(body);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    body.trim_end_matches('}')
}

/// The English label of a key, as PhotoCraft's filter dialogs write it: camelCase read as Title
/// Case words (`cellSize` is `Cell Size`), two fixed names, and a key with a space is already text.
// pc: crates/ui-egui/src/dialogs/filter_dialog.rs:616 `source_label`.
fn source_label(key: &str) -> String {
    match key {
        // Photoshop's name for the localized colour matching (Replace Color, Color Range).
        "localized" => return "Localized Color Clusters".into(),
        "bpc" => return "Black Point Compensation".into(),
        _ => {}
    }
    if key.contains(' ') {
        return key.to_owned();
    }
    let mut out = String::new();
    for (i, c) in key.chars().enumerate() {
        if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            if c.is_uppercase() {
                out.push(' ');
            }
            out.push(c);
        }
    }
    out
}

/// Where the footer sits (spec question 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamLayout {
    /// OK, Cancel and the Preview check box in a column at the top right, the parameters at the
    /// left, the preview pane under them: Photoshop's filter dialogs.
    SideColumn,
    /// Parameters, then the preview pane, then a bottom footer: PhotoCraft today.
    BottomFooter,
}

/// A dialog from a parameter list: one row per [`Param`], Preview with hold-to-compare, Reset on
/// Alt+Cancel, last-used values remembered per dialog, every value reachable by key.
///
/// ```ignore
/// let params = vec![
///     Param::new("radius", "Radius", ParamKind::Range { min: 0.1, max: 1000.0, default: 1.0, scale: SliderScale::Log }).unit(Unit::Pixels),
///     Param::new("method", "Blur Method", ParamKind::Choice { options: vec![("spin".into(), "Spin".into()), ("zoom".into(), "Zoom".into())], default: None }),
/// ];
/// dialogs.open(ParamDialog::new("radial-blur", "Radial Blur", params).remember("filter.radialBlur").preview(true));
/// // each pass, while open:
/// if let Some(dialog) = dialogs.get_mut::<ParamDialog>() {
///     if dialog.preview_on() { dialog.set_preview_image(render(dialog.fields())); }
/// }
/// if let Some(DialogResult::Ok(done)) = dialogs.take::<ParamDialog>() { run(done.key(), done.fields); }
/// ```
///
/// - The user sees the title, the rows, the preview pane (when the host supplies an image), the
///   Preview check box and the footer. Tab walks the fields only; Enter is OK unless a field owns
///   it; Esc cancels; Alt turns Cancel into Reset, which restores the opening values and keeps
///   the dialog open. The first field opens selected, so typing replaces it (Photoshop).
/// - Preview: the host renders from [`fields`](ParamDialog::fields) while
///   [`preview_on`](ParamDialog::preview_on) and hands the result to
///   [`set_preview_image`](ParamDialog::set_preview_image); holding the check box's label shows
///   the unfiltered image ([`Preview`](super::Preview)).
/// - Result: a [`ParamResult`], the dialog's [`key`](ParamDialog::key) with the [`DialogFields`] at OK. The remembered copy is stored on OK only; Cancel keeps
///   the previous one (PhotoCraft).
pub struct ParamDialog {
    id: DialogId,
    instance: Option<DialogId>,
    key: String,
    title: String,
    params: Vec<Param>,
    fields: DialogFields,
    layout: Option<ParamLayout>,
    width: Width,
    preview: bool,
    preview_image: Option<SizedTexture>,
    remember_key: Option<String>,
    picked_color: Option<ColorRequest>,
    picked_path: Option<PathRequest>,
    picked_eyedropper: Option<EyedropperRequest>,
    below: Option<Below>,
    reset_target: Option<DialogFields>,
    ok_enabled: bool,
    history: bool,
    close_label: Option<Option<String>>,
    footer_extra: Option<FooterExtra>,
    state: DialogState<DialogFields>,
}

/// What a dialog draws before its buttons ([`ParamDialog::footer_extra`]).
/// A section drawn under the rows ([`ParamDialog::below`]).
type Below = Box<dyn Fn(&mut Ui, &DialogFields) + Send>;

pub(super) type FooterExtra = Box<dyn FnMut(&mut Ui) + Send>;

impl ParamDialog {
    /// A dialog for `params`, opening with their defaults. `key` names the dialog for the stack
    /// and for placement memory (one per command, as PhotoCraft places them).
    pub fn new(key: impl Into<String>, title: impl Into<String>, params: Vec<Param>) -> Self {
        let key = key.into();
        Self {
            id: dialog_id(key.as_str()),
            instance: None,
            key,
            title: title.into(),
            params,
            fields: DialogFields::new(),
            layout: None,
            width: Width::Standard,
            preview: false,
            preview_image: None,
            remember_key: None,
            picked_color: None,
            picked_path: None,
            picked_eyedropper: None,
            below: None,
            reset_target: None,
            ok_enabled: true,
            history: false,
            close_label: None,
            footer_extra: None,
            state: DialogState::new(),
        }
    }

    /// The key the dialog was made with: which command it answers for. The result carries it.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Names this dialog among others made with the same `key`. The dialog's window, focus,
    /// colour and path requests are kept under its id, made from the key and `instance`, so two
    /// dialogs of one command (one per document, say) keep separate state. Without it the id
    /// comes from the key alone. A [`DialogHost`](super::DialogHost) keeps dialogs of different
    /// instances open together and replaces only one of the same instance; reach one with its
    /// `_by_id` calls and the id [`open`](super::DialogHost::open) returns.
    pub fn instance(mut self, instance: impl std::hash::Hash + fmt::Debug) -> Self {
        self.id = dialog_id((self.key.as_str(), instance));
        self.instance = Some(self.id);
        self
    }

    /// Opening values that override the defaults (a document's current setting). Remembered
    /// values, when any, come after these.
    pub fn values(mut self, values: DialogFields) -> Self {
        self.fields = values;
        self
    }

    /// Remembers the values under `key` in the app's settings (craft-settings `Remembered`) on OK
    /// and opens with them next time, dropping the ones that no longer fit the parameters
    /// (spec question 2).
    pub fn remember(mut self, key: impl Into<String>) -> Self {
        self.remember_key = Some(key.into());
        self
    }

    /// Whether OK can be pressed; for a dialog already open use
    /// [`set_ok_enabled`](ParamDialog::set_ok_enabled). Enter does nothing while it is off.
    pub fn ok_enabled(mut self, enabled: bool) -> Self {
        self.ok_enabled = enabled;
        self
    }

    /// [`ok_enabled`](ParamDialog::ok_enabled) for a dialog already open: the host updates it
    /// through [`DialogHost::get_mut`](super::DialogHost::get_mut) when it depends on the values
    /// (a filter that needs an input).
    pub fn set_ok_enabled(&mut self, enabled: bool) {
        self.ok_enabled = enabled;
    }

    /// The values Alt+Reset sets (PhotoCraft's `reset_fields`: schema defaults, the foreground
    /// color), instead of the values from when the dialog opened. Keys the target lacks keep their
    /// value. The host may call [`set_reset_to`](ParamDialog::set_reset_to) while the dialog is open.
    pub fn reset_to(mut self, target: DialogFields) -> Self {
        self.reset_target = Some(target);
        self
    }

    /// [`reset_to`](ParamDialog::reset_to) for a dialog already open.
    pub fn set_reset_to(&mut self, target: DialogFields) {
        self.reset_target = Some(target);
    }

    /// Cmd/Ctrl+Z steps the dialog's own settings ([`Dialog::history`]).
    pub fn history(mut self, history: bool) -> Self {
        self.history = history;
        self
    }

    /// The close box's accessible name; `None` removes the box ([`Dialog::close_label`]).
    pub fn close_label(mut self, label: Option<String>) -> Self {
        self.close_label = Some(label);
        self
    }

    /// Controls before the buttons in the [`ParamLayout::BottomFooter`] row (Color Table's Load
    /// and Save). The side column has no such row and ignores it.
    pub fn footer_extra(mut self, add: impl FnMut(&mut Ui) + Send + 'static) -> Self {
        self.footer_extra = Some(Box::new(add));
        self
    }

    /// Shows the Preview check box and the preview pane, on by default.
    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// Where the footer sits. Without it a dialog with [`preview`](ParamDialog::preview) uses
    /// [`ParamLayout::SideColumn`] and one without uses [`ParamLayout::BottomFooter`].
    pub fn layout(mut self, layout: ParamLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    /// The current values; what a script reads and what the host renders the preview from.
    pub fn fields(&self) -> &DialogFields {
        &self.fields
    }

    /// The current values, for a script to set.
    pub fn fields_mut(&mut self) -> &mut DialogFields {
        &mut self.fields
    }

    /// Whether the host should render the result onto the canvas now: the Preview check box is on.
    /// Holding the box's label to compare does not change it; it shows the unfiltered image in the
    /// dialog's pane only ([`comparing`](ParamDialog::comparing)).
    pub fn preview_on(&self) -> bool {
        self.preview && self.state.preview().on
    }

    /// Whether the Preview label is held to compare: the host shows the unfiltered image in the
    /// preview pane ([`set_preview_image`](ParamDialog::set_preview_image)) while it is.
    pub fn comparing(&self) -> bool {
        self.preview && self.state.preview().on && self.state.preview().compare
    }

    /// Turns the Preview check box on or off, as the user clicking it does: the state
    /// [`preview_on`](ParamDialog::preview_on) reads and the box shows. Takes effect before or
    /// after the dialog opens; it returns to on when the dialog closes.
    pub fn set_preview(&mut self, on: bool) {
        self.state.preview_mut().on = on;
    }

    /// The image the preview pane shows this pass; `None` leaves the last one.
    pub fn set_preview_image(&mut self, image: Option<SizedTexture>) {
        if image.is_some() {
            self.preview_image = image;
        }
    }

    /// The rows, for a body a host draws itself in the same form.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:810 `body`.
    pub fn rows(&mut self, form: &mut Form<'_>) {
        draw_rows(&self.params, self.id, form, &mut self.fields, self.preview_image.as_ref());
    }

    /// The colour parameter whose swatch the user pressed, once: the host opens its colour picker
    /// for the request (key = the parameter's key, title = its label, colour = the value shown) and
    /// answers with [`set_color`](Self::set_color).
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.picked_color.take()
    }

    /// The colour picker's answer for the parameter `key`, or for a key a [`Param::custom`] row
    /// asked for with [`CustomContext::pick_color`]; other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        set_color_field(&self.params, &mut self.fields, key, color);
    }

    /// A note or section under the rows that only reads the values (Color Settings' monitor
    /// note), as [`CommandForm::below`](super::CommandForm::below).
    pub fn below(mut self, draw: impl Fn(&mut Ui, &DialogFields) + Send + 'static) -> Self {
        self.below = Some(Box::new(draw));
        self
    }

    /// The eyedropper toggle of a [`Param::eyedropper`] row the user pressed, once: the host arms
    /// its dropper for the request's key when `armed` and disarms it otherwise, and answers a
    /// pick with [`set_color`](Self::set_color).
    pub fn pick_eyedropper(&mut self) -> Option<EyedropperRequest> {
        self.picked_eyedropper.take()
    }

    /// Pops the eyedropper toggle of the row `key` back up, once the host's dropper has ended.
    pub fn set_eyedropper(&self, ctx: &Context, key: &str, armed: bool) {
        set_eyedropper_state(ctx, self.id, key, armed);
    }

    /// The Browse button of a [`Param::browse`] row the user pressed, once: the host opens its
    /// file dialog (for [`PathKind::OpenFile`], with the extensions the request names) and writes the path with `fields_mut().set(key, FieldValue::Text(..))`.
    pub fn pick_path(&mut self) -> Option<PathRequest> {
        self.picked_path.take()
    }

    /// Loads the remembered values (if `remember` was set and the Host has them) and takes the
    /// opening snapshot; the host calls it on the first pass.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:345 `restore_remembered`.
    fn open(&mut self, ctx: &Context) {
        if let (Some(key), Some(host)) = (&self.remember_key, Host::try_of_ctx(ctx))
            && let Some(mut saved) = Remembered::<DialogFields>::new(key.as_str()).recall(&host)
        {
            let keys = self.remembered_keys();
            saved.retain_fitting(&keys, |key, value| self.params.iter().any(|p| p.accepts(key, value)));
            for (key, value) in saved.iter() {
                self.fields.set(key, value.clone());
            }
        }
        open_fields(ctx, self.id, &self.params, &mut self.fields);
        ctx.data_mut(|d| {
            d.remove::<f32>(preview_zoom_id(self.id));
            d.remove::<Vec2>(preview_pan_id(self.id));
        });
    }

    /// The keys whose values are remembered: PhotoCraft keeps the portable kinds (numbers, choices,
    /// check boxes) and drops colours, grids, points, channels and free text.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:335 `rememberable` (`Range`, `Choice`, `Bool`
    // and `Int` are kept; its angles are ranges).
    fn remembered_keys(&self) -> Vec<&str> {
        let kept = |kind: &ParamKind| {
            matches!(
                kind,
                ParamKind::Range { .. }
                    | ParamKind::Angle { .. }
                    | ParamKind::Int { .. }
                    | ParamKind::Number { .. }
                    | ParamKind::Bool { .. }
                    | ParamKind::Choice { .. }
            )
        };
        let units = self.params.iter().filter_map(Param::unit_key);
        self.params.iter().filter(|p| kept(&p.kind)).map(|p| p.key.as_str()).chain(units).collect()
    }

    /// Stores the values under the remember key.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:363 `remember`.
    fn store(&self, ctx: &Context) {
        let (Some(key), Some(host)) = (&self.remember_key, Host::try_of_ctx(ctx)) else { return };
        let mut saved = self.fields.clone();
        let keys = self.remembered_keys();
        saved.retain_fitting(&keys, |key, value| self.params.iter().any(|p| p.accepts(key, value)));
        if !saved.is_empty() {
            Remembered::<DialogFields>::new(key.as_str()).store(&host, &saved);
        }
    }

    /// The preview pane: zoom buttons and the zoom percentage over a well as wide as the dialog and
    /// `dialog.preview_pane` high, the image at the zoom in it (100% when the dialog opens), a
    /// checkerboard under transparency. The pointer drags the view along an axis the image
    /// overflows, and a click centres the view on the point clicked.
    // pc: crates/ui-egui/src/dialogs/filter_preview_ui.rs (the pane only; the worker stays in the app).
    fn preview_pane(ui: &mut Ui, dialog: DialogId, image: &SizedTexture) {
        let tokens = Tokens::of(ui);
        let words = DialogLabels::of_ctx(ui.ctx());
        let zoom_id = preview_zoom_id(dialog);
        let pan_id = preview_pan_id(dialog);
        let old_zoom = ui.data(|d| d.get_temp::<f32>(zoom_id)).unwrap_or(1.0);
        let mut zoom = old_zoom;
        direction::horizontal(ui, |ui| {
            let out = ui.add(IconButton::new(Icon::Minus, words.zoom_out.as_str()).id_salt((dialog, "zoom-out")).enabled(zoom > ZOOM_STEPS[0]));
            let readout = ui.add(Label::new(Units::of(ui).percent(f64::from(zoom) * 100.0)).selectable(false));
            ui.ctx().accesskit_node_builder(readout.id, |node| node.set_description(words.zoom.clone()));
            let into = ui.add(
                IconButton::new(Icon::Plus, words.zoom_in.as_str()).id_salt((dialog, "zoom-in")).enabled(zoom < ZOOM_STEPS[ZOOM_STEPS.len() - 1]),
            );
            if out.clicked() {
                zoom = ZOOM_STEPS.iter().rev().copied().find(|step| *step < zoom - 1e-4).unwrap_or(ZOOM_STEPS[0]);
            }
            if into.clicked() {
                zoom = ZOOM_STEPS.iter().copied().find(|step| *step > zoom + 1e-4).unwrap_or(ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
            }
        });
        let [least, height] = tokens.component.dialog.preview_pane;
        let (well, response) = ui.allocate_exact_size(vec2(ui.available_width().max(least), height), Sense::click_and_drag());
        let field = TextField::style(&tokens, ControlSize::Md);
        let swatch = Swatch::style(&tokens, ControlSize::Md);
        ui.painter().extend(field_box(ui, &field, well, BoxState { enabled: true, focused: false, hovered: false, invalid: false }));
        let size = image.size * zoom;
        let slack = ((size - well.size()) / 2.0).max(Vec2::ZERO);
        let mut pan = ui.data(|d| d.get_temp::<Vec2>(pan_id)).unwrap_or_default() * (zoom / old_zoom) + response.drag_delta();
        if let (true, Some(at)) = (response.clicked(), response.interact_pointer_pos()) {
            pan += well.center() - at;
        }
        let pan = pan.clamp(-slack, slack);
        ui.data_mut(|d| {
            d.insert_temp(zoom_id, zoom);
            d.insert_temp(pan_id, pan);
        });
        let shown = Rect::from_center_size(well.center() + pan, size);
        let painter = ui.painter_at(well);
        painter.extend(checkerboard(shown.intersect(well), CornerRadius::ZERO, swatch.checker, swatch.checker_cell));
        painter.image(image.id, shown, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        if slack != Vec2::ZERO {
            ui.ctx().set_cursor_icon(if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
    }
}

/// The zoom levels of the preview pane, as fractions of the image's size.
const ZOOM_STEPS: [f32; 11] = [0.125, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

fn preview_zoom_id(dialog: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::preview_zoom", dialog))
}

fn preview_pan_id(dialog: DialogId) -> Id {
    Id::unique(("craft_ui::dialog::params::preview_pan", dialog))
}

/// What a [`ParamDialog`] answers with at OK.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamResult {
    /// The dialog's [`key`](ParamDialog::key): the command the values are for.
    pub key: String,
    pub fields: DialogFields,
}

impl ParamResult {
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl ParamDialog {
    /// OK: stores the remembered values and builds the answer.
    fn answer(&self, ctx: &Context) -> ParamResult {
        self.store(ctx);
        ParamResult { key: self.key.clone(), fields: self.fields.clone() }
    }
}

impl StackDialog for ParamDialog {
    type Result = ParamResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs.rs:249 `filter_body` and :282 `button_row`; built on
    // `Dialog::new(..).stack(stack).footer(Footer::side().preview(..).reset(Reset::AltCancel(labels.reset)))`
    // with `show_with_below` for the preview pane in `ParamLayout::BottomFooter`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ParamResult>> {
        if !self.state.is_open() {
            self.open(ctx);
        }
        let labels = DialogLabels::of_ctx(ctx);
        let layout = self.layout.unwrap_or(if self.preview { ParamLayout::SideColumn } else { ParamLayout::BottomFooter });
        let mut preview = *self.state.preview();
        let (has_preview, preview_label) = (self.preview, labels.preview.as_str());
        let reset = Reset::AltCancel(labels.reset.clone());
        let footer = match layout {
            ParamLayout::SideColumn => {
                let footer = Footer::side().reset(reset).ok_enabled(self.ok_enabled);
                if has_preview {
                    footer.below(|ui| {
                        preview.checkbox(ui, preview_label);
                    })
                } else {
                    footer
                }
            }
            ParamLayout::BottomFooter => {
                let mut footer = Footer::standard().reset(reset).ok_enabled(self.ok_enabled);
                if let Some(extra) = &mut self.footer_extra {
                    footer = footer.trailing_extra(|ui| extra(ui));
                }
                if has_preview {
                    footer.leading(|ui| {
                        preview.checkbox(ui, preview_label);
                    })
                } else {
                    footer
                }
            }
        };
        let image = self.preview_image.as_ref();
        let (params, id, below) = (&self.params, self.id, &self.below);
        let mut dialog =
            Dialog::with_id(self.id, self.title.as_str()).stack(stack).width(self.width).body(Body::Scroll).footer(footer).history(self.history);
        if let Some(label) = &self.close_label {
            dialog = dialog.close_label(label.clone());
        }
        let shown = dialog.show_with_below(
            ctx,
            &mut self.state,
            &mut self.fields,
            |ui, fields| {
                first_field_focused(ui, id, |ui| {
                    FormGrid::new("params").show(ui, |form| draw_rows(params, id, form, fields, image));
                });
                if let Some(below) = below {
                    below(ui, fields);
                }
            },
            |ui, _| {
                if let Some(image) = image {
                    ui.add_space(ui.spacing().item_spacing.y);
                    Self::preview_pane(ui, id, image);
                }
            },
        );
        if self.state.is_open() {
            *self.state.preview_mut() = preview;
        }
        self.picked_color = take_color_press(ctx, self.id).or(self.picked_color.take());
        self.picked_path = take_path_press(ctx, self.id).or(self.picked_path.take());
        self.picked_eyedropper = take_eyedropper_press(ctx, self.id).or(self.picked_eyedropper.take());
        match shown.outcome? {
            Outcome::Ok => Some(DialogResult::Ok(self.answer(ctx))),
            Outcome::Cancel | Outcome::Close => Some(DialogResult::Cancel),
            Outcome::Reset => {
                match &self.reset_target {
                    Some(target) => {
                        for (key, value) in target.iter() {
                            self.fields.set(key, value.clone());
                        }
                    }
                    None => {
                        self.fields.reset();
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn instance(&self) -> Option<DialogId> {
        self.instance
    }

    fn confirm(&mut self, ctx: &Context) -> Option<ParamResult> {
        if !self.ok_enabled {
            return None;
        }
        if !self.state.is_open() {
            self.open(ctx);
        }
        Some(self.answer(ctx))
    }
}
