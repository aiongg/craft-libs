//! The effect page: one row per field of the effect's [`EffectSpec`], drawn by kind.

use egui::{AsIdSalt, Id, Modifiers, Ui, UiBuilder};

use craft_theme::{Srgba, TypeRole};
use craft_ui::composites::{Sampler, SliderRow};
use craft_ui::controls::{AngleDial, BlendChoice, Button, Checkbox, Select, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::edit::parts::{self, Edits};
use craft_ui::edit::{self, EditEvent, EditPolicy, EditResponse, Gesture};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};

use super::LayerStyleDialog;
use super::blending::BlendingOp;
use super::contour::{ContourAction, ContourField, ContourRequest};
use super::labels::{ContourFieldLabels, LayerStyleLabels};
use super::list::ListOp;
use crate::layer_style::{
    Contour, EffectField, EffectId, EffectInstance, EffectSpec, FieldKind, FieldUnit, LayerStyle, LayerStyleDraft, StyleParams, StyleValue,
};

/// A pattern the host offers for Pattern Overlay: its key (stored in the `pattern` parameter)
/// and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternChoice {
    /// The pattern's key, stored in the `pattern` parameter.
    pub key: String,
    /// The pattern's name.
    pub label: String,
}

/// What an effect page reads besides the instance.
pub(super) struct PageInput<'a> {
    pub spec: &'a EffectSpec,
    pub instance: &'a EffectInstance,
    pub blend_modes: &'a [BlendChoice],
    pub patterns: &'a [PatternChoice],
    pub user_contours: &'a [Contour],
    pub sampler: Option<&'a Sampler>,
    pub labels: &'a LayerStyleLabels,
    pub contour_labels: &'a ContourFieldLabels,
    pub numbers: &'a LocaleNumbers,
    pub enabled: bool,
}

/// What a part of the dialog did to the style: the operation a held event replays.
#[derive(Clone, Debug)]
pub(super) enum StyleOp {
    /// An effect list action that changes the style.
    List(ListOp),
    /// A page field: [`LayerStyleDraft::set_param`] of `key` of instance `id`.
    Param { id: EffectId, key: String, value: StyleValue },
    /// A Blending Options control.
    Blending(BlendingOp),
}

/// `op` applied to `style`; `None` when it changes nothing.
pub(super) fn apply_op(op: &StyleOp, style: &LayerStyle) -> Option<LayerStyle> {
    let mut draft = LayerStyleDraft::new(style.clone());
    match op {
        StyleOp::List(ListOp::Toggle(id)) => draft.toggle(*id),
        StyleOp::List(ListOp::Add { kind, defaults }) => {
            draft.add_instance(*kind, defaults);
        }
        StyleOp::List(ListOp::Remove(id)) => draft.remove_instance(*id),
        StyleOp::Param { id, key, value } => draft.set_param(*id, key, value.clone()),
        StyleOp::Blending(op) => op.apply(&mut draft.style.blending),
    }
    (draft.style != *style).then_some(draft.style)
}

/// `style` with parameter `key` of instance `id` set to `value`.
fn with_param(style: &LayerStyle, id: EffectId, key: &str, value: StyleValue) -> LayerStyle {
    let mut draft = LayerStyleDraft::new(style.clone());
    draft.set_param(id, key, value);
    draft.style
}

/// Draws the dialog's parts as one edit of `style`: replays what waited for its turn, runs
/// `body` with the parts' collector (every part draws from [`Edits::value`] and pushes its edit)
/// and returns the pass's event of the whole style. Esc is the parts': each cancels its own
/// gesture, which cancels this one.
pub(super) fn edit_style<R>(
    ui: &mut Ui,
    id: Id,
    style: &LayerStyle,
    policy: EditPolicy,
    enabled: bool,
    body: impl FnOnce(&mut Ui, &mut Edits<LayerStyle, StyleOp>) -> R,
) -> (R, Option<EditEvent<LayerStyle>>) {
    let parts_id = id.with("parts");
    let mut state: parts::State<StyleOp> = ui.data(|d| d.get_temp(parts_id)).unwrap_or_default();
    let outer = Gesture::load(ui, id.with("edit"), style, policy.cancellable(false)).enabled(enabled);
    let mut edits = Edits::<LayerStyle, StyleOp>::new(id, &state, &outer, edit::in_scope(ui));
    edits.replay(ui.ctx(), apply_op);
    let result = body(ui, &mut edits);
    let event = edits.feed(outer, &mut state).finish();
    ui.data_mut(|d| d.insert_temp(parts_id, state));
    (result, event)
}

