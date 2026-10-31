//! One row of a page: the library's control for its kind, in a [`Field`], reporting through the
//! host's `set`.

use std::ops::RangeInclusive;

use craft_theme::Rgba;
use egui::{Align, Context, Id, Layout, Ui, UiBuilder, vec2};

use super::body::PreferencesBody;
use super::model::{PrefValue, RowKind, RowSpec};
use super::model_trait::PreferencesModel;
use crate::composites::{Field, LabelAlign};
use crate::controls::{Checkbox, NumberField, Select, Swatch, TextField};
use crate::direction;
use crate::edit::EditEvent;
use crate::localized::{LocaleNumbers, Unit};
use crate::theme::{Tokens, TokensOf as _};

/// The characters of a colour's hex field: `#rrggbb`.
const HEX_CHARS: usize = 7;

/// A row's strings and value, copied from the host's borrowed model so that the host can be
/// borrowed again to draw a custom row.
#[derive(Clone, Debug)]
pub(super) struct ShownRow {
    pub key: String,
    pub label: String,
    pub help: Option<String>,
    pub enabled: bool,
    pub kind: Kind,
}

#[derive(Clone, Debug)]
pub(super) enum Kind {
    Bool(bool),
    Choice { value: String, options: Vec<(String, String)> },
    Color(Rgba),
    Int { value: i64, range: RangeInclusive<i64>, unit: Option<String> },
    Float { value: f64, range: RangeInclusive<f64>, decimals: u8, step: Option<f64>, unit: Option<String> },
    Text(String),
    Custom,
}

impl ShownRow {
    pub fn of(row: &RowSpec<'_>) -> Self {
        let kind = match &row.kind {
            RowKind::Bool(v) => Kind::Bool(*v),
            RowKind::Choice { value, options } => Kind::Choice {
                value: (*value).to_owned(),
                options: options.iter().map(|(id, label)| ((*id).to_owned(), (*label).to_owned())).collect(),
            },
            RowKind::Color(c) => Kind::Color(*c),
            RowKind::Int { value, range, unit } => Kind::Int { value: *value, range: range.clone(), unit: unit.map(str::to_owned) },
            RowKind::Float { value, range, decimals, unit } => {
                Kind::Float { value: *value, range: range.clone(), decimals: *decimals, step: row.step, unit: unit.map(str::to_owned) }
            }
            RowKind::Text(t) => Kind::Text((*t).to_owned()),
            RowKind::Custom => Kind::Custom,
        };
        Self { key: row.key.to_owned(), label: row.label.to_owned(), help: row.help.map(str::to_owned), enabled: row.enabled, kind }
    }
}

/// What a pass of rows saw.
#[derive(Default)]
pub(super) struct RowsOut {
    /// A row was edited.
    pub edited: bool,
    /// A colour swatch was pressed.
    pub pick_color: Option<String>,
}

/// The value to hand the host for `event`. A gesture's start is kept so that a cancelled gesture
/// restores the value from before it.
fn forward<T>(ctx: &Context, id: Id, event: Option<EditEvent<T>>, before: PrefValue, to_value: impl Fn(T) -> PrefValue) -> Option<PrefValue> {
    match event? {
        EditEvent::Begin { value, .. } => {
            ctx.data_mut(|d| d.insert_temp(id, before));
            value.map(to_value)
        }
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(to_value(value)),
        EditEvent::Cancel { .. } => ctx.data_mut(|d| d.get_temp::<PrefValue>(id)),
    }
}

/// The text a field reported this pass, whatever the gesture.
pub(super) fn typed_value(event: Option<EditEvent<String>>) -> Option<String> {
    match event? {
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(value),
        EditEvent::Begin { value, .. } => value,
        EditEvent::Cancel { .. } => None,
    }
}

/// A child `Ui` `width` wide whose controls fill it.
fn fixed<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = width.min(ui.available_width());
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down_justified(Align::Min), add).inner
}

fn step_for(range: &RangeInclusive<f64>, decimals: u8) -> f64 {
    if decimals == 0 || range.end() - range.start() >= 10.0 { 1.0 } else { 10f64.powi(-i32::from(decimals)) }
}

