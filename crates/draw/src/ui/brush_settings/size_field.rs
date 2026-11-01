//! The brush Size row ([`BrushSizeField`]): a value field over a log slider.

use craft_ui::controls::{IconButton, NumberField, Slider, SliderScale};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Text};
use egui::{Align, AsIdSalt, IdSalt, Layout, Ui};

use super::labels::BrushSettingsLabels;

/// The brush Size row of Brush Tip Shape and of the Brush Preset picker's head: the label and a
/// [`NumberField`](craft_ui::controls::NumberField) (px, whole pixels, 1 to the maximum, the
/// field's default width) on one line, a [`Slider`](craft_ui::controls::Slider) with
/// [`SliderScale::Log`](craft_ui::controls::SliderScale::Log) under it, and with
/// [`restore`](Self::restore) Photoshop's "Restore original size" icon button (sampled tips)
/// before the field.
///
/// The field and the slider show one value: a slider drag rounds to whole pixels and stays one
/// gesture (`Begin`, `Update`, `Commit`) while the app writes the rounded value back; a typed
/// value is a `Set`; restore is a `Set` of the original size. Esc during a drag cancels.
///
/// AccessKit: the field is a `SpinButton` named by the label, the slider a `Slider` of the same
/// name, the restore button a `Button` "Restore original size".
#[must_use = "show the field with `show`"]
pub struct BrushSizeField {
    id_salt: IdSalt,
    label: String,
    diameter: f64,
    max: f64,
    restore: Option<f64>,
    labels: Option<BrushSettingsLabels>,
}

impl BrushSizeField {
    /// A Size row named `label` for `diameter` pixels, 1 to `max` (the app's engine limit).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, diameter: f64, max: f64) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), diameter, max, restore: None, labels: None }
    }

    /// Shows the "Restore original size" button, which sets `original` (clamped to 1 to the
    /// maximum). `None`: no button.
    pub fn restore(mut self, original: Option<f64>) -> Self {
        self.restore = original;
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the row and reports the new diameter.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:129-150 `size_row` (field 74 wide, restore button, log slider, 4 pt after)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:232-247 (the picker head's Size: field and log slider)
        let labels = self.labels.unwrap_or_else(|| BrushSettingsLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let max = if self.max.is_finite() { self.max.max(1.0) } else { 1.0 };
        let shown = if self.diameter.is_finite() { self.diameter.clamp(1.0, max) } else { 1.0 };
        let mut restored = None;
        let field = ui
            .horizontal(|ui| {
                ui.add(Text::new(self.label.as_str()).content(Content::Muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let field = NumberField::new((&self.id_salt, "field"), self.label.as_str(), shown, &numbers)
                        .range(1.0..=max)
                        .step(1.0)
                        .decimals(0..=0)
                        .snap_typed(true)
                        .unit(Unit::new("{n} px"))
                        .show(ui);
                    if let Some(original) = self.restore
                        && original.is_finite()
                        && ui.add(IconButton::new(Icon::Undo, labels.restore_size.as_str()).id_salt((&self.id_salt, "restore"))).clicked()
                    {
                        restored = Some(original.clamp(1.0, max));
                    }
                    field
                })
                .inner
            })
            .inner;
        let slider = Slider::new((&self.id_salt, "slider"), self.label.as_str(), shown, 1.0..=max)
            .step(1.0)
            .scale(SliderScale::Log)
            .enabled(max > 1.0 && ui.is_enabled())
            .show(ui);
        ui.add_space(ui.spacing().item_spacing.y);
        let modifiers = ui.input(|i| i.modifiers);
        let (submitted, escaped) = (field.submitted, field.escaped);
        let event = match restored {
            Some(original) => Some(EditEvent::set(original, modifiers)),
            None => slider.event.or(field.event),
        };
        let mut out = EditResponse::new(slider.response, event);
        out.submitted = submitted;
        out.escaped = escaped || slider.escaped;
        out
    }
}
