//! The effect page: one row per field of the effect's [`EffectSpec`], drawn by kind (task AD-9).

#![cfg_attr(not(test), expect(dead_code, reason = "used by the dialog body (task AD-8)"))]

use egui::accesskit::Role;
use egui::{AsIdSalt, Modifiers, Ui, UiBuilder};

use craft_theme::{Srgba, TypeRole};
use craft_ui::composites::SliderRow;
use craft_ui::controls::{AngleDial, BlendChoice, Button, Checkbox, Select, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};

use super::LayerStyleDialog;
use super::contour::{ContourAction, ContourField};
use super::labels::{ContourFieldLabels, LayerStyleLabels};
use crate::layer_style::{Contour, EffectField, EffectInstance, EffectSpec, FieldKind, FieldUnit, StyleParams, StyleValue};

/// A pattern the host offers for Pattern Overlay: its key (stored in the `pattern` parameter)
/// and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternChoice {
    pub key: String,
    pub label: String,
}

/// What an effect page reads besides the instance.
pub(super) struct PageInput<'a> {
    pub spec: &'a EffectSpec,
    pub instance: &'a EffectInstance,
    pub blend_modes: &'a [BlendChoice],
    pub patterns: &'a [PatternChoice],
    pub user_contours: &'a [Contour],
    pub labels: &'a LayerStyleLabels,
    pub contour_labels: &'a ContourFieldLabels,
    pub numbers: &'a LocaleNumbers,
    pub enabled: bool,
}

/// One field's edit on a page, by parameter key.
pub(super) struct FieldEdit {
    pub key: String,
    pub event: EditEvent<StyleValue>,
}

/// What an effect page reports in one pass.
#[derive(Default)]
pub(super) struct PageOutput {
    pub edit: Option<FieldEdit>,
    /// A colour swatch was clicked: key `"{instance id}/{parameter key}"` (`"fx2/color"`),
    /// titled [`LayerStyleLabels::color_picker_title`].
    pub pick_color: Option<ColorRequest>,
    pub contour_action: Option<ContourAction>,
    /// Make Default: the page's shown values (the defaults under the instance's).
    pub make_default: Option<StyleParams>,
    /// Reset to Default was pressed.
    pub reset_to_default: bool,
}

/// Draws the page body under the title: each field of `input.spec` in order, showing the spec's
/// defaults under the instance's values ([`StyleParams::over`]), so only real edits reach the
/// instance. By [`FieldKind`](crate::layer_style::FieldKind):
///
/// - `Slider`: a `SliderRow` over the range with the unit; values rounded to whole numbers;
///   a drag is a gesture, a field entry or arrow a `Set`.
/// - `Angle`: an `AngleDial` row, −180..=180; with the instance's `useGlobalLight` on its edits
///   go to the shared light angle (the dialog applies them through
///   [`LayerStyleDraft::set_param`](crate::layer_style::LayerStyleDraft::set_param)).
/// - `Color`: a `Swatch` with the label before it; a click or Space reports `pick_color`.
/// - `Blend`: a `Select` over `blend_modes` (one id per field, so Bevel & Emboss's two lists stay
///   apart, #9368); a value missing from the list shows as its key.
/// - `Choice`: a `Select` over the options; a missing value shows the first.
/// - `GradientMethod`: Smooth, Perceptual, Linear, Classic, with Custom first while the value names
///   none of them; the host fills From and To when a method is first picked from Custom.
/// - `Pattern`: a `Select` over `patterns`; an empty value takes the first pattern as a `Set` in
///   the pass it shows.
/// - `Contour`: a [`ContourField`](super::ContourField) with the Anti-aliased check of the
///   field's `anti_alias` key.
/// - `Check`: a `Checkbox`.
///
/// Under the fields, Make Default and Reset to Default at the trailing end, wrapping when the
/// translated labels are wide.
///
/// A value of the wrong type for its field (only reachable when the host builds a bad draft)
/// shows one line, [`LayerStyleLabels::invalid_value`] naming the first such field
/// ([`first_invalid`]), in `status.bad.fg` instead of the fields, and the value stays as it is.
///
/// The page is an AccessKit group named by the effect. At most one field's edit is reported per
/// pass (the first drawn).
pub(super) fn effect_page(ui: &mut Ui, input: &PageInput<'_>) -> PageOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1008-1153 (defaults under the values, one arm
    // per field kind), :1155-1171 (Make Default, Reset to Default), :996-1000 (the invalid line).
    let mut out = PageOutput::default();
    let id = ui.make_persistent_id(("effectPage", input.instance.id.0));
    let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(input.labels.effect(input.spec.kind));
    if !input.enabled {
        builder = builder.disabled();
    }
    ui.scope_builder(builder, |ui| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            if let Some(field) = first_invalid(input.spec, &input.instance.params) {
                let name = field_label(input, field);
                let name = if name.is_empty() { field.key.to_string() } else { name };
                let tokens = Tokens::of(ui);
                let colour = LayerStyleDialog::style(&tokens).invalid_colour;
                let job = egui::text::LayoutJob::single_section(
                    craft_i18n::fmt(&input.labels.invalid_value, &[("field", name.as_str())]),
                    role_format(&tokens, TypeRole::Body, colour),
                );
                ui.add(egui::Label::new(job).selectable(false));
                return;
            }
            let shown = input.instance.params.over(&input.spec.defaults);
            for field in &input.spec.fields {
                field_row(ui, input, field, &shown, &mut out);
            }
            ui.with_layout(direction::trailing(ui).with_main_wrap(true), |ui| {
                let labels = input.labels;
                let reset = Button::new(labels.reset_to_default.as_str()).id_salt((id, "reset")).enabled(input.enabled);
                if ui.add(reset).clicked() {
                    out.reset_to_default = true;
                }
                let make = Button::new(labels.make_default.as_str()).id_salt((id, "make")).enabled(input.enabled);
                if ui.add(make).clicked() {
                    out.make_default = Some(shown.clone());
                }
            });
        });
    });
    out
}

