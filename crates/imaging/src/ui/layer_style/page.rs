//! The effect page: one row per field of the effect's [`EffectSpec`], drawn by kind (task AD-9).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-9)")]

use egui::Ui;

use craft_ui::controls::BlendChoice;
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;

use super::contour::ContourAction;
use super::labels::{ContourFieldLabels, LayerStyleLabels};
use crate::layer_style::{Contour, EffectField, EffectInstance, EffectSpec, StyleParams, StyleValue};

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
pub(super) fn effect_page(ui: &mut Ui, input: &PageInput<'_>) -> PageOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1008-1153 (defaults under the values, one arm
    // per field kind), :1155-1171 (Make Default, Reset to Default), :996-1000 (the invalid line).
    todo!()
}

/// The first field of `spec` whose value in `params` has the wrong type: a slider or angle not a
/// number, a check not a bool, a colour not a colour, a contour not a contour, a choice, blend or
/// pattern not text. Missing keys are fine (the defaults show).
pub(super) fn first_invalid<'s>(spec: &'s EffectSpec, params: &StyleParams) -> Option<&'s EffectField> {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:624-646 `validate_params` (the JSON shape
    // check, here per field type).
    todo!()
}
