//! [`ParamDialog`]: a filter or command dialog built from a parameter list, with Preview and
//! Reset. [`Param`] and [`ParamKind`] are the list; [`CommandForm`](super::CommandForm) shares them.

use std::fmt;
use std::sync::Arc;

use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{ControlSize, Rgba, Srgba};
use egui::load::SizedTexture;
use egui::{Color32, Context, CornerRadius, Id, Rect, Response, Sense, Ui, Vec2, pos2, vec2};

use super::color_request::ColorRequest;
use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::fields::{DialogFields, FieldValue};
use super::footer::{DialogLabels, Footer, Reset};
use super::size::{Body, Width};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{FieldControl as _, Form, FormGrid};
use crate::controls::text_field::{BoxState, TextField, field_box};
use crate::controls::{AngleDial, Button, Checkbox, NumberField, PointPicker, Select, Slider, SliderScale, Swatch, TooltipExt as _, checkerboard};
use crate::edit::{self, EditResponse};
use crate::localized::{Labels as _, LocaleNumbers, Unit, focus_first_field};
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
}

/// What draws a [`ParamKind::Custom`] row: it gets the row's control cell and the dialog's values
/// and returns the response the row label names and centres on. Equal only to itself.
#[derive(Clone)]
pub struct CustomRow(Arc<CustomDraw>);

type CustomDraw = dyn Fn(&mut Ui, &mut DialogFields) -> Response + Send + Sync;