/// The label of `field`: the installed word for a built-in key, else the spec's own.
fn field_label(input: &PageInput<'_>, field: &EffectField) -> String {
    input.labels.field(&field.key).map_or_else(|| field.label.to_string(), str::to_owned)
}

fn unit_template(unit: FieldUnit) -> &'static str {
    match unit {
        FieldUnit::Percent => "{n} %",
        FieldUnit::Pixels => "{n} px",
        FieldUnit::Degrees => "{n}°",
    }
}

/// Keeps the first edit of the pass.
fn report(out: &mut PageOutput, key: &str, event: Option<EditEvent<StyleValue>>) {
    if out.edit.is_none()
        && let Some(event) = event
    {
        out.edit = Some(FieldEdit { key: key.to_owned(), event });
    }
}

/// A caption in `text.muted` at the reading start and the control after it.
pub(super) fn caption_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    Row::new(Gap::Inline)
        .show(ui, |ui| {
            ui.add(Text::new(label).role(TypeRole::Label).content(Content::Muted));
            add(ui)
        })
        .inner
}

/// One `Select` over `options` (value, text, separator before); the picked value's event.
pub(super) fn pick<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    current: T,
    options: Vec<(T, String, bool)>,
    enabled: bool,
) -> Option<EditEvent<T>> {
    let mut value = current;
    let mut select = Select::new(id_salt, label, &mut value);
    for (option, text, separator) in options {
        if separator {
            select = select.separator();
        }
        select = select.option(option, text);
    }
    select.enabled(enabled).show(ui).event
}

/// `modes` as select options; a `current` key the list lacks is added at the end as its own text.
pub(super) fn blend_options(modes: &[BlendChoice], current: &str) -> Vec<(String, String, bool)> {
    let mut options: Vec<_> = modes.iter().map(|m| (m.key.clone(), m.label.clone(), m.separator_before)).collect();
    if !options.iter().any(|(key, ..)| key == current) {
        options.push((current.to_owned(), current.to_owned(), false));
    }
    options
}

fn text_of(value: Option<&StyleValue>) -> Option<&str> {
    match value {
        Some(StyleValue::Text(text)) => Some(text),
        _ => None,
    }
}