/// Pushes `edit` of field `key` of instance `id` as part `part`; `value` makes the field's value
/// of the control's. `typing` for a number field whose focus is a typing run's.
pub(super) fn push_param<T: Clone + Send + Sync + 'static>(
    edits: &mut Edits<LayerStyle, StyleOp>,
    edit: &EditResponse<T>,
    part: Id,
    (id, key): (EffectId, &str),
    typing: bool,
    value: impl Fn(&T) -> StyleValue,
) {
    let op = edit.event.as_ref().and_then(EditEvent::value).map(|v| StyleOp::Param { id, key: key.to_owned(), value: value(v) });
    let map = |v: T, base: &LayerStyle| with_param(base, id, key, value(&v));
    if typing {
        edits.typing(edit, op, apply_op, map);
    } else {
        edits.push_part(part, edit, op, apply_op, map);
    }
}

/// What an effect page reports in one pass.
#[derive(Default)]
pub(super) struct PageOutput {
    /// A colour swatch was clicked: key `"{instance id}/{parameter key}"` (`"fx2/color"`),
    /// titled [`LayerStyleLabels::color_picker_title`].
    pub pick_color: Option<ColorRequest>,
    /// A library request from a contour field.
    pub contour_action: Option<ContourAction>,
    /// A click on a contour field's thumbnail: the key `"{instance id}/{parameter key}"` and the
    /// contour, for a [`ContourEditorDialog`](super::ContourEditorDialog).
    pub edit_contour: Option<ContourRequest>,
    /// Make Default: the page's shown values (the defaults under the instance's).
    pub make_default: Option<StyleParams>,
    /// Reset to Default was pressed.
    pub reset_to_default: bool,
    /// A value the page shows from a default the instance lacks (a Pattern page with an empty
    /// value shows the first pattern): the parameter's key and the value. It is not an edit: the
    /// host writes it into its draft without an undo step.
    pub fill_default: Option<(String, StyleValue)>,
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
/// - `Pattern`: a `Select` over `patterns`; an empty value shows the first pattern and reports it as
///   `fill_default`, not as an edit.
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
/// The dialog names the page's AccessKit group. Every field's edit goes to `edits` in drawing
/// order, as an operation on the style.
pub(super) fn effect_page(ui: &mut Ui, input: &PageInput<'_>, edits: &mut Edits<LayerStyle, StyleOp>) -> PageOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1008-1153 (defaults under the values, one arm
    // per field kind), :1155-1171 (Make Default, Reset to Default), :996-1000 (the invalid line).
    let mut out = PageOutput::default();
    let id = ui.make_persistent_id(("effectPage", input.instance.id.0));
    let mut builder = UiBuilder::new().scope_id(id);
    if !input.enabled {
        builder = builder.disabled();
    }
    ui.scope_builder(builder, |ui| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            if let Some(field) = first_invalid(input.spec, &input.instance.params) {
                let name = field_label(input, field);
                let name = if name.is_empty() { field.key.to_string() } else { name };
                let tokens = Tokens::of(ui);
                let color = LayerStyleDialog::style(&tokens).invalid_color;
                let job = egui::text::LayoutJob::single_section(
                    craft_i18n::fmt(&input.labels.invalid_value, &[("field", name.as_str())]),
                    role_format(&tokens, TypeRole::Body, color),
                );
                ui.add(egui::Label::new(job).selectable(false));
                return;
            }
            let shown = input.instance.params.over(&input.spec.defaults);
            for field in &input.spec.fields {
                field_row(ui, input, field, &shown, edits, &mut out);
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

/// A caption in `text.muted` at the reading start and the control after it.
pub(super) fn caption_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    Row::new(Gap::Inline)
        .show(ui, |ui| {
            ui.add(Text::new(label).role(TypeRole::Label).content(Content::Muted));
            add(ui)
        })
        .inner
}

/// One `Select` over `options` (value, text, separator before); its response.
pub(super) fn pick<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    current: T,
    options: Vec<(T, String, bool)>,
    enabled: bool,
) -> EditResponse<T> {
    let mut value = current;
    let mut select = Select::new(id_salt, label, &mut value);
    for (option, text, separator) in options {
        if separator {
            select = select.separator();
        }
        select = select.option(option, text);
    }
    select.enabled(enabled).show(ui)
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

fn field_row(
    ui: &mut Ui,
    input: &PageInput<'_>,
    field: &EffectField,
    shown: &StyleParams,
    edits: &mut Edits<LayerStyle, StyleOp>,
    out: &mut PageOutput,
) {
    let (labels, enabled, key) = (input.labels, input.enabled, field.key.as_ref());
    let label = field_label(input, field);
    let salt = (input.instance.id.0, key);
    let target = (input.instance.id, key);
    let as_number = |v: &f64| StyleValue::Number(*v);
    let as_text = |v: &String| StyleValue::Text(v.clone());
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
            let row = row.show(ui);
            let whole = |v: &f64| StyleValue::Number(v.round());
            push_param(edits, &row.field, row.field.response.id, target, true, whole);
            push_param(edits, &row.slider, row.slider.response.id, target, false, whole);
        }
        FieldKind::Angle => {
            let value = shown.number(key).unwrap_or(0.0);
            let dial = AngleDial::new(salt, label.as_str(), value, input.numbers).range(-180.0..=180.0).enabled(enabled).show(ui);
            push_param(edits, &dial, dial.response.id, target, false, as_number);
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
            let response = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            push_param(edits, &response, response.response.id, target, false, as_text);
        }
        FieldKind::Choice(choices) => {
            let Some(first) = choices.first() else { return };
            let current = text_of(shown.get(key)).unwrap_or(&first.value);
            let options = choices
                .iter()
                .map(|c| (c.value.to_string(), labels.choice(&c.value).map_or_else(|| c.label.to_string(), str::to_owned), false))
                .collect();
            let response = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            push_param(edits, &response, response.response.id, target, false, as_text);
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
            let response = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            push_param(edits, &response, response.response.id, target, false, as_text);
        }
        FieldKind::Pattern => {
            let stored = text_of(shown.get(key)).filter(|t| !t.is_empty());
            let first = input.patterns.first();
            let current = stored.or(first.map(|p| p.key.as_str())).unwrap_or_default();
            let options = input.patterns.iter().map(|p| (p.key.clone(), p.label.clone(), false)).collect();
            let response = caption_row(ui, &label, |ui| pick(ui, salt, &label, current.to_owned(), options, enabled));
            if response.event.is_none() && stored.is_none() && enabled && first.is_some() {
                out.fill_default = Some((key.to_owned(), StyleValue::Text(current.to_owned())));
            }
            push_param(edits, &response, response.response.id, target, false, as_text);
        }
        FieldKind::Contour { anti_alias } => {
            let contour = match shown.get(key) {
                Some(StyleValue::Contour(contour)) => contour.clone(),
                _ => Contour::linear(),
            };
            let aa = anti_alias.as_deref().map(|k| shown.bool(k).unwrap_or(false));
            let mut field = ContourField::new(salt, label.as_str(), &contour)
                .anti_alias(aa)
                .user_contours(input.user_contours)
                .labels(input.contour_labels.clone())
                .enabled(enabled);
            if let Some(sampler) = input.sampler {
                field = field.sampler(sampler);
            }
            let response = field.show(ui);
            let edit = EditResponse::new(response.response.clone(), response.edit);
            push_param(edits, &edit, edit.response.id, target, false, |v: &Contour| StyleValue::Contour(v.clone()));
            if let Some(aa_key) = anti_alias.as_deref() {
                let set = response.anti_alias.map(|on| EditEvent::set(on, Modifiers::NONE));
                let edit = EditResponse::new(response.response.clone(), set);
                push_param(edits, &edit, edit.response.id.with("antiAlias"), (input.instance.id, aa_key), false, |v: &bool| StyleValue::Bool(*v));
            }
            match response.action {
                Some(ContourAction::Edit(contour)) => {
                    out.edit_contour = Some(ContourRequest { key: format!("{}/{key}", input.instance.id.key()), contour });
                }
                Some(action) => out.contour_action = Some(action),
                None => {}
            }
        }
        FieldKind::Check => {
            let mut on = shown.bool(key).unwrap_or(false);
            let response = Checkbox::bool(label.as_str(), &mut on).id_salt(salt).enabled(enabled).show(ui);
            push_param(edits, &response, response.response.id, target, false, |v: &bool| StyleValue::Bool(*v));
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