pub(super) fn show_row<H: PreferencesModel + ?Sized>(
    ui: &mut Ui,
    host: &mut H,
    row: &ShownRow,
    numbers: &LocaleNumbers,
    hex_label: &str,
    out: &mut RowsOut,
    checks_at_start: bool,
) {
    let ctx = ui.ctx().clone();
    let widths = PreferencesBody::style(&Tokens::of(ui));
    let key = row.key.as_str();
    let edit_id = ui.make_persistent_id(("preferences-before", key));
    let enabled = row.enabled;
    let mut new_value: Option<PrefValue> = None;
    let field = if matches!(row.kind, Kind::Bool(_)) {
        if checks_at_start { Field::spanning() } else { Field::unlabelled() }
    } else {
        Field::new(row.label.as_str()).label_align(LabelAlign::Start)
    };
    // PhotoCraft's number fields show no grouping separators.
    let numbers = &numbers.clone().ungrouped();
    let field = match &row.help {
        Some(help) => field.help(help.as_str()),
        None => field,
    };
    match &row.kind {
        Kind::Bool(value) => {
            let mut checked = *value;
            let show =
                |ui: &mut Ui| field.show(ui, |ui, _| Checkbox::bool(row.label.as_str(), &mut checked).id_salt(key).enabled(enabled).show(ui)).inner;
            let edit = if checks_at_start {
                let rect = ui.available_rect_before_wrap();
                let inset = widths.checks_inset;
                let rect = if direction::is_rtl(ui) { rect.with_max_x(rect.right() - inset) } else { rect.with_min_x(rect.left() + inset) };
                ui.scope_builder(UiBuilder::new().max_rect(rect), show).inner
            } else {
                show(ui)
            };
            new_value = forward(&ctx, edit_id, edit.event, PrefValue::Bool(*value), PrefValue::Bool);
        }
        Kind::Choice { value, options } => {
            let mut chosen = value.clone();
            let edit = field
                .show(ui, |ui, _| {
                    fixed(ui, widths.choice_width, |ui| {
                        let mut select = Select::new(key, row.label.as_str(), &mut chosen).enabled(enabled);
                        for (id, label) in options {
                            select = select.option(id.clone(), label.as_str());
                        }
                        select.show(ui)
                    })
                })
                .inner;
            new_value = forward(&ctx, edit_id, edit.event, PrefValue::Choice(value.clone()), PrefValue::Choice);
        }
        Kind::Int { value, range, unit } => {
            let edit = field
                .show(ui, |ui, invalid| {
                    let mut f = NumberField::new(key, row.label.as_str(), *value as f64, numbers)
                        .range(*range.start() as f64..=*range.end() as f64)
                        .decimals(0..=0)
                        .enabled(enabled)
                        .invalid(invalid);
                    if let Some(unit) = unit {
                        f = f.unit(Unit::new(unit));
                    }
                    f.show(ui)
                })
                .inner;
            new_value = forward(&ctx, edit_id, edit.event, PrefValue::Int(*value), |v| PrefValue::Int(v.round() as i64));
        }
        Kind::Float { value, range, decimals, step, unit } => {
            let edit = field
                .show(ui, |ui, invalid| {
                    let mut f = NumberField::new(key, row.label.as_str(), *value, numbers)
                        .range(range.clone())
                        .step(step.unwrap_or_else(|| step_for(range, *decimals)))
                        .decimals(usize::from(*decimals).min(1)..=usize::from(*decimals))
                        .enabled(enabled)
                        .invalid(invalid);
                    if let Some(unit) = unit {
                        f = f.unit(Unit::new(unit));
                    }
                    f.show(ui)
                })
                .inner;
            new_value = forward(&ctx, edit_id, edit.event, PrefValue::Float(*value), PrefValue::Float);
        }
        Kind::Text(value) => {
            let edit = field
                .show(ui, |ui, invalid| {
                    fixed(ui, widths.text_width, |ui| TextField::new(key, row.label.as_str(), value).enabled(enabled).invalid(invalid).show(ui))
                })
                .inner;
            new_value = forward(&ctx, edit_id, edit.event, PrefValue::Text(value.clone()), PrefValue::Text);
        }
        Kind::Color(color) => {
            let typed_id = edit_id.with("typed");
            let shown = ctx.data(|d| d.get_temp::<String>(typed_id)).unwrap_or_else(|| Swatch::hex(*color));
            let (mut pressed, mut event, mut focused) = (false, None, false);
            field.show(ui, |ui, invalid| {
                crate::direction::horizontal(ui, |ui| {
                    pressed = ui.add(Swatch::new((key, "swatch"), row.label.as_str(), *color).enabled(enabled)).clicked();
                    let edit = fixed(ui, widths.hex_width, |ui| {
                        TextField::new(key, hex_label, &shown).char_limit(HEX_CHARS).enabled(enabled).invalid(invalid).show(ui)
                    });
                    focused = edit.response.has_focus();
                    event = edit.event;
                    edit.response
                })
                .inner
            });
            if pressed {
                out.pick_color = Some(key.to_owned());
            }
            // Text that is not a colour yet stays in the field while it has the focus.
            new_value = match forward(&ctx, edit_id, event, PrefValue::Color(*color), PrefValue::Text) {
                Some(PrefValue::Text(text)) => {
                    let parsed = Rgba::from_hex(&text).map(|c| PrefValue::Color(Rgba([c.0[0], c.0[1], c.0[2], color.0[3]])));
                    ctx.data_mut(|d| d.insert_temp(typed_id, text));
                    parsed
                }
                other => {
                    ctx.data_mut(|d| d.remove_temp::<String>(typed_id));
                    other
                }
            };
            if !focused {
                ctx.data_mut(|d| d.remove_temp::<String>(typed_id));
            }
        }
        Kind::Custom if row.label.is_empty() => {
            ui.scope(|ui| host.custom_row(ui, key));
        }
        Kind::Custom => {
            field.show(ui, |ui, _| ui.scope(|ui| host.custom_row(ui, key)).response);
        }
    }
    if let Some(value) = new_value {
        out.edited = true;
        host.set(key, value);
    }
}