fn field_row(ui: &mut Ui, input: &PageInput<'_>, field: &EffectField, shown: &StyleParams, out: &mut PageOutput) {
    let (labels, enabled, key) = (input.labels, input.enabled, field.key.as_ref());
    let label = field_label(input, field);
    let salt = (input.instance.id.0, key);
    match &field.kind {
        FieldKind::Slider { min, max, unit } => {
            let value = shown.number(key).unwrap_or(*min);
            let mut row = SliderRow::new(salt, label.as_str(), value, *min..=*max, input.numbers)
                .unit(Unit::new(unit_template(*unit)))
                .decimals(0)
                .step(1.0)
                .enabled(enabled);
            if let Some(default) = input.spec.defaults.number(key) {
                row = row.default_value(default);
            }
            let event = row.show(ui).edits().next();
            report(out, key, event.map(|e| e.map(|v| StyleValue::Number(v.round()))));
        }
        FieldKind::Angle => {
            let value = shown.number(key).unwrap_or(0.0);
            let dial = AngleDial::new(salt, label.as_str(), value, input.numbers).range(-180.0..=180.0).enabled(enabled);
            report(out, key, dial.show(ui).event.map(|e| e.map(StyleValue::Number)));
        }
        FieldKind::Color => {
            let color = match shown.get(key) {
                Some(StyleValue::Color(color)) => *color,
                _ => Srgba::BLACK,
            };
            let clicked = caption_row(ui, &label, |ui| ui.add(Swatch::new(salt, label.as_str(), color).enabled(enabled)).clicked());
            if clicked {
                let request = ColorRequest::new(format!("{}/{key}", input.instance.id.key()), color);
                out.pick_color = Some(request.title(labels.color_picker_title.as_str()));
            }
        }
        FieldKind::Blend => {
            let current = text_of(shown.get(key)).unwrap_or("Normal");
            let options = blend_options(input.blend_modes, current);
            let event = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            report(out, key, event.map(|e| e.map(StyleValue::Text)));
        }
        FieldKind::Choice(choices) => {
            let Some(first) = choices.first() else { return };
            let current = text_of(shown.get(key)).unwrap_or(&first.value);
            let options = choices
                .iter()
                .map(|c| (c.value.to_string(), labels.choice(&c.value).map_or_else(|| c.label.to_string(), str::to_owned), false))
                .collect();
            let event = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            report(out, key, event.map(|e| e.map(StyleValue::Text)));
        }
        FieldKind::GradientMethod => {
            const METHODS: [&str; 4] = ["smooth", "perceptual", "linear", "classic"];
            let stored = text_of(shown.get(key)).unwrap_or("custom");
            let current = if METHODS.contains(&stored) { stored } else { "custom" };
            let mut options = Vec::new();
            if current == "custom" {
                options.push(("custom".to_owned(), labels.custom.clone(), false));
            }
            for (value, text) in METHODS.into_iter().zip([&labels.smooth, &labels.perceptual, &labels.linear, &labels.classic]) {
                options.push((value.to_owned(), text.clone(), false));
            }
            let event = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            report(out, key, event.map(|e| e.map(StyleValue::Text)));
        }
        FieldKind::Pattern => {
            let stored = text_of(shown.get(key)).filter(|t| !t.is_empty());
            let first = input.patterns.first();
            let current = stored.or(first.map(|p| p.key.as_str())).unwrap_or_default();
            let options = input.patterns.iter().map(|p| (p.key.clone(), p.label.clone(), false)).collect();
            let event = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            if event.is_none() && stored.is_none() && enabled && first.is_some() {
                report(out, key, Some(EditEvent::set(StyleValue::Text(current.to_owned()), Modifiers::NONE)));
            }
            report(out, key, event.map(|e| e.map(StyleValue::Text)));
        }
        FieldKind::Contour { anti_alias } => {
            let contour = match shown.get(key) {
                Some(StyleValue::Contour(contour)) => contour.clone(),
                _ => Contour::linear(),
            };
            let aa = anti_alias.as_deref().map(|k| shown.bool(k).unwrap_or(false));
            let response = ContourField::new(salt, label.as_str(), &contour)
                .anti_alias(aa)
                .user_contours(input.user_contours)
                .labels(input.contour_labels.clone())
                .enabled(enabled)
                .show(ui);
            report(out, key, response.edit.map(|e| e.map(StyleValue::Contour)));
            if let (Some(aa_key), Some(on)) = (anti_alias.as_deref(), response.anti_alias) {
                report(out, aa_key, Some(EditEvent::set(StyleValue::Bool(on), Modifiers::NONE)));
            }
            if response.action.is_some() {
                out.contour_action = response.action;
            }
        }
        FieldKind::Check => {
            let mut on = shown.bool(key).unwrap_or(false);
            let event = Checkbox::bool(label.as_str(), &mut on).id_salt(salt).enabled(enabled).show(ui).event;
            report(out, key, event.map(|e| e.map(StyleValue::Bool)));
        }
    }
}

/// The first field of `spec` whose value in `params` has the wrong type: a slider or angle not a
/// number, a check not a bool, a colour not a colour, a contour not a contour, a choice, blend or
/// pattern not text. Missing keys are fine (the defaults show). A contour's Anti-aliased key is
/// held to a bool as well.
pub(super) fn first_invalid<'s>(spec: &'s EffectSpec, params: &StyleParams) -> Option<&'s EffectField> {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:624-646 `validate_params` (the JSON shape
    // check, here per field type).
    let wrong = |key: &str, ok: fn(&StyleValue) -> bool| params.get(key).is_some_and(|v| !ok(v));
    spec.fields.iter().find(|field| match &field.kind {
        FieldKind::Slider { .. } | FieldKind::Angle => wrong(&field.key, |v| matches!(v, StyleValue::Number(_))),
        FieldKind::Check => wrong(&field.key, |v| matches!(v, StyleValue::Bool(_))),
        FieldKind::Color => wrong(&field.key, |v| matches!(v, StyleValue::Color(_))),
        FieldKind::Blend | FieldKind::Choice(_) | FieldKind::GradientMethod | FieldKind::Pattern => {
            wrong(&field.key, |v| matches!(v, StyleValue::Text(_)))
        }
        FieldKind::Contour { anti_alias } => {
            wrong(&field.key, |v| matches!(v, StyleValue::Contour(_)))
                || anti_alias.as_deref().is_some_and(|k| wrong(k, |v| matches!(v, StyleValue::Bool(_))))
        }
    })
}