impl CustomRow {
    pub fn new(draw: impl Fn(&mut Ui, &mut DialogFields) -> Response + Send + Sync + 'static) -> Self {
        Self(Arc::new(draw))
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PathKind {
    #[default]
    Folder,
    OpenFile,
    SaveFile,
}

/// A pressed Browse button: the host opens its file dialog for `kind` and writes the answer with
/// `fields_mut().set(key, FieldValue::Text(path))`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathRequest {
    pub key: String,
    pub kind: PathKind,
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
            ParamKind::Custom(_) => return None,
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
    /// A Browse button after a [`ParamKind::Text`] field.
    pub browse: Option<PathKind>,
    /// The hint and tooltip ([`Param::hint`], [`Param::tooltip`]), boxed so rows without them stay small.
    texts: Option<Box<RowTexts>>,
}

/// Texts a row may carry beyond its label.
#[derive(Clone, Debug, Default, PartialEq)]
struct RowTexts {
    hint: Option<String>,
    tooltip: Option<String>,
}

impl Param {
    pub fn new(key: impl Into<String>, label: impl Into<String>, kind: ParamKind) -> Self {
        Self { key: key.into(), label: label.into(), kind, unit: None, enabled_when: None, browse: None, texts: None }
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
    /// gets the control cell and the dialog's values; the row label names the response it returns.
    /// The row has no default and nothing remembered; `draw` reads and writes
    /// [`DialogFields`] under whatever keys it likes.
    pub fn custom(
        key: impl Into<String>,
        label: impl Into<String>,
        draw: impl Fn(&mut Ui, &mut DialogFields) -> Response + Send + Sync + 'static,
    ) -> Self {
        Self::new(key, label, ParamKind::Custom(CustomRow::new(draw)))
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

    /// The most digits shown after the decimal mark on a [`ParamKind::OptionalNumber`] row
    /// (`0` for an integer such as Batch quality); other kinds are unchanged.
    pub fn decimals(mut self, places: u32) -> Self {
        if let ParamKind::OptionalNumber { decimals, .. } = &mut self.kind {
            *decimals = places;
        }
        self
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
        let label = format!("{}:", translated(&ctx, self.label.trim_end_matches([':', '：'])));
        let key = self.key.as_str();
        let default = self.kind.default_value();
        let unit = &self.unit;
        match &self.kind {
            ParamKind::Range { min, max, scale, .. } => {
                let value = fields.number(key).or_else(|| number_of(default.as_ref())).unwrap_or(*min);
                let (places, step) = if (max - min).abs() <= 10.0 { (0..=2, 0.01) } else { (0..=1, 1.0) };
                form.row(label.as_str(), |ui, _| {
                    edit::scope(ui, |ui| {
                        let slider = Slider::new((key, "slider"), label.as_str(), value, *min..=*max).scale(*scale).show(ui);
                        let field = NumberField::new((key, "field"), label.as_str(), value, numbers).range(*min..=*max).step(step).decimals(places);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field).or_else(|| edited(&slider)) {
                            fields.set(key, FieldValue::Number(v));
                        }
                        (slider, field)
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
                        let field = NumberField::new((key, "field"), label.as_str(), value, numbers).range(range).step(*step).decimals(places);
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
                        let field = NumberField::new((key, "field"), label.as_str(), value as f64, numbers).range(range).decimals(0..=0);
                        let field = with_unit(field, unit).show(ui);
                        if let Some(v) = edited(&field) {
                            fields.set(key, FieldValue::Int(v.round() as i64));
                        }
                        field
                    })
                    .inner
                });
            }
            ParamKind::Bool { default } => {
                let mut value = fields.bool(key).unwrap_or(*default);
                let text = label.trim_end_matches([':', '：']);
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
                        let mut select = Select::new((key, "select"), label.as_str(), &mut value);
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
                        let dial = AngleDial::new((key, "angle"), label.as_str(), value, numbers).show(ui);
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
                    let swatch = ui.add(Swatch::new((key, "swatch"), label.as_str(), shown).mixed(color.is_none()));
                    if swatch.clicked() {
                        let request =
                            ColorRequest::new(key, Srgba::from_rgb_array(color.unwrap_or([0.0; 3]))).title(label.trim_end_matches([':', '：']));
                        ui.ctx().data_mut(|d| d.insert_temp(color_press_id(dialog), request));
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
                                        let name = format!("{label} {}, {}", row + 1, column + 1);
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
                        let picker = PointPicker::new((key, "point"), label.as_str(), value, preview.copied()).show(ui);
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
                        let mut select = Select::new((key, "channel"), label.as_str(), &mut index);
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
                        let mut text = TextField::new((key, "text"), label.as_str(), &value);
                        if let Some(hint) = self.texts.as_ref().and_then(|t| t.hint.as_ref()) {
                            text = text.placeholder(translated(&ctx, hint));
                        }
                        let text = text.show(ui);
                        if let Some(v) = edited(&text) {
                            fields.set(key, FieldValue::Text(v));
                        }
                        let browse = self.browse.map(|kind| {
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
                                let field = NumberField::new((key, "field"), label.as_str(), last, numbers)
                                    .range(*min..=*max)
                                    .decimals(0..=*decimals as usize);
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
            ParamKind::Custom(row) => {
                form.row(label.as_str(), |ui, _| (row.0)(ui, fields));
            }
        }
    }
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
pub(super) fn translated(ctx: &Context, text: &str) -> String {
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

/// Writes `color` into the colour parameter `key` of `params`; other keys are ignored.
pub(super) fn set_color_field(params: &[Param], fields: &mut DialogFields, key: &str, color: Srgba) {
    if params.iter().any(|param| param.key == key && matches!(param.kind, ParamKind::Color { .. })) {
        fields.set(key, FieldValue::Color(color.to_rgb_array()));
    }
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
            && let Some(value) = param.kind.default_value()
        {
            fields.set(param.key.as_str(), value);
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
#[allow(dead_code, reason = "reachable once dialog/mod.rs re-exports it (a root change for the lead)")]
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
#[allow(dead_code, reason = "reachable once dialog/mod.rs re-exports it (a root change for the lead)")]
#[must_use = "show the rows with `show`"]
pub struct ParamRows<'a> {
    id_salt: egui::IdSalt,
    params: &'a [Param],
    preview: Option<SizedTexture>,
}

#[allow(dead_code, reason = "reachable once dialog/mod.rs re-exports it (a root change for the lead)")]
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
    #[expect(clippy::todo, reason = "skeleton: K18")]
    pub fn show(self, ui: &mut Ui, fields: &mut DialogFields) -> ParamRowsResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:782-786 (the selected effect's parameters
        // drawn from its spec in a scrolling column), with `draw_rows` in a `FormGrid` as
        // `ParamDialog::show` draws its body here, and the presses taken with `take_color_press`
        // and `take_path_press` under the rows' id (`ui.make_persistent_id(id_salt)`).
        let _ = (self.id_salt, self.params, self.preview, ui, fields);
        todo!()
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
    reset_target: Option<DialogFields>,
    ok_enabled: bool,
    history: bool,
    close_label: Option<Option<String>>,
    footer_extra: Option<FooterExtra>,
    state: DialogState<DialogFields>,
}

/// What a dialog draws before its buttons ([`ParamDialog::footer_extra`]).
pub(super) type FooterExtra = Box<dyn FnMut(&mut Ui) + Send>;

impl ParamDialog {
    /// A dialog for `params`, opening with their defaults. `key` names the dialog for the stack
    /// and for placement memory (one per command, as PhotoCraft places them).
    pub fn new(key: impl Into<String>, title: impl Into<String>, params: Vec<Param>) -> Self {
        let key = key.into();
        Self {
            id: dialog_id(key.as_str()),
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

    /// Whether the host should render the result now: Preview is on and not held to compare.
    pub fn preview_on(&self) -> bool {
        self.preview && self.state.preview().shows_result()
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

    /// The colour picker's answer for the parameter `key`; other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        set_color_field(&self.params, &mut self.fields, key, color);
    }

    /// The Browse button of a [`Param::browse`] row the user pressed, once: the host opens its
    /// file dialog and writes the path with `fields_mut().set(key, FieldValue::Text(..))`.
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
            saved.retain_fitting(&keys, |key, value| self.params.iter().any(|p| p.key == key && p.kind.fits(value)));
            for (key, value) in saved.iter() {
                self.fields.set(key, value.clone());
            }
        }
        open_fields(ctx, self.id, &self.params, &mut self.fields);
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
        self.params.iter().filter(|p| kept(&p.kind)).map(|p| p.key.as_str()).collect()
    }

    /// Stores the values under the remember key.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:363 `remember`.
    fn store(&self, ctx: &Context) {
        let (Some(key), Some(host)) = (&self.remember_key, Host::try_of_ctx(ctx)) else { return };
        let mut saved = self.fields.clone();
        let keys = self.remembered_keys();
        saved.retain_fitting(&keys, |key, value| self.params.iter().any(|p| p.key == key && p.kind.fits(value)));
        if !saved.is_empty() {
            Remembered::<DialogFields>::new(key.as_str()).store(&host, &saved);
        }
    }

    /// The preview pane: the image at its size in a well as wide as the dialog and `dialog.preview_pane` high, a checkerboard under transparency, the pointer dragging the view
    /// along an axis the image overflows.
    // pc: crates/ui-egui/src/dialogs/filter_preview_ui.rs (the pane only; the worker stays in the app).
    fn preview_pane(ui: &mut Ui, image: &SizedTexture) {
        let tokens = Tokens::of(ui);
        let [least, height] = tokens.component.dialog.preview_pane;
        let (well, response) = ui.allocate_exact_size(vec2(ui.available_width().max(least), height), Sense::drag());
        let field = TextField::style(&tokens, ControlSize::Md);
        let swatch = Swatch::style(&tokens, ControlSize::Md);
        ui.painter().extend(field_box(ui, &field, well, BoxState { enabled: true, focused: false, hovered: false, invalid: false }));
        let slack = ((image.size - well.size()) / 2.0).max(Vec2::ZERO);
        let pan_id = response.id.with("pan");
        let pan = ui.data(|d| d.get_temp::<Vec2>(pan_id)).unwrap_or_default() + response.drag_delta();
        let pan = pan.clamp(-slack, slack);
        ui.data_mut(|d| d.insert_temp(pan_id, pan));
        let shown = Rect::from_center_size(well.center() + pan, image.size);
        let painter = ui.painter_at(well);
        painter.extend(checkerboard(shown.intersect(well), CornerRadius::ZERO, swatch.checker, swatch.checker_cell));
        painter.image(image.id, shown, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        if slack != Vec2::ZERO {
            ui.ctx().set_cursor_icon(if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
    }
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
        let (params, id) = (&self.params, self.id);
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
            },
            |ui, _| {
                if let Some(image) = image {
                    ui.add_space(ui.spacing().item_spacing.y);
                    Self::preview_pane(ui, image);
                }
            },
        );
        if self.state.is_open() {
            *self.state.preview_mut() = preview;
        }
        self.picked_color = take_color_press(ctx, self.id).or(self.picked_color.take());
        self.picked_path = take_path_press(ctx, self.id).or(self.picked_path.take());
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
